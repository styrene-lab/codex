+++
title = "Recro Omegon extension / skill package"
tags = ["recro","omegon","extension","skills","armory","coe-agent"]
+++

+++
id = "002a88b0-3ab8-4624-98c1-a9eaac0585af"
kind = "design_node"

[data]
title = "Recro Omegon extension / skill package"
status = "exploring"
issue_type = "packaging"
priority = 1
parent = "coe-agent-styrene-migration"
dependencies = []
open_questions = []
+++

## Overview

# Recro Omegon extension / skill package

---
id: recro-omegon-extension
title: "Recro Omegon extension / skill package"
status: exploring
parent: coe-agent-styrene-migration
tags: [recro, omegon, extension, skills, armory, coe-agent]
related:
  - coe-agent-styrene-migration
  - flynt-armory-packaging
open_questions:
  - "Should Recro-specific skills ship as an Omegon extension, an Armory skill package, or an extension that registers an Armory skill bundle?"
  - "Which skills are Recro-specific enough to migrate first: domain-language-and-terminology, opportunity-eval, write-proposal, recro-style-guide, report, scribe, recro-secrets?"
  - "How should package activation be scoped so Recro guidance does not leak into unrelated global sessions?"
  - "Should generated Claude plugin output remain supported for Recro users during transition?"
---

# Recro Omegon extension / skill package

## Overview

Move Recro-specific `coe-agent` skills out of the vendored Claude plugin layout and into a Styrene-controlled package or Omegon extension. The first goal is reducing reliance on `.claude-plugin` as source-of-truth while preserving the useful Recro/GovCon domain behavior.

## Candidate package contents

High-confidence Recro/domain assets:

```text
domain-language-and-terminology
opportunity-eval
write-proposal
recro-style-guide
report
scribe
recro-secrets
```

Potentially generic but currently Recro-flavored:

```text
github
k8s-operations
markdown
```

Native-superseded, not package source-of-truth:

```text
chronos
cleave
identity mechanics
visualizer rendering mechanics
OCI login mechanics
```

## Candidate package shape

If Armory package:

```text
packages/recro-coe/
  package.toml
  skills/domain-language-and-terminology/SKILL.md
  playbooks/opportunity-eval.md
  playbooks/write-proposal.md
  playbooks/report.md
  style-guide.md
  templates/
  policies/
```

If Omegon extension:

```text
extensions/recro-coe/
  extension.toml
  src/index.ts or equivalent
  skills/
  playbooks/
  style-guide.md
```

A hybrid may be best: an Omegon extension contributes tools/integration status only if needed, while Armory owns pure prompt/skill/playbook content.

## Migration order

1. Move `domain-language-and-terminology` as a pure knowledge skill.
2. Move `recro-style-guide` into package/project style guide form.
3. Move `opportunity-eval` and `write-proposal` as playbooks with structured output conventions.
4. Move `report` as a playbook backed by Flynt timeline/work logs.
5. Keep `scribe` as a migration/data playbook until the importer exists.
6. Decompose `recro-secrets`, `github`, and `k8s-operations` into generic shared pieces plus Recro policy overlays.

## Success criteria

- Recro-specific skills can be activated in Omegon/Flynt from Styrene packaging.
- `.claude-plugin/claude/skills` is no longer the active source-of-truth for migrated skills.
- Activation is scoped to Recro/Flynt contexts and does not leak globally.
- The operator can see which Recro package content influenced the agent.

## Open Questions
