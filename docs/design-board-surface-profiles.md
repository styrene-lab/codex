---
id: design-board-surface-profiles
title: "Design Board surface profiles"
status: seed
parent: design-sidebar-organization
tags: []
open_questions:
  - "What exact page/export presets should ship in the first profile registry: `web`, `letter`, `a4`, `slide`, `social-square`, `brochure`?"
  - "Should profile metadata expose future components/templates that do not exist yet, or only currently implemented renderable components?"
dependencies: []
related: []
---

# Design Board surface profiles

## Overview

Define `design_board_kind` as a surface profile that selects component groups, templates, validation profiles, export targets, and live operator knobs while preserving plain-text `.board` canonical storage.

## Decisions

### Kind selects live affordances, not file format

**Status:** decided

**Rationale:** `design_board_kind = "resume"` means a `.board` source should expose resume-oriented components, templates, validations, page presets, and export actions. It does not mean the source is a PDF.

### Wrapper frontmatter first, board JSON later

**Status:** decided

**Rationale:** The first slice should classify boards via wrapper frontmatter (`design_board_kind`, `design_board_template`) because the document index and sidebar can read it cheaply. A later slice can add optional `.board.kind` and synchronize both.

### Surface profile registry precedes rich UI controls

**Status:** decided

**Rationale:** Before building interactive knobs, define static surface profiles in core so the sidebar and agent tools share canonical product grammar for kinds, recommended components, templates, validations, exports, and primary actions.

## Open Questions

- What exact page/export presets should ship in the first profile registry: `web`, `letter`, `a4`, `slide`, `social-square`, `brochure`?
- Should profile metadata expose future components/templates that do not exist yet, or only currently implemented renderable components?
