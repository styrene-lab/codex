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

### Research: Omegon terminal substrate alignment

Omegon already has two terminal paths that Flynt should align with rather than reinventing ad hoc:

- `core/crates/omegon/src/tools/terminal.rs` owns an interactive background terminal registry using `portable_pty`, session ids/names, transcript files, bounded tail buffers, max session/input/transcript limits, stop/list/read/send actions, exit recording, and secure transcript directory/file permissions.
- `core/crates/omegon/src/extensions/host_actions.rs` implements `terminal.create@1` through policy validation before spawn, then converts an argv-only `TerminalCreateLaunchPlan` into a `HostTerminalCreateRequest` with `command`, `args`, `cwd`, `env`, and optional name. It returns `TerminalCreateResult { terminal_id, backend, actual_placement, warnings }`.
- `core/crates/omegon-extension/src/actions/terminal.rs` defines the public contract: `TerminalCreateParams { command, args, cwd, env, title, placement, reuse_key }` and `TerminalPlacement::{Default, SidePane, BottomPane, NewTab}`.
- `core/crates/omegon/src/host_context.rs` also exposes ACP host terminal delegation (`create_terminal`, `terminal_output`, `wait_for_terminal_exit`, `kill_terminal`, `release_terminal`) when the ACP client advertises terminal capability.

Flynt's terminal surface should therefore be a visual host/session manager for the same conceptual contract, not a parallel API. Because Auspex has the same need for host-managed validation and analysis terminals, the terminal substrate should be kept reusable: app-agnostic PTY/session/parser/rendering code first, Flynt- or Auspex-specific policy and placement adapters second. The revised working surface is:

1. Reuse Omegon's argv-only action shape: `command`, `args`, `cwd`, `env`, `title`, `placement`, `reuse_key`.
2. Reuse Omegon's result shape: `terminal_id`, `backend`, `actual_placement`, `warnings`.
3. Mirror Omegon lifecycle verbs in Flynt's terminal subsystem: create, read/output snapshot, wait/observe exit, kill, release/close.
4. Mirror Omegon safety defaults: no shell-string path for HostAction terminal creation, env deny-by-default, command allowlist, cwd root enforcement, max sessions/input/transcript caps.
5. Keep the reusable native renderer (`portable-pty` + `alacritty_terminal`) as the local visual backend when the app owns execution; use ACP host terminal delegation only when Flynt/Auspex is not the executing host or when an upstream host explicitly owns the terminal.
6. Keep app-specific concepts (Flynt project roots, agent rail cards, action journal placement, Auspex analysis panes) outside the reusable terminal substrate.

### Decision: Keep the terminal substrate reusable for Auspex

Status: accepted

The terminal subsystem should be designed as a reusable Styrene terminal surface rather than a Flynt-only feature. The current implementation may live inside `flynt-app` while it matures, but module boundaries should separate reusable terminal contract/session/PTY/parser/renderer code from Flynt-specific HostAction review, project policy, and placement. A future extraction target is a shared terminal crate usable by Flynt and Auspex.

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
