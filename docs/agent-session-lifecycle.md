---
id: agent-session-lifecycle
title: "Decouple Flynt agent panel lifecycle from Omegon ACP session lifecycle"
status: exploring
tags: [flynt, omegon, acp, agent-panel, lifecycle]
open_questions:
  - "[assumption] ACP ListSessionsRequest/LoadSessionRequest/CloseSessionRequest are supported by Omegon 0.26.4 and map to persisted/resumable conversation sessions, not only process-local sessions."
  - "[assumption] Flynt can hoist AgentRail session/transcript/status state into app-level context without destabilizing Dioxus signal ownership or the existing event loop."
  - "Should `End session` terminate only the ACP conversation, or also kill the ACP child transport when no active session remains?"
  - "What session metadata should Flynt show in history: title, timestamp, cwd/project, model, provider, last message preview, status?"
dependencies: []
related: []
---

# Decouple Flynt agent panel lifecycle from Omegon ACP session lifecycle

## Overview

Flynt currently treats agent panel visibility, ACP transport ownership, child process lifetime, and Omegon conversation/session lifetime as effectively the same lifecycle. This causes panel close/hide to terminate or strand the session, prevents explicit new/end/resume controls, and makes reconnect/auth flows harder to reason about. Design a lifecycle model and implementation path that matches tools like Zed ACP: close/hide is UI-only; session end/new/resume are explicit operator actions.

## Open Questions

- [assumption] ACP ListSessionsRequest/LoadSessionRequest/CloseSessionRequest are supported by Omegon 0.26.4 and map to persisted/resumable conversation sessions, not only process-local sessions.
- [assumption] Flynt can hoist AgentRail session/transcript/status state into app-level context without destabilizing Dioxus signal ownership or the existing event loop.
- Should `End session` terminate only the ACP conversation, or also kill the ACP child transport when no active session remains?
- What session metadata should Flynt show in history: title, timestamp, cwd/project, model, provider, last message preview, status?
