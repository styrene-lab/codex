---
id: flynt-omegon-artifact-scope
title: "Scoped Omegon artifact activation for Flynt"
status: decided
parent: flynt-omegon-deployment-contract
tags: [omegon, flynt-agent, profiles, skills, extensions, deployment, scope]
open_questions: []
related:
  - flynt-omegon-deployment-contract
---

# Scoped Omegon artifact activation for Flynt

## Overview

Flynt and user/system Omegon should share installable artifacts without sharing activation state. Profiles, skills, extensions, tones, and style guides are reusable artifacts. A Flynt ACP session is a scoped deployment that activates a selected subset of those artifacts under a project-local policy.

The guiding rule:

> Install once, activate by scope.

## Decision

Adopt a three-layer model:

1. **Artifact** — reusable package: profile, skill, extension, tone, style guide.
2. **Installation** — cached local copy in user, app-bundled, or project storage.
3. **Activation / deployment** — runtime-specific selection with scope, provenance, and compatibility checks.

Flynt will not maintain a private duplicate universe of Omegon artifacts. It will ship pinned fallbacks for reliability, but the runtime should prefer scoped activation records and show provenance.

## Resolution order

When resolving a Flynt ACP deployment, use this order:

1. Project override: `<project>/.flynt/omegon/...`
2. Flynt app-bundled pinned artifacts: `Flynt.app/Contents/Resources/omegon/...`
3. User-installed artifacts: `~/.omegon/armory/...`
4. Remote Armory catalog: install candidate only, never auto-active

Closer scope wins. The ACP panel must show artifact provenance.

Example visible state:

```text
Profile: flynt-agent@0.12.0 (Flynt bundled)
Extension: flynt@0.12.0 (project scoped)
Skills: vault@1.2.0 (user), flynt-design@0.12.0 (bundled)
Memory: project
Contract: v1
```

## Activation manifest

A Flynt project may declare deployment state in:

```text
<project>/.flynt/runtime/omegon.toml
```

Shape:

```toml
[deployment]
id = "flynt-project"
host = "flynt-app"
profile = "flynt-agent"
memory_scope = "project"
contract_version = 1

[activation]
extensions = ["flynt"]
skills = ["flynt-design", "vault"]
optional_skills = ["openspec", "git"]

[extension.flynt]
scope = "project"
project_root = "."
required_profile = "flynt-agent"
```

If absent, Flynt synthesizes an equivalent ephemeral deployment using bundled defaults.

## Artifact manifests

Shared artifacts need activation constraints.

Example Flynt design skill:

```toml
[plugin]
type = "skill"
id = "dev.styrene.flynt.design"
name = "Flynt Design"
version = "0.12.0"

[skill]
guidance = "SKILL.md"

[activation]
auto_activate = false
requires_extension = "flynt"
requires_tool = "flynt_surface_guide"
requires_profile_any = ["flynt-agent"]
host_any = ["flynt-app"]
```

Example Flynt profile:

```toml
[agent]
id = "styrene.flynt-agent"
name = "Flynt Agent"
version = "0.12.0"
domain = "knowledge"

[persona]
directive = "PERSONA.md"

[settings]
memory_scope = "project"

[[extensions]]
name = "flynt"
version = ">=0.11.0"

[activation]
host_any = ["flynt-app"]
default_active = false
```

## Memory boundary

Flynt ACP defaults to project memory. Global memory writes require explicit operator intent.

| Fact type | Destination |
|---|---|
| Flynt project decision | Project memory |
| Flynt repo architecture | Project memory |
| Temporary UI state | No durable memory |
| Operator global preference | User memory only when explicitly global |

## Tradeoffs

Benefits:

- Avoids duplicate copies of common skills like `git`, `rust`, `vault`, and `openspec`.
- Prevents Flynt-specific assumptions from leaking into non-Flynt sessions.
- Enables Flynt to ship reliable pinned defaults while still consuming shared user-installed artifacts.
- Makes profile/extension mismatch visible instead of implicit.

Costs:

- Requires an activation resolver and provenance UI.
- Requires Armory/catalog metadata to represent activation constraints.
- Requires migration from ad hoc profile files into package manifests.

## Implementation notes

First implementation slice:

1. Define typed deployment and resolved-artifact data structures in Flynt app.
2. Parse optional `<project>/.flynt/runtime/omegon.toml`.
3. Synthesize defaults when the file is absent.
4. Surface deployment status in the ACP/Omegon settings panel.
5. Update Armory metadata for the Flynt extension with scope/profile contract.

Later slices:

- Package `flynt-agent` profile in Armory catalog.
- Package `flynt-design` as a scoped skill plugin.
- Teach Omegon runtime to enforce activation constraints natively.
- Add ACP warnings for profile/memory/contract mismatch.
