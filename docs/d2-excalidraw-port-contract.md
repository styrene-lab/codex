+++
id = "d2-excalidraw-port-contract"
kind = "design_node"

[data]
title = "D2 to Excalidraw port contract"
status = "decided"
issue_type = "architecture"
priority = 2
parent = "flynt-ui-gaps-and-issues"
dependencies = []
open_questions = []
tags = ["d2", "excalidraw", "drawing", "diagram", "skills", "portability"]
+++

# D2 to Excalidraw port contract

## Decision

Flynt/Omegon diagram tooling must support two explicit D2 → Excalidraw port modes:

1. **SVG import mode** — import or reference the rendered D2 SVG directly as a static visual artifact.
2. **Semantic translation mode** — parse/read the D2 source and translate its meaning into editable Excalidraw/Flynt drawing components.

These modes are not interchangeable. Agents must choose and state the mode before porting.

## Why this matters

D2 is deterministic, source-controlled, and good for compact structural diagrams. Excalidraw is spatial, editable, and good for narrative architecture maps. Real workflows need both:

- “Take this deterministic D2 diagram and put the rendered SVG in a drawing.”
- “Take this D2 diagram and turn its meaning into editable Excalidraw components.”

The first preserves exact rendered output. The second preserves semantic intent and editability.

## Port mode A — SVG import

Use when the operator wants the D2 render as-is:

- exact output matters
- D2 remains canonical
- the drawing is a presentation/container artifact
- no element-level Excalidraw editing is needed

Behavior:

- Read `.d2` source path and resolve sibling `.svg`.
- If `.svg` is stale/missing, render D2 first.
- Add the SVG as an image/static element or reference in the Excalidraw scene.
- Record provenance in wrapper/spec metadata:
  - `derived_from = "diagrams/name.d2"`
  - `port_mode = "svg_import"`
  - `source_role = "canonical"`

Limitations:

- Imported SVG is not semantically editable as Excalidraw nodes/edges.
- Subsequent D2 changes require re-import/re-render.

## Port mode B — semantic translation

Use when the operator wants an editable drawing that carries the D2 diagram’s meaning:

- spatial layout needs improvement
- labels/edges need hand-routed clarity
- D2 output is clipped/spaghetti/panoramic
- Excalidraw should become a human-editable architecture map

Behavior:

- Read the `.d2` source, not just the SVG.
- Extract:
  - title
  - containers/groups
  - nodes
  - nested nodes
  - edges
  - edge labels
  - style classes
  - legends/callouts
- Map D2 concepts to drawing spec/Excalidraw elements:

| D2 concept | Excalidraw/drawing mapping |
|---|---|
| title | title text |
| container | panel/lane/bounding group |
| node | card/rectangle |
| nested node | child card within panel |
| edge | connector arrow |
| edge label | small connector label or callout |
| class/style | semantic palette role |
| dashed edge | optional/future/async relationship |
| thick edge | primary flow |
| legend | side/footer legend panel |

- Preserve graph meaning, but do not promise D2 round-trip.
- Record provenance:
  - `derived_from = "diagrams/name.d2"`
  - `port_mode = "semantic_translation"`
  - `round_trip = false`

## Agent rules

- Do not silently choose SVG import when the user asked for editable components.
- Do not silently choose semantic translation when the user asked for exact D2 output.
- If the user says “import the SVG,” use SVG import mode.
- If the user says “translate,” “make editable,” “turn into Excalidraw components,” or “make this clearer as a drawing,” use semantic translation mode.
- If ambiguous, ask which mode: exact SVG import or editable semantic translation.
- Never create drawing wrappers manually; use Flynt drawing tools.
- Prefer drawing spec tools for semantic translation so the result remains structured.

## Skill implications

### `d2-authoring`

Must explain when D2 should remain canonical and when a diagram is a candidate for Excalidraw porting.

### `flynt-drawing`

Must understand D2 as an input format. It does not need to lint D2 deeply, but it must preserve containers, nodes, edges, labels, and provenance when translating.

### `diagram-surface-selection`

Should route diagram requests among:

- D2 repair/regeneration
- D2 SVG import
- D2 semantic translation to Excalidraw
- native Excalidraw drawing
- Flow graph
- Design Board diagram panel

## Implementation outline

1. Add Armory skills:
   - `d2-authoring`
   - `flynt-drawing`
2. Add cross-references between those skills.
3. Add Flynt agent tool or helper for D2 port planning:
   - input: `d2_path`, `mode`
   - output: port plan + provenance metadata
4. Implement SVG import mode first if Excalidraw image embedding is straightforward.
5. Implement semantic translation using drawing spec components and connector routing.
6. Add tests/fixtures using current `v1-is-state-redis-coupling.d2` and `v2-ought-state-direction-e.d2`.
