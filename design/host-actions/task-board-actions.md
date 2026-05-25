+++
id = "task-board-actions"
kind = "design_node"
title = "Task and board HostActions"
status = "exploring"
tags = ["host-actions", "tasks", "kanban", "planning", "openspec"]

[data]
issue_type = "feature"
priority = 2
parent = "flynt-host-actions-platform"
+++

# Task and board HostActions

## Overview

Task/board HostActions let agents propose work items, board moves, and planning updates as reviewable operations rather than direct database writes.

## Candidate action families

- `flynt.task.create@1`
- `flynt.task.update@1`
- `flynt.task.move@1`
- `flynt.board.open@1`
- `flynt.board.filter@1`

## Use cases

- Convert a design doc into a set of proposed tasks.
- Create follow-up tasks from an adversarial assessment.
- Move completed tasks after validation passes.
- Open a board filtered to a release, design node, OpenSpec change, or engagement.

## Open questions

- [assumption] Tasks remain persisted through Flynt's existing task store and task-file projection machinery.
- Should task-create actions dedupe against existing titles/tags/design-node links?
- Should task update/move actions be undoable by default?
- How should grouped task creation handle partial approval?
- Should OpenSpec/design-tree lifecycle transitions also become HostActions?

## Proposed decisions

### Decision: Task creation actions are manual-only in v1

Status: proposed

Agents can propose tasks, but Flynt should show the task list for approval/edit before creating them.
