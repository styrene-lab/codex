---
id: flynt-omegon-cli-contract
title: "Flynt compatibility with Omegon CLI consolidation"
status: decided
parent: flynt-omegon-deployment-contract
tags: [omegon, cli, acp, compatibility, migration]
open_questions: []
related:
  - flynt-acp-runtime-contract
  - flynt-omegon-deployment-contract
---

# Flynt compatibility with Omegon CLI consolidation

## Overview

Flynt embeds Omegon through CLI-spawned ACP. Omegon is consolidating slash and CLI commands, so Flynt must stop scattering raw command spellings across the app.

The embedded agent panel currently depends on these command shapes:

```text
omegon acp --cwd <project> -y [--agent <id>]
omegon auth login <provider>
```

If Omegon renames subcommands or flags, Flynt's agent panel can fail to launch or silently start the wrong profile.

## Decision

Centralize all Omegon CLI command construction behind a Flynt-owned compatibility adapter.

Initial adapter contract:

```rust
OmegonCliContract::current()
  .acp_args(project_root, agent_id)
  .auth_login_args(provider)
```

The first implementation preserves today's stable aliases:

```text
acp --cwd <project> -y --agent <profile>
auth login <provider>
```

Future implementation can discover a machine-readable Omegon command contract and switch flags/subcommands by version.

## Compatibility requirements for Omegon

During the consolidation migration, Omegon should preserve these aliases or expose replacements through a machine-readable contract:

```text
omegon acp
omegon auth login
--cwd
--agent
-y
```

Preferred future probe:

```text
omegon capabilities --json
```

Expected shape:

```json
{
  "cli_contract_version": 1,
  "commands": {
    "acp": {
      "path": ["acp"],
      "flags": { "cwd": "--cwd", "profile": "--agent", "auto_approve": "-y" }
    },
    "auth_login": { "path": ["auth", "login"] }
  }
}
```

## Flynt diagnostic rule

Until runtime contract metadata proves otherwise, Flynt treats the current command contract as `legacy-compatible` rather than `verified-modern`.

## Tests

- ACP args preserve current launch shape.
- Auth login args preserve current login shape.
- Empty provider falls back to `anthropic`.
- Agent/profile argument is omitted when no profile is configured.
