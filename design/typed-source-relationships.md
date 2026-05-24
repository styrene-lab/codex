+++
id = "typed-source-relationships"
kind = "design_node"
title = "Typed source relationships"
status = "seed"
tags = ["research", "sources", "graph", "relationships"]

[data]
parent = "zotero-research-workspace"
issue_type = "feature"
priority = 3
trigger = "zotero-source-workspace-followup"
+++

# Typed source relationships

## Thesis

Flynt should eventually model relationships between research sources, claims,
annotations, and notes as typed graph edges. This should not be implemented as
arbitrary nested frontmatter metadata in the first source-note slice: the current
metadata index supports scalar values and string lists, while source
relationships need structured targets, edge kinds, provenance, and graph/query
semantics.

## Relationship vocabulary

Initial source-to-source edge kinds:

- `cites`
- `supports`
- `contradicts`
- `extends`
- `critiques`
- `replicates`
- `dataset_of`
- `method_of`

Later claim-level edge kinds may need separate modeling because the target may
be a heading, block, annotation, or extracted claim rather than a document.

## Accepted constraints

### Accepted: do not block the first source-note slice

The immediate Zotero-inspired source workspace should use flat metadata fields
that Flynt already indexes: `source_type`, `doi`, `isbn`, `url`, `authors`,
`publication`, `published`, `accessed`, `citation_key`, `zotero_key`, and
similar scalar/string-list fields.

Typed relationships are deferred because they require structured edge semantics
and should not force a storage redesign into the first import/rendering work.

### Accepted: relationships are graph data, not generic metadata

Typed source relationships should be represented through a dedicated graph/link
model or an explicit sidecar index, not by depending on nested arbitrary
frontmatter objects. Frontmatter may still provide an authoring syntax, but the
runtime representation should be first-class graph edges.

Reason: relationship queries need target resolution, edge kind filters,
provenance, duplicate handling, and graph rendering. These are graph concerns,
not column metadata concerns.

## Candidate authoring syntax

A human-editable syntax can still live in source notes:

```toml
[[relationships]]
target = "sources/smith-2024"
kind = "extends"
evidence = "Builds on the evaluation method in section 3."

[[relationships]]
target = "sources/chen-2025"
kind = "contradicts"
evidence = "Reports opposite result on comparable dataset."
```

This syntax is intentionally a future candidate, not a first-slice requirement.
The implementation should validate edge kinds and resolve targets to document
IDs where possible.

## Design questions

- [assumption] A dedicated relationship table or graph edge index can coexist
  with markdown-first storage without making the SQLite index the source of
  truth.
- Should relationships target only documents initially, or support headings and
  block anchors from the start?
- Should relationship evidence be plain text, a wikilink/block ref, or an
  annotation ID?
- Should relationship kinds be fixed in code for v1, or project-configurable?
- How should imports map Zotero `related` items, citation links, and extracted
  bibliography references into Flynt relationships?
- What graph UI affordance is enough for v1: filter by edge kind, color by edge
  kind, or source map presets?

## Future implementation sketch

1. Add a typed relationship domain model in `flynt-core`.
2. Add a relationship index/table maintained during markdown reindexing.
3. Parse optional source-note relationship blocks into typed edges.
4. Extend graph payload edges with relationship kinds and source/target IDs.
5. Add graph filters for source relationship kinds.
6. Add importer hooks that can create relationship candidates without
   auto-asserting uncertain relationships.

## Anti-goals

- Do not store nested relationship objects as opaque generic metadata and call it
  graph support.
- Do not infer `supports`/`contradicts` automatically without user confirmation.
- Do not require typed relationships for basic source import, rendering, or
  citation-key workflows.
