# Design: HostAction Permission Review

## Overview

Flynt's ACP client currently auto-selects an allow option in `request_permission`. This change inserts a UI-mediated review bridge. The ACP client emits a pending permission event containing display data and a response channel. AgentRail renders the request and sends an explicit decision back to the waiting ACP method.

## Components

### ACP bridge

`crates/flynt-app/src/acp.rs` owns protocol-facing request/response types. It will add Flynt-local review structs and replace auto-allow with `AcpEvent::PermissionRequested`.

### AgentRail review UI

`crates/flynt-app/src/components/agent_rail.rs` owns chat/review rendering. It will add a `ChatItem::PermissionReview` and buttons for approve/reject.

### Terminal HostAction extraction

`crates/flynt-app/src/host_actions/terminal.rs` parses `terminal.create@1` raw input into `TerminalCreateParams` and produces a concise review summary.

### Terminal execution

Approved terminal requests call `TerminalManager::create`. ACP approval is returned only if local terminal creation succeeds.

## Tradeoffs

- The first version uses card-only terminal result display instead of building placement UI immediately. This proves the permission bridge while keeping the slice bounded.
- Persistent approvals are intentionally ignored; security policy storage should be designed separately.
