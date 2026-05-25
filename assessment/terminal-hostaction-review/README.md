+++
id = "2d9df4fa-c672-4dc4-a363-b93ed21bdd9e"
tags = []
aliases = []
imported_reference = false

[publication]
enabled = false
visibility = "private"
+++

# Terminal HostAction Review Assessment

Use this workspace to test the end-to-end Flynt/Omegon terminal HostAction flow.

## Purpose

Verify that a real Omegon ACP permission request for `terminal.create@1` is reviewed by Flynt, approved by the operator, executed through `TerminalManager`, and opened in Terminal Lab.

Expected flow:

```text
Omegon requests terminal.create@1
→ Flynt shows AgentRail review card
→ operator clicks Approve
→ TerminalManager creates session
→ Flynt switches to Terminal Lab
→ created terminal is selected and interactive
```

## Test prompt to send in AgentRail

Ask Omegon something like:

> Request a reviewed terminal.create@1 HostAction to run `cargo check -p flynt-app` in this workspace. Do not run it directly; ask Flynt to open a terminal for the command.

If Omegon does not emit the HostAction, try the more explicit version:

> Use the host action `terminal.create@1` with params `{ "command": "cargo", "args": ["check", "-p", "flynt-app"], "cwd": "/Users/wilson/workspace/styrene-labs/flynt", "placement": "bottom_pane", "reuse_key": "cargo-check-flynt-app", "title": "cargo check flynt-app" }`. Request permission from Flynt before execution.

## What to verify

- [ ] AgentRail displays a permission review card instead of auto-allowing.
- [ ] The card summarizes command, args, cwd, placement, and reuse key.
- [ ] Reject path returns reject/cancel and creates no terminal.
- [ ] Approve path creates a terminal session.
- [ ] Flynt switches to Terminal Lab after approval.
- [ ] The created terminal appears in the session strip.
- [ ] The created terminal is selected automatically.
- [ ] Terminal output is readable and interactive.
- [ ] Reusing the same `reuse_key` reuses the existing session instead of spawning duplicates.

## Expected raw input shape

Flynt currently recognizes either wrapped action input:

```json
{
  "action": "terminal.create@1",
  "params": {
    "command": "cargo",
    "args": ["check", "-p", "flynt-app"],
    "cwd": "/Users/wilson/workspace/styrene-labs/flynt",
    "placement": "bottom_pane",
    "reuse_key": "cargo-check-flynt-app",
    "title": "cargo check flynt-app"
  }
}
```

or direct params:

```json
{
  "command": "cargo",
  "args": ["check", "-p", "flynt-app"],
  "cwd": "/Users/wilson/workspace/styrene-labs/flynt",
  "placement": "bottom_pane",
  "reuse_key": "cargo-check-flynt-app",
  "title": "cargo check flynt-app"
}
```

## Likely mismatch to watch for

If the review card appears but approval does not create a terminal, inspect the ACP `raw_input` shape in the card. The likely next patch is to expand `host_actions::terminal::extract_terminal_create` to match Omegon's actual emitted shape.

## Useful local validation commands

```bash
cargo test -p flynt-app terminal --lib
cargo test -p flynt-app terminal_create --lib
cargo check -p flynt-app
```

## Cleanup

The test may create or reuse terminal sessions with ids like:

```text
term-cargo-check-flynt-app
```

Use the Terminal Lab **Kill** / **Release** buttons to clean up sessions after testing.

## Correct 0.24 HostAction exercise path

Do **not** test this by asking for a generic terminal or `ls`. The model will choose the ordinary `bash` tool, which bypasses HostActions entirely.

The HostAction-producing extension installed on this machine is `omegon-reader`. Exercise HostActions through its tools:

1. First check readiness:

   > Run `reader_doctor` and report whether HostActions are available.

2. Then emit the declarative action without execution:

   > Use `reader_open_dry_run` with path `/Users/wilson/workspace/styrene-labs/omegon-extensions/omegon-reader/validation/sample-book.txt`.

3. Then attempt execution:

   > Use `reader_open` with path `/Users/wilson/workspace/styrene-labs/omegon-extensions/omegon-reader/validation/sample-book.txt` and `execute = true`.

Expected with current Omegon 0.24.x:

- `reader_open_dry_run` should return a tool result containing a `terminal.create@1` HostAction envelope.
- `reader_open execute=true` may execute inside Omegon's own HostAction runtime and report `actual_placement = "background_session"`.
- If Flynt does not show a review card, the missing integration is that Flynt currently listens for ACP `request_permission`, while Omegon 0.24 HostActions are surfaced through tool-result `details.host_action_outcomes` / MCP metadata rather than an ACP client permission request.

Flynt follow-up if no review card appears:

- Extend ACP tool-call handling to preserve raw output/details metadata.
- Detect `host_actions` / `host_action_outcomes` in tool-call updates.
- Render those as HostAction cards in AgentRail instead of relying only on `request_permission`.
