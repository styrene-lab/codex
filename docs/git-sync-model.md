---
title: Git Sync Model
status: exploring
tags: [sync, git, architecture, contracts]
date: 2026-06-10
---

# Git Sync Model

## Decision frame

Flynt Git sync is **personal project continuity**, not collaboration.

The supported contract is single-user, multi-machine durability through a normal Git remote. Flynt may help move project files between the operator's machines, but it must not imply real-time collaboration, shared editing, or conflict-free concurrent mutation.

## Product contract

Flynt supports Git-backed personal sync when:

- one human operator owns the project;
- the project root is a Git working tree;
- the operator has configured a remote and branch;
- Flynt can prove the repository is in a safe state before mutating it;
- divergent histories are surfaced instead of guessed through implicit merges.

Flynt does **not** currently promise:

- multi-user collaboration;
- real-time sync;
- automatic semantic merge of boards, drawings, flows, tasks, or diagrams;
- conflict-free editing on two machines at once;
- recovery from arbitrary Git states without operator intervention.

Recommended public copy:

> Git sync keeps your plain-file Flynt project durable across your own machines using a remote Git repository. It is safest when one machine edits at a time. If histories diverge or conflicts appear, Flynt stops and asks you to resolve them rather than guessing.

## Sync boundary

### Portable project state

These files are eligible for Git sync:

- Markdown documents and task files.
- Project documentation under normal user-authored folders.
- `.flynt/config.toml` when it describes portable project configuration.
- `.flynt/lenses/*.toml`.
- `.board` design board sources.
- `.excalidraw` drawing sources when intentionally edited.
- `.drawing.json` semantic drawing sidecars.
- `.flow` node-flow graphs.
- `.d2` diagram sources.
- User-authored templates, if Flynt distinguishes them from generated defaults.

### Local/runtime/generated state

These files must not be committed by Flynt auto-sync:

- `.flynt/local/`
- `.flynt/runtime/forge-sync.db`
- `.flynt/runtime/operator-settings.json`
- `.flynt/runtime/omegon.toml`
- `.flynt/local/registry/project-registry.snapshot.json`
- `.omegon/`
- `ai/` runtime/agent state, unless a future explicit export path is added.
- `.DS_Store`
- temporary files and partial downloads.
- generated exports such as `drawings/*.svg`.

Open decision:

- `.flynt/templates/`: portable only if user-authored; generated default templates should either live outside the project or be ignored until customized.

## Safety model

Git sync has three phases:

1. **Observe** — inspect repository state without mutation.
2. **Plan** — decide whether a sync operation is safe and what would happen.
3. **Mutate** — commit, fetch, fast-forward, and push only after preflight passes.

Auto-sync and manual `Sync now` must use the same preflight and operation contracts.

## Core data models

### `SyncMode`

```rust
pub enum SyncMode {
    Disabled,
    Manual,
    Auto { interval_secs: u64 },
}
```

Meaning:

- `Disabled`: no background Git behavior.
- `Manual`: operator can run `Sync now`; Flynt may still show diagnostics.
- `Auto`: Flynt periodically attempts the safe sync plan; terminal blockers stop the loop.

### `GitSyncConfig`

```rust
pub struct GitSyncConfig {
    pub remote: String,
    pub branch: String,
    pub mode: SyncMode,
    pub divergence_policy: DivergencePolicy,
    pub upstream_check: UpstreamCheckPolicy,
    pub tag_fetch_policy: TagFetchPolicy,
}
```

For 0.12/0.13 hardening, `divergence_policy` should default to fast-forward-only.

### `TagFetchPolicy`

```rust
pub enum TagFetchPolicy {
    DoNotFetchTags,
    AutoFollow,
    AllTags,
}
```

Default to `AutoFollow`, matching normal Git fetch behavior. `AllTags` is useful for explicit history/release inspection but should not be enabled silently for every background freshness check because large repositories can have many tags.

### `UpstreamCheckPolicy`

```rust
pub enum UpstreamCheckPolicy {
    ManualOnly,
    OnSync,
    Periodic { interval_secs: u64 },
}
```

Meaning:

- `ManualOnly`: only refresh upstream state when the operator presses Refresh/Sync.
- `OnSync`: fetch before a manual or auto sync attempt.
- `Periodic`: periodically fetch remote refs to distinguish fresh/stale/out-of-date state without committing or pushing.

The upstream checker is observe-only: it may fetch remote refs, but it must not merge, checkout, commit, or push.

### `DivergencePolicy`

```rust
pub enum DivergencePolicy {
    FastForwardOnly,
    MergeCommit,
}
```

Initial contract:

- `FastForwardOnly`: supported.
- `MergeCommit`: future/experimental until merge commit tests and conflict UX are complete.

### `GitRepositoryState`

```rust
pub struct GitRepositoryState {
    pub head: Option<String>,
    pub branch: Option<String>,
    pub detached_head: bool,
    pub repository_state: GitOperationState,
    pub has_index_conflicts: bool,
    pub conflicted_paths: Vec<String>,
    pub dirty_paths: Vec<String>,
    pub ahead: Option<usize>,
    pub behind: Option<usize>,
    pub remote_ref_available: bool,
    pub remote_reachable: Option<bool>,
    pub upstream: UpstreamFreshness,
    pub refs: GitRefInventory,
}
```

This is observation-only. Local inspection must not fetch unless the caller explicitly requests a remote refresh. A fresh upstream check may run `fetch` to update remote-tracking refs, but it still must not mutate the worktree, index, local branch tip, or tags.

### `UpstreamFreshness`

```rust
pub struct UpstreamFreshness {
    pub freshness: FreshnessStatus,
    pub relation: UpstreamRelation,
    pub last_checked_at: Option<chrono::DateTime<chrono::Utc>>,
    pub local_head: Option<String>,
    pub upstream_head: Option<String>,
    pub ahead: Option<usize>,
    pub behind: Option<usize>,
    pub stale_after_secs: u64,
}
```

Freshness and graph relation are separate dimensions. A relation can be known-but-stale: for example, Flynt may have last observed `InSync`, but if that check is older than the threshold the UI must show that the cached result needs refresh.

### `FreshnessStatus`

```rust
pub enum FreshnessStatus {
    Unknown,
    Fresh,
    Stale,
    Unreachable { message: String },
}
```

### `UpstreamRelation`

```rust
pub enum UpstreamRelation {
    Unknown,
    InSync,
    OutOfDate { behind: usize },
    Ahead { ahead: usize },
    Diverged { ahead: usize, behind: usize },
    MissingRemoteRef,
}
```

Meanings:

- `Unknown`: Flynt has not checked upstream yet.
- `Fresh`: Flynt checked upstream within the configured threshold.
- `Stale`: the last successful upstream check is older than the configured threshold.
- `Unreachable`: fetch/remote inspection failed due to network, auth, or remote errors.
- `InSync`: local HEAD equals the configured upstream ref as of the last successful check.
- `OutOfDate`: upstream has commits not present locally; fast-forward may be possible.
- `Ahead`: local has commits not present upstream; push may be needed.
- `Diverged`: both sides have unique commits; fast-forward-only sync must block.
- `MissingRemoteRef`: the configured remote branch/tag ref does not exist.

### `GitRefInventory`

```rust
pub struct GitRefInventory {
    pub local_branches: Vec<GitRefSummary>,
    pub remote_branches: Vec<GitRefSummary>,
    pub tags: Vec<GitRefSummary>,
    pub current: Option<GitRefSummary>,
    pub upstream: Option<GitRefSummary>,
}

pub struct GitRefSummary {
    pub name: String,
    pub full_ref: String,
    pub target: Option<String>,
    pub kind: GitRefKind,
}

pub enum GitRefKind {
    LocalBranch,
    RemoteBranch,
    Tag,
}
```

Branch/tag support starts as discovery and selection metadata. Sync mutation is branch-only until tag checkout/pinning semantics are explicitly designed.

### `SyncTarget`

```rust
pub enum SyncTarget {
    Branch { remote: String, branch: String },
}
```

The sync target is intentionally branch-only. Tags are immutable-ish labels for inspection, history, and release navigation; they are not sync destinations. If the operator opens a tag, Flynt should treat it as detached/read-only unless they explicitly create or switch to a branch.

### `GitOperationState`

```rust
pub enum GitOperationState {
    Clean,
    Merge,
    Rebase,
    CherryPick,
    Revert,
    Bisect,
    ApplyMailbox,
    Unknown(String),
}
```

Any non-clean state is a sync blocker.

### `SyncBlocker`

```rust
pub enum SyncBlocker {
    NotGitRepository,
    MissingHead,
    DetachedHead,
    OperationInProgress { state: GitOperationState },
    IndexConflicts { paths: Vec<String> },
    MissingRemote { remote: String },
    MissingLocalBranch { branch: String },
    MissingRemoteRef { remote: String, branch: String },
    OutOfDate { behind: usize },
    Diverged { local: String, remote: String, ahead: usize, behind: usize },
    DirtyConflictMarkers { paths: Vec<String> },
    UnsafePortableBoundary { paths: Vec<String> },
    AuthenticationRequired,
    RemoteUnavailable { message: String },
}
```

Blockers are terminal for one sync attempt. Auto-sync may retry transient remote/auth failures, but must halt for conflicts/divergence until operator intervention.

### `SyncWarning`

```rust
pub enum SyncWarning {
    DirtyWorkingTree { count: usize },
    UntrackedPortableFiles { count: usize },
    AutoSyncStopped,
    GeneratedFilesIgnored { count: usize },
    SingleWriterRecommended,
}
```

Warnings do not prevent sync; blockers do.

### `SyncPreflight`

```rust
pub struct SyncPreflight {
    pub safe_to_sync: bool,
    pub state: GitRepositoryState,
    pub blockers: Vec<SyncBlocker>,
    pub warnings: Vec<SyncWarning>,
    pub next_action: Option<SyncPlan>,
}
```

Contract:

- If `safe_to_sync == false`, no mutation may occur.
- `next_action` is advisory and must be derived from the same observed state.
- Manual and auto-sync both call this before mutation.

### `SyncPlan`

```rust
pub enum SyncPlan {
    RefreshUpstream,
    CommitOnly,
    CommitThenPush,
    PullFastForward,
    PushOnly,
    PullThenPush,
    Blocked,
    Noop,
}
```

`RefreshUpstream` is a fetch-only observation plan. It updates remote-tracking refs and upstream freshness diagnostics, but it must not alter the worktree, index, local branch tip, or tags.

Planning rules for the fast-forward-only policy:

- dirty + `InSync`/`Fresh` → `CommitThenPush`;
- clean + `OutOfDate` → `PullFastForward`;
- clean + `Ahead` → `PushOnly`;
- clean + `Diverged` → `Blocked`;
- dirty + `OutOfDate`/`Diverged` → `Blocked` until the operator resolves ordering manually;
- stale/unknown upstream → `RefreshUpstream` before choosing a mutating plan.

For fast-forward-only sync, no plan may include merge unless `DivergencePolicy::MergeCommit` is explicitly enabled.

### `SyncRun`

```rust
pub struct SyncRun {
    pub id: uuid::Uuid,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub finished_at: Option<chrono::DateTime<chrono::Utc>>,
    pub trigger: SyncTrigger,
    pub phase: SyncPhase,
    pub preflight: SyncPreflight,
    pub outcome: Option<SyncOutcome>,
}
```

This is the diagnostic record shown in Settings/sidebar and optionally persisted to local runtime state.

### `SyncTrigger`

```rust
pub enum SyncTrigger {
    Manual,
    Auto,
    Startup,
    BeforeClose,
}
```

### `SyncPhase`

```rust
pub enum SyncPhase {
    Observing,
    RefreshingUpstream,
    Preflight,
    AutoCommit,
    Fetch,
    FastForward,
    Push,
    Reindex,
    Complete,
    Blocked,
    Failed,
}
```

### `SyncOutcome`

```rust
pub enum SyncOutcome {
    Noop,
    UpstreamChecked { freshness: FreshnessStatus, relation: UpstreamRelation },
    Committed { commit: String, files: usize },
    FastForwarded { from: String, to: String },
    Pushed { commit: String },
    Synced { committed: bool, pulled: bool, pushed: bool },
    Blocked { blockers: Vec<SyncBlocker> },
    Failed { message: String, retryable: bool },
}
```

## Operation contracts

### Diagnostic contract

`diagnostic()` is read-only by default.

It may report:

- backend;
- remote;
- branch;
- available local branches, remote branches, and tags;
- current ref and configured upstream ref;
- head;
- dirty file count/list;
- ahead/behind relative to the last fetched remote-tracking ref;
- upstream freshness: fresh/stale/unknown/unreachable;
- upstream relation: in-sync/out-of-date/ahead/diverged/missing-ref;
- detached HEAD;
- repository operation state;
- index conflicts;
- blockers.

It must not mutate the repository unless called with an explicit refresh/fetch mode.

### Upstream checker contract

`check_upstream()` is the only diagnostic operation allowed to contact the remote.

It may:

- fetch the configured branch's remote-tracking ref using a narrow branch refspec;
- optionally fetch tags according to `TagFetchPolicy` when branch/tag inventory is requested;
- update `last_checked_at` in local runtime state;
- compute fresh/stale/out-of-date/ahead/diverged from local HEAD and the fetched upstream ref.

It must not:

- commit;
- merge;
- checkout;
- fast-forward;
- push;
- create or delete tags;
- change the active branch.

Freshness thresholds:

- `Fresh`: the last upstream check completed within the configured threshold.
- `Stale`: the last successful check is older than the threshold, regardless of the cached relation.
- `Unreachable`: the attempted check failed; retain the previous successful relation separately if available.

Relation states:

- `InSync`: the last successful check found local HEAD equal to upstream.
- `OutOfDate`: the last successful check found the local branch behind upstream.
- `Ahead`: the last successful check found local commits not pushed upstream.
- `Diverged`: the last successful check found both local and upstream-only commits.

The UI should distinguish **stale** from **out-of-date**. Stale means Flynt does not know enough yet; out-of-date means Flynt checked and found upstream commits.

### Preflight contract

`preflight()` returns blockers before any mutation.

It blocks when:

- the repository is not clean from Git's operation-state perspective;
- HEAD is detached;
- index conflicts exist;
- remote or branch is missing;
- remote ref is missing and first-push mode is not explicit;
- histories diverged under `FastForwardOnly`;
- upstream relation is stale or unknown before a mutating operation and the configured policy requires a fresh check;
- upstream relation is `OutOfDate` or `Diverged` while the worktree has local dirty files;
- upstream relation is `OutOfDate` and the operation would push before fast-forwarding;
- conflict markers are detected in files that would be auto-committed;
- generated/runtime files would be staged due to missing ignore policy.

### Auto-commit contract

`auto_commit()` may stage only sync-eligible paths.

It must:

- respect `.gitignore`;
- apply Flynt's own portable-boundary filter before staging;
- run only after preflight has a fresh enough upstream relation;
- refuse to commit when preflight blockers exist;
- refuse to commit when local files are dirty and upstream is known to be `OutOfDate` or `Diverged`;
- refuse to commit conflict markers;
- create no commit when the resulting tree equals HEAD;
- use a Flynt-specific commit message such as `[flynt] auto-sync`.

It must not:

- stage all files blindly without a boundary check;
- commit generated exports;
- commit local runtime databases or session state;
- create a local commit on top of a stale/unknown upstream relation;
- run after a conflict status without explicit operator reset.

### Pull contract

For the initial supported policy, `pull()` is fast-forward-only.

It must:

- fetch the configured remote/branch;
- return no-op when up-to-date;
- fast-forward the local branch when safe;
- force checkout only as part of a proven fast-forward;
- reindex after files change.

It must not:

- perform an implicit merge;
- leave staged merge results for later auto-commit;
- call `cleanup_state()` to hide an incomplete merge;
- continue auto-sync after conflicts/divergence.

### Push contract

`push()` may run only after preflight passes.

It must:

- push the configured local branch to the configured remote branch;
- fail visibly on non-fast-forward remote rejection;
- classify auth/network failures as retryable or non-retryable.

It must not:

- force push;
- create remote branches unless first-push mode is explicit;
- push after a conflict/divergence blocker.

### Auto-sync loop contract

Auto-sync is a convenience loop around the same manual sync operation.

Required behavior:

```text
observe → preflight → maybe auto_commit → fetch/pull fast-forward → push → reindex → idle
```

Conflict/divergence behavior:

```text
Conflict/Diverged → status Blocked/Conflict → stop loop until explicit operator resume
```

Auto-sync may retry:

- temporary network failure;
- remote unavailable;
- credential helper transient failure.

Auto-sync must halt on:

- merge/rebase/cherry-pick in progress;
- detached HEAD;
- index conflicts;
- divergence under fast-forward-only policy;
- detected conflict markers;
- unsafe generated/runtime files about to be staged.

## UI contracts

Settings and sync status surfaces should show:

- backend: Git;
- remote and branch;
- available branch/tag inventory;
- mode: disabled/manual/auto;
- last upstream check time;
- upstream freshness: fresh/stale/unknown/unreachable;
- upstream relation: in-sync/out-of-date/ahead/diverged/missing-ref;
- last sync time;
- current phase;
- dirty count;
- ahead/behind counts;
- remote availability;
- blocked/conflict status;
- last error;
- clear next action.

Operator-facing states:

| State | Meaning | Operator action |
|---|---|---|
| Idle | Clean and synced relative to last observation | None |
| Fresh + In sync | Upstream was recently checked and matches local HEAD | None |
| Stale | Upstream has not been checked recently enough | Refresh upstream |
| Out of date | Upstream has newer commits and fast-forward may be possible | Sync now |
| Local changes | Files need commit/push | Sync now |
| Ahead | Local commits have not been pushed | Sync now |
| Diverged | Local and upstream both have unique commits | Resolve manually |
| Behind | Remote has newer commit and fast-forward is possible | Sync now |
| Syncing | Flynt is mutating Git state | Wait |
| Blocked | Preflight found a safe-stop condition | Resolve listed blocker |
| Conflict | Git conflict/divergence needs manual resolution | Resolve in Git/editor, then Resume sync |
| Offline | Remote unreachable | Retry later |
| Auth required | Credential helper/token missing | Configure credentials |

## Persistence model

Portable config belongs in `.flynt/config.toml`:

```toml
[sync.git]
remote = "origin"
branch = "main"
mode = "manual"
divergence_policy = "fast-forward-only"
upstream_check = "on-sync"
tag_fetch_policy = "auto-follow"
stale_after_secs = 300
```

Branch/tag selection should remain explicit project configuration. Tags may be listed and opened for inspection/history, but normal sync targets a branch, not a tag.

Local runtime state belongs outside the sync boundary, under the per-project userspace runtime root:

```toml
[last_sync]
status = "idle"
last_success_at = "2026-06-10T00:00:00Z"
last_error = ""
last_head = "..."

[upstream]
last_checked_at = "2026-06-10T00:00:00Z"
freshness = "fresh"
relation = "in-sync"
local_head = "..."
upstream_head = "..."
```

Do not write last-sync runtime state into the vault unless it is explicitly designed as portable audit history.

## Validation requirements

Minimum tests before broad enablement:

1. **Upstream checker freshness**
   - start with local and upstream equal;
   - run `check_upstream()`;
   - expect `freshness = Fresh` and `relation = InSync` with `ahead = 0`, `behind = 0`;
   - advance time or configure a short threshold;
   - expect `freshness = Stale` while retaining the cached `relation = InSync` before the next remote refresh.

2. **Upstream checker out-of-date**
   - second clone pushes a new document;
   - first clone runs `check_upstream()`;
   - expect `freshness = Fresh` and `relation = OutOfDate { behind: 1 }`;
   - no worktree, index, or local branch mutation occurs.

3. **Branch/tag inventory**
   - create multiple local branches, remote branches, and tags;
   - diagnostic lists them with names, full refs, targets, and kind;
   - current branch and configured upstream are identified.

4. **Dirty local files with upstream out-of-date**
   - local has uncommitted markdown edits;
   - second clone pushes a new document;
   - first clone runs preflight after upstream refresh;
   - expect a blocker rather than auto-commit;
   - no local commit is created before the operator chooses how to order pull/rebase/stash/manual resolution.

5. **Fast-forward pull**
   - second clone pushes a new document;
   - first clone pulls;
   - new document appears;
   - worktree is clean.

6. **Divergent non-conflicting histories**
   - local commits `A.md`;
   - remote commits `B.md`;
   - sync under fast-forward-only returns blocked/diverged;
   - no merge state remains;
   - no push occurs.

7. **Divergent conflicting histories**
   - local and remote edit same file;
   - sync returns conflict/diverged;
   - auto-sync halts;
   - no conflict markers are auto-committed.

8. **Generated/local ignore protection**
   - dirty `.flynt/local`, `.omegon`, `ai`, `drawings/*.svg`, `.DS_Store` plus a normal markdown file;
   - auto-commit commits only eligible project files.

9. **Open-without-edit cleanliness**
   - open a clean project in Flynt;
   - configure sync;
   - close without user edits;
   - only intentional portable config may be dirty.

10. **Drawing view does not dirty source**
   - open an Excalidraw wrapper;
   - wait;
   - no `.excalidraw` or generated `.svg` change occurs without intentional edit/export.

## Implementation sequence

1. Add explicit `SyncPreflight`, `SyncBlocker`, `SyncWarning`, `SyncPlan`, `SyncRun`, `FreshnessStatus`, `UpstreamRelation`, and `GitRefInventory` models.
2. Add an observe-only upstream checker that can fetch remote refs and classify freshness and relation separately.
3. Add branch/tag inventory APIs for Settings and future checkout/history UX.
4. Change manual sync and auto-sync to call a shared `BackgroundSyncRunner` helper before mutation, and to refresh stale/unknown upstream state before any commit or push.
5. Keep Git pull fast-forward-only for the release path.
6. Make conflict/divergence terminal for auto-sync until explicit operator resume.
7. Replace blind `index.add_all(["*"])` with a portable-boundary staging filter.
8. Harden generated/runtime ignore policy for default Flynt projects.
9. Add validation tests for upstream freshness, out-of-date detection, branch/tag inventory, fast-forward, divergence, conflicts, and ignore protection.
10. Expand Settings/sidebar diagnostics around the new model.

## Design decision

Adopt **FastForwardOnly Git sync** as the supported model for 0.12/0.13 hardening.

Rationale:

- It matches the single-writer-by-convention product contract.
- It avoids unsafe implicit merges for structured artifacts.
- It gives honest failure modes.
- It keeps the implementation small enough to validate.

Future work can add artifact-aware merge support behind a separate `MergeCommit`/experimental policy once there are tests and UI for conflict resolution.
