---
id: flynt-acp-runtime-contract
title: "Flynt ACP runtime contract metadata"
status: exploring
parent: flynt-omegon-deployment-contract
tags: [omegon, acp, runtime, profile, memory, diagnostics]
open_questions:
  - "What Omegon ACP type should carry runtime profile and memory-scope metadata: initialize _meta, session info _meta, or both?"
  - "Should Flynt block ACP usage when active profile is unknown, or warn until Omegon exposes authoritative runtime metadata?"
related:
  - flynt-omegon-cli-contract
  - flynt-acp-deployment-diagnostics
---

# Flynt ACP runtime contract metadata

## Overview

Flynt's deployment manifest and `flynt-agent` extension metadata say what the session *should* be. They do not prove what Omegon actually launched.

To prevent cross-pollination, Omegon ACP must eventually report authoritative runtime state:

```json
{
  "runtime": {
    "active_profile": "flynt-agent",
    "memory_scope": "project",
    "memory_path": "<project-local mind db>",
    "cli_contract_version": 1,
    "slash_contract_version": 1
  }
}
```

## Decision direction

Flynt should consume this metadata when available and classify diagnostics as:

- `Ready` only when actual runtime profile and memory scope match the manifest.
- `Warning` when extension metadata exists but actual profile/memory are unknown.
- `Blocked` when actual profile or memory scope mismatch.

## First implementation slice

Flynt already consumes `_meta.flynt` from ACP initialize/session metadata. The next runtime contract step is an Omegon-side patch to populate:

```json
_meta.flynt.runtime.active_profile
_meta.flynt.runtime.memory_scope
_meta.flynt.runtime.cli_contract_version
```

Flynt's classifier should then use those fields to graduate from "extension metadata observed" to "actual runtime verified".

## Test matrix

- Runtime profile missing + extension metadata present => Warning, not Ready.
- Runtime profile == manifest profile and memory == project => Ready.
- Runtime profile mismatch => Blocked.
- Runtime memory mismatch => Blocked.
- CLI contract mismatch => Blocked or Warning depending on severity.
