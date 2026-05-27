+++
id = "source-task-design-board-projections"
kind = "design_node"
title = "Source task and Design Board projections"
status = "exploring"
tags = ["research", "sources", "tasks", "design-board", "projection"]

[data]
parent = "zotero-research-workspace"
issue_type = "feature"
priority = 2
trigger = "operator-design-question"
+++

# Source task and Design Board projections

## Thesis

Flynt should treat source documents as canonical research objects. Research
tasks and Design Board cells should reference source documents and render projections
of them rather than duplicating bibliographic metadata or owning independent
source state.

```text
Source note             Research task              Design Board component
-----------             -------------              ----------------
kind = "source"   --->  source refs + status  ---> renderable source card
canonical markdown      workflow/action layer      visual projection layer
```

## Decisions

### Accepted: source docs are canonical research objects

A source note owns bibliographic metadata, annotations, summaries, extracted
claims, and durable synthesis. It remains ordinary markdown with `kind =
"source"` frontmatter and flat indexed fields.

Research tasks and Design Board cells should link to the source by stable document
identity where available, with path/citation-key fallback for portability and
human readability.

Cost: projected task/card views need refresh semantics to avoid stale displays.
Benefit: avoids metadata forks and keeps markdown as the source of truth.

### Accepted: tasks own workflow, not source metadata

Research tasks represent work to perform against sources:

- read this source
- extract claims
- import annotations
- compare sources
- synthesize findings into a project note
- verify or challenge a claim

Tasks should reference sources but not duplicate bibliographic fields. Their
state is workflow state (`todo`, `doing`, `blocked`, `done`, priority, due date,
assigned context), not source truth.

### Accepted: Design Board cells are visual projections

Design Board should render source documents as components such as source cards,
evidence cards, reading-task cards, annotation cards, and source clusters. A
Design Board cell may cache generated HTML for rendering, but it must retain enough
reference metadata to refresh from the source document.

The Design Board is the layout/composition layer. It should not become a second
research database.

## Source-to-task reference convention

First-slice task integration should use flat source reference fields until task
metadata indexing supports richer structures consistently:

```toml
+++
title = "Extract claims from Vaswani 2017"
kind = "task"
status = "todo"
tags = ["research", "source-review"]
source_paths = ["sources/vaswani-2017-attention.md"]
source_document_ids = ["8fcbf8df-8b54-4cf3-a7ab-857f7ec75fb5"]
source_citation_keys = ["vaswani2017attention"]
source_zotero_keys = ["ABCD1234"]
+++
```

Recommended task body pattern:

```markdown
## Objective

Extract reusable claims and implementation notes from
[[sources/vaswani-2017-attention|Vaswani 2017]].

## Checklist

- [ ] Read abstract/introduction
- [ ] Extract major claims
- [ ] Pull useful quotes
- [ ] Identify supporting or contradicting sources
- [ ] Summarize implications for the current project

## Output

Write durable synthesis back to the source note under `## Synthesis`.
```

## Design Board source component model

A source-backed Design Board cell should store both its rendered card and a stable
reference payload. Candidate cell metadata:

```json
{
  "id": "source-card--vaswani-2017-attention",
  "kind": "source_card",
  "source_ref": {
    "document_id": "8fcbf8df-8b54-4cf3-a7ab-857f7ec75fb5",
    "path": "sources/vaswani-2017-attention.md",
    "citation_key": "vaswani2017attention",
    "zotero_key": "ABCD1234"
  },
  "variant": "compact"
}
```

The initial implementation can generate ordinary Design Board HTML/CSS, but the cell
should preserve the `source_ref` so a refresh command can re-read the source and
regenerate the card without changing its grid position.

## Renderable component variants

### Compact source card

Use for bibliographies, source dashboards, and reading lists.

Shows:

- source type
- title
- authors
- year/publication
- citation key
- DOI/URL availability
- source status badge when available

### Evidence card

Use for argument maps and future typed source relationships.

Shows:

- claim or quote
- source title
- page/section/annotation reference
- relationship label such as `supports`, `contradicts`, or `critiques`

This should depend on [[typed-source-relationships]] before becoming fully
semantic.

### Reading-task card

Use for research planning Design Board boards.

Shows:

- source title
- linked research task status
- next action
- due date/priority if present

### Annotation card

Use for visual synthesis from imported PDF/web annotations.

Shows:

- extracted quote or note
- source title
- page/locator
- synthesis status

### Source cluster

Use for topic maps.

Shows a central topic with multiple source cards around it. Relationship edges
can be added after typed relationships land.

## Refresh semantics

Design Board source cards should be refreshable:

1. Find Design Board cells with `source_ref`.
2. Resolve by `document_id`, then path, then citation/Zotero key fallback.
3. Load the source document.
4. Re-render card HTML from source metadata/body excerpt.
5. Preserve Design Board position, size, theme, and variant.
6. Report unresolved or duplicate references instead of guessing silently.

## Recommended implementation phases

### Phase 1: conventions and agent generation

- Document task source reference fields.
- Teach the agent to create source-linked research tasks.
- Teach the agent to render a source note into a Design Board source card.
- Prefer `sources/` for new source notes, but treat `kind = "source"` as the
  authority.

### Phase 2: source card refresh

- Add Design Board cell metadata for `source_ref` if not already available.
- Add a refresh command/tool that regenerates source cards from source docs.
- Preserve cell geometry and only replace projected content.

### Phase 3: research task lenses

Add lenses for:

- source inbox
- sources missing metadata
- sources with open research tasks
- annotated but not synthesized
- research tasks grouped by source

### Phase 4: relationship-aware Design Board maps

After [[typed-source-relationships]] is implemented:

- render source relationship edges
- color by relationship kind
- generate evidence cards from claim/annotation edges
- create source clusters from graph queries

## Open questions

- [assumption] Design Board cells can retain structured metadata such as `source_ref`
  without breaking the current HTML/CSS cell rendering model.
- Should source card generation be an agent-only workflow first, or a native
  Design Board command?
- Should task source references be flat fields forever, or migrate to a
  `[source_refs]` table once task indexing supports it cleanly?
- What is the smallest source card variant that remains useful at low Design Board
  grid sizes?
- Should source cards display body excerpts, `## Summary`, or only frontmatter
  in v1?

## Anti-goals

- Do not copy full source metadata into tasks or Design Board cells as independent
  state.
- Do not make Design Board the authoritative store for source relationships.
- Do not require typed source relationships before basic source-card rendering.
- Do not implement a general arbitrary-document renderer before source-specific
  cards prove the projection model.
