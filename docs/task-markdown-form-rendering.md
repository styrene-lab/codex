---
id: task-markdown-form-rendering
title: "Task markdown form rendering"
status: seed
tags: [tasks, editor, markdown, ui]
open_questions:
  - "[assumption] Task markdown files remain the source of truth, and form controls must update the underlying TOML frontmatter/data fields rather than creating a parallel task state."
  - "Which task metadata fields should render as first-class controls in the editor: title, status, priority, board, column, tags, due date, external refs, design node, OpenSpec change, engagement, execution config, triggers?"
  - "Should task metadata controls appear in the normal markdown editor, the preview/live-preview layer, a task-specific inspector panel, or a hybrid layout?"
  - "How should advanced or sparse fields be represented so common task editing stays clean while sentry/execution/lifecycle metadata remains inspectable?"
dependencies: []
related: []
---

# Task markdown form rendering

## Overview

Improve Flynt-owned task markdown rendering so task metadata fields are presented as structured form controls instead of raw frontmatter/plain text. The goal is to make task files readable and editable in the in-editor experience while preserving markdown as the source of truth.

## Open Questions

- [assumption] Task markdown files remain the source of truth, and form controls must update the underlying TOML frontmatter/data fields rather than creating a parallel task state.
- Which task metadata fields should render as first-class controls in the editor: title, status, priority, board, column, tags, due date, external refs, design node, OpenSpec change, engagement, execution config, triggers?
- Should task metadata controls appear in the normal markdown editor, the preview/live-preview layer, a task-specific inspector panel, or a hybrid layout?
- How should advanced or sparse fields be represented so common task editing stays clean while sentry/execution/lifecycle metadata remains inspectable?
