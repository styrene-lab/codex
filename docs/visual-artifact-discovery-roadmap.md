+++
title = "Visual Artifact Discovery Roadmap"
tags = ["design","visual-artifacts","diagrams","excalidraw","design-boards"]
+++

# Visual Artifact Discovery Roadmap

---
title: Visual Artifact Discovery Roadmap
status: exploring
tags: [design, visual-artifacts, diagrams, excalidraw, design-boards]
---

# Visual Artifact Discovery Roadmap

## Context

Flynt is moving from raw file-tree rendering toward conceptual visual artifacts. A diagram, drawing, or design board is not just one file; it is an editable source plus generated render artifacts and optional wrapper documents.

Current completed phases:

1. **Core render/status primitives** exist in `flynt_core::visual_artifacts`.
2. **Sidebar D2 freshness badges** use the shared core `RenderStatus` helpers instead of local mtime logic.

The next bottleneck is discovery. Sidebar still owns too much filesystem-specific D2 discovery logic. Before extending the model to Excalidraw and Design Boards, D2 discovery should be centralized.

## Product model

A visual artifact should represent one conceptual object:

```rust
VisualArtifact {
    kind,
    title,
    source_path,
    wrapper_path,
    renders,
}
```

Where:

- `source_path` is the editable canonical source (`.d2`, `.excalidraw`, `.board`, `.flow`).
- `wrapper_path` is optional markdown navigation/indexing glue.
- `renders` are generated outputs (`svg`, `png`, `html`) with freshness state.

Rendered artifacts should not flood the sidebar as separate conceptual documents. They should appear as badges/actions attached to the primary source row.

## Phase 3 — D2 visual artifact discovery

### Goal

Add centralized D2 discovery, probably in `flynt_core::visual_artifacts`:

```rust
pub fn discover_d2_artifacts(project_root: &Path) -> Vec<VisualArtifact>
```

It should scan:

```text
<project>/diagrams/**/*.d2
```

and produce one `VisualArtifactKind::D2Diagram` per `.d2` source.

### Required behavior

- Discover nested `.d2` files under `diagrams/`.
- Ignore `.svg` and `.png` as primary artifacts.
- Preserve hierarchy via `source_path` relative to project root.
- Attach SVG/PNG render artifacts with freshness state.
- Detect wrappers like markdown documents whose body is a single embed:

```md
![[foo.d2]]
```

and set `wrapper_path` instead of treating the wrapper as a separate diagram artifact.

### Render path strategy

Candidate lookup order:

1. Sibling render next to source:
   - `diagrams/d2/foo.svg`
   - `diagrams/d2/foo.png`
2. Canonical rendered directory:
   - `diagrams/rendered/foo.svg`
   - `diagrams/rendered/foo.png`

If no render exists, choose canonical future output path:

```text
diagrams/rendered/foo.svg
diagrams/rendered/foo.png
```

This avoids encouraging generated files to live beside source files long-term.

### Tests

Add tests for:

- nested `.d2` discovery
- `.svg`/`.png` ignored as primary artifacts
- sibling render selected when present
- rendered directory fallback selected when sibling absent
- stale/current/missing status calculation
- wrapper pairing

## Phase 4 — Sidebar migration to D2 artifacts

Replace sidebar D2 filesystem crawling with core discovery:

```rust
for artifact in discover_d2_artifacts(&ctx.project_root()) {
    add_visual_artifact_row(...)
}
```

Sidebar display should remain:

```text
foo.d2  [SVG current/stale/missing] [PNG current/stale/missing]
```

But sidebar should no longer compute render paths/status itself.

## Phase 5 — Excalidraw artifact discovery

After D2 discovery is centralized, add discovery for:

```text
drawings/foo.md
drawings/foo.excalidraw
drawings/rendered/foo.svg
drawings/rendered/foo.png
```

Represent as:

```rust
VisualArtifactKind::ExcalidrawDrawing
```

Rules:

- Wrapper and `.excalidraw` scene pair to one artifact.
- Renders appear as badges/actions.
- Opening the artifact should use the drawing viewer/editor, not raw markdown unless explicitly choosing source/wrapper.

## Phase 6 — Core Flynt Design Boards

Design Boards should be first-class visual artifacts, not just exported pages. They are the composition surface that can consume and arrange the other artifact kinds.

Add discovery for:

```text
boards/foo.md
boards/foo.board
boards/rendered/foo.html
boards/rendered/foo.png
```

Represent as:

```rust
VisualArtifactKind::DesignBoard
```

Rules:

- Board source is canonical.
- Wrapper is navigation/indexing glue.
- HTML/PNG exports are generated render artifacts.
- Design Boards can reference and embed discovered D2 diagrams.
- Design Boards can reference and embed discovered Excalidraw drawings.
- Embedded visual artifacts remain independently editable at their source while the board owns layout/composition.
- Board discovery should preserve dependency relationships so the UI can show when a board consumes a diagram or drawing.
- Board opening should use the board/editor surface, not raw markdown or exported HTML unless explicitly choosing source/export.

Capability target:

```rust
VisualArtifactKind::DesignBoard
  consumes: Vec<VisualArtifactRef>
```

Where consumed artifacts may include:

- `VisualArtifactKind::D2Diagram`
- `VisualArtifactKind::ExcalidrawDrawing`
- future visual artifact kinds

## Phase 7 — Unified artifact actions

Once discovery is shared, add common actions:

```rust
ArtifactAction::Open
ArtifactAction::OpenSource
ArtifactAction::OpenRender(RenderFormat)
ArtifactAction::Regenerate(RenderFormat)
ArtifactAction::RegenerateAll
ArtifactAction::RevealInFinder
```

Backend implementations differ by artifact kind:

- D2: run `d2` into canonical render paths.
- Excalidraw: export scene to SVG/PNG.
- Design Board: export/capture HTML/PNG.

## Non-goals for immediate next phase

- Do not add Excalidraw discovery before D2 discovery is clean.
- Do not implement regeneration buttons until discovery is centralized.
- Do not invent a full `Route::Artifact` yet.
- Do not make render artifacts first-class sidebar rows.

## Known sharp edges

- Sidebar still has too much artifact-specific logic.
- `.d2` opening currently depends on indexing behavior that is not ideal for non-markdown files.
- Wrapper markdown files can collide with source artifact identity.
- Render status exists in both sidebar and viewer; phase 3 should reduce duplication further.
- A future route/view model should distinguish normal notes from visual artifacts.
