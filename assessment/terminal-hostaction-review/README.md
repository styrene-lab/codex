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

## 2026-05-25 live observation — generic HostAction prompt

Prompt used:

> Run a `cargo check -p flynt-app` in a new terminal using HostAction

Observed result:

- Omegon used its own `terminal` tool directly.
- It started an Omegon-managed background terminal session named `cargo-check-flynt-app` with session id `31f2e34a-1899-42a4-9527-f6bbce921950`.
- It then called `terminal read` to inspect progress.
- Flynt did **not** show a HostAction review card.
- Terminal Lab did **not** become the placement surface for this session.

Conclusion:

- Generic “use HostAction” prompts are still resolved by Omegon as the built-in `terminal` tool, not as a `terminal.create@1` HostAction envelope for Flynt review.
- This is useful evidence but does **not** exercise Flynt's HostAction review path.
- Continue with the reader-extension path (`reader_doctor`, `reader_open_dry_run`, `reader_open execute=true`) to force actual HostAction metadata.

Flynt follow-up remains:

- Detect HostAction metadata/outcomes from ACP tool-call updates.
- Separately consider whether the built-in Omegon `terminal` tool should be host-delegated to Flynt through ACP rather than launching an Omegon-owned background PTY.

## 2026-05-25 live observation — reader_doctor

Prompt used:

> Run reader_doctor and report whether HostActions are available.

Observed result:

- `reader_doctor` was callable from AgentRail.
- Bookokrat was found at `/opt/homebrew/bin/bookokrat`.
- The HostAction command is `bookokrat`.
- The command matches the reader extension manifest policy.
- Omegon reported: HostActions are available.

Conclusion:

- The 0.24.x Omegon runtime and `omegon-reader` extension are correctly loaded.
- HostAction readiness is confirmed at the extension/manifest policy layer.
- Proceed to `reader_open_dry_run` to inspect the declarative `terminal.create@1` envelope path.
