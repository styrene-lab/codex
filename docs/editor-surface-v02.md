---
id: editor-surface-v02
title: "Editor Surface v0.2 — CodeMirror-first Markdown Workspace"
status: exploring
tags: [editor, markdown, project-view, codemirror]
open_questions:
  - "[assumption] CodeMirror 6 remains the canonical editor substrate instead of switching to a different editor engine such as Monaco."
dependencies: []
related: []
---

# Editor Surface v0.2 — CodeMirror-first Markdown Workspace

## Overview

Upgrade Flynt's note editor from a basic source/preview surface into a CodeMirror-first Markdown workspace with stable editor state, keyboard-first navigation, and extensibility hooks for wikilinks, search, context, and agent-native editing.

## Decisions

### Prioritize editor substrate before advanced Markdown or agent features

**Status:** accepted

**Rationale:** Autocomplete, context panes, selection-aware agent edits, and project navigation all depend on stable editor identity, selection/cursor state, dirty tracking, and Dioxus↔JS lifecycle. Building those features before the substrate repeats the rendering/lifecycle issues just fixed in flow.

## Open Questions

- [assumption] CodeMirror 6 remains the canonical editor substrate instead of switching to a different editor engine such as Monaco.
