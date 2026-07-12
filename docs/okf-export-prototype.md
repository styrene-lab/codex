+++
id = "9b5c4f8d-5e5f-4db4-a7d4-84c7cf33adf0"
kind = "design_node"

[data]
title = "Open Knowledge Format export prototype"
status = "implementing"
issue_type = "feature"
priority = 2
parent = "flynt-root"
dependencies = ["portable-analysis-bundles", "architecture"]
open_questions = [
  "[assumption] OKF v0.1 remains a Markdown-directory bundle with YAML frontmatter fields type, title, description, resource, tags, and timestamp.",
  "Should OKF export include generated task/board summaries in the first public UI, or stay document-only until the bundle shape is proven?",
  "Should OKF import be deferred until export consumers validate the profile?"
]
tags = ["okf", "export", "interop", "agent-context", "knowledge-graph"]
+++

# Open Knowledge Format export prototype

## Overview

Google's Open Knowledge Format (OKF) formalizes the LLM-wiki pattern as a portable directory of Markdown files with YAML frontmatter. Flynt already stores project knowledge as Markdown, frontmatter, wikilinks, tasks, design nodes, OpenSpec artifacts, and indexed graph metadata. The integration should therefore be an interchange/export profile over Flynt's native model, not a storage migration.

OKF is also a brand-new proposed standard. Treat upstream compatibility as intentionally provisional: field names, type vocabulary, bundle conventions, and Google Knowledge Catalog ingestion expectations are likely to drift significantly over the next few months. Flynt's OKF support should therefore stay isolated, easy to revise, and covered by small profile tests rather than deeply coupled to core storage.

The prototype goal is to prove that a Flynt project can produce an OKF-shaped bundle that external agents, Google Cloud Knowledge Catalog, and future OKF-compatible tools can consume without understanding Flynt internals.

## Decisions

### Accepted: OKF is an export/interchange surface, not Flynt's native schema

**Rationale:** Flynt's native model is richer than OKF v0.1: boards, tasks, design nodes, drawings, flows, engagements, OpenSpec changes, and agent state have semantics that should not be flattened into OKF as the canonical store. OKF should be a projection.

### Accepted: first pass is document-only plus index

**Rationale:** Markdown documents and design-node documents are the lowest-risk conformant surface. Task/board/OpenSpec summaries are valuable, but they need a more deliberate profile to avoid leaking internal or runtime state into an interchange bundle.

### Accepted: preserve Flynt-specific metadata under `flynt:`

**Rationale:** OKF top-level fields should stay close to the upstream profile. Flynt-specific ids, paths, entity kinds, and publication metadata belong under a namespaced extension key.

## Implementation notes

Initial implementation should live in `crates/flynt-store/src/project.rs` near the existing publication export path:

- Add `OkfExportReport`.
- Add `Project::export_okf_bundle(&self, output_root: &Path) -> Result<OkfExportReport>`.
- Export indexed Markdown documents to the same relative paths under the output root.
- Normalize frontmatter to include OKF fields:
  - `type`
  - `title`
  - `description`
  - `resource`
  - `tags`
  - `timestamp`
  - `flynt`
- Generate `index.md` listing exported concepts.
- Skip hidden/generated/runtime paths such as `.flynt/`, `target/`, and existing export directories.
- Preserve Markdown bodies as-is for first pass.

## First-pass acceptance criteria

- A project can export an OKF bundle from store code.
- The bundle contains `index.md`.
- Exported documents have OKF-compatible YAML frontmatter.
- Flynt metadata is namespaced under `flynt:`.
- Existing Markdown body content is preserved.
- Regression test covers at least one note and one design node.

## Integration path

### Phase 1 — OKF-compatible Flynt profile

Ship Flynt's first integration as **OKF v0.1 plus Flynt profile extensions**, not as a fork and not as a native-storage migration.

The profile should remain conformant with the permissive OKF core:

- every non-reserved `.md` file has YAML frontmatter;
- every concept has a non-empty `type`;
- `index.md` is used for progressive disclosure;
- unknown extension keys are preserved/tolerated.

Flynt-specific guarantees should live under a namespaced `flynt:` extension key:

- stable Flynt document id;
- original project-relative path;
- Flynt entity kind;
- lifecycle status when applicable;
- source publication/doc metadata when applicable.

This keeps the bundle useful to generic OKF consumers while giving Flynt-aware consumers stable identities that do not depend on OKF's current path-as-concept-id model.

### Phase 2 — Public-site discovery convention

OKF v0.1 does not specify web discovery. Flynt's public-site integration should therefore use a documented deployment convention rather than pretending this is an upstream compliance rule:

- expose a stable `/okf/index.md`;
- optionally expose `/okf.tar.gz` and/or `/okf.zip`;
- link the bundle from human docs/site chrome;
- advertise the bundle in `/llms.txt`;
- optionally add HTML `<link rel="alternate" ...>` hints from docs pages.

For `omegon.styrene.io`, the expected shape is:

```text
https://omegon.styrene.io/okf/index.md
https://omegon.styrene.io/okf.tar.gz
https://omegon.styrene.io/llms.txt
```

This should be documented as a Flynt/Omegon convention and revised if upstream OKF later standardizes discovery.

### Phase 3 — Upstream-first proposals

Before diverging, use the public upstream channels in `GoogleCloudPlatform/knowledge-catalog`:

- GitHub Issues for spec proposals and questions;
- Pull Requests for concrete spec/example changes;
- Discussions if the proposal needs broader design conversation.

Candidate upstream proposals:

- optional web discovery convention for public OKF bundles;
- guidance for stable producer-defined ids;
- optional `aliases` and `status` fields;
- profile guidance for non-data-catalog knowledge bases.

Contributions require the Google CLA, so small proposal issues should precede PR work.

### Phase 4 — Profile before fork; fork only if necessary

The preferred long-term shape is:

```text
OKF v0.1-compatible core
        +
Flynt OKF Profile
```

Forking is legally available because the upstream repository is Apache-2.0 licensed, but it should remain a last resort. Fork only if upstream direction becomes incompatible with Flynt's needs, for example:

- discovery or ingestion becomes Google-product-specific;
- path-as-identity hardens in a way that conflicts with stable Flynt ids;
- useful interop proposals are rejected or the spec stalls;
- future OKF versions add constraints that make Flynt's existing Markdown knowledge model non-conformant.

If a fork becomes necessary, use a distinct name such as `Flynt OKF Profile` or `Open Agent Knowledge Profile` rather than creating ambiguity around the OKF name.

## LoE estimate

- **Prototype store export:** 0.5-1 day.
- **Tests and fixture hardening:** 0.5 day.
- **CLI/app command surface:** 0.5-1 day after prototype.
- **Docs/site mention:** 0.25-0.5 day if release-visible.
- **Upstream discoverability proposal:** 0.25 day for an issue, 0.5-1 day for a polished PR if upstream responds positively.
- **Import support:** defer; likely 1-2 days once export profile is validated.

Overall: **1 day for a credible store-level export prototype**, **2 days to make it operator-facing and release-ready**, plus **0.25-1 day** for upstream standards engagement depending on whether we stop at an issue or prepare a PR.
