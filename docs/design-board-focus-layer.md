---
id: design-board-focus-layer
title: "Design Board focus layer"
status: exploring
parent: design-board-surface-profiles
tags: [design-board, focus-layer, inspection, agent-context, ux]
open_questions:
  - "[assumption] Iframe-injected click/hover scripts can post focus events to the parent Dioxus view without creating unacceptable security or performance issues."
  - "[assumption] Focus metadata can be added to rendered HTML without changing canonical `.board` JSON serialization."
  - "Should focus state be mirrored in `.flynt/ui-state.json`, ACP session metadata, or both?"
  - "How granular should component-part focus be in v1: cell root only, component root, or named subparts such as title/body/action?"
  - "Should raw HTML cells expose only whole-cell focus by default, or should agent-authored `data-flynt-*` labels be honored in v1?"
related:
  - design-board-visual-substrate
  - design-board-surface-profiles
  - design-sidebar-organization
---

# Design Board focus layer

## Overview

Add a visual focus/inspection layer for Design Board surfaces. The focus layer is distinct from both surface kind and interaction mode: it lets the operator point at a rendered cell/component/region so the agent panel can reason about “this” without requiring full direct-manipulation editing.

This layer should work for agent-generated boards, interactive boards, templates, and references. In non-interactive surfaces it provides selection and agentic revision context, not resize handles or property editing.

## Decisions

### Accepted: focus is a third axis, not interaction mode

**Status:** decided

**Rationale:** `design_board_kind` defines semantic/use-case profile. `design_board_interaction` defines mutation ownership and live editing affordances. The focus layer defines selection/inspection semantics. Agent-generated and reference surfaces should still be focusable even when direct editing is hidden.

### Accepted: focus metadata is rendered, not canonical state

**Status:** decided

**Rationale:** Components and raw-cell wrappers may emit `data-flynt-focus-*` attributes in rendered HTML, but this does not change `.board` JSON. The plain-text canonical source remains component props/raw HTML plus wrapper frontmatter.

### Accepted: focus improves agent precision before direct manipulation

**Status:** decided

**Rationale:** The first useful capability is “operator points at a rendered thing, then asks the agent to change/explain it.” Drag/drop handles, resize handles, and property inspectors are later layers.

## Data attributes

Rendered focusable regions should use stable data attributes:

```html
<section
  data-flynt-focus-kind="component"
  data-flynt-cell-id="hero"
  data-flynt-component="Frame"
  data-flynt-component-part="root"
>
</section>
```

For raw HTML cells, v1 can wrap the body with:

```html
<div
  data-flynt-focus-kind="raw-cell"
  data-flynt-cell-id="custom-html"
  data-flynt-component-part="root"
>
  ...raw cell HTML...
</div>
```

## Focus event contract

Future iframe script should post parent messages shaped like:

```json
{
  "type": "flynt-design-focus",
  "board_path": "boards/Landing.board",
  "cell_id": "hero",
  "focus_kind": "component",
  "component": "Frame",
  "component_part": "title",
  "text_excerpt": "Visual surfaces",
  "bounds": { "x": 742, "y": 724, "w": 1518, "h": 486 }
}
```

The parent view can persist active focus in runtime UI state and expose it to the agent rail.

## Interaction profile matrix

| Interaction | Focus layer | Focus-oriented actions |
| --- | --- | --- |
| `agent_generated` | enabled | ask agent to revise, explain selection, duplicate as interactive |
| `interactive` | enabled | edit props, ask agent to assist, delete selected |
| `template` | enabled | use section, create from template |
| `reference` | enabled | inspect source, ask agent about selection, duplicate as interactive |

## Implementation phases

### Phase 1: core interaction profile contract

- Add `DesignBoardInteraction` and `DesignBoardInteractionProfile` if not already present.
- Add `supports_focus_layer` and `focus_actions` to interaction profiles.
- Test that every interaction profile enables focus but exposes different actions.

### Phase 2: rendered metadata

- Add focus attributes to component renderer roots.
- Wrap raw HTML cells with focus metadata in `build_srcdoc`.
- Test that rendered output contains focus metadata.
- Test that `.board` serialization is unchanged.

### Phase 3: browser/iframe event wiring

- Inject click/hover listener into cell iframes.
- Post `flynt-design-focus` events to parent.
- Store active focus in Dioxus state and UI-state mirror.

### Phase 4: agent rail integration

- Show focused element summary in agent panel.
- Include active focus in agent runtime context/tool metadata.
- Add focused-element CTAs: revise, explain, duplicate/promote.

## Non-goals

- Drag/drop editing.
- Resize handles.
- Full component property inspector.
- Arbitrary DOM mutation from browser inspect.
