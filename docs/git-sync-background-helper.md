---
title: Git Background Sync Helper
status: exploring
tags: [sync, git, architecture, background]
date: 2026-06-10
---

# Git Background Sync Helper

## Purpose

Flynt needs a small helper layer between UI triggers/auto-sync timers and low-level `GitSync` operations.

The helper owns operation ordering, freshness requirements, terminal blockers, retry classification, and status reporting. Low-level Git functions should remain narrow primitives: inspect, fetch, commit, fast-forward, push, list refs.

Without this helper, every caller must remember the same safety sequence:

```text
observe → refresh stale upstream → preflight → commit only if safe → pull fast-forward → push → reindex/report
```

That is too easy to get wrong, especially in background auto-sync.

## Non-goals

The helper does not implement collaboration, semantic merge, conflict resolution, or arbitrary Git repair. It coordinates safe single-user branch sync.

It also does not own UI rendering. It emits structured events/results that Settings, toolbar, and logs can render.

## Module shape

Proposed store module:

```text
crates/flynt-store/src/sync/
  git.rs              # low-level git primitives
  vcs.rs              # adapter trait + Git/JJ-colocated implementations
  planner.rs          # pure-ish state → plan logic
  runner.rs           # executes plans and emits events
  auto.rs             # timer/task wrapper around runner
```

If we want the smallest first patch, `planner.rs`, `vcs.rs`, and `runner.rs` can start as private submodules and `auto.rs` can call only `BackgroundSyncRunner`.

## Core types

### `BackgroundSyncRunner`

```rust
pub struct BackgroundSyncRunner<V: SyncVcs> {
    vcs: V,
    config: GitSyncConfig,
    stale_after: std::time::Duration,
}
```

Responsibilities:

- run one bounded sync attempt;
- guarantee stale/unknown upstream is refreshed before commit/push;
- reconcile colocated jj state before normal Git sync when applicable;
- translate low-level VCS errors into `SyncOutcome`/`SyncBlocker`;
- emit phase updates;
- return terminal conflict/blocker outcomes without retry loops hiding them.

### `SyncVcs`

```rust
pub trait SyncVcs {
    fn flavor(&self) -> VcsFlavor;
    fn reconcile_local_vcs(&self, branch: &str) -> Result<VcsReconcileOutcome>;
    fn diagnostic(&self) -> Result<GitRepositoryState>;
    fn check_upstream(&self, tag_policy: TagFetchPolicy) -> Result<UpstreamFreshness>;
    fn list_refs(&self, tag_policy: TagFetchPolicy) -> Result<GitRefInventory>;
    fn auto_commit_filtered(&self, message: &str) -> Result<Option<String>>;
    fn pull_fast_forward(&self) -> Result<SyncOutcome>;
    fn push(&self) -> Result<SyncOutcome>;
    fn supports_auto_commit(&self) -> bool;
}
```

The runner depends on `SyncVcs`, not `GitSync` directly. This keeps Git operation ordering independent from the concrete VCS flavor and gives Flynt a safe seam for jj-colocated compatibility.

### `VcsFlavor`

```rust
pub enum VcsFlavor {
    GitOnly,
    JjColocated,
}
```

### `VcsReconcileOutcome`

```rust
pub enum VcsReconcileOutcome {
    NotNeeded,
    AlreadyAligned,
    FastForwardedBranch { branch: String, from: String, to: String },
    ExportedJjState,
    Blocked(Vec<SyncBlocker>),
}
```

`reconcile_local_vcs()` is local-only. It must not contact the remote. Its purpose is to align local VCS metadata before upstream freshness checks run.

### `SyncRequest`

```rust
pub struct SyncRequest {
    pub trigger: SyncTrigger,
    pub allow_commit: bool,
    pub allow_pull: bool,
    pub allow_push: bool,
    pub require_fresh_upstream: bool,
}
```

Default requests:

```rust
impl SyncRequest {
    pub fn manual_sync() -> Self;
    pub fn auto_sync_tick() -> Self;
    pub fn refresh_only() -> Self;
}
```

Semantics:

- `manual_sync`: refresh, commit eligible local changes only when safe, fast-forward if needed, push if ahead.
- `auto_sync_tick`: same safety as manual, but terminal blockers stop the loop instead of prompting inline.
- `refresh_only`: fetch/check upstream and update diagnostics, no worktree/index/branch mutation.

### `SyncEvent`

```rust
pub enum SyncEvent {
    PhaseChanged(SyncPhase),
    VcsReconciled(VcsReconcileOutcome),
    Diagnostic(SyncPreflight),
    Warning(SyncWarning),
    Blocked(Vec<SyncBlocker>),
    Completed(SyncOutcome),
}
```

The runner should accept a callback or channel sender:

```rust
pub type SyncEventSink = Arc<dyn Fn(SyncEvent) + Send + Sync>;
```

`auto.rs` can map events to `AutoSyncStatus`; UI can later consume richer event streams.

### `RetryClass`

```rust
pub enum RetryClass {
    Retryable,
    TerminalUntilOperatorAction,
}
```

Retryable:

- network unavailable;
- remote temporarily unreachable;
- credential helper transient failure if classified that way;
- lock contention if another Git operation is currently active and no merge/rebase state exists.

Terminal until operator action:

- detached HEAD;
- merge/rebase/cherry-pick/revert in progress;
- index conflicts;
- dirty files while upstream is out-of-date/diverged;
- divergence under fast-forward-only;
- conflict markers detected;
- unsafe generated/runtime files would be staged;
- missing branch/remote unless the operator is in setup flow.

## Runner algorithm

For `manual_sync` and `auto_sync_tick`:

```text
1. emit ReconcilingLocalVcs
2. reconcile_local_vcs(config.branch)
3. if reconciliation blocks → Blocked
4. emit Observing
5. diagnostic(local-only)
6. if repo has terminal local blockers → Blocked
7. if upstream unknown/stale and request.require_fresh_upstream:
   a. emit RefreshingUpstream
   b. check_upstream(fetch narrow branch refspec)
   c. if unreachable → Failed(retryable)
8. preflight with fresh relation
9. if blockers → Blocked
10. choose plan
11. if plan requires commit and `supports_auto_commit() == false` → Blocked
12. execute plan:
   - Noop → Complete(Noop)
   - PullFastForward → pull_ff → reindex → Complete
   - PushOnly → push → Complete
   - CommitThenPush → auto_commit_filtered → check_upstream again if commit changed HEAD → push → Complete
   - PullThenPush → pull_ff → reindex → push → Complete
   - Blocked → Blocked
13. after any Git mutation, run final diagnostic local-only
```

For `refresh_only`:

```text
1. emit RefreshingUpstream
2. check_upstream(fetch narrow branch refspec)
3. emit Completed(UpstreamChecked { freshness, relation })
```

## VCS adapter model

The helper adopts the relevant Omegon git-internal pattern: centralize repository mutation ownership behind a small adapter boundary, and treat jj-colocated repositories as a compatibility mode rather than a public sync backend.

### `GitVcsAdapter`

`GitVcsAdapter` wraps the existing `GitSync` implementation.

Responsibilities:

- expose existing diagnostics in the new model shape;
- implement `check_upstream()` as fetch-only;
- implement branch/tag inventory;
- implement filtered auto-commit;
- implement fast-forward-only pull and normal push.

### `JjColocatedVcsAdapter`

`JjColocatedVcsAdapter` is selected when `.jj/` exists beside the Git repository.

Flynt still presents the backend as Git. jj is an implementation compatibility concern for operators who already use jj-colocated repos.

Initial policy: jj-colocated repositories are safe for upstream freshness checks, branch/tag inventory, fast-forward pull, and push after local reconciliation. Flynt auto-commit is disabled for jj-colocated repos until there is an explicit jj commit/change contract. This avoids creating Git commits that bypass jj's working-copy/change model.

Local reconciliation algorithm:

```text
1. detect .jj
2. run `jj git export`
3. resolve the jj revision that should back the configured branch:
   - prefer `jj log -r <branch>` or the matching bookmark when present;
   - fall back to `@-` only when the working-copy parent is clearly the branch tip;
   - block if the working copy has uncommitted jj changes and Flynt would need to auto-commit them.
4. read configured git branch ref (`refs/heads/{branch}`)
5. if equal → AlreadyAligned
6. if git branch is ancestor of resolved jj branch revision → fast-forward branch ref to that revision
7. otherwise → Blocked(JjGitDiverged)
8. if Git HEAD is detached at the aligned commit, reattach to the configured branch when safe
```

This is adapted from Omegon's `sync-jj-to-git.sh`, but generalized away from hardcoded `main`.

The adapter must not:

- require jj for normal Git users;
- expose jj as a Flynt sync backend yet;
- use jj-lib for required runtime behavior;
- perform remote network operations during local reconciliation;
- abandon, rebase, squash, or otherwise rewrite jj changes;
- create ordinary Git commits behind jj's back;
- fast-forward when ancestry is not proven.

The first implementation can shell out to the jj CLI for reconciliation. That matches Omegon's stance: jj CLI is the stable mutation contract, while jj-lib is pre-1.0 and should be avoided unless a read-only query later provides clear value.

### JJ-specific blockers

Add blockers to the shared model:

```rust
pub enum SyncBlocker {
    // existing variants...
    JjUnavailable,
    JjGitDiverged { branch: String, git_head: String, jj_head: String },
    JjGitExportFailed { message: String },
    JjAutoCommitUnsupported,
}
```

These are terminal until operator action. Auto-sync must stop.

### JJ-specific phase

Add a phase:

```rust
pub enum SyncPhase {
    ReconcilingLocalVcs,
    // existing variants...
}
```

## Critical invariants

1. **Local VCS metadata reconciles before upstream checks.**
   If the repo is jj-colocated, export jj state and align the configured Git branch before checking upstream freshness. Never compare stale Git branch refs to the remote while jj has unexported/unaligned local commits.

2. **No mutation before upstream freshness when required.**
   If the request requires fresh upstream, no commit/push/fast-forward happens before `check_upstream()` succeeds.

3. **No commit on stale/unknown relation.**
   Auto-commit cannot create a local commit until the runner knows whether upstream is in sync, ahead, behind, or diverged.

4. **No commit when dirty + upstream out-of-date/diverged.**
   This avoids manufacturing divergence. The operator must decide whether to stash, manually commit, pull externally, or resolve.

5. **No push unless relation allows it.**
   Push is valid only when relation is `InSync` after a new commit or `Ahead` with a clean worktree. It is invalid for `OutOfDate`, `Diverged`, `Unknown`, or `MissingRemoteRef`.

6. **No background retries after terminal blockers.**
   Auto-sync stops on terminal blockers and waits for explicit Resume/Sync Now after operator action.

7. **Refresh-only is fetch-only.**
   It may update remote-tracking refs and local runtime diagnostics; it must not change the worktree, index, branch tip, or tags beyond the configured tag fetch policy.

8. **No ordinary Git auto-commit in jj-colocated repos until designed.**
   A jj-colocated repo's working copy is a jj change. Creating normal Git commits from Flynt would bypass jj state and can desynchronize the operator's VCS model. Initial jj compatibility supports reconciliation and remote sync only when the jj/git branch refs are aligned and the worktree does not require Flynt to create a commit.

## Planner rules

Inputs:

- dirty eligible files count;
- local Git blockers;
- `FreshnessStatus`;
- `UpstreamRelation`;
- request permissions;
- divergence policy.

Outputs:

- `SyncPlan`;
- blockers;
- warnings.

Rules:

| Condition | Plan |
|---|---|
| terminal local blocker | `Blocked` |
| freshness unknown/stale and fresh required | `RefreshUpstream` |
| dirty + `OutOfDate` | `Blocked` |
| dirty + `Diverged` | `Blocked` |
| dirty + `InSync` + commit/push allowed + auto-commit supported | `CommitThenPush` |
| dirty + `InSync` + auto-commit unsupported | `Blocked` |
| clean + `OutOfDate` + pull allowed | `PullFastForward` |
| clean + `Ahead` + push allowed | `PushOnly` |
| clean + `Diverged` | `Blocked` |
| clean + `InSync` | `Noop` |

`PullThenPush` is reserved for cases where local is ahead and behind only if `MergeCommit` becomes supported. Under `FastForwardOnly`, ahead+behind is `Diverged` and blocks.

## Auto-sync integration

`auto.rs` should stop owning Git operation ordering. It should own only:

- timer/backoff;
- cancellation;
- converting runner events into `AutoSyncStatus`;
- stopping on terminal blockers;
- invoking reindex after runner reports pulled/fast-forwarded files.

Current shape should move from:

```text
auto_commit → pull → push
```

to:

```text
BackgroundSyncRunner::run_once(SyncRequest::auto_sync_tick())
```

This prevents future regressions where a loop commits conflict markers or creates local commits before checking upstream.

## Manual Sync Now integration

The Settings/toolbar `Sync now` action should call the same runner:

```text
BackgroundSyncRunner::run_once(SyncRequest::manual_sync())
```

Manual sync differs from auto-sync only in UX response:

- auto-sync stops quietly with visible blocked status;
- manual sync returns a structured blocker list and recommended next action immediately.

## First implementation slice

Implement the helper in the smallest useful sequence:

1. Add planner types and pure planning tests.
2. Add `SyncVcs`, `GitVcsAdapter`, and local-only `JjColocatedVcsAdapter` reconciliation.
3. Add `GitSync::check_upstream()` as fetch-only.
4. Add `GitSync::list_refs()` for branch/tag inventory.
5. Add `BackgroundSyncRunner::run_once()` for reconcile/refresh/noop/block/commit-push/pull-ff/push-only.
6. Change `auto.rs` to call the runner instead of direct `auto_commit → pull → push`.
7. Add integration tests for:
   - stale → refresh before commit;
   - jj-colocated branch alignment fast-forwards only when ancestry is proven;
   - jj/git divergence blocks;
   - jj-colocated dirty work requiring commit blocks instead of creating a Git commit;
   - commit-then-push refreshes upstream relation after the commit and before push;
   - dirty + out-of-date blocks;
   - clean + out-of-date fast-forwards;
   - clean + ahead pushes;
   - diverged blocks and halts auto-sync.

## Open questions

1. Should manual sync offer a built-in stash/pull/pop flow for dirty + out-of-date, or should that remain external Git for now?
2. Should the upstream checker persist last freshness/relation immediately, or should the app-level runtime state own that write?
3. Should tag inventory use default auto-follow only, or should Settings have an explicit “fetch all tags” button?
4. How aggressively should Flynt reattach detached Git HEAD in a jj-colocated repo? Initial policy: only when HEAD already points at the configured branch target after reconciliation.
