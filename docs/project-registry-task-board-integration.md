---
title: Project Registry Task and Board Integration
status: exploring
tags: [project-registry, tasks, boards, project-management]
---

# Project Registry Task and Board Integration

## Intent

Project management entities should become first-class ProjectRegistry projections without compromising Flynt's plaintext/local-first model.

The registry should represent tasks, boards, engagements, and their relationships as typed project graph nodes/edges while leaving the existing stores as the source of truth.

## Source of truth

Task/board truth remains in existing Flynt stores and plaintext project files where applicable. The registry is a derived semantic projection over those stores.

Do not duplicate task bodies or board definitions into an authoritative registry database.

## Registry additions

The existing placeholder:

```rust
pub struct TaskRegistryView {
    pub task_ids: Vec<String>,
}
```

should evolve into:

```rust
pub struct TaskRegistryView {
    pub tasks: Vec<TaskRecord>,
    pub boards: Vec<BoardRecord>,
    pub engagements: Vec<EngagementRecord>,
}

pub struct TaskRecord {
    pub id: TaskId,
    pub title: String,
    pub status: TaskStatus,
    pub board_id: Option<BoardId>,
    pub column: Option<String>,
    pub document_id: Option<DocumentId>,
    pub path: Option<PathBuf>,
    pub tags: Vec<String>,
    pub priority: Option<String>,
    pub engagement_id: Option<EngagementId>,
}

pub struct BoardRecord {
    pub id: BoardId,
    pub name: String,
    pub columns: Vec<String>,
}

pub struct EngagementRecord {
    pub id: EngagementId,
    pub name: String,
    pub status: String,
}
```

Exact field names should follow existing `flynt_models` types when implemented. Avoid shadow interfaces if upstream SDK/store types are already exported.

## Graph nodes

Extend or reuse `ProjectNodeRef` for:

```rust
ProjectNodeRef::Task(String)
ProjectNodeRef::Board(String)       // add
ProjectNodeRef::Engagement(String)  // add
ProjectNodeRef::Document(DocumentId)
ProjectNodeRef::Spec(String)
```

Current `ProjectNodeRef::Task(String)` can remain as a stable bridge until a typed `TaskId` node variant is practical.

## Graph relations

Task/board graph edges should include:

```text
Task BELONGS_TO Board
Task BELONGS_TO BoardColumn
Task REFERENCES Document
Document IMPLEMENTS Task
Task DEPENDS_ON Task
Task REFERENCES ExternalRef
Task REFERENCES VisualArtifact
Task BELONGS_TO Engagement
Engagement REFERENCES Repo
Spec IMPLEMENTS Task
Task IMPLEMENTS SpecScenario
```

Use existing `ProjectRelation` variants where possible:

- `BelongsTo`
- `DependsOn`
- `References`
- `Implements`
- `LinksTo`

Add new variants only when semantics are meaningfully distinct.

## Discovery path

`ProjectRegistry::discover(project_root, store)` should eventually also call:

```rust
let task_refs = TaskRegistryView::from_store(store)?;
```

Then edge building should receive task refs:

```rust
let edges = build_project_edges(
    &documents,
    &visual_artifacts,
    &external_refs,
    &task_refs,
);
```

## Startup diagnostics

Task/board projection should emit diagnostics, not fail startup, for:

- task references board id that does not exist
- task references document id/path that does not exist
- task references engagement id that does not exist
- task has column not present in board definition
- task dependency points to missing task
- board has duplicate column names

These are project health signals, not hard errors.

## Persistence and sync

Task/board registry projection is derived. Do not persist it as authoritative registry state.

Potential true persistence belongs only to non-derivable user intent, such as:

- saved kanban view filters
- saved board layout preferences
- graph layout pins
- project-management dashboard preferences

Task/board content itself remains in existing stores/files.

## Initial implementation recommendation

Start with a minimal registry view:

1. list boards from `ProjectStore::list_boards`
2. list tasks from `ProjectStore::list_tasks(TaskFilter::default())`
3. project task ids and board ids as nodes
4. emit `Task BelongsTo Board` edges when `board_id` exists
5. emit diagnostics for missing boards/columns
6. avoid engagement/repo/spec wiring until the basic board graph is stable

This gives project-management graph value without over-architecting.
