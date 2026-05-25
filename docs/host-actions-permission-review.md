---
id: host-actions-permission-review
title: "HostAction permission review plumbing"
status: implementing
parent: flynt-host-actions-platform
tags: [host-actions, acp, terminal]
open_questions: []
dependencies: []
related: []
---

# HostAction permission review plumbing

## Overview

Route ACP permission requests through Flynt review cards instead of auto-allowing, starting with `terminal.create@1` integration into `TerminalManager`.

## Constraints

- Flynt must stop auto-allowing ACP permission requests.
- Permission review must be operator-driven: approve or reject.
- The first concrete HostAction is `terminal.create@1`; avoid creating a generic HostAction framework before a second action exists.
- Terminal execution must use `TerminalManager`, not ad hoc PTY spawning.
- Terminal commands remain argv-based (`command` + `args`), not shell-string execution.
- Persistent approvals (`AllowAlways`) are out of scope until Flynt has policy storage.

## Decisions

### Decision: Bridge ACP permission requests into AgentRail review cards

Status: accepted

`FlyntAcpClient::request_permission` will send a pending review event to the UI and await an explicit operator decision. The current auto-allow behavior is a placeholder and is not acceptable for terminal or mutation HostActions.

### Decision: Execute `terminal.create@1` host-side only after approval

Status: accepted

Approved terminal requests are deserialized into `TerminalCreateParams` and executed through `TerminalManager::create`. If terminal creation fails, Flynt must not return approval to ACP.

### Decision: Keep first placement result card-only

Status: accepted

The first slice can show the created terminal id/backend/placement in the review card. Rich terminal placement/listing can follow once the permission bridge is proven.

## Implementation notes

File scope:

- `crates/flynt-app/src/acp.rs`
  - Add permission request event/decision structs.
  - Replace auto-allow in `request_permission` with UI review await.
  - Add allow/reject option selection helpers and tests.
- `crates/flynt-app/src/components/agent_rail.rs`
  - Add `ChatItem::PermissionReview`.
  - Render pending/approved/rejected/failed review cards.
  - On approve, call `TerminalManager::create` for recognized terminal requests before replying to ACP.
- `crates/flynt-app/src/host_actions/mod.rs`
  - Shared review/extraction helpers if needed.
- `crates/flynt-app/src/host_actions/terminal.rs`
  - Detect and deserialize `terminal.create@1` from ACP raw input.
  - Provide display summary helpers.
- `crates/flynt-app/src/lib.rs`
  - Export new host action module if added.

Tests:

- Permission helper tests choose allow/reject options deterministically.
- Terminal HostAction extraction accepts wrapped `{"action":"terminal.create@1","params":...}` input.
- Terminal HostAction extraction rejects unrelated/raw shell-string shapes.

## Acceptance criteria

1. ACP permission requests no longer auto-allow.
2. Flynt displays a review card for permission requests.
3. Reject returns reject/cancel and creates no terminal.
4. Approve on `terminal.create@1` creates a terminal via `TerminalManager` and returns allow.
5. Local validation passes with targeted tests and `cargo check -p flynt-app`.
