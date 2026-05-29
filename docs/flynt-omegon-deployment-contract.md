# Flynt ↔ Omegon Deployment Contract

## Status

Decided. This contract is binding architecture for Flynt's embedded ACP/Omegon integration.

Flynt's ACP panel is not a generic chat sidebar. It is an operator-facing viewport into a configured Omegon runtime deployment scoped to the active Flynt project.

## Core model

```text
Flynt App
  owns: workspace UI, documents, tasks, design artifacts, project state

flynt-agent extension
  owns: project-local ACP tool surface into that Flynt workspace

Omegon runtime instance
  owns: model, selected profile, skills, memory policy, tool orchestration

ACP panel
  owns: interactive operator access to that configured runtime instance
```

A valid Flynt ACP session is therefore:

```text
Omegon Runtime
  + selected profile: flynt-agent
  + project memory scope
  + loaded flynt-agent extension
  + Flynt project root
  + Flynt UI-state mirror
  + Flynt surface capability guide
```

Flynt-specific assumptions must not leak into global Omegon behavior or unrelated repositories.

## Authority boundaries

### Omegon core

Omegon core owns:

- model orchestration
- profile loading
- tool routing
- memory policy
- session lifecycle
- core directives and safety policy
- extension host protocol

Omegon core does **not** own:

- Flynt product semantics
- Flynt artifact rules
- Flynt UX maturity claims
- Flynt-specific design workflow assumptions

### Extension SDK

The external extension SDK owns:

- protocol types
- handshake schema
- tool declaration schema
- tool call/result transport
- extension lifecycle conventions
- compatibility and version negotiation

The SDK does **not** own:

- Flynt policy
- Flynt profile behavior
- project-specific memory decisions
- global user preferences

### flynt-agent extension

`flynt-agent` owns:

- project-local Flynt tools
- project-local document/task/graph/artifact operations
- `get_ui_state`
- `flynt_surface_guide`
- capability and maturity metadata for Flynt surfaces
- hard validation of Flynt filesystem invariants

`flynt-agent` does **not** own:

- global Omegon behavior
- global design style
- user-level memory policy
- automatic activation outside Flynt
- claims that a backend tool implies polished GUI maturity

### flynt-agent profile

The `flynt-agent` Omegon profile owns:

- Flynt deployment posture
- when to use Flynt tools
- how to interpret Flynt maturity labels
- anti-cross-pollination rules
- memory scoping rules
- active-surface-first behavior
- ask-before-creating behavior for experimental artifacts

The profile does **not** own:

- hard tool validation
- filesystem writes
- Flynt app internals
- SDK compatibility

### Flynt app

Flynt app owns:

- GUI state
- project root
- document/task/artifact UI
- ACP panel
- launching or connecting to the configured Omegon runtime
- ensuring the runtime is launched with the correct profile and extension

Flynt app does **not** own:

- global Omegon profile defaults
- unrelated Omegon sessions
- non-Flynt extension behavior

## Deployment validity

A Flynt ACP session is valid only if all of these hold:

```text
runtime.profile == "flynt-agent"
runtime.memory_scope == "project"
runtime.project_root == Flynt.active_project_root
extension.name == "flynt"
extension.scope == "project"
extension.project_root == Flynt.active_project_root
extension.sdk_version is compatible with runtime.sdk_host_version
surface_guide.version is present
```

If any of these checks fail, the ACP panel must enter a degraded state and show a visible warning. It must not silently fall back to a generic Omegon profile.

## Initialize handshake contract

`flynt-agent` must report deployment metadata during `initialize`.

Required shape:

```json
{
  "protocol_version": 2,
  "extension_info": {
    "name": "flynt",
    "version": "0.11.2",
    "sdk_version": "0.x.y",
    "sdk_repo": "omegon-extension-sdk",
    "runtime_min_version": "0.x.y",
    "scope": "project",
    "project_root": "/path/to/project",
    "recommended_profile": "flynt-agent",
    "required_profile": "flynt-agent",
    "surface_guide_version": 1,
    "capability_contract_version": 1
  },
  "capabilities": {
    "tools": true,
    "widgets": false,
    "mind": true,
    "vox": false,
    "resources": false,
    "prompts": false,
    "sampling": false,
    "elicitation": false,
    "streaming": false
  },
  "policy": {
    "memory_scope": "project",
    "cross_pollination": "forbidden",
    "requires_ui_state_for_open_surface_claims": true,
    "requires_surface_guide_for_artifact_selection": true
  },
  "tools": []
}
```

Rigid rule: if `required_profile != active_profile`, Flynt must not present the ACP panel as fully operational.

## Profile contract

The `flynt-agent` profile must contain the following rules.

### Scope rule

Flynt assumptions apply only when:

- the active runtime was launched by Flynt, or
- Flynt tools are loaded for the current project root, or
- the operator explicitly asks for Flynt work.

Otherwise, Flynt artifact assumptions are inactive.

### Active-state rule

When the operator refers to "this", "the open thing", "what I'm looking at", "the current design", or equivalent current-context phrasing, the agent must call `get_ui_state` before acting.

### Surface-selection rule

Before choosing among notes, source notes, drawings, D2 diagrams, design boards, or flow graphs, the agent must call `flynt_surface_guide` unless the operator explicitly named the artifact type.

### Maturity rule

The agent must obey maturity labels:

| Maturity | Meaning | Agent behavior |
|---|---|---|
| `stable` | Product path is established and suitable as a default choice. | Proceed when appropriate. |
| `usable` | Works end-to-end but UX/schema may still be maturing. | Prefer active surface; verify after changes. |
| `experimental` | Feature exists but may change. | Ask before creating unless explicitly requested. |
| `avoid_direct` | Legacy, internal, or dangerous direct surface. | Do not use unless the operator explicitly requests legacy handling. |

### Wrapper rule

Never create wrapper-backed artifacts with generic document tools.

Forbidden:

```text
create_document("drawings/Foo.md", "![[Foo.excalidraw]]")
create_document("boards/Foo.md", "![[Foo.board]]")
```

Required:

```text
create_drawing
design_board_create
flow_create
```

### Memory rule

Flynt facts must be scoped.

Good:

```text
In Flynt, design boards use boards/*.board with .md wrappers.
```

Bad:

```text
Design boards use .board files.
```

Project-specific Flynt decisions must not be stored as global user preferences.

### UX honesty rule

Tool existence is not UX maturity. The agent must not imply a polished GUI surface when only an experimental or backend tool exists.

## Surface guide contract

`flynt_surface_guide` is the runtime source of truth for Flynt artifact surfaces.

It must include:

```json
{
  "maturity_legend": {},
  "global_rules": [],
  "surfaces": [
    {
      "kind": "note",
      "maturity": "stable",
      "paths": [],
      "tools": [],
      "use_for": "",
      "rules": []
    }
  ]
}
```

Required surfaces:

- `note`
- `source_note`
- `drawing`
- `d2_diagram`
- `design-board`
- `flow_graph`
- `legacy_canvas`

Required caveat:

```text
canvas/.canvas/canvases terminology is legacy and must not be used for new Flynt design work.
```

## Tool description contract

Every tool description must:

1. State exactly what the tool writes or reads.
2. State whether the artifact is wrapper-backed.
3. State whether generic document tools are forbidden.
4. Avoid product-marketing language.
5. Avoid implying GUI maturity.
6. Mention active-surface tools when relevant.
7. Mention experimental maturity when relevant.

Forbidden phrases unless strictly true:

```text
everything you need
fully supported
complete design system
use whenever the user asks to design
canvas
```

Preferred phrases:

```text
experimental
project-scoped
use when the operator specifically asks
prefer the active artifact
call *_active first
wrapper-backed
```

## Runtime launch contract

Flynt should launch or connect to Omegon with equivalent semantics to:

```text
omegon runtime
  --profile flynt-agent
  --project-root <active Flynt project>
  --extension flynt-agent=<bundled extension path>
  --memory-scope project
```

Required runtime facts:

```json
{
  "profile": "flynt-agent",
  "project_root": "...",
  "memory_scope": "project",
  "extensions": [
    {
      "name": "flynt",
      "path": "...",
      "scope": "project"
    }
  ]
}
```

## ACP panel status contract

The ACP panel must expose deployment state visibly.

Minimum UI fields:

```text
Agent profile: flynt-agent
Runtime: Omegon <version>
Extension: flynt <version>
SDK: <version>
Project scope: <project name/path>
Memory: project
Surface guide: v1
```

If mismatched:

```text
Agent configuration mismatch:
Expected profile flynt-agent, got <profile>.
Flynt tools are available, but behavior policy is not active.
```

This warning must not be hidden in logs.

## Compatibility contract

Four versions must be tracked:

```text
Flynt app version
flynt-agent extension version
Extension SDK version
Omegon runtime version
```

Compatibility matrix:

```json
{
  "flynt_app": "0.11.2",
  "flynt_agent": "0.11.2",
  "extension_sdk": "0.x.y",
  "omegon_runtime_min": "0.x.y",
  "contract_version": 1
}
```

Allowed states:

| State | Meaning |
|---|---|
| `ok` | Versions compatible. |
| `warning` | Compatible but nonmatching patch/minor. |
| `blocked` | SDK or contract incompatible. |
| `unknown` | Missing metadata; warning at minimum. |

Rigid rule: the ACP panel must not silently run a stale extension against a newer Flynt app if the contract version differs.

## Memory boundary contract

Flynt-deployed Omegon uses project memory by default.

| Fact type | Destination |
|---|---|
| Flynt project decision | Project memory |
| Flynt repo architecture | Project memory |
| Operator global preference | User memory, only if explicitly global |
| Temporary UI observation | No durable memory |
| Extension/tool limitation | Project memory or extension docs; global only if broadly applicable |

Rigid rule: do not store Flynt product facts as global memories without explicit Flynt scoping.

## Skill activation contract

Flynt-specific skills must be gated.

`flynt-design` activates only if:

- `flynt_surface_guide` is available, or
- active profile is `flynt-agent`, or
- user explicitly says "in Flynt", or
- active UI state is a Flynt design artifact.

It must not activate merely because the user says:

```text
design
canvas
dashboard
mockup
layout
```

outside Flynt context.

## Extension packaging contract

`flynt-agent` must ship as a package with:

```text
flynt-agent binary
extension manifest
recommended/required profile artifact
tool definitions
surface guide version
compatibility metadata
tests for tool-description policy
```

Example manifest:

```toml
name = "flynt"
version = "0.11.2"
sdk_version = "0.x.y"
runtime_min_version = "0.x.y"
scope = "project"
required_profile = "flynt-agent"
surface_guide_version = 1
capability_contract_version = 1

[activation]
host = "flynt-app"
requires_project_root = true

[memory]
default_scope = "project"
global_writes = "explicit_only"
```

## Required tests

### Extension initialize tests

- `initialize` includes SDK version.
- `initialize` includes required profile.
- `initialize` includes project scope.
- `initialize` includes contract version.

### Surface guide tests

- Every surface has `maturity`.
- `legacy_canvas` exists and is `avoid_direct`.
- `design-board` is `experimental`.
- Surface guide mentions `get_ui_state`.
- Surface guide warns against wrapper creation via generic docs.

### Tool description tests

- No design-board tool says "everything you need".
- No design-board tool says "when the user asks to design something fresh".
- No current Flynt design-board tool advertises `.canvas` as current.
- Wrapper-backed tools mention dedicated creation tools.

### Profile tests / install tests

- `flynt-agent` profile exists.
- Profile contains cross-pollination guardrail.
- Profile contains maturity interpretation.
- Profile requires `get_ui_state` for current/open references.
- Profile requires `flynt_surface_guide` for artifact selection.

### App launch tests / diagnostics

- ACP panel can report active profile.
- ACP panel can report extension version.
- ACP panel can report SDK version.
- ACP panel warns on profile mismatch.

## Non-negotiable invariants

1. No Flynt assumptions outside Flynt deployment.
2. No generic profile pretending to be Flynt-aware.
3. No wrapper-backed artifact creation through generic document tools.
4. No artifact selection without active UI or surface guide evidence.
5. No global memory writes for project-local Flynt facts.
6. No hidden version/profile mismatch.
7. No marketing language in tool descriptions.
8. No current-surface claims using legacy `canvas/.canvas` terminology.
9. No backend-tool existence implies GUI maturity.
10. No ACP panel green state unless profile, extension, SDK, and project scope match.

## Implementation order

1. Document this contract.
2. Add the `flynt-agent` profile artifact.
3. Extend `flynt-agent` initialize metadata.
4. Add enforcement tests.
5. Add ACP panel diagnostics.
6. Enforce runtime launch/profile selection.

## Summary

Flynt ACP is a scoped deployment of Omegon, not ambient global Omegon behavior. The extension provides project-local capabilities; the profile provides Flynt-specific behavior; the SDK provides the boundary; the app verifies the deployment state. Anything outside that boundary is a bug.
