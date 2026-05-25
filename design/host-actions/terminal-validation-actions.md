+++
id = "terminal-validation-actions"
kind = "design_node"
title = "Terminal validation HostActions"
status = "exploring"
tags = ["host-actions", "terminal", "validation", "omegon-0.24"]

[data]
issue_type = "feature"
priority = 1
parent = "flynt-host-actions-platform"
+++

# Terminal validation HostActions

## Overview

Use Omegon 0.24 `terminal.create@1` actions to let Flynt agents propose managed validation terminals: cargo checks, targeted tests, development servers, log tails, and local viewer processes.

## User value

The agent can say “Run validation” as an action card instead of dumping a command into chat. Flynt/Omegon validates the executable, cwd, and env before launching a terminal pane.

## Candidate actions

- Run `cargo check -p flynt-app`.
- Run targeted `cargo test` commands.
- Start `dioxus serve` or local docs viewers.
- Launch artifact readers such as Bookokrat/Eidolon side viewers.

## Decision log

### Decision: Build Flynt terminal surfaces on portable-pty + alacritty_terminal

Status: accepted

The Dioxus-terminal crate was useful for a quick embedded PTY proof, but it is not suitable as Flynt's production terminal widget. Its renderer processes PTY output byte-by-byte and casts bytes directly to `char`, which corrupts UTF-8 prompt glyphs and other Kitty-adjacent terminal output. Flynt will keep the learning but move terminal parsing/state to `alacritty_terminal` and PTY ownership to `portable-pty`, with a Flynt-owned Dioxus renderer.

This preserves the right ownership boundary: Flynt owns terminal execution, review UI, rendering, lifecycle, and policy; ACP/HostActions are proposal transport only.

## Open questions

- [assumption] Flynt can surface HostAction cards in the agent rail from Omegon ACP/tool result metadata.
- Which commands should Flynt's extension manifest allow by default: `cargo`, `just`, `mdserve`, `dioxus`, `bookokrat`?
- Should terminal validation actions reuse panes via `reuse_key` per command family?
- How should completed terminal outcomes be reflected back into Flynt's action card?
- Should Flynt define a higher-level `flynt.validation.run@1` that compiles down to `terminal.create@1`?

## Proposed acceptance criteria

- Agent/tool result can carry a `terminal.create@1` action for a validation command.
- Flynt renders the action with command, cwd, and safety summary.
- Operator can approve/deny the action.
- Approved action opens a managed terminal with a stable title/reuse key.
- Denied/failed/completed outcomes are visible in the agent rail.
