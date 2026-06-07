---
title: ACP Session Transcript Resume
status: exploring
tags: [agent, acp, session-history, ux]
date: 2026-06-07
---

# ACP Session Transcript Resume

## Overview

Flynt currently supports switching the active ACP session id through Omegon `session/list` and `session/load`, but it does **not** replay or reconstruct the previous chat transcript in the agent rail.

The current UI copy was corrected from `Resumed session <id>` to `Switched to session <id>` because Flynt only changes the active ACP session id. It does not prove conversational continuity to the operator.

Related implementation:

- `crates/flynt-app/src/acp.rs` — `AcpSession::list_sessions`, `AcpSession::load_session`
- `crates/flynt-app/src/components/agent_rail.rs` — History drawer and session switch UI

## Problem

The operator can select a saved session from History. ACP accepts the session id and Flynt updates its local `session_id`, but Flynt clears the visible chat items and has no transcript payload to render.

This creates a misleading state if the UI says “resumed”: the model may or may not have server-side session context, but the operator cannot see the prior transcript, and the agent may still answer as if it has no local turn context.

## Current behavior

When loading a saved session succeeds, Flynt now:

1. clears the visible chat rail,
2. closes the history drawer,
3. sets the visible session title/id,
4. shows lifecycle copy: `Switched to session <id>.`,
5. inserts an explicit assistant message explaining that Flynt does not currently replay the prior transcript.

This is honest but incomplete.

## Desired behavior

A future implementation should make History resume semantically complete:

```text
History → select session → transcript appears → active session id is set → future prompts continue from that visible context
```

Minimum acceptable UX:

- The operator sees prior user/assistant/tool messages for the selected session.
- The displayed transcript is explicitly tied to the loaded ACP session id.
- If transcript replay is unavailable, the UI says so before allowing the operator to infer continuity.

## Research: Zed ACP session surfaces

We attempted upstream web search for Zed/ACP session transcript behavior on 2026-06-07. Search engines were unavailable from the harness:

- `Zed Agent Client Protocol ACP session history load session transcript`
- `site:zed.dev Agent Client Protocol session load session transcript ACP Zed`
- `Zed ACP Agent Client Protocol session history loadSession transcript`

All searches failed due search-provider parsing/bot-detection errors. No upstream Zed behavior should be asserted from that failed search.

Follow-up research should inspect the ACP schema and Zed source directly, preferably looking for:

- `ListSessionsRequest`
- `LoadSessionRequest`
- transcript/history payload types
- whether Zed treats `load_session` as context-only or transcript-bearing
- whether transcript replay is supplied by agent server, client persistence, or both

## Open questions

- [assumption] ACP `load_session` may switch server-side conversational state without returning transcript messages.
- Does the ACP schema define a session transcript/history request, or must clients persist transcript locally?
- Does Zed replay transcript from its own local storage rather than from ACP?
- Should Flynt persist its own visible chat transcript keyed by ACP session id?
- Should Flynt expose two different operations: `Switch session` and `Restore transcript`?

## Candidate implementation

### Option A — Client-side transcript persistence

Persist Flynt-visible chat items locally keyed by ACP session id.

Pros:

- Works regardless of ACP transcript support.
- Gives operator deterministic visible continuity.
- Can include Flynt-specific UI/tool rendering state.

Cons:

- Transcript may diverge from what the ACP server/model considers context.
- Requires schema/versioning for stored chat items.
- Tool result payloads may be large.

### Option B — ACP transcript fetch

Use an ACP/Omegon transcript surface if available or add one upstream.

Potential surface:

```json
_session/transcript { "session_id": "..." }
```

or schema-level request:

```text
session/transcript
```

Pros:

- Server is source of truth.
- Better match between model context and visible transcript.

Cons:

- Requires upstream ACP/Omegon support.
- Need to understand Zed’s expected behavior first.

### Option C — Hybrid

Use ACP transcript when available; otherwise fall back to Flynt-local transcript persistence with a visible banner:

```text
Transcript restored from Flynt local history; ACP server context may differ.
```

## Preliminary decision

Use the honest interim copy now: **Switched to session**, not **Resumed session**.

Do not claim transcript/context continuity until Flynt can render the prior transcript or explicitly prove the ACP server supplies it.

## Acceptance criteria for future implementation

- Loading a saved session renders prior messages or clearly states no transcript is available.
- The UI distinguishes session-id switching from transcript restoration.
- The agent rail does not imply memory/context continuity when none is visible.
- Upstream Zed ACP behavior is researched from source/schema, not guessed.
- Tests cover successful session switch with transcript, switch without transcript, and failed transcript fetch.
