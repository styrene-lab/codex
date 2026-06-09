---
id: qa-surface-fixture-release-hardening
title: "QA surface fixture release hardening"
status: seed
tags: [qa, surfaces, release]
open_questions:
  - "[assumption] Markdown embed resolution and Surfaces tab artifact discovery currently use different lookup paths, causing `![[QA Architecture.excalidraw]]` to be visible as a surface but missing as a note/embed."
  - "[assumption] The QA Flow file either fails `flynt_flow::load_flow` validation or is dropped by the flow renderer bridge despite containing nodes/edges."
dependencies: []
related: []
---

# QA surface fixture release hardening

## Overview

Fix release-blocking gaps surfaced by the compact QA vault: markdown embeds do not resolve surface artifacts, and non-empty QA .flow content still renders as an empty starter canvas. The goal is to unify artifact resolution/rendering enough for release QA without expanding the fixture corpus.

## Decisions

### Treat QA vault failures as release hardening defects, not fixture-only problems

**Status:** proposed

**Rationale:** The QA vault is intentionally small and uses normal user-facing constructs. If direct artifact embeds and non-empty flow files fail there, users can hit the same gaps in ordinary projects.

## Open Questions

- [assumption] Markdown embed resolution and Surfaces tab artifact discovery currently use different lookup paths, causing `![[QA Architecture.excalidraw]]` to be visible as a surface but missing as a note/embed.
- [assumption] The QA Flow file either fails `flynt_flow::load_flow` validation or is dropped by the flow renderer bridge despite containing nodes/edges.
