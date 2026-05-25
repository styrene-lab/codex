+++
id = "flynt-host-actions-platform"
kind = "design_node"
title = "Flynt HostActions platform"
status = "exploring"
tags = ["host-actions", "omegon-0.24", "agent-ui", "safety", "platform"]

[data]
issue_type = "epic"
priority = 1
+++

# Flynt HostActions platform

## Overview

Omegon 0.24 introduces HostActions: declarative, host-managed side effects returned by extensions or requested through `actions/execute`. Flynt can use this as the trust boundary between agent reasoning and workspace mutation.

The platform direction is to make Flynt the host UI that receives proposed actions, validates them against project/operator policy, previews the effect, executes approved actions, and records outcomes.

Related child nodes:

- [[design/host-actions/terminal-validation-actions|Terminal validation actions]]
- [[design/host-actions/flynt-native-action-schemas|Flynt-native HostAction schemas]]
- [[design/host-actions/action-review-and-journal|Action review and journal]]
- [[design/host-actions/source-artifact-actions|Source artifact actions]]
- [[design/host-actions/canvas-composition-actions|Canvas composition actions]]
- [[design/host-actions/task-board-actions|Task and board actions]]

## Problem

Flynt agents increasingly need to perform side effects: opening viewers, running tests, creating tasks, patching canvases, exporting bundles, and preparing forge changes. Direct mutation from tools is too blunt. Text-only suggestions are too weak.

HostActions offer a middle path: tools return action intent; Flynt owns rendering, policy, confirmation, execution, audit, and undo strategy.

## Capabilities unlocked

- Agent responses can include actionable buttons/cards instead of instructions.
- Mutations can be previewed and approved before execution.
- Terminal validation becomes a host-managed action rather than a shell string buried in prose.
- Flynt-native surfaces can expose semantic actions: open document, patch canvas, create task, open source artifact.
- MCP-origin actions can remain deny-by-default while still preserving action metadata for Omegon-aware hosts.

## Open questions

- [assumption] Flynt will receive HostActions through ACP/MCP metadata in a shape compatible with Omegon 0.24's `_meta["omegon/hostActions"]` bridge.
- What part of HostAction execution belongs in Omegon vs Flynt for Flynt-native action types?
- Should Flynt-native action schemas live in the Flynt repo, Omegon extension SDK, or a shared Styrene protocol crate?
- What is the minimum action preview interface that is useful across documents, tasks, canvases, and terminals?
- How do we model undo for actions whose side effects leave Flynt, such as forge issue creation or terminal process launch?

## Initial decisions

### Decision: Treat HostActions as proposed workspace operations, not automatic tool side effects

Status: proposed

Flynt should default to manual/reviewed execution for workspace mutations. Automatic execution can be introduced later for safe navigation actions or trusted internal action families.

### Decision: Start with terminal and navigation actions before mutation actions

Status: proposed

The adoption order should be:

1. `terminal.create@1` for validation/local viewers.
2. Flynt navigation actions such as document/source open.
3. Manual-only task creation.
4. Previewed document/canvas patches.
5. Remote forge actions.

## Implementation notes

- File scope likely includes `crates/flynt-app/src/acp.rs`, `crates/flynt-app/src/components/agent_rail.rs`, `crates/flynt-agent/src/extension.rs`, and new schema/types in `flynt-core` or `flynt-models`.
- HostAction support must be feature/version gated until Omegon 0.24 lands.
- Every action family needs tests for parse, deny, preview, execute, and audit behavior.
