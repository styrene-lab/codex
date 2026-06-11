+++
title = "Scoped Watching and Indexing"
tags = ["design","performance","watcher","indexing"]
+++

# Scoped Watching and Indexing

---
title: Scoped Watching and Indexing
status: exploring
tags: [design, performance, watcher, indexing]
---

# Scoped Watching and Indexing

## Problem

Flynt currently has code paths that treat the project root as the unit of scanning or watching. In a large repository, recursive root watching and duplicated skip logic create launch stalls and unbounded event pressure.

## Direction

Introduce a shared `IndexingScopePlan` used by indexing, file watching, artifact discovery, graph building, and project registry refreshes.

```rust
struct IndexingScopePlan {
    included_roots: Vec<PathBuf>,
    excluded_roots: Vec<PathBuf>,
    file_extensions: HashSet<String>,
    max_initial_files: Option<usize>,
}
```

## Decisions to make

- Default included roots for new projects.
- How existing projects migrate to scoped mode.
- Whether large-workspace mode is automatic, user-confirmed, or configurable.
- How explicit operator-added scopes are represented in project config.

## Acceptance criteria

- No recursive watcher is installed on the full repository root by default for large projects.
- Scope logic is defined once and consumed by all file traversal systems.
- Generated directories such as `target`, `node_modules`, `.git`, and vendor/cache trees are never watched or indexed unless explicitly configured.
