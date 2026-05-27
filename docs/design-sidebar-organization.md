---
id: design-sidebar-organization
title: "Design-oriented sidebar organization"
status: exploring
parent: design-board-visual-substrate
tags: [sidebar, design-board, navigation, design-mode, operator-ux]
open_questions:
  - "[assumption] Adding a first-class Design mode can happen without replacing the existing NotesView wrapper/tab routing for Design Boards, Excalidraw drawings, and Flow files."
  - "[assumption] Document metadata is sufficient to discover Design Board wrappers and drawings in the first slice without parsing every file body on each sidebar refresh."
  - "Should Design mode be a new `Route::Design`, or should it be a sidebar filter over `Route::Notes` while the main content still opens documents?"
  - "Should Design Board kind classification live in wrapper frontmatter first, `.board` JSON first, or both with wrapper metadata as the operator-facing index?"
  - "How should the Design sidebar expose component examples: copy JSON, insert via agent, or open docs only in the first slice?"
related:
  - design-board-visual-substrate
  - design-board-component-registry
---

# Design-oriented sidebar organization

## Overview

Disaggregate Flynt's left panel so it stops treating every visual surface as just another markdown wrapper in a generic file tree. The left panel currently mixes workspace navigation, surface selection, bookmarks, route switching, and creation affordances. That worked when Flynt was mostly notes/tasks/graph. It will not scale now that Design Board is becoming a flexible visual substrate for websites, PDFs, resumes, whiteboards, diagrams, brochures, and product collateral.

The target organization separates two jobs:

1. **Workspace navigation** — what artifacts exist in this project?
2. **Surface/tool orientation** — what kind of thing am I working on, and which affordances belong to it?

Design becomes a first-class mode in the sidebar, but the first implementation should preserve existing tab/wrapper routing to avoid destabilizing the editor.

## Reference pillars

Keep the same product pillars from [[design-board-visual-substrate]] visible in this UI work:

- **Claude pillar:** fast generative flow remains available; the operator should see what the agent can render without needing to memorize prompts.
- **Canva pillar:** templates, brand/style, assets, and export affordances belong in Design mode, not buried in a file tree.
- **Figma pillar:** components, variants, frames, and layout primitives should be discoverable as design-system vocabulary, even before drag/drop or inspector UX exists.

## Proposed sidebar architecture

### 1. Narrow left rail: primary modes

Stable, mode-level navigation:

- Project
- Design
- Tasks
- Graph
- Search / Lenses
- Terminal
- Settings

`Design` is a first-class mode, not a note-folder convention.

### 2. Mode-specific sidebar body

The content area of the sidebar changes by mode.

#### Project mode

Document-oriented workspace navigation:

- notes
- sources
- design docs
- task files if file-backed
- generic markdown
- current bookmarks panel

This is the cleaned-up successor to the current file tree.

#### Design mode

Visual artifact organization:

```text
Design
  Boards
    Website mockups
    Documents / PDFs
    Resumes
    Whiteboards
    Brochures
    Dashboards
    Diagrams
    Research
    Unsorted
  Drawings
    Excalidraw
  Flows
    .flow diagrams
  Templates
    Landing page
    Resume
    Brochure
    Whiteboard
    Architecture overview
  Components
    Layout
    Typography
    Media
    Actions
    Cards
    Data
    Diagram
  Assets
    Images
    Logos
    PDFs
```

#### Tasks mode

Kanban boards, task lenses, task notes, and task-specific creation affordances.

#### Graph mode

Graph view presets, saved graph filters, and graph-related navigation.

#### Terminal mode

Terminal sessions and HostAction review/placement affordances.

### 3. Design Board kind metadata

Design Boards need lightweight operator-facing classification so the sidebar can group them by intent.

First-slice wrapper frontmatter:

```toml
+++
title = "Personal Resume"
tags = ["design_board"]
design_board_kind = "resume"
design_board_template = "resume-one-page"
+++

![[Personal Resume.board]]
```

Canonical `design_board_kind` values:

- `website`
- `document`
- `resume`
- `brochure`
- `whiteboard`
- `diagram`
- `dashboard`
- `research`
- `other`

Decision direction: wrapper frontmatter is the operator-facing index first; `.board` JSON remains render-critical state.

### 4. Design sidebar tabs or sections

The Design sidebar should expose three primary sections:

```text
Design
  Files
  Templates
  Components
```

#### Files

Actual project artifacts:

- Design Board wrappers/data files
- Excalidraw drawings
- Flow files
- relevant media assets

#### Templates

Starting points:

- Landing page
- SaaS/product one-pager
- Resume
- Product brochure
- Personal whiteboard
- Architecture overview
- Dashboard
- Research/source board

#### Components

Registry vocabulary grouped by category:

- Layout: `Frame`, `Columns`, `Stack`, `Panel`
- Typography: `TextBlock`
- Media: `ImagePlaceholder`
- Actions: `ButtonRow`
- Future: website, document, whiteboard, data, diagram groups

First slice can display names, categories, descriptions, variants, and examples. Drag/drop and property inspectors are explicitly deferred.

### 5. Current-surface summary

When a Design Board is active, Design mode can show a compact board summary:

```text
Current Board
  Kind: Website mockup
  Theme: Alpharius
  Grid: 12×8
  Cells: 7
  Template: Landing page

Actions
  Change theme
  Duplicate board
  Export image
  Export PDF
  Ask agent to revise

Components used
  Frame
  TextBlock
  ButtonRow
  ImagePlaceholder
```

This is a contextual orientation surface, not a full inspector.

## Implementation nodes

### Node A — Route and shell split

Add a first-class Design mode route/shell while preserving existing document-opening behavior.

Likely files:

- `crates/flynt-app/src/state.rs`
- `crates/flynt-app/src/app.rs`
- `crates/flynt-app/src/components/sidebar.rs`
- `crates/flynt-app/src/components/command_palette.rs`
- `crates/flynt-app/src/ui_state.rs`

Acceptance:

- Sidebar has a Design mode entry.
- `Route::Design` is persisted in UI state.
- Existing Notes, Tasks, Graph, Lenses, Terminal, Settings routes still work.
- Opening a Design Board from Design mode still routes through the existing document tab/wrapper path if needed.

### Node B — Artifact classification and discovery

Classify design artifacts for sidebar display.

Artifacts:

- Design Board wrappers tagged `design_board`
- `.board` files with wrappers
- Excalidraw wrappers/drawings
- `.flow` files
- media assets by extension

Likely files:

- `crates/flynt-core/src/design_board.rs`
- `crates/flynt-app/src/components/sidebar.rs`
- possibly a new `crates/flynt-app/src/design_inventory.rs`

Acceptance:

- Design mode lists boards, drawings, and flows separately.
- Boards can be grouped by `design_board_kind` where present.
- Missing kind falls into `Unsorted` or `Other`.

### Node C — Design mode panel UI

Render the first design-oriented sidebar body.

Sections:

- Files
- Templates
- Components

Acceptance:

- Component groups are populated from `flynt_core::design_components::list_components()`.
- Template placeholders are visible even before full template creation lands.
- Clicking artifacts opens existing tabs/routes without new editor semantics.

### Node D — Template creation affordance

Extend board creation to understand template and kind.

Likely files:

- `crates/flynt-core/src/design_board.rs`
- `crates/flynt-app/src/components/command_palette.rs`
- `crates/flynt-app/src/menu.rs`
- future `crates/flynt-core/src/design_templates.rs`

Acceptance:

- New Design Board can be created as blank or from a visible template choice.
- Wrapper frontmatter stamps `design_board_kind` and `design_board_template`.
- First templates can be placeholders until template JSON is ready.

### Node E — Current board summary

Display contextual info for the active Design Board.

Acceptance:

- When a board wrapper is active, Design mode shows theme, grid, cell count, kind, and components used.
- Summary is read-only in the first slice.

## First implementation slice

Recommended smallest useful change:

1. Add `Route::Design`.
2. Add a Design nav button in the sidebar.
3. Add `DesignPanel` sidebar content for `Route::Design`.
4. `DesignPanel` lists:
   - Design Boards from docs tagged `design_board`
   - Excalidraw drawings/wrappers
   - Flow files
   - Components from `flynt_core::design_components::list_components()`
5. Clicking a board/drawing/flow opens through the existing Notes/document tab path where possible.
6. Do not implement drag/drop, property editing, graph interactions, or rich inspectors.

## Adversarial assessment

### Risk: becoming a Canva clone too early

If the sidebar prioritizes templates/assets/export before rendering semantics are strong, we create a broad but shallow UI.

Countermeasure: first Design mode shows templates/components as orientation and creation vocabulary, not as a full asset editor.

### Risk: becoming a Figma clone too early

If the sidebar introduces components with expectations of drag/drop, variants editing, and layout inspectors, the interaction surface will outrun the renderer.

Countermeasure: component palette is read-only/discoverable first. Insert/edit remains agent-driven or JSON/tool-driven until direct manipulation is explicitly designed.

### Risk: remaining too Claude-like and prompt-only

If everything stays hidden behind the agent, operators cannot build a mental model of the visual substrate.

Countermeasure: Design mode exposes the available render vocabulary, templates, and artifact categories without requiring prompt memorization.

## Non-goals

- Direct manipulation component placement.
- Full property inspector.
- Template marketplace/library management.
- Asset management beyond basic listing.
- Replacing NotesView wrapper routing in the first slice.
- Graph or diagram interaction redesign.
