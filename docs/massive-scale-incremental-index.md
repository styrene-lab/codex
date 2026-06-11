+++
title = "Incremental Indexing and Backpressure"
tags = ["design","performance","indexing"]
+++

# Incremental Indexing and Backpressure

---
title: Incremental Indexing and Backpressure
status: exploring
tags: [design, performance, indexing]
---

# Incremental Indexing and Backpressure

## Problem

Full `reindex()` is a blunt instrument. Even off the UI thread, full refreshes contend with startup, graph building, search, sync, and file watching.

## Direction

Replace broad reindex operations with an incremental job queue:

```rust
enum IndexJob {
    ScanRoot(PathBuf),
    IndexFile(PathBuf),
    DeletePath(PathBuf),
    RefreshArtifact(PathBuf),
}
```

Use file fingerprints:

- path
- size
- mtime
- optional content hash
- last indexed time
- index schema version

## Backpressure

- Debounce watcher bursts.
- Coalesce multiple changes by path.
- Cap queue size.
- If overflow occurs, schedule a scoped rescan rather than processing every event.
- Prioritize currently open documents and visible sidebar directories before low-priority global refresh.

## Acceptance criteria

- Startup loads existing SQLite state immediately.
- Changed-file indexing is incremental.
- Full rebuild is explicit and progress-reported.
- Watcher event storms do not produce unbounded SQLite work.
