+++
title = "Flow authoring node catalog"
tags = ["flows","react-flow","surfaces","node-catalog","design"]
+++

+++
id = "4b2fa48c-15b7-410b-bd92-102f1819851b"
kind = "design_node"

[data]
title = "Flow authoring node catalog"
status = "exploring"
priority = 2
parent = "design-sidebar-organization"
dependencies = []
open_questions = []
+++

## Overview

# Flow authoring node catalog

---
title: Flow authoring node catalog
status: exploring
tags: [flows, react-flow, surfaces, node-catalog, design]
parent: design-sidebar-organization
---

# Flow authoring node catalog

## Overview

Flynt Flows should become a ComfyUI-style structured graph surface built on React Flow: operators compose node-edge graphs from a growing catalog of useful node primitives, while agents can inspect, patch, validate, and generate those graphs as data.

Flows are not freeform drawings and not design boards. A Flow is a typed, interactive graph where nodes, sockets, edges, and positions are first-class data in `.flow` files.

## Product direction

Use React Flow as the interaction substrate for a ComfyUI/n8n-like authoring experience:

- add nodes from a palette
- connect typed handles/sockets
- drag/arrange graph nodes
- edit node properties
- persist nodes/edges/positions to `.flow`
- eventually validate graph structure and export/run/derive workflows

The existing Flynt implementation already has the low-level substrate:

- `flynt-flow` schema crate
- `.flow` frontmatter + JSON body
- React Flow bundle under `crates/flynt-app/build/flow`
- custom node renderer
- sockets/handles
- edge persistence
- debounced save loop
- file watcher reloads

The missing product layer is the authoring catalog and creation affordance.

## Decision: start with generalist primitive nodes

Before adding many domain-specific nodes for existing Flynt entities (`Document`, `PlainText`, `Artifact`, etc.), establish a small generalist primitive catalog that can model common workflow/system structures.

Initial primitive node kinds:

| Kind | Purpose | Default sockets |
|---|---|---|
| `input` | external input, trigger, starting value | outputs: `out` |
| `process` | generic transformation/work step | inputs: `in`; outputs: `out` |
| `decision` | branch point or conditional | inputs: `in`; outputs: `yes`, `no` |
| `branch` | fan-out / route to multiple paths | inputs: `in`; outputs: `a`, `b` |
| `merge` | fan-in / join multiple paths | inputs: `a`, `b`; outputs: `out` |
| `output` | terminal result/sink | inputs: `in` |
| `note` | explanatory annotation with no graph semantics | no sockets |

These primitives are deliberately boring. They let operators build useful graphs before Flynt knows every domain-specific type.

## Extensibility model

The node catalog should be data-driven, not hardcoded ad hoc inside the renderer.

A catalog entry should eventually describe:

```ts
interface FlowNodeDefinition {
  kind: string;
  label: string;
  description: string;
  category: "primitive" | "flynt" | "agent" | "integration";
  defaultData: Record<string, unknown>;
  sockets: SocketJson[];
  style?: {
    accent?: string;
  };
}
```

The first implementation can live in the React bundle as a local constant. Later, the catalog can move to Rust/core or be merged from extension manifests.

## First authoring slice

Implement the smallest meaningful ComfyUI-style Flow experience:

1. Add a primitive node catalog to the React Flow bundle.
2. Show an empty-state overlay when a flow has no nodes.
3. Provide `Add node` actions for primitive node kinds.
4. Create nodes with default sockets and sensible positions.
5. Persist added nodes through the existing save loop.
6. Keep the `.flow` schema unchanged.

Recommended empty-state copy:

> Flows model systems as nodes and edges. Add primitive nodes like Input, Process, Decision, Branch, Merge, and Output, then connect their handles to describe direction, dependency, or data movement.

## Starter templates

After the primitive catalog lands, add templates:

- Blank
- Input → Process → Output
- Decision branch
- Fan-out/fan-in
- Agent pipeline
- Architecture flow

Templates should be graph snippets constructed from the same catalog definitions.

## Future domain-specific nodes

Once primitives exist, add Flynt-native node types:

- `document`
- `plaintext`
- `artifact`
- `surface`
- `task`
- `spec`
- `agent`
- `tool`
- `validation`

These should not block the primitive catalog. Domain nodes become more useful when operators can connect them to generic process/decision/branch structure.

## Open questions

- [assumption] The first catalog can live in the React bundle before moving to a shared Rust/extension registry.
- [assumption] Socket type strings remain documentation-only for v1; validation can warn later without blocking editing.
- Should primitive node definitions be versioned independently from `.flow` schema version?
- Should the first `New Flow` create a blank canvas with empty-state actions, or a default Input → Process → Output template?
- Should selected-node editing live inside the Flow canvas overlay first, or integrate with Flynt's right context rail?

## Non-goals for first slice

- Full ComfyUI parity
- Executable graph runtime
- Extension-loaded custom nodes
- Advanced type checking
- Auto-layout
- Export/render pipeline

## Open Questions
