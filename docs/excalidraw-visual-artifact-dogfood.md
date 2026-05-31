---
title: Excalidraw Visual Artifact Dogfood Path
status: exploring
tags: [visual-artifacts, excalidraw, dogfood]
---

# Excalidraw Visual Artifact Dogfood Path

## Principle

`.excalidraw` files are Visual Artifact source files. `.md` files beside them are indexable/openable wrappers, not the owner of rendering semantics. Rendering, editing, export, reveal-source, inspection, and dependency behavior should route through the Visual Artifact action/surface layer.

## Step 1 — Stabilize the existing editor path

Goal: make the current Excalidraw editor usable again before changing the surface model.

- Keep current wrapper-to-editor dispatch temporarily.
- Fix the mount target mismatch: Rust must render the DOM id that JS mounts into.
- Avoid broad behavior changes until the editor can render again.
- Validate the Excalidraw view code and wrapper-detection tests.

## Step 2 — Introduce artifact surface resolution

Add an app-side resolver that maps a tab/document/artifact action to an explicit surface:

- `Note`
- `D2Preview { source_path }`
- `ExcalidrawPreview { source_path }`
- `ExcalidrawEditor { source_path }`
- `DesignBoard { source_path }`
- `Flow { source_path }`

The resolver should centralize existing wrapper/body/frontmatter/sibling detection currently scattered across NotesView, UI state, and artifact opening.

## Step 3 — Split Excalidraw preview from editor

Define distinct behavior:

- `Open` shows a preview/read surface using SVG/PNG sidecars when available.
- `Edit` opens the live Excalidraw editor.
- `RevealSource` opens the raw `.excalidraw` source file.
- `Render(Svg/Png)` exports sidecar renders.
- `Inspect` shows source, wrapper, render freshness, and consumers.

This likely requires adding `ArtifactActionKind::Edit` rather than overloading `Open`.

## Step 4 — Move wrapper dispatch out of NotesView

NotesView should ask the Visual Artifact resolver what surface to render instead of hardcoding Excalidraw wrapper behavior. The wrapper remains a document tab, but its body is interpreted through the artifact layer.

## Step 5 — Dogfood sidebar and commands

Sidebar Visual Artifact entries should use artifact actions consistently:

- left click: `Open` preview
- context menu: `Edit`, `Reveal Source`, `Render SVG`, `Render PNG`, `Inspect`

Command palette and agent tools should invoke the same action executor.

## Step 6 — Make Excalidraw rendering contract explicit

Replace singleton remount guessing with an explicit JS contract:

- `mount(container_id, scene_json, on_change)` for first editor attach
- `loadScene(scene_json)` or equivalent `_api.updateScene(...)` for tab/source changes
- `unmount()` for teardown
- `exportSvg()` / `exportPng()` for renders

## Step 7 — Test the artifact lifecycle

Add tests for:

- wrapper resolves to Excalidraw preview
- edit action resolves to Excalidraw editor
- reveal-source opens `.excalidraw`
- missing wrapper repair still works
- wrapper docs are hidden from the file tree when represented by Visual Artifact entries
- switching two drawings does not preserve the first drawing's scene

## Exit criteria

Excalidraw is fully dogfooded when no user-facing path needs to know that a `.md` wrapper is "really" a drawing except the Visual Artifact resolver/action layer.
