+++
title = "Progressive Graph Rendering"
tags = ["design","performance","graph"]
+++

# Progressive Graph Rendering

---
title: Progressive Graph Rendering
status: exploring
tags: [design, performance, graph]
---

# Progressive Graph Rendering

## Problem

The graph renderer can lock the WebView main thread by creating many SVG nodes and running expensive layout loops in one synchronous JS call.

## Direction

Render progressively:

1. Paint status immediately.
2. Build graph data structures.
3. Create edges/nodes in frame-budgeted batches.
4. Run layout in frame-budgeted chunks.
5. Progressively reveal labels.
6. Cancel stale renders when filters or route change.

## Scale modes

- Small graph: SVG with full labels and normal physics.
- Medium graph: SVG/Canvas with reduced labels and sampled layout.
- Huge graph: cluster overview or local-neighborhood mode by default.

## Acceptance criteria

- Graph status appears within one frame.
- No graph render function performs unbounded work without yielding.
- Large graphs do not run O(n²) layout on the WebView main thread.
- The renderer can cancel a stale render when settings change or the route changes.
