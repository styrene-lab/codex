+++
id = "action-review-and-journal"
kind = "design_node"
title = "HostAction review and journal"
status = "exploring"
tags = ["host-actions", "audit", "review", "undo", "safety"]

[data]
issue_type = "feature"
priority = 1
parent = "flynt-host-actions-platform"
+++

# HostAction review and journal

## Overview

Flynt should provide a consistent review surface for HostActions and record outcomes in an action journal. This is the operator safety rail for agent-proposed side effects.

## Review surface

Action cards should show:

- Origin: native extension, MCP server, internal Flynt agent, or external connector.
- Action type and version.
- Label and user-facing summary.
- Scope: files, board, canvas, terminal command, source bundle, or remote forge target.
- Risk level: navigation, local mutation, process launch, remote mutation.
- Preview/diff when available.
- Approve, deny, and possibly edit controls.

## Journal records

Each executed or denied action should record:

- action id and host-scoped id
- origin identity
- session/tool-call id
- action type/version
- preview summary
- operator decision
- outcome status
- error code/message if failed
- optional undo handle or inverse action

## Open questions

- [assumption] HostAction outcomes from Omegon include enough origin/session identity for Flynt to display trustworthy provenance.
- Should the journal be local runtime state, portable project metadata, or both?
- How long should action history persist?
- What action families can be made undoable in v1?
- Should denied actions remain visible in the conversation transcript or only in the journal?

## Proposed decisions

### Decision: The first journal is local runtime state

Status: proposed

HostAction audit history may include local paths, process commands, and operator decisions. Default it to local runtime state, with explicit export later if needed.

### Decision: Preview is required for document/canvas patch execution

Status: proposed

Do not execute document or canvas patch actions without a host-computed preview/diff.
