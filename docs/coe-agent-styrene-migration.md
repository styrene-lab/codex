+++
title = "coe-agent Styrene migration umbrella"
tags = ["coe-agent","recro","armory","compatibility","migration"]
+++

+++
id = "f7a84d10-d83f-47df-b6eb-1654eb38256a"
kind = "design_node"

[data]
title = "coe-agent Styrene migration umbrella"
status = "exploring"
issue_type = "architecture"
priority = 2
parent = "flynt-armory-packaging"
dependencies = []
open_questions = []
+++

## Overview

# coe-agent Styrene migration umbrella

---
id: coe-agent-styrene-migration
title: "coe-agent Styrene migration umbrella"
status: exploring
parent: flynt-armory-packaging
tags: [coe-agent, recro, armory, compatibility, migration]
related:
  - flynt-armory-packaging
  - flynt-omegon-artifact-scope
  - flynt-acp-runtime-contract
open_questions:
  - "[assumption] recro/coe-agent should remain available as a legacy Claude Code plugin until Styrene package distribution can generate or replace that format."
  - "Should the migrated Recro content live in omegon-armory as a Recro package, in a separate recro-owned extension repo, or both with one generated from the other?"
  - "What is the minimum package/extension shape needed to load Recro-specific skills in Omegon without vendoring the old .claude-plugin tree?"
  - "Which legacy Claude plugin artifacts must be executable compatibility targets versus inert migration inputs?"
---

# coe-agent Styrene migration umbrella

## Overview

`recro/coe-agent` is a legacy Claude Code/Pi-era agent package and a useful compatibility dataset. It combines runtime mechanics, Recro/GovCon domain skills, Scribe engagement data conventions, commands, prompts, hooks, and an old Pi extension in one repository.

The migration goal is not to rename or vendor the repository wholesale. The goal is to decompose it into Styrene ecosystem layers:

- native Flynt/Omegon runtime capabilities for mechanics,
- Armory/Omegon extension content for Recro-specific skills and playbooks,
- Flynt importers for Scribe data and legacy Claude plugin layouts,
- compatibility fixtures for older agentic tooling formats.

## Research snapshot

Local compatibility clone:

```text
/Users/wilson/workspace/styrene-labs/compat-datasets/coe-agent
```

Observed upstream shape:

```text
.claude-plugin/plugin.json
.claude-plugin/marketplace.json
.claude-plugin/claude/skills/*
.claude-plugin/claude/commands/*
.claude-plugin/claude/hooks/hooks.json
.claude-plugin/scripts/hooks/*.sh
extensions/coe-agent.ts
prompts/*.md
```

The repo has repeated concepts across multiple surfaces: identity, chronos/date grounding, cleave, OCI login, shell safety, dirty repo guard, status bar, Scribe reporting, and visualization.

## Decisions under consideration

### Candidate decision: decompose rather than vendor

Treat `coe-agent` as a legacy distribution and source of domain content. Styrene should absorb runtime mechanics into Flynt/Omegon core and package Recro/GovCon domain behavior as Armory/Omegon extension content.

Tradeoff: this requires migration tooling, but prevents ongoing reliance on stale Claude plugin/hook layouts.

### Candidate decision: old hooks are migration inputs, not executable runtime

Legacy Claude hook scripts should be translated into declarative policy where possible. Unknown hooks should be flagged for manual review. Flynt should not execute arbitrary old hook scripts by default.

### Candidate decision: Recro skills can move first, Scribe importer later

The fastest reduction in vendored-tool reliance is to package Recro-specific skills/playbooks as a Recro Omegon extension or Armory skill bundle before full Scribe data import parity lands.

## Child design nodes

- [[coe-agent-legacy-inventory|coe-agent legacy inventory and classifier]]
- [[recro-omegon-extension|Recro Omegon extension / skill package]]
- [[scribe-to-flynt-migration|Scribe to Flynt migration]]
- [[legacy-hook-policy-translation|Legacy hook policy translation]]

## Success criteria

1. Flynt can inspect `coe-agent` and classify its legacy artifacts.
2. Recro-specific skills can load through Styrene/Omegon without vendoring `.claude-plugin` as the source of truth.
3. Native Flynt/Omegon runtime capabilities replace duplicated mechanics such as chronos, cleave, identity checks, shell safety, and status display.
4. Scribe data can be dry-run imported into Flynt partnerships, engagements, repo bindings, work logs, docs, tasks, and graph links.
5. The legacy repo remains useful as a compatibility fixture while active work moves under Styrene packaging.

## Open Questions
