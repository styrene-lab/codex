+++
title = "Scribe to Flynt migration"
tags = ["scribe","flynt","engagement","multi-repo","importer","recro"]
+++

+++
id = "7df3ed33-e134-4511-a72f-1781aaa3a7f1"
kind = "design_node"

[data]
title = "Scribe to Flynt migration"
status = "exploring"
issue_type = "migration"
priority = 2
parent = "coe-agent-styrene-migration"
dependencies = []
open_questions = []
+++

## Overview

# Scribe to Flynt migration

---
id: scribe-to-flynt-migration
title: "Scribe to Flynt migration"
status: exploring
parent: coe-agent-styrene-migration
tags: [scribe, flynt, engagement, multi-repo, importer, recro]
related:
  - coe-agent-styrene-migration
open_questions:
  - "[assumption] Scribe daily engagement JSON should import as Flynt work logs, not as one Flynt Engagement per daily file."
  - "Should default import grouping create one Flynt engagement per partnership, per SOW, or per contract/project?"
  - "Which Scribe metadata should become typed Flynt model fields versus markdown docs/graph nodes?"
  - "Should Flynt RepoBinding grow access_level, git_url, and repo_type fields for Scribe parity?"
---

# Scribe to Flynt migration

## Overview

Scribe was a git-backed consulting engagement ledger and reporting system. Flynt has absorbed the multi-repo engagement core, but Scribe's business/reporting metadata still needs a migration path.

## Known model delta

Flynt already supports many repos per engagement:

```rust
Engagement { repos: Vec<RepoBinding> }
```

Flynt engagement tools operate on `engagement_id` plus `repo` for issue listing/sync/creation.

Scribe's broader model includes:

```text
.scribe/partnership.json
.scribe/team_assignments.json
.scribe/stakeholders.json
.scribe/contracts.json
.scribe/sows.json
.scribe/risks.json
.scribe/decisions.json
.scribe/repositories.json
engagements/YYYY-MM/YYYY-MM-DD.json
reports/*.pdf
```

Important semantic mismatch: Scribe "Engagement" means a daily work record; Flynt "Engagement" means a bounded work effort such as a project, sprint, or contract.

## Import mapping

| Scribe artifact | Flynt target |
|---|---|
| partnership metadata | Partnership + markdown doc |
| repositories.json | RepoBinding[] plus metadata |
| daily engagement JSON | WorkLogEntry[] |
| work_items | development work logs / tasks |
| stakeholder_interactions | meeting work logs |
| deliverables | deliverable docs/report facts |
| SOWs/contracts | docs + graph links, maybe typed metadata later |
| risks | docs/tasks |
| decisions | design docs/ADR docs |
| reports | Flynt documents/export artifacts |

## First implementation slice

1. Add dry-run Scribe detector.
2. Count partnerships, repos, SOWs, daily engagement files, risks, decisions, and reports.
3. Present import grouping strategies: per partnership, per SOW, per contract/project.
4. Generate a migration plan without mutation.
5. Add minimal import for repositories and daily logs after approval.

## Model gaps to evaluate

- `RepoBinding.access_level`: read_write, read_only, reference_only.
- `RepoBinding.git_url`.
- `RepoBinding.repo_type`.
- Work-log metadata for SOW, repo refs, deliverables, stakeholders, and created_by.

## Non-goals

- Recreate Scribe as a separate CRM inside Flynt.
- Treat every Scribe daily JSON file as a Flynt engagement by default.

## Open Questions
