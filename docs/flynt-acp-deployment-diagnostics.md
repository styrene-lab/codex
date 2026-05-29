---
id: flynt-acp-deployment-diagnostics
title: "Flynt ACP deployment diagnostics"
status: decided
parent: flynt-omegon-deployment-contract
tags: [omegon, acp, diagnostics, profile, extension, ui]
open_questions: []
related:
  - flynt-omegon-deployment-contract
  - flynt-omegon-artifact-scope
---

# Flynt ACP deployment diagnostics

## Overview

The Flynt ACP panel must not silently run with an incorrect Omegon profile, stale extension, wrong memory scope, or project-root mismatch. The operator needs a visible runtime diagnostic so they can distinguish:

- fully valid Flynt ACP deployment
- Flynt tools loaded but Flynt behavior policy inactive
- stale/incompatible extension contract
- global/user Omegon context leaking into project-scoped Flynt work

## Decision

Add a deployment diagnostic model and UI status surface for the ACP/Omegon rail.

Minimum diagnostic fields:

```text
Runtime: Omegon <version>
Profile: flynt-agent / <actual>
Memory: project / user / unknown
Extension: flynt <version>
SDK: <version>
Project root: <path>
Required profile: flynt-agent
Surface guide: v1
Capability contract: v1
Status: ok | warning | blocked | unknown
```

## Status rules

### OK

All of the following are true:

```text
active_profile == required_profile
memory_scope == project
extension.scope == project
extension.project_root == Flynt.active_project_root
surface_guide_version is compatible
capability_contract_version is compatible
```

### Warning

Use warning when Flynt can still function but behavior may be wrong:

- active profile is unknown
- memory scope is unknown
- extension version is older/newer but contract compatible
- project deployment manifest is absent and Flynt synthesized defaults
- shared artifact provenance cannot be verified

### Blocked

Use blocked when continuing would violate the contract:

- required profile mismatch is known
- project root mismatch is known
- capability contract is incompatible
- extension handshake is missing required metadata
- Flynt extension is absent but ACP panel is opened as Flynt-aware

## Operator-facing text

Example OK:

```text
Flynt ACP ready — flynt-agent profile, project memory, flynt extension v0.11.2, contract v1.
```

Example warning:

```text
Flynt ACP degraded — active profile unknown. Tools are available, but Flynt behavior policy may not be active.
```

Example blocked:

```text
Flynt ACP blocked — expected profile flynt-agent, got default. Switch profile or open a scoped Flynt session.
```

## Implementation notes

First slice:

- Store extension `initialize` metadata from `flynt-agent`.
- Compare it against Flynt project root and expected contract constants.
- Add a diagnostic struct in `flynt-app`.
- Render a compact status row in the Omegon settings/runtime panel.

Later slices:

- Ask Omegon runtime for active profile and memory scope when available.
- Add repair buttons:
  - switch to `flynt-agent`
  - reload extension
  - open runtime settings
  - regenerate project deployment manifest

## Tests

- Missing extension metadata yields warning or blocked, not OK.
- Required profile mismatch yields blocked.
- Project root mismatch yields blocked.
- Compatible metadata yields OK.
- Synthesized deployment yields warning with explicit provenance.
