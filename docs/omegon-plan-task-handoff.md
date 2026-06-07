# Omegon Plan/Task ACP Handoff

Status: handoff note for Flynt ↔ Omegon plan/task integration

## Context

Omegon recently added ACP plan/task projection surfaces:

```text
_plans/list
_plans/show
_plans/events
_plans/switch
_plans/detach
_tasks/list
_tasks/show
_tasks/bind
_tasks/events
```

Flynt reviewed these surfaces after the 0.12.0 sync hardening work. The conclusion: the current Omegon surface is good enough for read-only projection and manual linking, but not yet sufficient for automatic bidirectional mapping into Flynt Tasks.

## Current Flynt-side work

Commit:

```text
58afcc4 feat(sync): define omegon plan task links
```

Added:

```text
crates/flynt-core/src/omegon_plan_link.rs
docs/omegon-plan-task-acp-mapping.md
```

### Link primitive

Flynt can now store local durable links to Omegon projected tasks in `Task.external_refs`:

```text
omegon-plan:<json>
```

Rust type:

```rust
pub struct OmegonPlanTaskLink {
    pub plan_id: String,
    pub task_id: String,
    pub label: Option<String>,
    pub revision: Option<String>,
}
```

Helpers:

```rust
OmegonPlanTaskLink::to_external_ref()
OmegonPlanTaskLink::from_external_ref()
find_omegon_plan_task_links(...)
upsert_omegon_plan_task_link(...)
```

Validation:

```bash
cargo test -q -p flynt-core omegon_plan_link
cargo check -q
```

passed.

## Important boundary decision

Flynt-owned `omegon-plan:<json>` refs are local durable mappings. They do **not** prove Omegon persisted a reciprocal binding.

Current Omegon `_tasks/bind` response appears session/control-plane oriented and reports:

```text
mutation = "session_view_only"
```

So Flynt must not treat `_tasks/bind` as durable until Omegon explicitly guarantees repo/session durability semantics.

## Required Omegon contract before direct mapping

Before Flynt can automatically map Omegon task projections directly into Flynt Tasks, Omegon needs:

1. Stable task identity
   - projection ID should survive reorder/group title/label changes, or provide `stable_id` separately.

2. Revision/concurrency token
   - tasks/plans should expose `revision`.
   - mutations should accept `expected_revision`.

3. Durable binding response
   - `_tasks/bind` should return `durability: "repo" | "session" | "none"`.
   - Flynt can only trust `repo` for bidirectional mapping.

4. Explicit supported mutations
   - not just `writable: true`; expose `supported_mutations`.

5. Status enum contract
   - document Omegon `WorkItemStatus` variants and intended Flynt mappings.

6. Real events or polling revisions
   - current `_plans/events` / `_tasks/events` are placeholders.

7. Pagination/filtering
   - necessary for large repos.

8. Structured errors
   - machine-readable error codes such as `not_found`, `stale_revision`, `not_writable`, `conflict`.

See full contract details:

```text
docs/omegon-plan-task-acp-mapping.md
```

## Recommended Flynt rollout

### Phase 1 — read-only display

Add Flynt ACP wrappers for:

```text
_runtime/capabilities
_plans/list
_plans/show
_tasks/list
_tasks/show
```

Render Omegon plans/tasks as read-only external planning context.

### Phase 2 — manual link/import

Allow operator to link/import a projected Omegon task into a Flynt task.

Flynt stores the durable local mapping in `Task.external_refs` using `omegon-plan:<json>`.

Optionally call `_tasks/bind` as a best-effort session hint, but display whether Omegon reports session-only or durable binding.

### Phase 3 — bidirectional sync

Only after Omegon supplies stable IDs, revisions, durable binding semantics, supported mutations, and events/revision polling.

## Non-goals for 0.12.x

Do not implement automatic bidirectional task sync yet.

Do not assume Omegon projection IDs are stable enough for authoritative mapping.

Do not mutate OpenSpec/design lifecycle files from Flynt unless Omegon exposes a durable mutation contract.

## Current release posture

This integration is not required for Flynt 0.12.0. It is a 0.12.x follow-up opportunity.

Suggested target:

```text
0.12.1/0.12.2: read-only Omegon plan/task projection panel
0.12.3+: manual link/import UX
later: bidirectional sync after Omegon contract hardens
```
