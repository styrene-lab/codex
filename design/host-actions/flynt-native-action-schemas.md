+++
id = "flynt-native-action-schemas"
kind = "design_node"
title = "Flynt-native HostAction schemas"
status = "exploring"
tags = ["host-actions", "schemas", "protocol", "flynt-native"]

[data]
issue_type = "feature"
priority = 1
parent = "flynt-host-actions-platform"
+++

# Flynt-native HostAction schemas

## Overview

Define versioned Flynt action families for safe, semantic workspace operations. These are not generic shell commands; they describe Flynt concepts the host can validate, preview, execute, and audit.

## Candidate schemas

### `flynt.document.open@1`

Open a document by path/id/slug, optionally at an anchor and in preview/edit mode.

### `flynt.document.patch@1`

Propose a unified diff or structured markdown patch with preview before write.

### `flynt.task.create@1`

Create a kanban task with title, body, board/column placement, tags, and optional design/OpenSpec links.

### `flynt.canvas.patch@1`

Upsert/delete canvas cells with a before/after preview.

### `flynt.source.open@1`

Open a source artifact or analysis bundle in the appropriate viewer.

### `flynt.validation.run@1`

Semantic validation action that can map to terminal, CI, or internal validation tools.

## Open questions

- [assumption] Flynt-native action schemas can initially live in Flynt and later move to a shared SDK if useful.
- Should actions address resources by path, id, slug, or all three?
- How strict should schemas be about relative paths staying inside the content root?
- Should mutation schemas require an explicit preview payload, or should the host compute previews from params?
- How do action schemas compose into grouped workflows?

## Proposed decisions

### Decision: All Flynt-native action types are versioned

Status: proposed

Use explicit suffixes such as `flynt.task.create@1`. Never accept unversioned action types.

### Decision: Mutation actions are manual by default

Status: proposed

Document, canvas, task, and forge mutation actions require review/confirmation unless a future policy explicitly allows auto execution.
