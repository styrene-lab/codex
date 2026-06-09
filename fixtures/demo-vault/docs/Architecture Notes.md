---
title: Architecture Notes
tags: [architecture, demo]
---

# Architecture Notes

The demo vault models Flynt's file-first contract. Portable project metadata is separate from generated local/runtime state.

```text
.flynt/
  config.toml
  local/
  runtime/
.omegon/
  agent-journal.md
```

## Surfaces

- Write reads markdown files directly.
- Graph projects links and artifact relationships.
- Tasks indexes task markdown into board views.
- Design discovers wrappers for boards, drawings, diagrams, and flows.

See [[diagrams/surface-map|Surface Map]].


Related: [[notes/Product Brief|Product Brief]]
