+++
id = "flynt-armory-skill-profile-extraction"
kind = "design_node"

[data]
title = "Armory extraction plan for Flynt skills and profile"
status = "decided"
issue_type = "architecture"
priority = 2
parent = "flynt-omegon-deployment-contract"
dependencies = ["flynt-omegon-artifact-scope", "d2-excalidraw-port-contract"]
open_questions = [
  "Which Armory artifact kind should represent lightweight profiles if profile packages diverge from catalog agents?",
  "What exact skill activation identifier should Flynt use long-term: short folder names or full plugin IDs?",
]
tags = ["armory", "skills", "profiles", "flynt-agent", "d2", "excalidraw", "omegon"]
+++

# Armory extraction plan for Flynt skills and profile

## Decision

Extract reusable diagram guidance into Armory skills and define `flynt-agent` as a recommended profile/composition, not as a hard runtime lock.

Flynt should require capabilities and compatible contracts, not a specific named profile. The `flynt-agent` profile is the default harness for Flynt's embedded ACP session, but custom profiles may be valid if they activate equivalent tools, skills, prompts, memory scope, and surface-policy behavior.

## Artifact model

### Reusable skills

Reusable guidance should live in Armory as inert installable skill artifacts. Installing a skill must not globally activate Flynt behavior.

Initial skill extractions:

1. `d2-authoring`
   - generic, reusable outside Flynt
   - teaches robust D2 source authoring and render validation
   - references Flynt preview behavior only in a compatibility section

2. `excalidraw-authoring`
   - generic Excalidraw/drawing composition skill
   - includes a Flynt-specific section for `create_drawing`, `drawing_active`, and drawing spec tools when available
   - understands D2 diagrams as possible source inputs

Later candidate:

3. `diagram-surface-selection`
   - routes between D2, Excalidraw/drawing spec, Flow, and Design Board
   - defer until the routing logic is large enough to justify a standalone skill

### Flynt profile

`flynt-agent` is a profile/composition. It is not a skill, not a required identity, and not a closed bundle. It is the starting harness Flynt selects for embedded ACP sessions so a fresh project has sane defaults. The Flynt operator must still be able to install, update, replace, and overlay newer Armory packages, extensions, skills, prompts, and profiles without waiting for a Flynt app release.

The profile activates or recommends:

- Extension: `flynt`
- Skills:
  - `vault`
  - `d2-authoring`
  - `excalidraw-authoring`
  - `flynt-design` once cleaned up
  - optional: `git`, `openspec`, `security`
- Prompt/policy fragments:
  - call `get_ui_state` before claiming active/open context
  - call `flynt_surface_guide` before choosing artifact surface unless explicitly named
  - obey maturity labels
  - project-scope Flynt memory
  - do not create wrapper-backed artifacts with generic document tools
  - no cross-pollination into global user/system Omegon

## Armory layout

### Skills

Use the existing Armory skill package shape:

```text
omegon-armory/skills/d2-authoring/
  plugin.toml
  SKILL.md
  examples/
    topology-template.d2
    state-authority-template.d2
    command-flow-template.d2

omegon-armory/skills/excalidraw-authoring/
  plugin.toml
  SKILL.md
  examples/
    d2-svg-import-plan.md
    d2-semantic-translation-plan.md
```

Suggested plugin IDs:

```toml
id = "dev.styrene.omegon.skill.d2-authoring"
id = "dev.styrene.omegon.skill.excalidraw-authoring"
```

Short activation names remain:

```text
d2-authoring
excalidraw-authoring
```

until the resolver definitively requires full plugin IDs.

### Profile

Preferred final profile location depends on Armory conventions. Existing Armory contains both `catalog/` agents and `profiles/`, so first implementation should inspect/confirm profile package support.

Candidate locations:

```text
omegon-armory/profiles/flynt-agent/
  profile.toml
  PROFILE.md
```

or, if catalog agent bundles remain the supported distribution unit:

```text
omegon-armory/catalog/styrene.flynt-agent/
  agent.toml
  PERSONA.md
  agent.pkl
```

Do not duplicate the D2/Excalidraw skill content inside the profile. Reference skill IDs instead.

## Operator package evolution

Flynt's bundled/default profile is only a bootstrap. Runtime package resolution must allow the operator to move faster than the Flynt app bundle.

Resolution order for skills, profiles, prompts, and extensions:

1. Project-local overrides: `<project>/.flynt/omegon/...`
2. Operator-installed Armory artifacts: `~/.omegon/armory/...` or the active Omegon artifact cache
3. Flynt app bundled pinned fallbacks
4. Remote Armory catalog as an install/update candidate, never silently active

Rules:

- Flynt may seed `flynt-agent` and recommended skills, but must not freeze them.
- The operator can install newer `d2-authoring`, `excalidraw-authoring`, `flynt-design`, or `flynt-agent` packages from Armory.
- Flynt should show artifact provenance and version drift: project override, user Armory, bundled fallback, or remote candidate.
- Flynt should warn on incompatible contracts, not on mere version differences.
- A newer compatible Armory package should win over an older bundled fallback.
- A project-local pin should win over user/global Armory packages.
- Remote packages require explicit operator install/update; Flynt must not auto-upgrade profile behavior silently.

This preserves offline correctness via bundled fallbacks while allowing the operator to evolve the agent harness and skills independently.

## Non-locking deployment contract

Replace hard profile requirements with recommended profile/capability validation.

### Bad

```json
"required_profile": "flynt-agent"
```

and diagnostics that block when the active profile name differs.

### Good

```json
"recommended_profile": "flynt-agent",
"required_capabilities": [
  "get_ui_state",
  "flynt_surface_guide",
  "project_memory_scope",
  "wrapper_backed_artifact_discipline"
]
```

Profile mismatch should be a warning unless capabilities are missing or the memory scope/contract is unsafe.

## Flynt default scoped deployment manifest

Default target after extraction:

```toml
[deployment]
id = "flynt-project"
host = "flynt-app"
profile = "flynt-agent"              # selected default, not a lock
recommended_profile = "flynt-agent"
memory_scope = "project"
capability_contract_version = 1
surface_guide_version = 1

[activation]
extensions = ["flynt"]
skills = [
  "vault",
  "d2-authoring",
  "excalidraw-authoring",
  "flynt-design",
]
optional_skills = ["git", "openspec", "security"]
```

Custom profile example:

```toml
[deployment]
profile = "company-flynt-engineer"
recommended_profile = "flynt-agent"
```

This is valid if the custom profile provides the same required capabilities and policy behavior.

## Skill extraction content

### `d2-authoring`

Source of truth draft:

- `docs/d2-authoring-contract.md`
- `crates/flynt-core/src/d2_contract.rs`

Must include:

- no multiline `|md` bodies inside ordinary nodes
- details as child nodes
- short edge labels
- split by primary question
- minimize cross-container edges
- avoid panoramic diagrams
- inspect SVG `foreignObject` sizing
- when to repair D2 vs port to Excalidraw

### `excalidraw-authoring`

Source of truth draft:

- `docs/d2-excalidraw-port-contract.md`
- current style guide Excalidraw semantic palette
- Flynt drawing tool descriptions

Must include:

- editable drawing composition rules
- semantic palette roles
- lanes/panels/whitespace/connector routing discipline
- Flynt wrapper/backing-file discipline when tools are present
- D2 intake rules
- D2 port modes:
  - SVG import
  - semantic translation
- provenance requirements

## Implementation phases

### Phase 1 — Armory skill package drafts

- Create `skills/d2-authoring` in `omegon-armory`.
- Create `skills/excalidraw-authoring` in `omegon-armory`.
- Validate TOML and markdown.
- Do not publish yet; treat as local package drafts.

### Phase 2 — Flynt manifest alignment

- Add `d2-authoring` and `excalidraw-authoring` to Flynt default deployment manifest.
- Change metadata wording from `required_profile` to `recommended_profile` / capabilities.
- Change diagnostics: profile mismatch warning, missing capabilities/contract blocked.

### Phase 3 — Profile artifact

- Inspect Armory profile/catalog conventions.
- Create `flynt-agent` profile package as a composition referencing skills and Flynt extension.
- Keep profile policy concise and avoid duplicating reusable skills.

### Phase 4 — Tooling and verification

- Add or extend Armory validation for skill packages.
- Add Flynt tests that default deployment includes the two diagram skills.
- Add extension/surface guide tests that D2 and Excalidraw surfaces reference reusable skills.

## Acceptance criteria

- Generic Omegon users can install `d2-authoring` and `excalidraw-authoring` without installing Flynt.
- Flynt's default ACP profile composes these skills, but custom profiles can substitute equivalent capability bundles.
- The Flynt app uses bundled profile/skill fallbacks only as a starting point; operator-installed compatible Armory artifacts can supersede them.
- Flynt shows artifact provenance and warns when bundled, project, and user-installed versions differ.
- No Flynt code blocks solely because the profile name is not `flynt-agent`.
- D2 and Excalidraw skills cross-reference each other for porting workflows.
- Flynt docs and tool descriptions no longer imply Excalidraw and D2 are isolated worlds.
