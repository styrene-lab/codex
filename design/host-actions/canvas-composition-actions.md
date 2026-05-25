+++
id = "canvas-composition-actions"
kind = "design_node"
title = "Canvas composition HostActions"
status = "exploring"
tags = ["host-actions", "canvas", "design", "preview", "composition"]

[data]
issue_type = "feature"
priority = 2
parent = "flynt-host-actions-platform"
+++

# Canvas composition HostActions

## Overview

Canvas composition actions let agents propose visual updates without silently mutating the canvas. The host computes a preview, shows cell-level changes, and applies only with operator approval.

## Candidate action families

- `flynt.canvas.open@1`
- `flynt.canvas.patch@1`
- `flynt.canvas.apply_theme@1`
- `flynt.canvas.capture_viewport@1`

## Use cases

- Turn a design node into a canvas architecture sketch.
- Convert source evidence into an evidence map.
- Apply a theme suggested by a design brief.
- Capture viewport after applying changes for visual validation.

## Open questions

- [assumption] Canvas patches can be represented as upsert/delete cell operations compatible with existing `canvas_set_cells` semantics.
- Should theme changes and cell changes be one action group or separate actions?
- What preview granularity is required: cell count, rendered screenshot, structural diff, or all three?
- Should canvas capture be a HostAction or remain an internal design-tool operation?
- How do we handle active-canvas targeting safely when multiple tabs are open?

## Proposed decisions

### Decision: Canvas patch actions require preview

Status: proposed

Canvas patches are visual mutations. The host must show a summary and ideally a rendered preview before applying.
