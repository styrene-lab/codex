---
title: Editor Bridge Contract v0
status: exploring
tags: [editor, codemirror, bridge, markdown]
---

# Editor Bridge Contract v0

## Purpose

Flynt's notes editor already uses CodeMirror 6, but the current interface is an ad hoc global bridge built around `window._flyntCM` and stringly typed `_flyntNotify(type, data)` messages. This contract defines the stable editor API that Rust/Dioxus code should call instead.

The first implementation is a compatibility wrapper around the current CM6 instance. Later work should extract the editor into a dedicated `build/editor` bundle and migrate editor behavior into CM6 extensions/plugins.

## Goals

- Keep `EditorView` ownership inside one JavaScript adapter.
- Stop direct external mutation of `window._flyntCM`.
- Expose structured APIs for document content, selection, focus, dirty state, save, and editor-state snapshots.
- Give Rust a stable event surface independent of CM6 internals.
- Preserve tab state across document switches: cursor, selection, scroll, and dirty flag.
- Avoid rebuilding `EditorView` for routine configuration changes.

## Non-goals for v0

- Full WYSIWYG editing.
- Replacing the current CM6 inline implementation in one step.
- Multi-level undo/redo beyond CM6's own history.
- Complete wikilink autocomplete implementation.

## Public API

The editor bridge is exposed as:

```ts
window.FlyntEditor
```

### `mount(elementId, options)`

Mounts the editor into a DOM element. In v0 this is still handled by the existing `cm6_init_js` path; future bundle extraction should move mount ownership here.

```ts
interface MountOptions {
  document?: EditorDocument;
  config?: Partial<EditorConfig>;
  onEvent?: (event: FlyntEditorEvent) => void;
}
```

### `unmount()`

Destroys the active `EditorView` and clears bridge-owned timers/listeners.

### `setDocument(document, options?)`

Replace the current document content and identity.

```ts
interface EditorDocument {
  id?: string;
  path?: string;
  content: string;
  revision?: number;
}

interface SetDocumentOptions {
  preserveSelection?: boolean;
  preserveScroll?: boolean;
  force?: boolean;
}
```

Safety rule: if the current document is dirty and the incoming document does not match the active document/revision, `setDocument` should refuse unless `force: true`.

### `getDocument()`

Returns the current document snapshot.

```ts
interface EditorDocumentSnapshot {
  id?: string;
  path?: string;
  content: string;
  revision?: number;
  dirty: boolean;
}
```

### `focus()`

Focuses the active editor.

### `getSelection()`

Returns the active selection.

```ts
interface SerializedSelection {
  anchor: number;
  head: number;
  ranges?: Array<{ anchor: number; head: number }>;
}
```

### `replaceSelection(text)`

Replaces the active selection with text and focuses the editor.

### `saveNow()`

Requests an immediate save. In compatibility mode this emits the legacy `save` notification with current content.

### `markSaved(revision?, content?)`

Marks the active document clean after Rust confirms persistence. If `content` is omitted, current editor content becomes the saved baseline.

### `isDirty()`

Returns whether current content differs from the last saved baseline.

### `getEditorState()`

Returns serializable UI state:

```ts
interface SerializedEditorState {
  selection?: SerializedSelection;
  scrollTop: number;
  scrollLeft: number;
  dirty: boolean;
}
```

### `restoreEditorState(state)`

Restores selection and scroll state.

### `reconfigure(config)`

Applies dynamic configuration using CM6 compartments. Future config keys:

```ts
interface EditorConfig {
  lineWrapping: boolean;
  readOnly: boolean;
  livePreview: boolean;
  theme: string;
}
```

## Event API

Future bridge events should be structured:

```ts
type FlyntEditorEvent =
  | { type: "editor.change"; docId?: string; path?: string; content: string; dirty: boolean; revision?: number }
  | { type: "editor.saveRequested"; docId?: string; path?: string; content: string; revision?: number }
  | { type: "editor.selectionChanged"; docId?: string; path?: string; selection: SerializedSelection }
  | { type: "navigation.openWikilink"; target: string; sourceDocId?: string; sourcePath?: string }
  | { type: "artifact.open"; path: string; kind: "drawing" | "flow" | "canvas" }
  | { type: "editor.error"; message: string; stack?: string };
```

Compatibility mode may still emit legacy events:

- `edit`
- `autosave`
- `save`
- `mode`
- `nav`
- `open-drawing`
- preview events

But new callers should use `window.FlyntEditor` methods rather than direct globals.

## CM6 best practices

### Adapter ownership

External code must not mutate `window._flyntCM` directly. The compatibility wrapper may read that global while legacy code remains, but the bridge API is the public interface.

### Compartments

Dynamic behavior should use `Compartment.reconfigure`, not `EditorView` rebuilds, for:

- line wrapping
- readonly
- theme
- keymaps
- live preview mode
- diagnostics
- Markdown intelligence extensions

### Extensions over global listeners

Editor behavior should live in CM6 extensions (`StateField`, `StateEffect`, `ViewPlugin`, `updateListener`) rather than scattered `document` listeners where possible.

### Structured messages

Bridge events should be JSON objects with a `type` field. Avoid magic string events plus untyped payloads for new features.

## Migration plan

1. Add compatibility `window.FlyntEditor` wrapper around current `window._flyntCM`.
2. Migrate command palette/sidebar/tab callers away from direct `window._flyntCM` access.
3. Add Rust-side typed event enum for editor events.
4. Persist per-tab editor UI state.
5. Extract CM6 implementation into `crates/flynt-app/build/editor` and committed `assets/vendor/editor.bundle.js`.
