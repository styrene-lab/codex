---
id: design-board-visual-substrate
title: "Design Board visual substrate"
status: exploring
parent: design-board-component-registry
tags: [design-board, visual-substrate, rendering, components, templates]
open_questions:
  - "[assumption] Static HTML/CSS component rendering is sufficient for first-class website mockups, document one-pagers, whiteboards, brochures, and simple diagrams before adding an interactive React/runtime layer."
  - "[assumption] The board grid plus component-internal layout primitives can cover page-like designs without introducing nested freeform editors in v1."
  - "Should board-level exports target browser screenshots first, print/PDF CSS first, or both in parallel?"
  - "How much rich text is allowed in structured props before the system needs a portable rich-text AST?"
  - "What minimum template library proves generality across website, document, brochure, whiteboard, and diagram use cases?"
related:
  - design-board-focus-layer
  - design-board-component-registry
  - design-sidebar-organization
---

# Design Board visual substrate

## Overview

Reframe Design Board from a dashboard/research-card surface into a flexible visual substrate for composing website mockups, PDFs, resumes, personal whiteboards, diagrams, product pamphlets, brochures, dashboards, and research projections.

The renderer remains static and deterministic for v1: semantic component cells render into isolated iframe HTML/CSS/SVG using theme tokens, structured props, escaping-by-default, and no arbitrary React runtime. Raw HTML remains the escape hatch.

## Decisions

### Accepted: page/layout primitives come before domain-specific cards

**Status:** decided

**Rationale:** A substrate that starts with `SourceCard`/`MetricCard` becomes a narrow dashboard tool. A substrate that starts with `Frame`, `TextBlock`, `Columns`, `ImagePlaceholder`, and `ButtonRow` can express websites, resumes, brochures, whiteboards, and dashboards.

### Accepted: components are semantic render specs, not editor widgets

**Status:** decided

**Rationale:** Current scope is "how to render" and "what to render," not direct manipulation. Component props should describe intent and produce deterministic static output. Operator interaction, drag handles, graph manipulation, and inspector UX are later layers.

### Accepted: raw HTML remains the pressure valve

**Status:** decided

**Rationale:** Competing products win by flexibility. The component registry should cover common structure, but uncommon layouts must not wait for custom Rust components.

### Accepted: every component needs examples, variants, and export discipline

**Status:** decided

**Rationale:** Agents need examples to use components well. Operators need exportable output. Components must fill cells, use theme tokens, avoid Tailwind arbitrary values, and render predictably in screenshot/PDF paths.

## Implementation nodes

### Node A — Layout foundation

Purpose: establish substrate grammar independent of domain.

Components:

- `Frame` — visual region/artboard section; variants `plain`, `card`, `bordered`, `hero`, `muted`, `accent`.
- `TextBlock` — structured typography; variants `body`, `heading`, `lead`, `quote`, `caption`, `fine-print`.
- `Columns` — two/three/asymmetric content columns.
- `Stack` or `List` — vertical repeated content with icon/bullet/check support.
- `ButtonRow` — primary/secondary action grouping.
- `ImagePlaceholder` — screenshot/media placeholder with aspect/caption.

Files:

- `crates/flynt-core/src/design_components.rs`
- `crates/flynt-app/src/views/design_board.rs`
- `crates/flynt-agent/src/extension.rs`

Acceptance:

- A simple landing page section, resume section, and brochure panel can be rendered using only these primitives.
- Every example renders an outermost `h-full` element.
- All string props are escaped.

### Node B — Website and product mockup components

Purpose: make Design Board credible for web/app mockups and product one-pagers.

Components:

- `Navbar`
- `Hero`
- `FeatureGrid`
- `CallToAction`
- `DeviceMockup`
- `FormMock`

Acceptance:

- Example board: startup landing page with nav, hero, feature grid, CTA, and browser mockup.
- Components support theme token variants without hardcoded brand colors.
- Layout remains legible at common board cell sizes.

### Node C — Document, PDF, resume, and brochure components

Purpose: support page-like static documents and marketing collateral.

Components:

- `ResumeHeader`
- `ExperienceItem`
- `SkillList`
- `Timeline`
- `BrochurePanel`
- `PullQuote`
- `ContactCard`

Acceptance:

- Example board: one-page resume.
- Example board: tri-fold/product pamphlet approximation.
- Print/PDF export constraints are documented, even if final export implementation is deferred.

### Node D — Whiteboard and ideation components

Purpose: support personal thinking surfaces without requiring polished marketing layout.

Components:

- `StickyNote`
- `Checklist`
- `Swimlane`
- `ProcessSteps`
- `MindMapLite`

Acceptance:

- Example board: personal project planning whiteboard.
- Components tolerate concise, messy content while still applying theme discipline.

### Node E — Data and comparison components

Purpose: support dashboards, decision docs, and structured comparisons.

Components:

- `MetricCard`
- `StatGrid`
- `ComparisonTable`
- `PricingCard`
- `ProgressCard`

Acceptance:

- Example board: product dashboard / pricing comparison.
- Table/list props reject invalid row shapes with actionable errors.

### Node F — Diagram rendering components

Purpose: support simple diagrams before and alongside D2.

Components:

- `ProcessSteps` if not completed in Node D.
- `FlowDiagram` — semantic boxes/arrows with constrained layout.
- `ArchitectureMap` — semantic nodes/edges/groups compiled to themed SVG/D2.
- `DiagramPanel` — raw D2 escape hatch rendered to sanitized SVG.

Acceptance:

- Diagram render errors become visible error cells.
- SVG is sanitized before embedding.
- D2 output is cached or precomputed if runtime rendering is too expensive.

### Node G — Templates and examples

Purpose: compete with template-driven products, not just component APIs.

Templates:

- Landing page
- SaaS/product one-pager
- Resume/CV
- Product brochure/pamphlet
- Personal whiteboard
- Decision matrix
- Architecture overview
- Research/source board

Acceptance:

- Each template is represented as `.board` JSON using semantic component cells where possible.
- Agent docs explain which template to start from for common operator briefs.

### Node H — Export/render fidelity

Purpose: make output useful outside Flynt.

Areas:

- viewport screenshot export
- print/PDF styling constraints
- page presets: `web`, `letter`, `a4`, `slide`, `social-square`, `brochure`
- font and image asset handling

Acceptance:

- Board examples can be captured as images without obvious clipping/dead-space bugs.
- PDF-oriented templates document current limitations.

## Adversarial assessment against similar products

### Claude Artifacts / Claude Design class

Observed competitive strength:

- Generates polished UI/mockups inline from natural language.
- Can produce functioning HTML/React-like artifacts quickly.
- Excellent for one-off interactive prototypes.

Design Board risk:

- Static Rust-rendered components may feel less magical and less flexible than model-generated live HTML/React.
- Component catalog lag creates visible gaps; if the requested visual pattern has no component, agents fall back to raw HTML and lose semantic patchability.

Design Board counter-position:

- Local-first persistent `.board` files with canonical structured cells.
- Deterministic rendering and validation rather than opaque generated app blobs.
- Raw HTML escape hatch preserves Claude-like flexibility when registry coverage is missing.
- Component props make later patching safer than editing arbitrary generated HTML.

Required response:

- Keep raw HTML path first-class.
- Make component examples rich enough for agents to compose polished output.
- Do not over-constrain the renderer into a dashboard-only DSL.

### Canva class

Observed competitive strength:

- Huge template library.
- Brand kits, fonts, logos, reusable styles.
- Strong export story for PDFs, social images, websites, and marketing collateral.
- Rich visual elements and media handling.

Design Board risk:

- A component registry alone does not compete with Canva. Without templates, page presets, brand/style-guide ingestion, and export discipline, Design Board is just a developer-flavored card grid.
- Brochures/resumes need typography, image placeholders, page sizing, and print constraints, not only cards.

Design Board counter-position:

- Agent-native generation and structured mutation inside a local knowledge workspace.
- Project style guide and theme tokens can act as a lightweight brand kit.
- `.board` JSON is portable, reviewable, and git-friendly compared with opaque design files.

Required response:

- Build templates early.
- Add page/export presets.
- Treat style guide/theme as a brand system, not just colors.
- Add media/image-placeholder semantics before diagram complexity.

### Figma class

Observed competitive strength:

- Auto layout, constraints, variants, component instances, design systems.
- High-fidelity direct manipulation and collaboration.

Design Board risk:

- Without auto-layout semantics inside components, every complex layout becomes hand-coded HTML.
- Without variants and constraints, resizing cells can degrade visual quality.

Design Board counter-position:

- We are not competing on direct-manipulation design tooling in this phase.
- We can borrow Figma's best rendering ideas: frames, auto-layout-like stacks/columns, variants, and tokenized component definitions.

Required response:

- Implement layout primitives (`Frame`, `Stack`, `Columns`) before more decorative components.
- Every component must define resize behavior and minimum useful cell size.
- Variants should be explicit and schema-published.

## Reference pillars

Keep three upstream product classes on the horizon while building. They are reference pillars, not parity commitments.

1. **Claude Artifacts / Claude Design pillar — generative flexibility.** Design Board must preserve fast idea-to-render flow, tolerate one-off custom layouts, and keep raw HTML as an escape hatch when the component catalog is incomplete.
2. **Canva pillar — templates, brand, and export.** Design Board must grow toward reusable templates, style-guide/theme-as-brand-kit behavior, media placeholders/assets, and reliable image/PDF-oriented output.
3. **Figma pillar — layout, variants, and constraints.** Design Board must borrow the rendering lessons of frames, auto-layout-like stacks/columns, variants, minimum sizes, and predictable resize behavior without chasing direct-manipulation editor parity in this slice.

These pillars prevent the implementation from collapsing into a narrow dashboard/research-card system.

## Updated priority

1. Layout foundation: `Frame`, `TextBlock`, `Columns`, `List/Stack`, `ButtonRow`, `ImagePlaceholder`.
2. Website/product mockups: `Navbar`, `Hero`, `FeatureGrid`, `CallToAction`, `DeviceMockup`, `FormMock`.
3. Document/brochure/resume: `ResumeHeader`, `ExperienceItem`, `SkillList`, `Timeline`, `BrochurePanel`.
4. Whiteboard: `StickyNote`, `Checklist`, `Swimlane`, `ProcessSteps`.
5. Data/comparison: `MetricCard`, `StatGrid`, `ComparisonTable`, `PricingCard`.
6. Diagrams: `FlowDiagram`, `ArchitectureMap`, `DiagramPanel`.
7. Templates and export presets across the above.

## Non-goals for this design slice

- Direct manipulation handles, graph interaction, or rich editor inspectors.
- Full Canva/Figma parity.
- Arbitrary React runtime in cells.
- Unvalidated rich HTML props in semantic components.
