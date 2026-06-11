+++
title = "Massive Scale Milestone 0.13.0"
tags = ["design","performance","0.13.0"]
+++

# Massive Scale Milestone 0.13.0

---
title: Massive Scale Milestone 0.13.0
status: exploring
tags: [design, performance, 0.13.0]
---

# Massive Scale Milestone 0.13.0

Flynt 0.13.0 targets massive workspace scale: interactive paths must be bounded by visible or changed state, not by total repository size.

## Goal

Flynt should remain responsive when opened inside repositories that are orders of magnitude larger than a personal note vault.

Interactive operations must avoid:

- full repository scans
- recursive whole-workspace watches
- full graph rebuilds on view open
- O(n²) graph layout on the WebView main thread
- global embed/artifact index construction during tab activation

## Design nodes

- [[docs/massive-scale-scoped-watching|Scoped Watching and Indexing]]
- [[docs/massive-scale-staged-startup|Staged Startup Runtime]]
- [[docs/massive-scale-incremental-index|Incremental Indexing and Backpressure]]
- [[docs/massive-scale-graph-rendering|Progressive Graph Rendering]]
- [[docs/massive-scale-graph-snapshots|Persisted Graph Snapshots]]
- [[docs/massive-scale-regression-harness|Massive Scale Regression Harness]]

## 0.12.x bridge fixes

The final 0.12.x patch train should ship short-term mitigations already identified from the Omegon-scale repository:

- Delay recursive watcher startup until after first paint.
- Keep startup reindexing off the launch path.
- Bound artifact wrapper discovery to Flynt artifact directories.
- Keep tab activation off global embed/index work.
- Make graph initial render visibly staged and avoid full O(n²) repulsion on large graphs.

## Acceptance posture for 0.13.0

- Opening a large repository paints the shell immediately from cached state.
- Watchers use configured scopes, not whole-repo recursion.
- Indexing is incremental, cancellable, and reports progress.
- Graph view defaults to overview/focus behavior for huge graphs.
- Graph rendering is progressive and cancellable.
- Synthetic massive-vault tests protect launch, tab switch, indexing, and graph budgets.
