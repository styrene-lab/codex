+++
title = "Staged Startup Runtime"
tags = ["design","performance","startup"]
+++

# Staged Startup Runtime

---
title: Staged Startup Runtime
status: exploring
tags: [design, performance, startup]
---

# Staged Startup Runtime

## Problem

The app can display only the macOS title bar while startup work competes with WebView first paint. Background threads can still cause IO or SQLite pressure that makes the app appear frozen.

## Direction

Startup must be explicitly phased:

1. Create window immediately.
2. Open config and store only.
3. Render shell/sidebar from cached metadata.
4. Start lightweight background jobs.
5. Start expensive watchers/indexers after first paint.
6. Hydrate graph/search/artifacts incrementally.

## Runtime state

Expose startup progress with a typed phase model:

```rust
enum ProjectRuntimePhase {
    OpeningStore,
    ReadyStaleIndex,
    Indexing { scanned: usize, indexed: usize },
    Watching,
    FullyReady,
    Degraded { reason: String },
}
```

## Acceptance criteria

- First paint does not wait for reindex, watcher setup, graph build, artifact discovery, or global embed indexing.
- The UI shows stale/cached/background status instead of a grey window.
- Expensive jobs are cancellable when runtime state is replaced.
