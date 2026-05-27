---
id: design-board-plain-text-canon
title: "Design Board plain-text canonical state"
status: seed
parent: design-board-visual-substrate
tags: []
open_questions:
  - "[assumption] `.board` JSON remains the only editable board-state format through v1; templates and exports derive from it rather than replacing it."
  - "Should intentional operator exports default to `exports/design/`, while preview/cache exports default to `.flynt-local/exports/design/`?"
dependencies: []
related: []
---

# Design Board plain-text canonical state

## Overview

Keep Design Board backing files as plain-text source of truth. `.board` JSON, wrapper markdown/frontmatter, template files, themes, and text diagram sources are canonical; PNG/PDF/SVG/static HTML are generated exports or referenced assets, not editable state.

## Decisions

### Plain-text source of truth

**Status:** decided

**Rationale:** Design Board files must remain reviewable, diffable, patchable, and agent-editable. Binary exports are useful deliverables but cannot become the editable canonical state in v1.

### Exports are generated edge artifacts

**Status:** decided

**Rationale:** PNG, PDF, SVG, and static HTML outputs are produced from `.board` JSON and related text assets. They may be intentionally saved by the operator, but reopening/editing must not depend on them.

### Referenced assets stay external

**Status:** decided

**Rationale:** Image/media assets can be referenced by path with metadata such as alt text, crop, fit, and caption. The board should not embed base64 image blobs by default because that destroys diffability.

## Open Questions

- [assumption] `.board` JSON remains the only editable board-state format through v1; templates and exports derive from it rather than replacing it.
- Should intentional operator exports default to `exports/design/`, while preview/cache exports default to `.flynt-local/exports/design/`?
