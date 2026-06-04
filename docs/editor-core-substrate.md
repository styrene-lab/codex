---
id: editor-core-substrate
title: "Editor Core Substrate"
status: exploring
parent: editor-surface-v02
tags: [editor, codemirror, substrate]
open_questions:
  - "What is the minimum stable JS bridge contract for editor mount, unmount, setDocument, getDocument, focus, selection, dirty state, and save events?"
  - "Should autosave be default, manual save default, or hybrid dirty-state with debounce plus explicit Cmd+S flush?"
dependencies: []
related: []
---

# Editor Core Substrate

## Overview

Replace or stabilize the note editing substrate around CodeMirror 6 as the canonical Markdown editor. The editor must support first-class text editing behavior: per-tab cursor/scroll state, dirty state, autosave/manual save, undo/redo, Markdown syntax highlighting, soft-wrap, and robust Dioxus ↔ JS bridge lifecycle.

## Decisions

### Use CodeMirror 6 compartments and extension-based configuration instead of rebuilding editor for option changes

**Status:** accepted

**Rationale:** CM6 best practice is to compose editor behavior with extensions and use Compartment.reconfigure for dynamic settings such as line wrapping, theme, readonly, keymaps, and live preview modes. Recreating EditorView for routine option changes loses selection/scroll/history and risks lifecycle bugs.

### Bridge should expose structured document/state APIs and avoid direct global EditorView mutation outside the adapter

**Status:** accepted

**Rationale:** The Dioxus app should not reach into window._flyntCM directly. A FlyntEditor adapter should own EditorView, EditorState, compartments, debounce timers, and bridge events. Rust and command palette code should use stable API methods such as getDocument, setDocument, getSelection, replaceSelection, focus, saveNow, and reconfigure.

## Open Questions

- What is the minimum stable JS bridge contract for editor mount, unmount, setDocument, getDocument, focus, selection, dirty state, and save events?
- Should autosave be default, manual save default, or hybrid dirty-state with debounce plus explicit Cmd+S flush?
