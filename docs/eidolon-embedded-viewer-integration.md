---
id: eidolon-embedded-viewer-integration
title: "Eidolon embedded file viewer integration"
status: exploring
parent: obsidian-feature-parity
tags: [eidolon, canvas, viewer, projection, structured-data]
open_questions:
  - "[assumption] Flynt can depend on Eidolon crates directly without creating unacceptable workspace coupling or release friction."
  - "[assumption] .eid files should live under an eidolon/ folder by default, while remaining openable anywhere in the project."
  - "Should the first viewer integration use Eidolon as a Rust library, invoke the Eidolon CLI, or embed the existing web viewer bundle?"
  - "Should .eid wrapper markdown be mandatory like drawings/<name>.md, or optional with direct .eid opening supported?"
  - "What is the minimum Canvas reference cell: link-only, static preview, or interactive mini-view?"
  - "How should imported source files and generated .eid files remain linked for refresh/reconciliation?"
dependencies: []
related:
  - source-task-canvas-projections
  - zotero-research-workspace
---

# Eidolon embedded file viewer integration

## Overview

Integrate Eidolon into Flynt as a dedicated embedded `.eid` file viewer,
analogous to Excalidraw wrappers, while keeping Flynt Canvas focused on
composition. Canvas cells should link to or preview Eidolon views; they should
not become the semantic renderer for complex structured data.

## Decisions

### Accepted: Canvas remains composition, Eidolon owns complex structured views

**Status:** 

**Rationale:** Flynt should not turn Canvas into a universal schema/API/database/ontology graph
renderer. Canvas can reference, preview, and summarize Eidolon views, but deep
structured-data exploration belongs in a dedicated Eidolon viewer.

### Accepted: `.eid` should be a first-class visual file type in Flynt

**Status:** 

**Rationale:** Flynt should recognize `.eid` files and route them to an Eidolon-specific viewer
rather than treating them as ordinary text or Canvas files.

### Accepted: first Canvas integration is reference-card based

**Status:** 

**Rationale:** The first Canvas bridge should be an `eidolon_view_ref` card that opens the
dedicated viewer. Static previews and mini embedded viewports are follow-ups.

### Accepted: the agent should inspect Eidolon projections before summarizing

**Status:** 

**Rationale:** The integrated agent should use Eidolon projection/reasoning outputs as the
source of semantic truth for complex structures instead of inferring canonical
groupings from raw source files or rendered Canvas cells.

### Canvas remains composition, Eidolon owns complex structured views

**Status:** accepted

**Rationale:** Flynt should not turn Canvas into a universal schema/API/database/ontology graph renderer. Canvas can reference, preview, and summarize Eidolon views, but deep structured-data exploration belongs in a dedicated Eidolon viewer.

### .eid should be a first-class visual file type in Flynt

**Status:** accepted

**Rationale:** Flynt should recognize .eid files and route them to an Eidolon-specific viewer rather than treating them as ordinary text or Canvas files.

### First Canvas bridge is an Eidolon reference card

**Status:** accepted

**Rationale:** The first Canvas integration should be an eidolon_view_ref card that opens the dedicated viewer. Static previews and mini embedded viewports are follow-ups after the link-card workflow proves useful.

### Agent inspects Eidolon projections before summarizing complex structures

**Status:** accepted

**Rationale:** The integrated agent should use Eidolon projection/reasoning outputs as semantic truth for complex structures instead of inferring canonical groupings from raw source files or rendered Canvas cells.

## Open Questions

- [assumption] Flynt can depend on Eidolon crates directly without creating unacceptable workspace coupling or release friction.
- [assumption] .eid files should live under an eidolon/ folder by default, while remaining openable anywhere in the project.
- Should the first viewer integration use Eidolon as a Rust library, invoke the Eidolon CLI, or embed the existing web viewer bundle?
- Should .eid wrapper markdown be mandatory like drawings/<name>.md, or optional with direct .eid opening supported?
- What is the minimum Canvas reference cell: link-only, static preview, or interactive mini-view?
- How should imported source files and generated .eid files remain linked for refresh/reconciliation?
