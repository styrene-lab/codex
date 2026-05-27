+++
id = "zotero-research-workspace"
kind = "design_node"
title = "Zotero-inspired source-backed research workspace"
status = "exploring"
tags = ["research", "sources", "zotero", "citations", "agent"]

[data]
parent = "obsidian-feature-parity"
issue_type = "feature"
priority = 2
trigger = "operator-research-request"
+++

# Zotero-inspired source-backed research workspace

## Thesis

Flynt should borrow Zotero's highest-leverage research workflows without trying
to become a full citation manager. The target is a **source-backed research
workspace**: markdown notes remain the source of truth, while bibliographic
metadata, annotations, captures, and source relationships become first-class
project data the agent and graph can reason over.

Zotero's own positioning is: collect, organize, annotate, cite, and share
research. Flynt should adopt the parts that strengthen local-first markdown and
agentic synthesis, and interoperate with Zotero for the parts Zotero already
owns well.

## Research summary

Observed Zotero capability areas from Zotero's public site/docs:

- Browser collection via Zotero Connector and translators that recognize
  research objects on webpages.
- Structured bibliographic records: title, creators, publication, dates, DOI,
  ISBN, URL, item type, and related metadata.
- PDF reader with highlights, underlines, annotations, and note extraction.
- Notes attached to sources, including extracted PDF annotations and links back
  to PDF pages.
- Collections, tags, saved searches, related items, and duplicate detection.
- Citation and bibliography generation with Word, LibreOffice, Google Docs, and
  thousands of CSL styles.
- Sync, groups/shared libraries, and a Web API.

## Decisions

### Accepted: position Flynt as a source-backed research workspace

Flynt should not clone Zotero's database-centric citation-manager UI. Flynt's
advantage is that sources can become normal markdown files in the project graph,
with TOML frontmatter and agent-visible structure.

Cost: Flynt needs a clear source schema and migration/interoperability story.
Benefit: research material becomes queryable, linkable, git-syncable, and
available to Omegon tools without a separate library UI.

### Accepted: interoperate with Zotero before replacing it

Zotero import/sync should come before any attempt at full citation management or
PDF reader parity. Preserve Zotero item keys and library metadata so Flynt can
round-trip or re-sync later.

First import targets:

1. BibTeX / CSL JSON files.
2. Zotero Web API.
3. Local Zotero database only if the API/export path is insufficient.

### Accepted: defer full citation rendering and word-processor integration

Flynt should support citation keys and markdown citation syntax, but should not
reimplement CSL or Word/LibreOffice/Google Docs plugins early. Use existing
interchange formats and external tooling where possible.

### Accepted: source docs are canonical; tasks and Design Board render projections

Source notes own bibliographic metadata, annotations, summaries, extracted
claims, and durable synthesis. Research tasks and Design Board cells should reference
source notes by stable identity/path/citation key and render workflow or visual
projections instead of duplicating source truth.

The task and Design Board integration design is tracked in
[[source-task-design-board-projections]].

### Accepted: defer typed source relationships to a graph design node

Typed source relationships should not be implemented as nested generic metadata
in the first source-note slice. They need graph-edge semantics: target
resolution, edge kinds, provenance, candidate review, and graph filtering.

The future design is tracked in [[typed-source-relationships]].

Source notes are ordinary markdown files with `kind = "source"` frontmatter.
Initial schema:

```toml
+++
title = "Paper or Article Title"
kind = "source"
source_type = "article" # webpage, paper, book, video, podcast, dataset, legal
url = "https://example.com/article"
doi = "10.0000/example"
isbn = "9780000000000"
authors = ["Jane Doe", "John Smith"]
editors = []
publication = "Journal or Site"
publisher = "Publisher"
published = "2026-05-20"
accessed = "2026-05-20"
captured = "2026-05-20T03:30:00Z"
citation_key = "doe2026title"
zotero_key = "ABCD1234"
zotero_library = "user:123456"
zotero_version = 42
tags = ["source", "research"]
+++
```

The schema should tolerate partial metadata. Many web captures will only have a
title and URL.

## Feature candidates

### 1. Source note foundation

- Define and document `kind = "source"`.
- Add source-specific rendering in the note view.
- Add source lenses/query presets:
  - all sources
  - unread sources
  - sources missing DOI/author/date
  - annotated but not synthesized
  - cited in current note
- Teach agent surface guide how to create and query source notes.

This is the recommended first implementation slice.

### 2. Zotero / BibTeX / CSL import

- Import BibTeX and CSL JSON into source notes.
- Preserve citation keys and Zotero keys.
- Convert Zotero tags/collections into Flynt tags/folders/lenses.
- Convert Zotero notes into markdown sections or child notes.
- Add duplicate detection during import.

### 3. Browser capture

- Browser extension, bookmarklet, or share target that writes a source note.
- Capture title, URL, canonical URL, author, site/publication, date, selected
  text, and description.
- Optional readability/snapshot markdown body.
- Add captures to a source inbox.

A minimal bookmarklet/share flow should precede a full browser extension.

### 4. Annotation extraction

- Import PDF annotations from Zotero export or embedded PDF annotations.
- Render highlights as markdown quote blocks with page refs.
- Link annotations back to the source note and PDF page.
- Add an agent command to synthesize annotations into claims, summaries, and
  open questions.

Do not build a full PDF reader until annotation import proves demand.

### 5. Research task and Design Board projections

- Create research tasks that reference source notes rather than duplicating
  source metadata.
- Render source notes as Design Board components such as compact source cards,
  evidence cards, reading-task cards, annotation cards, and source clusters.
- Preserve source references in Design Board cells so projected cards can be refreshed
  from canonical source notes.
- Track the detailed projection design in [[source-task-design-board-projections]].

### 6. Typed source graph

Represent relationships between sources and claims with typed edges:

- cites
- supports
- contradicts
- extends
- critiques
- replicates
- dataset-of
- method-of

Possible frontmatter representation:

```toml
related = [
  { target = "sources/smith-2024", kind = "extends" },
  { target = "sources/chen-2025", kind = "contradicts" },
]
```

The graph should filter by relation type so research maps become legible.

### 7. Duplicate detection

Detect likely duplicate sources by:

- DOI
- ISBN
- canonical URL
- title + author + year
- Zotero key
- PDF hash

Expose merge candidates rather than auto-merging.

### 8. Source packs

A Flynt-native equivalent to Zotero group/shared libraries:

```text
sources/
  ai-safety-pack/
    index.md
    papers/
    annotations/
    bibliography.bib
```

Source packs can be git subfolders, archives, or imported collections.

## Anti-goals

- Do not build a full Zotero clone.
- Do not implement Word/LibreOffice/Google Docs plugins early.
- Do not implement a full CSL engine unless using an existing maintained
  library/tool.
- Do not build a PDF reader before annotation import and source-note workflows
  prove useful.
- Do not introduce a second opaque research database; source metadata should be
  visible in markdown/frontmatter.

## Open questions

- Resolved: Flynt's current query/lens system can index flat source metadata
  such as `doi`, `citation_key`, `zotero_key`, `source_type`, and string-list
  fields such as `authors`, because `Frontmatter` flattens unknown top-level
  fields into `metadata`, `DocumentMeta` carries that metadata, and lens columns
  read arbitrary metadata keys. However, it does **not** support nested source
  metadata such as `related = [{ target = ..., kind = ... }]` without extending
  `MetadataValue` and the SQLite metadata index beyond scalar/string-list
  values.
- [assumption] Browser capture can be implemented safely with a local bridge or
  share/bookmarklet flow without opening broad unauthenticated write access to a
  project.
- Which citation interchange should be the first import/export path: BibTeX,
  CSL JSON, or Zotero Web API?
- Should source notes live in a conventional `sources/` folder by default, or be
  allowed anywhere with `kind = "source"` as the only authority?
- How should extracted annotations be represented: sections inside the source
  note, child notes, or sidecar files?
- What is the minimum duplicate-detection UI that is useful without becoming a
  merge-conflict workflow?

## Recommended first slice

Implement **source note foundation + BibTeX/CSL import**:

1. Define the source frontmatter schema.
2. Add source lenses and source-specific rendering.
3. Add importer for BibTeX or CSL JSON.
4. Preserve citation keys.
5. Add source-linked research task conventions.
6. Add agent-generated Design Board source cards as refreshable projections.
7. Add agent guidance/tools for creating and querying source notes.

This gives Flynt immediate research leverage while keeping Zotero as the system
of record for advanced capture/citation workflows until Flynt has evidence that
more native functionality is worth building.
