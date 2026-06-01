---
title: Boards as Task Registry Projections
status: decided
tags: [project-registry, boards, tasks, agents]
---

# Boards as Task Registry Projections

## Decision

Boards are projected views over the Task Registry. They are not the canonical owner of task truth.

```text
TaskRegistry = indexed task/work-contract truth
Board        = filter + grouping + ordering + display/policy lens
```

This keeps tasks useful even when they are not on a visible board and makes the system agent-operable first.

## Rationale

Flynt's task/project-management system is expected to be used primarily by agents working cooperatively with the operator. The manual UI is a convenience and inspection layer, not the primary control surface.

Therefore agents need a stable task model independent of whichever board the operator is viewing.

## Ownership boundaries

### Task owns

- objective
- status
- scope
- constraints
- blockers
- evidence requirements
- evidence satisfaction
- links to documents/specs/artifacts/external refs
- tags, priority, engagement
- optional explicit board/column assignment

### Board owns

- which tasks it shows
- how tasks are grouped
- column definitions
- WIP/evidence policy hints
- display preferences
- saved filters
- manual ordering/pinning preferences

### ProjectRegistry owns

- derived relationships
- graph edges
- evidence rollups
- diagnostics
- resolved links
- cross-cutting projections

## Board types

### Assignment board

Traditional kanban semantics:

```text
task.board_id = X
task.column = "In Progress"
```

This is useful for manual workflows and explicit sprint boards.

### Query board

Dynamic projection:

```text
show tasks where evidence_state != satisfied
group by evidence_state
```

Useful for agent workflows and project health views.

Examples:

- Agent Ready
- Evidence Missing
- Done but Evidence Stale
- By Engagement
- By Spec
- Blocked
- Visual Artifacts Dogfood

## Board column policy

Columns may define advisory evidence/workflow policy:

```rust
pub struct BoardColumnPolicy {
    pub name: String,
    pub accepted_statuses: Vec<TaskStatus>,
    pub required_evidence: Vec<EvidenceRequirementKind>,
    pub max_wip: Option<usize>,
}
```

Policy is advisory, not a hard UI lock. The operator can move a card, but the registry should surface diagnostics such as:

```text
Task in Done column has missing evidence
Task in Review has no test evidence
Task marked done but evidence stale
```

## Persistence model

Persist board definitions and non-derivable user intent:

- board name
- query/filter
- grouping
- columns
- display preferences
- manual card ordering/pinning
- column policy

Do not persist the board's current visible task list as truth. It is derived from the Task Registry.

## Agent-facing model

Agents should primarily consume `TaskRegistryView` and `AgentTaskContract` records.

Boards are useful context:

```text
Give me tasks in the Evidence Missing board.
Show me Agent Ready tasks grouped by engagement.
Explain why this Done task is not evidence-satisfied.
```

But agents should not require an open board UI to understand work.

## Implementation path

1. Expand `TaskRegistryView` from placeholder ids into task/board projection records.
2. Add task evidence state types.
3. Project existing `ProjectStore::list_tasks` and `ProjectStore::list_boards` into registry records.
4. Emit `Task BelongsTo Board` edges for explicit board membership.
5. Add diagnostics for missing boards/columns.
6. Add query-board definitions later.
7. Wire board UI to consume registry projections incrementally.

## Summary

```text
Boards are lenses.
Tasks are work contracts.
Evidence chains prove task state.
ProjectRegistry ties all of it together.
```
