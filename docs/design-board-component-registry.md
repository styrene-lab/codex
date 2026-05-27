---
id: design-board-component-registry
title: "Design Board component registry"
status: exploring
parent: flynt-root
tags: [design-board, components, shadcn, tweakcn, d2, agent-authoring]
open_questions:
  - "[assumption] D2 rendering can be invoked synchronously or cheaply enough from the Design Board render path for the first slice; otherwise rendered SVG must be cached or precomputed by the agent/tool layer."
  - "[assumption] The existing vendored Tailwind subset includes enough shadcn-compatible classes for the first built-in components; otherwise the component slice must expand the generated Tailwind CSS bundle."
  - "Should D2-backed components accept raw D2 source in the first implementation, semantic node/edge specs only, or both with raw D2 treated as an escape hatch?"
  - "Where should component rendering live long-term: `flynt-core` for shared validation/rendering, `flynt-app` for UI-local rendering, or split with schemas/types in core and renderers in app?"
dependencies: []
related: []
---

# Design Board component registry

## Overview

Introduce a small internal Design Board component registry that combines shadcn-style HTML component shells, tweakcn theme tokens, and D2-backed diagram components. First slice keeps existing raw HTML cells while adding typed component cells rendered to static HTML/SVG inside the existing iframe pipeline.

## Decisions

### First slice uses static component rendering, not a React runtime

**Status:** decided

**Rationale:** The current Design Board renderer is reliable because cells are isolated, portable iframe documents. Static rendering preserves that property while making agent authoring semantic and patchable.

### shadcn anatomy plus tweakcn tokens define the visual layer

**Status:** decided

**Rationale:** This matches Flynt's existing primitive/theme direction and keeps generated boards visually coherent across cells and themes.

### D2 backs diagram components inside shadcn-style shells

**Status:** decided

**Rationale:** D2 is well matched to architecture and relationship diagrams, while shadcn/tweakcn is better for UI framing and theme coherence.

### Component cells are additive and semantic

**Status:** decided

**Rationale:** Keeping raw cells avoids blocking uncommon layouts, while semantic component cells make common designs easier to generate, validate, and patch.

### Stable v1 definition

**Status:** decided

**Rationale:** This bounds the milestone around a reliable semantic authoring and rendering contract rather than full low-code editor ambitions.

## Open Questions

- [assumption] D2 rendering can be invoked synchronously or cheaply enough from the Design Board render path for the first slice; otherwise rendered SVG must be cached or precomputed by the agent/tool layer.
- [assumption] The existing vendored Tailwind subset includes enough shadcn-compatible classes for the first built-in components; otherwise the component slice must expand the generated Tailwind CSS bundle.
- Should D2-backed components accept raw D2 source in the first implementation, semantic node/edge specs only, or both with raw D2 treated as an escape hatch?
- Where should component rendering live long-term: `flynt-core` for shared validation/rendering, `flynt-app` for UI-local rendering, or split with schemas/types in core and renderers in app?
- Remove legacy top-level `html/css/js` Design Board cell deserialization before declaring the component registry v1 stable. Legacy input is accepted only as a temporary compatibility shim and must canonicalize to `content.kind = "html"` on save.

## Implementation Notes

### File Scope

- `crates/flynt-core/src/design_board.rs` — add `CellContent`, board schema versioning, legacy raw-cell deserialization, and canonical component-cell serialization.
- `crates/flynt-core/src/design_components/` — define component metadata, JSON schemas, examples, static renderers, HTML escaping helpers, and D2 source/spec compilation if shared rendering lives in core.
- `crates/flynt-app/src/views/design_board.rs` — resolve cell content through the component renderer before calling the existing iframe `srcdoc` builder; surface render failures as visible error cells.
- `crates/flynt-agent/src/extension.rs` — expose `design_board_list_components`, extend `design_board_set_cells` schema, and validate component cells during tool calls.
- `crates/omegon-design/src/extension.rs` — update critique/capture support so component validation and D2 render failures are reported separately from raw HTML lint.
- `docs/design-board-component-registry.md` — document the v1 contract, component catalog, D2 bridge, stability checklist, and non-goals.

### Constraints

- Keep raw HTML/CSS/JS cells as an escape hatch through v1.
- No arbitrary React runtime before v1.
- All string props escaped by default; rich text must be structured explicitly.
- Every built-in component must render an outermost h-full element and avoid Tailwind arbitrary-value classes.
- D2-backed components must sanitize rendered SVG before embedding.
- Component definitions must expose schema, examples, category, variants, and rendering constraints to agents.

## Stable v1 Plan

### Phase 0 — Contract cleanup

Clean up the renamed Design Board foundation before adding component semantics.

- Rename lingering internal constants and messages: `CANVAS_VERSION` becomes `DESIGN_BOARD_VERSION`; user-facing errors say "design board" rather than `design_board`.
- Normalize CSS and DOM class naming around `board-*` or `design-board-*`, avoiding underscore-style class names in rendered HTML.
- Keep the canonical wrapper tag as `design-board`.
- Audit docs for accidental Excalidraw/Design Board terminology crossover after the rename.

**Acceptance:** focused Design Board tests pass; remaining `canvas` references are either browser-native/Excalidraw-specific or intentionally absent from Design Board code.

### Phase 1 — Cell content schema

Add an explicit content layer so a cell can be raw markup or a semantic component. This phase uses a temporary Option-B compatibility shim: legacy top-level `html/css/js` cell fields are accepted only at input boundaries, immediately canonicalized into `CellContent::Html`, and never written back out by normal serialization.

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum CellContent {
    #[serde(rename = "html")]
    Html {
        html: String,
        #[serde(default)]
        css: String,
        #[serde(default)]
        js: Option<String>,
    },
    #[serde(rename = "component")]
    Component {
        component: String,
        #[serde(default)]
        props: serde_json::Value,
        #[serde(default)]
        variant: Option<String>,
    },
}
```

The first implementation may deserialize legacy `html/css/js` fields into `CellContent::Html`; the stable v1 format should prefer `content` as the canonical representation. Legacy support is tracked as a removal gate, not an indefinite dual-format contract.

```rust
#[derive(Deserialize)]
#[serde(untagged)]
enum CellWire {
    V1(Cell),
    Legacy(LegacyCell),
}
```

Only the canonical `Cell` shape implements normal serialization. A board loaded from legacy cells must save back without top-level `html`, `css`, or `js` fields.

**Acceptance:** raw cells and component cells both round-trip through `.board` JSON, invalid content kinds produce clear errors, legacy cells load with a deprecation warning, and canonical serialization drops legacy fields.

### Phase 2 — Static component registry

Introduce a registry that exposes metadata and deterministic renderers.

```rust
pub struct RenderedCell {
    pub html: String,
    pub css: String,
    pub js: Option<String>,
}

pub struct DesignComponentDefinition {
    pub name: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    pub variants: &'static [&'static str],
    pub props_schema: fn() -> serde_json::Value,
    pub examples: fn() -> Vec<serde_json::Value>,
    pub render: fn(&serde_json::Value, Option<&str>) -> anyhow::Result<RenderedCell>,
}
```

The registry owns component lookup, variant validation, schema publication, and render dispatch. Renderers return static HTML/CSS/SVG that enters the existing iframe `srcdoc` path.

**Acceptance:** unknown components and variants produce actionable errors; every registered component has schema, examples, and tests.

### Phase 3 — shadcn/tweakcn visual primitives

Ship a small visual catalog that proves theme coherence and agent ergonomics.

Must-have v1 primitives:

- `Panel` — generic card shell with optional title, description, footer, and badge.
- `SectionHeader` — compact heading block for page structure.
- `Hero` — high-emphasis title/subtitle/action composition.
- `MetricCard` — dashboard KPI cell with delta tone.
- `StatGrid` — grouped compact metrics.
- `FeatureCard` — title/body/icon/badge explanatory card.
- `Callout` — highlighted note/warning/success message.
- `ComparisonTable` — structured comparison table.

Each component uses shadcn-style anatomy and resolves visual roles through tweakcn theme tokens (`bg-card`, `text-foreground`, `text-muted-foreground`, `border-border`, `bg-primary`, etc.).

**Acceptance:** examples for every primitive render without structural lint warnings and every outermost element fills the cell.

### Phase 4 — D2-backed diagram components

Pair shadcn/tweakcn shells with D2 interiors.

Must-have v1 diagram components:

- `DiagramPanel` — card shell around raw D2 source rendered to sanitized SVG.
- `ArchitectureMap` — semantic nodes/edges compiled to themed D2, then rendered to SVG.

`DiagramPanel` provides the escape hatch for agents already fluent in D2. `ArchitectureMap` provides the patchable semantic path:

```json
{
  "component": "ArchitectureMap",
  "props": {
    "title": "HostAction approval path",
    "nodes": [
      { "id": "agent", "label": "Agent", "tone": "ai" },
      { "id": "host", "label": "Flynt Host", "tone": "primary" }
    ],
    "edges": [
      { "from": "agent", "to": "host", "label": "proposes action" }
    ]
  }
}
```

D2 rendering should be cached by source/spec hash, active theme, and renderer version. SVG output must be sanitized before embedding.

**Acceptance:** D2 render errors appear as visible error panels, not broken iframes; semantic diagrams inherit the active Design Board theme.

### Phase 5 — Agent discovery and mutation tools

Add a first-class discovery endpoint:

```text
design_board_list_components
```

It returns component metadata in a compact Storybook-like shape:

```json
{
  "version": 1,
  "components": [
    {
      "name": "MetricCard",
      "category": "dashboard",
      "description": "Compact KPI card.",
      "variants": ["default"],
      "props_schema": {},
      "examples": []
    }
  ]
}
```

Extend `design_board_set_cells` to accept either legacy raw fields or `content.kind = "component"`.

**Acceptance:** an agent can create a complete board using only `design_board_list_components` plus `design_board_set_cells`.

### Phase 6 — Critique, capture, and validation

Update the design review path so component cells are first-class citizens.

- Validate component name, variant, and props.
- Validate D2 source/spec before or during render.
- Preserve existing screenshot capture and fill-ratio metrics.
- Distinguish raw-cell lint, component validation, D2 validation, and visual-capture findings.

**Acceptance:** `design_critique` reports component-specific failures with enough context for the agent to patch props rather than rewrite the cell.

### Phase 7 — Documentation and examples

Document the v1 contract and ship example boards.

Example boards:

- Dashboard board: `MetricCard`, `StatGrid`, `ComparisonTable`, `Callout`.
- Architecture board: `Hero`, `DiagramPanel`, `ArchitectureMap`.
- Research/evidence board: `Panel`, `FeatureCard`, `Callout`, future `EvidenceMap` placeholder.

**Acceptance:** examples load in Flynt, render cleanly, and are usable as agent few-shot references.

## v1 Stability Checklist

- [ ] `.board` schema supports `CellContent`.
- [ ] Raw HTML/CSS/JS cells remain supported.
- [ ] Component cells render deterministically.
- [ ] Component registry is discoverable by agent tools.
- [ ] At least eight shadcn/tweakcn-style components are registered.
- [ ] At least two D2-backed components are registered.
- [ ] Props are JSON-schema-described.
- [ ] String props are escaped by default.
- [ ] D2 SVG output is sanitized.
- [ ] Render failures produce visible error cells.
- [ ] Capture and critique work on component cells.
- [ ] Example boards pass validation.
- [ ] Extension points and non-goals are documented.
