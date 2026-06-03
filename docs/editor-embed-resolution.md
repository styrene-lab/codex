---
title: Editor Embed Resolution
status: decided
tags: [editor, embeds, artifacts, registry]
---

# Editor Embed Resolution

## Decision

Editor embeds are differentiated by **resolved project identity**, not by filename or extension.

The editor bridge owns portable Markdown mechanics:

- parse embed syntax such as `![[...]]`
- ask a resolver for identity
- render the resolver result
- dispatch a generic open event

Flynt owns local project/artifact semantics:

- resolve refs against notes, aliases, wrapper documents, visual artifacts, source artifacts, and assets
- identify surface kind: note, image, drawing, flow, canvas, board, asset
- route open actions to the correct Flynt surface

File extension or filename matching is a low-confidence fallback only, mainly for image previews when no registry identity is available.

## Problem

A naive implementation classifies embeds by string shape:

```text
.excalidraw → drawing
.flow       → flow
.canvas     → canvas
.png        → image
```

That fails for Flynt because many valid references are symbolic or wrapper-backed:

- `![[Auth Flow]]` may resolve to `flows/Auth Flow.flow`
- `![[System Map]]` may resolve to a drawing wrapper, a note, or an image
- aliases may not match filenames
- wrapper `.md` documents may represent artifact source files
- multiple project objects can share the same slug/title
- future artifact types should not add scattered extension conditionals

## Embed Reference Model

An embed ref is a symbolic project reference until resolved:

```md
![[Auth Flow]]
![[drawings/System Map.excalidraw]]
![[Hero Canvas]]
![[logo.png]]
```

The editor bridge must not infer Flynt-local artifact type from this string except as fallback.

## Resolution Contract

```ts
interface EmbedResolution {
  status: "resolved" | "missing" | "ambiguous";
  ref: string;
  canonicalPath?: string;
  title?: string;
  kind?: "note" | "image" | "drawing" | "flow" | "canvas" | "board" | "asset" | "unknown";
  surface?: "note" | "drawing" | "flow" | "canvas" | "board" | "asset-preview" | "unknown";
  icon?: string;
  label?: string;
  candidates?: EmbedResolution[];
}

interface EmbedResolver {
  resolve(ref: string): EmbedResolution;
  open(resolution: EmbedResolution): void;
}
```

## Bridge Responsibilities

The editor bridge should provide an embed extension that accepts a resolver:

```ts
FlyntEditorCompat.embedExtension({
  EditorView,
  Decoration,
  WidgetType,
  resolver: window.FlyntEmbedResolver,
})
```

It should:

1. parse `![[...]]` embed lines
2. skip rendering when the cursor is editing that embed line
3. call `resolver.resolve(ref)`
4. render according to `EmbedResolution`
5. call `resolver.open(resolution)` on click

It should not hardcode `.excalidraw`, `.flow`, or `.canvas` semantics.

## Flynt Responsibilities

Flynt should hydrate a project embed index derived from project registry and visual-artifact discovery:

```js
window._flyntEmbedIndex = {
  byRef: {
    "auth-flow": {
      status: "resolved",
      ref: "Auth Flow",
      canonicalPath: "flows/Auth Flow.flow",
      title: "Auth Flow",
      kind: "flow",
      surface: "flow",
      icon: "⛓"
    }
  }
}
```

Resolution order should support:

1. exact path
2. slug match
3. title match
4. alias/frontmatter match
5. wrapper target
6. artifact source path
7. image asset path
8. ambiguity detection

## Open Event

The bridge should emit a generic event through the resolver:

```js
window._flyntNotify("editor.embed.open", JSON.stringify(resolution))
```

Rust routes by resolved surface:

```rust
match resolution.surface {
    "drawing" => open_drawing(resolution.canonicalPath),
    "flow" => open_flow(resolution.canonicalPath),
    "canvas" => open_canvas(resolution.canonicalPath),
    "note" => open_note(resolution.canonicalPath),
    "asset-preview" => open_asset_preview(resolution.canonicalPath),
    _ => show_missing_or_ambiguous_state(resolution),
}
```

## Rendering Guidance

### Resolved image

Inline preview, bounded by editor width.

### Resolved local artifact

Render as a chip/card, not inline source content:

```text
📐 Drawing: System Diagram
⛓ Flow: Auth Pipeline
▦ Canvas: Hero Mockup
```

### Missing reference

Render a neutral missing chip with create/open-search affordance later.

### Ambiguous reference

Render an ambiguous chip and surface candidate count. Do not silently choose one.

## Implementation Plan

### Slice 1: resolver-injected embed extension

Move the embed widget/plugin into the editor bridge but require a resolver. No hardcoded Flynt extension policy.

### Slice 2: project embed index

Expose `window._flyntEmbedIndex` and `window.FlyntEmbedResolver` from Rust/JS using project registry and visual artifacts.

### Slice 3: generic open routing

Add `editor.embed.open` handling in Rust and route by resolved surface.

### Slice 4: remove old direct events

Remove direct `open-drawing` embed emission once generic routing is verified.

## Non-goals

- Do not bake Flynt artifact extensions into the generic editor bridge.
- Do not resolve ambiguous refs silently.
- Do not make the editor bridge depend on Rust project registry internals.
