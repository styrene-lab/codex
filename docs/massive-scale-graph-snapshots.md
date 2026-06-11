+++
title = "Persisted Graph Snapshots"
tags = ["design","performance","graph","storage"]
+++

# Persisted Graph Snapshots

---
title: Persisted Graph Snapshots
status: exploring
tags: [design, performance, graph, storage]
---

# Persisted Graph Snapshots

## Problem

Graph view currently builds a graph payload on demand and serializes it into JavaScript. This repeats global work and prevents stable spatial memory.

## Direction

Maintain persisted graph tables and layout snapshots:

- graph_nodes
- graph_edges
- graph_snapshots
- graph_layout_positions

A snapshot records:

```rust
struct GraphSnapshot {
    version: u64,
    node_count: usize,
    edge_count: usize,
    generated_at: DateTime<Utc>,
    layout_seed: u64,
}
```

## Acceptance criteria

- Graph opens from the latest snapshot immediately.
- Incremental index updates mutate affected nodes and edges.
- Layout positions persist across app launches.
- Full graph recomputation is a background refresh, not a view-open requirement.
