---
id: artifact-actions
title: "Unified Artifact Actions"
status: exploring
tags: [visual-artifacts, artifact-actions, phase-7]
open_questions:
  - "[assumption] The initial action executor only needs to preserve existing Open behavior and can defer Render/Inspect/Patch execution to later phases."
  - "[assumption] RevealSource can initially mean opening the source file as a document/editor tab when indexed or as a virtual document when not indexed."
dependencies: []
related: []
---

# Unified Artifact Actions

## Overview

Define and implement a shared action model for visual artifacts so D2 diagrams, Excalidraw drawings, Design Boards, and future Flow artifacts expose consistent open, reveal source, render, inspect, patch, dependency, and wrapper-repair behavior. The model should regularize existing scattered behavior rather than replace every surface at once.

## Research

### Visual Artifact Registry struct set and primitive map

Drafted docs/visual-artifact-registry.md. The proposed model introduces VisualArtifactRegistry, VisualArtifactRecord, ArtifactSource, ArtifactWrapper, RenderArtifact, ArtifactSurfaceCapability, ArtifactMetadata, and VisualArtifactEdge. Initial primitive map covers D2, Excalidraw, image formats, external URIs, HTML/CSS components, Design Boards, and Flow graphs. Key relation distinction: wrapper.md WRAPS foo.excalidraw, architecture.md EMBEDS foo.excalidraw, board.board CONSUMES foo.excalidraw, foo.excalidraw RENDERS_TO foo.svg.

## Decisions

### Split action model from executor

**Status:** accepted

**Rationale:** flynt-core should own serializable action/request/capability types, while flynt-app owns route changes, tab opening, wrapper repair, and document-store side effects.

### Begin with existing behavior

**Status:** accepted

**Rationale:** The first implementation should migrate existing sidebar and DesignBoard dependency Open behavior onto ArtifactAction rather than introduce new rendering or patching semantics immediately.

### Dogfood Excalidraw through Visual Artifact surfaces

**Status:** accepted

**Rationale:** Excalidraw .excalidraw files are source artifacts and .md wrappers are index/open shims; rendering/editing should be resolved through the Visual Artifact action/surface layer rather than NotesView special cases.

### Introduce VisualArtifactRegistry as the artifact source of truth

**Status:** accepted

**Rationale:** Markdown wrappers remain the standardized document interface, but raw artifact identity, wrappers, renders, consumers, and surfaces should be modeled in a typed registry rather than repeatedly inferred from filesystem conventions and wrapper bodies.

### Scope VisualArtifactRegistry to one open project root

**Status:** accepted

**Rationale:** Artifact identity, wrapper paths, renders, consumers, and sync semantics must be bounded to the currently open Flynt project/vault/repo root. Registry records should store project-relative paths and never merge artifacts across roots.

## Open Questions

- [assumption] The initial action executor only needs to preserve existing Open behavior and can defer Render/Inspect/Patch execution to later phases.
- [assumption] RevealSource can initially mean opening the source file as a document/editor tab when indexed or as a virtual document when not indexed.
