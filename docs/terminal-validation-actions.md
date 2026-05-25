---
id: terminal-validation-actions
title: "Terminal validation HostActions"
status: seed
parent: flynt-host-actions-platform
tags: [host-actions, terminal, validation, omegon-terminal]
open_questions: []
dependencies: []
related: []
---

# Terminal validation HostActions

## Overview

Use terminal.create@1-compatible HostActions to let Flynt agents propose host-managed validation terminals while Flynt owns review, policy, lifecycle, and rendering when it is the executing host.

## Research

### Omegon terminal subsystem alignment

Omegon already has a terminal subsystem and terminal.create@1 HostAction contract that Flynt should align with. Relevant Omegon surfaces: core/crates/omegon/src/tools/terminal.rs provides a portable_pty-backed terminal registry with session id/name, command/cwd/pid, transcript path, bounded tail, exit recording, start/send/read/stop/list lifecycle verbs, secure transcript permissions, and caps for max sessions/input/transcripts. core/crates/omegon-extension/src/actions/terminal.rs defines TerminalCreateParams { command, args, cwd, env, title, placement, reuse_key } and TerminalCreateResult { terminal_id, backend, actual_placement, warnings }. core/crates/omegon/src/extensions/host_actions.rs validates terminal.create@1 policy before spawn: argv-only, command allowlist, env deny-by-default/allowlist, cwd root enforcement. core/crates/omegon/src/host_context.rs exposes ACP terminal delegation verbs create_terminal, terminal_output, wait_for_terminal_exit, kill_terminal, release_terminal when an upstream ACP host owns terminal execution.

### Host terminal ownership boundary

Terminal execution should be modeled as host-owned execution behind a shared HostAction contract. Omegon's terminal.create@1 runtime has already drawn the safety boundary: extensions provide argv-shaped intent, manifests/runtime policy validate it, and the host terminal backend returns typed outcomes. Flynt should act as a visual/reviewing host for that same contract, not as an ACP-only terminal passthrough and not as a separate incompatible terminal API.

### Reusable terminal substrate for Auspex

The terminal subsystem should be designed as a reusable Styrene terminal surface rather than a Flynt-only feature. The current spike may mature inside `flynt-app`, but module boundaries should separate reusable terminal contract/session/PTY/parser/renderer code from app-specific HostAction review, project policy, and placement. Auspex should be able to reuse the same `portable-pty` + `alacritty_terminal` substrate with its own analysis-pane placement and policy adapters.

## Decisions

### Align Flynt terminal surface with Omegon terminal.create@1 contract

**Status:** accepted

**Rationale:** Flynt should not invent a parallel terminal action model. Its native terminal subsystem should mirror Omegon's argv-only TerminalCreateParams and TerminalCreateResult shapes plus lifecycle verbs create/read-or-output/wait-or-observe-exit/kill/release. Flynt's local backend remains portable-pty + alacritty_terminal + reusable renderer when Flynt owns execution; ACP host terminal delegation is an alternate backend when an upstream host owns execution. Keep the reusable terminal substrate separate from Flynt-specific policy and UI so Auspex can adopt the same terminal surface.
