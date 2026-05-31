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

## Decisions

### Split action model from executor

**Status:** accepted

**Rationale:** flynt-core should own serializable action/request/capability types, while flynt-app owns route changes, tab opening, wrapper repair, and document-store side effects.

### Begin with existing behavior

**Status:** accepted

**Rationale:** The first implementation should migrate existing sidebar and DesignBoard dependency Open behavior onto ArtifactAction rather than introduce new rendering or patching semantics immediately.

## Open Questions

- [assumption] The initial action executor only needs to preserve existing Open behavior and can defer Render/Inspect/Patch execution to later phases.
- [assumption] RevealSource can initially mean opening the source file as a document/editor tab when indexed or as a virtual document when not indexed.
