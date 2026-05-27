+++
id = "adc22f9f-e8b5-41e1-b37c-3660751d2913"
tags = []
aliases = []
imported_reference = false

[publication]
enabled = false
visibility = "private"
+++

## Summary

Flynt needs Omegon to expose `terminal.create@1` HostActions to ACP visual hosts before execution, and to support a host-delegated terminal backend so Flynt can own review, placement, lifecycle, and rendering for terminal actions.

This came out of Flynt's local HostAction assessment against Omegon 0.24.x / 0.24.2. The terminal HostAction substrate is present in Omegon, but native extension actions are currently consumed/executed inside Omegon before Flynt can review or satisfy them.

## What Flynt has implemented locally

Flynt now has:

- A native terminal substrate built on `portable-pty` + `alacritty_terminal`.
- A reusable `TerminalManager` lifecycle:
  - `create`
  - `send_input`
  - `poll_snapshot`
  - `status`
  - `kill`
  - `release`
  - `list`
- `terminal.create@1`-compatible request/result types.
- AgentRail permission/review card plumbing for ACP `request_permission` events.
- Parsing for ACP tool-result HostAction metadata/outcomes:
  - `actions`
  - `host_actions`
  - `hostActions`
  - `_meta["omegon/hostActions"]`
  - `host_action_outcomes`
  - `hostActionOutcomes`
- Detection of ACP `ToolCallContent::Terminal` terminal embeds.

Flynt can create and display a terminal session once it receives a reviewable `terminal.create@1` candidate.

## Current Omegon behavior observed

### Generic terminal prompt

Prompt:

```text
Run a `cargo check -p flynt-app` in a new terminal using HostAction
```

Observed behavior:

- Omegon uses its built-in `terminal` tool directly.
- It creates an Omegon-owned session, e.g. `cargo-check-flynt-app`.
- Flynt only sees normal ACP tool rows / assistant prose.
- No Flynt review card is possible because no reviewable HostAction candidate reaches Flynt.

### Reader extension dry-run

Prompt:

```text
Use reader_open_dry_run with path /Users/wilson/workspace/styrene-labs/omegon-extensions/omegon-reader/validation/sample-book.txt.
```

Relevant reader code:

```rust
ToolResult::text(...).with_action(action)
```

Omegon currently parses that envelope, then immediately processes `ToolResult.actions` in `core/crates/omegon/src/extensions/mod.rs`:

```rust
if !envelope.host_actions.is_empty() {
    let outcomes = host_actions::process_declarative_host_actions(
        envelope.host_actions,
        &self.runtime.manifest,
        &self.runtime.name,
        call_id,
    );
    envelope.host_actions = Vec::new();
    envelope.host_action_outcomes.extend(outcomes);
}
```

So the candidate action is consumed before an ACP host can review it.

## What Flynt requires from Omegon

### 1. Surface pending HostAction candidates to ACP hosts before execution

For declarative native extension `ToolResult.actions`, Omegon needs a path that lets a visual ACP host see and decide on the action before Omegon executes it.

Acceptable shapes include either:

- ACP `request_permission` carrying the HostAction payload in `ToolCallUpdate.raw_output` / metadata, or
- a dedicated ACP host-action notification/update, or
- a host-terminal backend call that delegates directly to the host.

Minimum payload Flynt needs:

```json
{
  "id": "open-reader",
  "type": "terminal.create@1",
  "params": {
    "command": "bookokrat",
    "args": ["/abs/book.txt"],
    "cwd": "/optional/cwd",
    "env": {"OPTIONAL": "value"},
    "title": "Reader: book.txt",
    "placement": "side_pane",
    "reuse_key": "reader-main"
  },
  "origin": {
    "kind": "native_extension",
    "identity": "omegon-reader"
  },
  "tool_call_id": "...",
  "session_id": "..."
}
```

### 2. Add a host-delegated `terminal.create@1` backend

Omegon already has the right internal seam:

- `HostActionExecutorRegistry`
- `TerminalBackendRegistry`
- `TerminalCreateBackend`
- `RealTerminalCreateBackend`

Flynt needs a backend option that delegates terminal creation to the ACP host when available.

Desired selection:

```text
terminal.create@1
→ if ACP host advertises terminal-create support: delegate to ACP host
→ else use Omegon portable PTY/background fallback
```

The host-delegated backend should return a normal `TerminalCreateResult`/HostAction outcome so extensions do not need to know whether Flynt or Omegon satisfied the request.

### 3. Preserve explicit outcomes

Flynt needs Omegon outcomes to distinguish:

- `approved_executed_by_host`
- `rejected_by_host`
- `host_unavailable`
- `fallback_background_session`
- `manifest_denied`
- `unsupported_action`
- `executor_failed`

The current `host_action_outcomes` shape is fine if it carries enough detail.

### 4. Do not auto-execute reviewable visual-host actions before host decision

For visual placements (`side_pane`, `bottom_pane`, `new_tab`) a capable visual host should get first refusal.

A background fallback is useful, but it should be explicit in the outcome. Otherwise Flynt cannot give the operator the review/control surface we are building.

## Why this matters

Flynt is the visual host. It owns:

- terminal panes/tabs
- operator review UX
- local terminal lifecycle and cleanup
- rendering and keyboard input
- workspace-aware placement
- future persistent terminal surfaces

Omegon should remain the agent/runtime/orchestrator. Extensions should continue emitting `terminal.create@1` intent without coupling to Flynt, Zellij, Kitty, Ghostty, etc.

## Suggested implementation direction

1. Add ACP host capability advertisement for `terminal.create@1` / host terminal backend.
2. Add an ACP-backed `TerminalCreateBackend` in Omegon.
3. When processing declarative native extension HostActions, select the ACP host backend before `RealTerminalCreateBackend` if the host supports it.
4. Have the ACP backend request operator decision from the host and await an outcome.
5. Return the same typed HostAction outcome shape Omegon already uses.
6. Keep portable PTY as fallback when no capable host exists.

## Validation scenario

With Flynt running as ACP host:

```text
Use reader_open_dry_run with path /Users/wilson/workspace/styrene-labs/omegon-extensions/omegon-reader/validation/sample-book.txt.
```

Expected future behavior:

1. Omegon receives reader `ToolResult.actions` containing `terminal.create@1`.
2. Omegon delegates review/execution to Flynt because Flynt advertises support.
3. Flynt AgentRail shows a HostAction review card.
4. Operator approves.
5. Flynt `TerminalManager` creates the terminal session and displays it.
6. Omegon receives a completed outcome with host-created terminal id.

## Related Flynt work

Flynt side commits from the assessment:

- native terminal surface and `TerminalManager`
- ACP permission review cards
- `terminal.create@1` parsing
- HostAction metadata/outcome parsing
- ACP embedded terminal id surfacing

Flynt is ready to consume the contract once Omegon exposes/delegates the HostAction before local execution.
