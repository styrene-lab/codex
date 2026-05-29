---
id: flynt-armory-packaging
title: "Flynt Armory packaging for profiles and skills"
status: exploring
parent: flynt-omegon-artifact-scope
tags: [armory, profiles, skills, extension, packaging, catalog]
open_questions:
  - "Should `flynt-agent` be represented as a catalog agent bundle, a plugin persona, or both during the transition?"
  - "Should `flynt-design` live in the Flynt repo and be published into Armory, or move to Armory as source-of-truth with Flynt pinning a release?"
  - "What exact activation constraint schema should Armory accept for skills and profiles?"
related:
  - flynt-omegon-artifact-scope
  - flynt-omegon-deployment-contract
---

# Flynt Armory packaging for profiles and skills

## Overview

Flynt should reuse the shared Omegon artifact ecosystem without cross-pollinating behavior into global user sessions. This requires Armory metadata that distinguishes installable artifacts from active deployment context.

## Target packages

### `styrene.flynt-agent`

A Flynt-specific agent/profile package.

Contents:

```text
catalog/styrene.flynt-agent/
  agent.toml
  agent.pkl          # optional after first slice
  PERSONA.md
```

The profile should activate the `flynt` extension and Flynt-scoped skills only in Flynt deployments.

### `dev.styrene.flynt.design`

A Flynt-specific skill plugin.

Contents:

```text
plugins/dev.styrene.flynt.design/
  plugin.toml
  SKILL.md
```

The skill should not auto-activate globally. Activation requires Flynt context.

### `flynt` extension metadata

Current Armory file:

```text
extensions/flynt.toml
```

Needs additional fields:

```toml
[extension.deployment]
scope = "project"
required_profile = "flynt-agent"
default_memory_scope = "project"
capability_contract_version = 1
surface_guide_version = 1

[extension.activation]
host_any = ["flynt-app"]
auto_activate_global = false
```

## Sharing strategy

- Generic skills (`git`, `rust`, `vault`, `openspec`) remain shared artifacts.
- Flynt profile references them as dependencies instead of duplicating guidance.
- Flynt-specific skills (`flynt-design`) are gated by Flynt context.
- Flynt app may bundle pinned copies for offline correctness but should show provenance.

## Armory schema gap

Existing Armory specs describe plugins and catalog agents, but activation constraints are not first-class enough for Flynt's needs.

Required additions:

```toml
[activation]
auto_activate = false
requires_extension = "flynt"
requires_tool = "flynt_surface_guide"
requires_profile_any = ["flynt-agent"]
host_any = ["flynt-app"]
```

For extensions:

```toml
[extension.deployment]
scope = "project"
required_profile = "flynt-agent"
default_memory_scope = "project"
capability_contract_version = 1
```

## First implementation slice

1. Update `omegon-armory/extensions/flynt.toml` with deployment metadata.
2. Add `catalog/styrene.flynt-agent/` profile package.
3. Add `plugins/dev.styrene.flynt.design/` skill package or a documented placeholder if plugin directory conventions are not ready.
4. Update `catalog-registry.toml` to include `styrene.flynt-agent`.
5. Add validation notes/tests in Armory for activation constraints.

## Risk

If Armory treats install as activate, Flynt behavior will leak globally. The Armory install flow must install artifacts as inert until a scoped deployment/profile activates them.
