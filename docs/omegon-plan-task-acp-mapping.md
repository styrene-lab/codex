# Omegon Plan/Task ACP Mapping Contract

Status: proposed for Flynt 0.12.x integration

## Current Omegon surface

Omegon exposes read-oriented ACP methods:

- `_plans/list`
- `_plans/show`
- `_tasks/list`
- `_tasks/show`
- `_plans/events` / `_tasks/events` placeholder event surfaces

and control methods:

- `_plans/switch`
- `_plans/detach`
- `_tasks/bind`

The current projection is useful for read-only display and manual linking. It is not yet a durable bidirectional task sync contract.

## Flynt integration posture

Flynt may consume Omegon plan/task projections as external planning context.

For 0.12.x, Flynt should support:

1. read-only display of Omegon projected plans/tasks;
2. manual link/import of an Omegon projected task into a Flynt task;
3. Flynt-owned durable links stored in `Task.external_refs`.

Flynt should not automatically sync status/completion bidirectionally until Omegon provides stable identity, durability, revision, mutation, and event semantics.

## Flynt-owned durable link format

Flynt stores local durable mappings using `Task.external_refs` entries with prefix:

```text
omegon-plan:<json>
```

Payload shape:

```json
{
  "plan_id": "openspec:sync-hardening",
  "task_id": "openspec:sync-hardening:group:Validation:1.2",
  "label": "Validate iCloud open-idle behavior",
  "revision": "sha256:optional"
}
```

This is a Flynt-local mapping. It does not prove Omegon persisted a reciprocal binding.

Implementation lives in:

```text
crates/flynt-core/src/omegon_plan_link.rs
```

## Required Omegon contract for direct mapping

Before Flynt treats Omegon projections as authoritative/bidirectional task mappings, Omegon should provide the following.

### 1. Stable task identity

Each task projection needs an identity stable across harmless source edits.

Required fields:

```json
{
  "id": "projection-id",
  "stable_id": "stable-task-id",
  "source": {
    "kind": "openspec|design|session|hybrid",
    "path": "openspec/changes/foo/tasks.md",
    "anchor": "..."
  }
}
```

`id` may remain display/projection identity. `stable_id` is what Flynt should bind to.

The stable ID must survive:

- OpenSpec task group title edits;
- task reordering;
- minor label edits;
- task renumbering where possible.

### 2. Revision/concurrency token

Every plan/task projection should expose a revision token:

```json
{
  "revision": "sha256:..."
}
```

Mutating calls should accept:

```json
{
  "expected_revision": "sha256:..."
}
```

and reject stale updates with a structured conflict error.

### 3. Durable binding semantics

`_tasks/bind` should report whether the binding was persisted.

Desired response:

```json
{
  "accepted": true,
  "durability": "repo|session|none",
  "revision": "sha256:...",
  "binding": {
    "task_id": "...",
    "system": "flynt",
    "external_task_id": "..."
  }
}
```

Flynt may only treat `durability: "repo"` as authoritative for bidirectional mapping.

### 4. Explicit supported mutations

`writable: true` is not enough. Each projection should declare supported operations:

```json
{
  "writable": true,
  "supported_mutations": [
    "bind_external_ref",
    "set_status",
    "append_evidence",
    "complete",
    "reopen"
  ]
}
```

Flynt must disable mutation UI for unsupported operations.

### 5. Status mapping contract

Omegon should document `WorkItemStatus` variants and their meaning.

Flynt can map conservatively:

| Omegon | Flynt |
|---|---|
| `pending` | `todo` |
| `in_progress` | `in_progress` |
| `done` | `done` |
| `blocked` | `todo` + blocked tag/status detail |
| `skipped` / `deferred` | archived/deferred presentation, not automatic delete |

Until exact variants are documented, Flynt should display raw status and avoid automatic status sync.

### 6. Real events or poll revisions

Current event surfaces are placeholders. Direct mapping needs one of:

- event streams with cursors; or
- polling with revision comparison.

Suggested event shape:

```json
{
  "cursor": "...",
  "events": [
    {
      "type": "task.updated",
      "task_id": "...",
      "stable_id": "...",
      "revision": "sha256:...",
      "changed_fields": ["status", "evidence"]
    }
  ]
}
```

### 7. Pagination/filtering

Large repos need filtered listing:

```json
{
  "plan_id": "...",
  "source": "openspec|design|hybrid",
  "status": "pending|in_progress|done",
  "limit": 100,
  "cursor": "..."
}
```

### 8. Structured errors

Flynt should receive machine-readable error codes:

```json
{
  "accepted": false,
  "code": "not_found|stale_revision|not_writable|unsupported_source|conflict",
  "error": "human readable detail"
}
```

## Flynt rollout plan

### Phase 1: read-only projection

- Query `_runtime/capabilities`.
- If plan surfaces are available, query `_plans/list` and `_tasks/list`.
- Render Omegon plans/tasks as external read-only context.

### Phase 2: manual link/import

- Operator chooses an Omegon projected task.
- Flynt creates or updates a Flynt task.
- Flynt stores `omegon-plan:<json>` in `Task.external_refs`.
- Flynt optionally calls `_tasks/bind` as a best-effort session hint.
- UI displays whether Omegon reported durable or session-only binding.

### Phase 3: bidirectional sync

Allowed only after Omegon supplies stable IDs, revisions, durable bind responses, supported mutations, and events/revision polling.

## Non-goals for 0.12.x

- No automatic bidirectional status sync.
- No automatic completion propagation.
- No assumption that `_tasks/bind` persists to repo state.
- No mutation of OpenSpec/design lifecycle files from Flynt unless Omegon explicitly exposes a durable mutation contract.
