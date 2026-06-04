---
id: project-navigation-command-layer
title: "Project Navigation and Command Layer"
status: exploring
parent: editor-surface-v02
tags: [navigation, command-palette, project-view, tabs]
open_questions: []
dependencies:
  - editor-core-substrate
related: []
---

# Project Navigation and Command Layer

## Overview

Make Project view keyboard-first with command palette, quick open, document history, tab commands, reveal/copy-path/rename actions, and integration points for opening Write/Lenses/Graph subviews without footer sprawl.

## Decisions

### Treat Project as the top-level workspace for Write, Lenses, and Graph

**Status:** accepted

**Rationale:** Footer consolidation already groups Write/Lenses/Graph into Project. Editor navigation should preserve that mental model: notes are the editable center, Lenses and Graph are project projections, not independent workspaces.
