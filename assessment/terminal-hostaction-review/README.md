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

Verify that real Omegon 0.24 HostAction output is visible to Flynt, and determine whether Flynt must consume HostActions from ACP permission requests, tool-result metadata, or both.

Target flow for Flynt:

```text
Omegon/extension emits terminal.create@1
→ Flynt shows AgentRail review card or HostAction outcome card
→ operator can inspect the action/result
→ approved/executed terminal is visible in Terminal Lab or reported as background_session
```

## Launch setup

Use the local Omegon 0.24+ worktree binary, not stale `~/.omegon/versions/v0.21.2`:

```bash
FLYNT_PROJECT="$PWD/assessment/terminal-hostaction-review" \
OMEGON_BIN="$HOME/workspace/styrene-labs/omegon/target/debug/omegon" \
RUST_BACKTRACE=1 \
cargo run -p flynt-app
```

Known local binaries:

```text
~/workspace/styrene-labs/omegon/target/release/omegon  → 0.24.0
~/workspace/styrene-labs/omegon/target/debug/omegon    → 0.24.1
```

## Correct Omegon 0.24 HostAction exercise path

Do **not** test by asking for a generic terminal or `ls`. The model will choose the ordinary `bash` tool, which bypasses HostActions entirely.

The HostAction-producing extension installed on this machine is `omegon-reader`. Exercise HostActions through its tools.

### Step 1 — readiness

Send in AgentRail:

> Run `reader_doctor` and report whether HostActions are available.

Expected:

- The agent calls `reader_doctor`.
- Output reports whether Bookokrat and HostAction support are available.
- No Flynt terminal review card is expected yet.

### Step 2 — declarative terminal.create dry run

Send in AgentRail:

> Use `reader_open_dry_run` with path `/Users/wilson/workspace/styrene-labs/omegon-extensions/omegon-reader/validation/sample-book.txt`.

Expected:

- The agent calls `reader_open_dry_run`.
- The tool result should contain a declarative `terminal.create@1` HostAction envelope.
- If Flynt shows only ordinary tool output and no HostAction/review card, record that as evidence that Flynt is not yet reading HostActions from tool-result metadata.

### Step 3 — execution path

Send in AgentRail:

> Use `reader_open` with path `/Users/wilson/workspace/styrene-labs/omegon-extensions/omegon-reader/validation/sample-book.txt` and `execute = true`.

Expected with current Omegon 0.24.x:

- The agent calls `reader_open`.
- Omegon may execute the HostAction inside its own HostAction runtime.
- The result may report `actual_placement = "background_session"`.
- If Flynt does not show a review card, the result likely arrived as `host_action_outcomes`, not ACP `request_permission`.

## What to verify

- [ ] Flynt is running against Omegon 0.24.x, not v0.21.2.
- [ ] `reader_doctor` is visible/callable.
- [ ] `reader_open_dry_run` returns a `terminal.create@1` HostAction envelope.
- [ ] `reader_open execute=true` returns a HostAction outcome.
- [ ] Note whether Flynt renders a review card, an outcome card, or only plain tool output.
- [ ] If execution happens, note whether placement is `background_session` or a Flynt-visible terminal.
- [ ] Note whether Terminal Lab lists any new session.

## Expected raw HostAction shape

The reader extension should produce an action like:

```json
{
  "id": "open-reader",
  "type": "terminal.create@1",
  "params": {
    "command": "bookokrat",
    "args": ["/Users/wilson/workspace/styrene-labs/omegon-extensions/omegon-reader/validation/sample-book.txt"],
    "cwd": "/Users/wilson/workspace/styrene-labs/omegon-extensions/omegon-reader"
  }
}
```

## Likely Flynt follow-up if no review card appears

Flynt currently handles ACP `request_permission` events. Omegon 0.24 HostActions may instead arrive through ACP tool-call output/details metadata as:

```text
host_actions
host_action_outcomes
_meta["omegon/hostActions"]
```

If no review card appears, patch Flynt to:

1. Preserve ACP tool-call raw output/details metadata.
2. Detect `host_actions` and `host_action_outcomes` in tool-call updates.
3. Render those as AgentRail HostAction cards.
4. For pending executable actions, route approved `terminal.create@1` into `TerminalManager`.

## Useful local validation commands

```bash
cargo test -p flynt-app terminal --lib
cargo test -p flynt-app terminal_create --lib
cargo check -p flynt-app
```

## Cleanup

Use Terminal Lab **Kill** / **Release** buttons for Flynt-created sessions.

Omegon-native HostAction execution may create background terminal sessions owned by Omegon rather than Flynt; inspect Omegon output for transcript/session ids if needed.
