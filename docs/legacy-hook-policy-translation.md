+++
title = "Legacy hook policy translation"
tags = ["policy","hooks","claude-plugin","shell-safety","migration"]
+++

+++
id = "3be96f79-57d2-461a-b6c2-11fc002771de"
kind = "design_node"

[data]
title = "Legacy hook policy translation"
status = "exploring"
issue_type = "policy"
priority = 3
parent = "coe-agent-styrene-migration"
dependencies = []
open_questions = []
+++

## Overview

# Legacy hook policy translation

---
id: legacy-hook-policy-translation
title: "Legacy hook policy translation"
status: exploring
parent: coe-agent-styrene-migration
tags: [policy, hooks, claude-plugin, shell-safety, migration]
related:
  - coe-agent-legacy-inventory
  - coe-agent-styrene-migration
open_questions:
  - "[assumption] Known Claude hook safety checks can be represented as declarative Flynt/Omegon policy."
  - "Where should translated policy live: project .flynt policy, user policy, Armory package policy, or runtime defaults?"
  - "How should conflicting policies compose when a package and project both declare shell/git rules?"
---

# Legacy hook policy translation

## Overview

`coe-agent` includes Claude hook scripts for pre-bash command checks, post-edit placeholders, session start preambles, and session-end dirty repo warnings. These should not be executed directly in Flynt/Omegon. They should be translated into declarative policy where possible.

## Legacy behaviors to translate

- Block AI attribution in commit messages.
- Block destructive shell patterns such as `rm -rf /`, `mkfs`, device writes, recursive chmod/chown on root.
- Confirm risky commands such as force push, hard reset, git clean, broad `rm -rf`, and destructive SQL.
- Warn about uncommitted/unpushed changes on shutdown or session transitions.
- Inject session preamble/status context.

## Candidate policy shape

```toml
[policy.git]
block_ai_attribution = true
warn_unpushed_commits = true
warn_uncommitted_changes = true

[policy.shell]
deny = [
  "rm -rf /",
  "rm -rf ~",
  "mkfs\\.",
  "dd if=.* of=/dev/"
]
confirm = [
  "git push.*--force",
  "git reset --hard",
  "git clean -fd",
  "rm -rf"
]
```

## First implementation slice

1. Inventory hook scripts and hook JSON without executing them.
2. Recognize known shell/git policy patterns.
3. Emit proposed declarative policy.
4. Mark unknown scripts as manual review.
5. Surface native replacements for session preamble/status behaviors.

## Non-goals

- Execute legacy hook scripts.
- Preserve Claude hook JSON schema as a runtime dependency.
- Let package policy silently override project/operator policy.

## Open Questions
