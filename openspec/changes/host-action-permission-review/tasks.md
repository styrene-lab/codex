# Tasks

## 1. ACP permission bridge
<!-- specs: host-actions/permission-review -->

- [x] 1.1 Add Flynt-local permission review structs and decision enum in `acp.rs`.
- [x] 1.2 Replace auto-allowing `request_permission` with event emission plus decision await.
- [x] 1.3 Add deterministic allow/reject option selection helpers and tests.

## 2. AgentRail review cards
<!-- specs: host-actions/permission-review -->

- [x] 2.1 Add `ChatItem::PermissionReview` and review state model.
- [x] 2.2 Render pending review cards with raw input summary and approve/reject buttons.
- [x] 2.3 Wire approve/reject buttons to the pending ACP responder.

## 3. terminal.create@1 execution
<!-- specs: host-actions/permission-review -->

- [x] 3.1 Add terminal HostAction extraction helpers and tests.
- [x] 3.2 On approve, execute recognized terminal requests through `TerminalManager`.
- [x] 3.3 Mark card approved with terminal result or failed with error.
- [x] 3.4 Ensure ACP approval is returned only after successful terminal creation.
