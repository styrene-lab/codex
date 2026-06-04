+++
title = "coe-agent legacy inventory and classifier"
tags = ["coe-agent","compatibility","claude-plugin","inventory","migration"]
+++

+++
id = "a41e6536-03cc-4ee1-96bf-586a064cd9f3"
kind = "design_node"

[data]
title = "coe-agent legacy inventory and classifier"
status = "exploring"
issue_type = "compatibility"
priority = 2
parent = "coe-agent-styrene-migration"
dependencies = []
open_questions = []
+++

## Overview

# coe-agent legacy inventory and classifier

---
id: coe-agent-legacy-inventory
title: "coe-agent legacy inventory and classifier"
status: exploring
parent: coe-agent-styrene-migration
tags: [coe-agent, compatibility, claude-plugin, inventory, migration]
related:
  - coe-agent-styrene-migration
  - recro-omegon-extension
open_questions:
  - "[assumption] Legacy Claude plugin files can be classified without executing hooks, scripts, or extension code."
  - "Should the inventory be exposed as a Flynt UI action, an Omegon tool, a CLI command, or all three?"
  - "What classification vocabulary is sufficient for first-pass migration: native_superseded, import_as_skill, import_as_playbook, import_as_policy, import_as_style_guide, import_as_data, manual_review, unsupported_legacy_extension?"
---

# coe-agent legacy inventory and classifier

## Overview

Build a scanner/classifier for legacy agent package layouts using `recro/coe-agent` as the target fixture. The scanner should identify Claude plugin manifests, skills, commands, hooks, prompts, scripts, and old Pi/Flynt extensions, then recommend Styrene destinations.

## Evidence from fixture

`coe-agent` contains:

```text
.claude-plugin/plugin.json
.claude-plugin/marketplace.json
.claude-plugin/claude/skills/*/SKILL.md
.claude-plugin/claude/commands/*.md
.claude-plugin/claude/hooks/hooks.json
.claude-plugin/scripts/hooks/*.sh
extensions/coe-agent.ts
prompts/*.md
```

Observed duplicate concepts include command/prompt pairs for `new-repo`, `oci-login`, and `whoami`.

## Classification targets

| Legacy artifact | Likely classification |
|---|---|
| `chronos` | native_superseded |
| `/cleave` | native_superseded / alias |
| `identity.sh`, `/whoami` | native_superseded / policy |
| `pre-bash.sh` | import_as_policy |
| `session-start.sh`, `session-end.sh` | import_as_policy / native lifecycle |
| `recro-style-guide` | import_as_style_guide |
| `opportunity-eval` | import_as_playbook |
| `write-proposal` | import_as_playbook |
| `scribe` | import_as_data + import_as_playbook |
| `extensions/coe-agent.ts` | unsupported_legacy_extension with native replacement suggestions |

## First implementation slice

1. Detect `.claude-plugin/plugin.json` and `.claude-plugin/marketplace.json`.
2. Parse `SKILL.md` frontmatter names/descriptions.
3. List commands/prompts and detect basename duplicates.
4. List hook files and scripts without executing them.
5. List old extension source files and SDK imports.
6. Emit a migration report suitable for Flynt UI and docs.

## Non-goals

- Execute old Claude hooks.
- Compile old Pi extensions.
- Automatically mutate package content without operator approval.

## Open Questions
