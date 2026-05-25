# HostAction Permission Review

## Intent

Replace Flynt's ACP permission auto-allow placeholder with operator-reviewed HostAction approval, starting with `terminal.create@1`.

## Scope

- Route ACP `request_permission` calls into Flynt UI review cards.
- Let the operator approve or reject each request.
- Execute approved `terminal.create@1` requests through `TerminalManager` before returning ACP approval.
- Display approval/rejection/failure state in the AgentRail review card.

## Out of scope

- Persistent allow/deny policy.
- Generic HostAction registry beyond the terminal create case.
- Full terminal placement UI.
- Shell-string execution.
