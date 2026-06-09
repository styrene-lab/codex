---
title: Sync Gap Analysis and Release Action Plan for 0.12.0
status: exploring
tags: [sync, release, icloud, git, risk]
date: 2026-06-07
---

# Sync Gap Analysis and Release Action Plan for 0.12.0

## Executive decision

For 0.12.0, treat sync as two separate delivery tracks:

1. **iCloud folder replication** — target first. This is an internal two-Mac validation effort. Flynt does not merge; iCloud replicates files. Flynt must prove it can safely operate inside an iCloud-backed vault without corrupting project state or confusing the operator.
2. **Git sync** — target second. This needs stronger correctness work before broad enablement because Flynt currently owns commit/pull/push/merge behavior.

Do **not** collapse these into one “sync works” milestone. They have different failure modes and different ownership boundaries.

---

# Track A — iCloud-first internal validation

## Product stance

Ship wording should be:

> iCloud support stores the Flynt project inside iCloud Drive and relies on iCloud for file replication. Flynt indexes and edits the replicated folder; conflict resolution remains iCloud/operator-owned.

Do not call this conflict-safe sync.

## Goal

Prove that two Macs can use the same Flynt project through iCloud Drive with acceptable behavior for normal document/task/design/lens workflows.

## Scope for 0.12.0

In scope:

- project creation/migration into iCloud Drive
- project open on Mac A and Mac B
- file replication detected by Flynt
- reindex after remote changes arrive
- simple edits on one Mac appear on the other
- operator-visible warning/copy for iCloud conflict limitations

Out of scope for 0.12.0:

- automatic semantic conflict merge
- resolving iCloud conflict copies
- multi-user concurrent editing guarantees
- cloud status API integration beyond local filesystem observation

## Current implementation touched

- `crates/flynt-store/src/sync/icloud.rs`
- `crates/flynt-store/src/sync/cloud.rs`
- `crates/flynt-app/src/sync_prereq.rs`
- `crates/flynt-store/src/migrate.rs`
- `crates/flynt-store/src/watcher.rs`
- `crates/flynt-app/src/bootstrap.rs`

## Concrete action plan

### A1. Confirm iCloud prerequisite and path detection

**Implementation check**

- Verify `sync_prereq::evaluate_icloud(project_root)` blocks non-macOS.
- Verify it only reports available when `project_root` is inside the canonical iCloud Drive path.
- Verify Settings copy says “iCloud Drive folder replication,” not “merge-safe sync.”

**Test**

Add/confirm unit tests for:

- outside iCloud path → blocked/warning
- inside iCloud path → available
- missing HOME/iCloud path → blocked

**Acceptance**

Settings cannot silently enable iCloud for a non-iCloud folder.

### A2. Validate migration/create path

**Implementation check**

- `create_icloud_project(project_name)` refuses existing project path.
- migration copies expected project files and preserves local-only ignore policy.
- migration does not copy obvious runtime caches that should remain machine-local.

**Manual test**

On Mac A:

1. create project outside iCloud
2. migrate to iCloud
3. open migrated project
4. verify documents/tasks/design boards/lenses still load

**Acceptance**

No missing project metadata, no duplicate root, no runtime-local cache promoted unintentionally.

### A3. File replication smoke test between two Macs

**Manual two-Mac script**

Mac A:

1. Open Flynt project in iCloud.
2. Create note `Sync Smoke A.md` with frontmatter/title.
3. Create/edit one task.
4. Create one lens under `.flynt/lenses/`.
5. Wait for iCloud upload completion in Finder.

Mac B:

1. Open same iCloud project.
2. Wait for iCloud download completion.
3. Press Refresh/reindex if needed.
4. Verify note appears.
5. Verify task appears.
6. Verify lens appears in Lenses tab and executes.

Then reverse B → A.

**Acceptance**

Both directions work without restarting Flynt, or if restart/reindex is required, document the requirement and expose a UI refresh path.

### A4. Watcher/reindex behavior after remote file arrival

**Risk**

Remote iCloud changes may arrive as filesystem events that differ from normal local saves. The watcher may miss batched/downloaded/renamed files.

**Implementation check**

- `ProjectWatcher` canonicalizes path and watches correct root.
- generated/cache directories are ignored or do not thrash the index.

**Manual test**

Mac B creates note while Mac A Flynt is already open. On Mac A, verify whether the note appears without app restart.

**Acceptance**

One of:

- automatic watcher reindex works reliably, or
- the UI provides a clear manual Refresh/Reindex action for iCloud projects.

### A5. Offline/placeholder files

**Risk**

iCloud may leave placeholder files or partially downloaded files. `ensure_downloaded()` exists, but must be validated.

**Manual test**

On Mac B:

1. Use Finder “Remove Download” on a subset of project files if available.
2. Open Flynt.
3. Verify Flynt either downloads them or reports a clear error.

**Acceptance**

No silent partial index where missing files look deleted unless they are truly deleted.

### A6. Conflict copy behavior

**Risk**

iCloud conflict copies are provider-generated files. Flynt should not pretend to resolve them.

**Manual test**

1. Disconnect Mac B from network.
2. Edit same note on Mac A and Mac B.
3. Reconnect Mac B.
4. Observe iCloud conflict/duplicate behavior.
5. Verify Flynt indexes conflict copies as files or surfaces them clearly.

**Acceptance**

Flynt does not corrupt either version. Operator can identify conflict copy. Release notes say conflict resolution is manual.

### A7. 0.12.0 iCloud go/no-go

Go if:

- create/migrate/open works
- A↔B note/task/lens replication works
- remote changes are indexed or manually refreshable
- conflict copies do not corrupt original data
- Settings copy is honest

No-go if:

- remote changes are silently ignored until restart with no clear refresh path
- placeholder/offline files look like deletions
- conflict copies cause parser/index crashes
- migration copies unsafe local runtime state

---

# Track B — Git sync hardening

## Product stance

For 0.12.0, Git sync should be guarded/beta until divergence and conflict behavior is proven.

Suggested copy:

> Git sync beta commits local project changes and synchronizes with a configured remote. Conflict and divergent-history handling is guarded; unresolved conflicts require operator action.

## Current implementation touched

- `crates/flynt-store/src/sync/git.rs`
- `crates/flynt-store/src/sync/auto.rs`
- `crates/flynt-store/tests/git_sync.rs`
- `crates/flynt-app/src/components/toolbar.rs`
- `crates/flynt-app/src/views/settings.rs`

## Highest-risk current code

### `GitSync::pull()` non-fast-forward path

Current code attempts `repo.merge`, detects conflicts, then calls `repo.cleanup_state()` and returns without clearly writing a merge commit for non-conflicting merges.

Risk: merged files may be staged/dirty but not committed; later `auto_commit()` may commit merge results under `[flynt] auto-sync`, obscuring merge semantics.

### `start_auto_sync()` conflict loop

Current auto-sync continues after conflict with backoff. Next loop starts with `auto_commit()`, which could commit conflict markers if the worktree remains dirty.

Risk: conflict markers get committed/pushed.

## Concrete action plan

### B1. Add Git preflight model

Create a preflight function before mutation:

```rust
pub struct SyncPreflight {
    pub safe_to_sync: bool,
    pub blockers: Vec<SyncBlocker>,
    pub warnings: Vec<String>,
}
```

Blockers should include:

- not a git repo
- no HEAD and no initial commit policy
- detached HEAD
- merge/rebase/cherry-pick in progress
- index conflicts
- missing remote
- missing local branch
- remote ref missing unless first-push mode is explicit

**Acceptance**

`Sync now` and auto-sync do not mutate when blockers exist.

### B2. Make conflict state terminal for auto-sync

Change `start_auto_sync()` so `AutoSyncStatus::Conflict` halts sync attempts until explicit operator reset/resolution.

Current unsafe behavior:

```text
Conflict → consecutive_failures += 1 → continue → next loop auto_commit
```

Required behavior:

```text
Conflict → status Conflict → stop loop or wait on explicit resume signal
```

**Acceptance**

A conflict can never be followed by auto-commit/push without operator intervention.

### B3. Decide divergence policy for 0.12.0

Pick one:

#### Option 1 — Fast-forward-only for release

If `merge_analysis` is non-fast-forward and not up-to-date:

```text
return Diverged/Conflict status; do not merge
```

Pros: safest and simple.
Cons: operator must resolve by external Git or future UI.

#### Option 2 — Implement real merge commit

After non-conflicting merge:

- write index tree
- create merge commit with both parents
- checkout/cleanup state
- verify worktree clean

Pros: better UX.
Cons: more risk before release.

**Recommendation for 0.12.0:** Option 1 unless we have time for full merge tests.

### B4. Add P0 Git tests

Add tests in `crates/flynt-store/tests/git_sync.rs`.

#### Test 1: Fast-forward pull

Setup:

- bare remote
- local clone
- second clone pushes new doc
- local `pull()`

Assert:

- new doc exists
- status clean
- diagnostic behind becomes 0 after fetch/pull

#### Test 2: Divergent non-conflicting histories

Setup:

- local commits A.md
- remote commits B.md
- local sync

If fast-forward-only:

- expect divergence/blocker
- no worktree mutation
- no push

If merge enabled:

- expect both files present
- merge commit exists or local branch ahead coherently
- status clean

#### Test 3: Divergent conflicting histories

Setup:

- local edits same file
- remote edits same file
- sync/pull

Assert:

- conflict status returned
- no push
- auto-sync does not auto-commit conflict markers

#### Test 4: Generated/local ignore protection

Setup dirty files:

- `.flynt/local/foo`
- `.omegon/runtime/foo`
- `.DS_Store`
- `normal.md`

Run `auto_commit()`.

Assert:

- `normal.md` committed
- local/generated files not committed

### B5. Audit `.gitignore` generation

Verify `Project::open` creates/maintains `.gitignore` with at least:

```text
.flynt/local/
.omegon/runtime/
.omegon/codescan.db*
.omegon/*.lock
.DS_Store
```

Decide whether `.omegon/audit-log.jsonl`, `.omegon/agent-journal.md`, and `.flynt/registry/` are portable or local.

**Acceptance**

Auto-commit cannot commit machine-local state in a default Flynt project.

### B6. Improve sync diagnostics UI

Before enabling broad Git sync, toolbar/settings should show:

- backend
- remote/branch
- dirty count
- ahead/behind
- remote reachable
- conflict/merge state
- auto-sync running/stopped
- last sync result/error

**Acceptance**

Operator can tell whether sync is safe, blocked, or degraded before pressing Sync Now.

---

# Release sequencing

## Phase 1 — iCloud internal validation

1. Run A1–A7 on two Macs.
2. Patch copy/refresh/indexing issues found.
3. If acceptable, enable iCloud-backed projects in 0.12.0 as folder replication.

## Phase 2 — Git safety hardening

1. Implement B1 preflight.
2. Implement B2 conflict halt.
3. Choose and implement B3 divergence policy.
4. Add B4 tests.
5. Audit B5 ignores.
6. Improve B6 diagnostics.

## Phase 3 — Git internal validation

Run two-machine or two-clone tests:

- clean push/pull
- fast-forward pull
- divergent non-conflicting edits
- conflicting edits
- auth failure
- remote unavailable
- large binary/design/lens changes

## Phase 4 — Release decision

For 0.12.0:

- iCloud can ship if Track A passes.
- Git can ship as beta only if B1–B4 pass.
- Auto-sync should remain disabled by default unless B2 and B4 conflict-marker tests pass.

---

# Immediate next engineering task

Start with iCloud Track A, not Git.

Concrete next work item:

> Build an internal iCloud validation checklist/test harness and patch Settings copy so iCloud is described as provider-backed folder replication. Then run the two-Mac A↔B smoke test with notes, tasks, and lenses.

After that, move to Git Track B P0 tests and hardening.
