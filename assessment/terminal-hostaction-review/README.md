---
title: Terminal HostAction Review Assessment
status: ready
tags: [assessment, host-actions, terminal, acp]
---

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
