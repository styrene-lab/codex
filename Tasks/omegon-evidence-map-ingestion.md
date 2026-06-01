---
title: Recognize Omegon evidence maps as incoming EvidenceRegistry source
status: todo
priority: high
tags: [project-registry, evidence-registry, omegon, ingestion]
---

# Recognize Omegon evidence maps as incoming EvidenceRegistry source

## Context

Omegon is adding a project-wide generated evidence/surface substrate under `.omegon/evidence/`.

This is intended to be a known Flynt ingestion point, similar in spirit to discovering `.claude/`, `.codex/`, generated Doxygen sites, Sphinx/ReadTheDocs output, rustdoc, coverage reports, or other specialized project evidence/artifact sources.

Boundary:

```text
Omegon owns generation + normalization + evidence-map layout.
Flynt owns discovery + indexing + relationship/UI composition.
```

Flynt does **not** need to generate these records initially. It should discover and index them as a known incoming EvidenceRegistry source.

## Incoming layout

Expected project-root-relative layout:

```text
.omegon/evidence/
├── manifest.json
├── records.jsonl
├── surfaces.jsonl
├── edges.jsonl
├── artifacts.jsonl
└── indexes/
```

`indexes/` is derived/rebuildable. Canonical streams are manifest + JSONL files.

## Manifest handshake

Discovery rule:

```text
if <project-root>/.omegon/evidence/manifest.json exists:
    register EvidenceSourceKind::OmegonEvidenceMap
    read manifest.files
    index records/surfaces/edges/artifacts as derived registry state
```

Manifest schema identifier:

```json
{
  "schema": "omegon-evidence-manifest/v1"
}
```

Canonical file map is expected under:

```json
{
  "files": {
    "records": "records.jsonl",
    "surfaces": "surfaces.jsonl",
    "edges": "edges.jsonl",
    "artifacts": "artifacts.jsonl"
  }
}
```

## Record streams

Omegon plans these JSONL schemas:

```text
evidence-record/v1   -> temporal evidence/proof/status records
surface-record/v1    -> generated deterministic project/API/tool/config/spec surfaces
evidence-edge/v1     -> relationships between evidence, surfaces, claims, artifacts, source anchors
artifact-record/v1   -> generated/external artifact descriptors and open targets
```

Flynt can initially ingest them as loose typed records without full validation. The first requirement is discovery + visibility + stable IDs available for links/citations.

## Minimal Flynt requirements

1. Add an `EvidenceSourceKind::OmegonEvidenceMap` or equivalent classification.
2. Discover `.omegon/evidence/manifest.json` from the open project root.
3. Expose the source in `ProjectRegistry` / future `EvidenceRegistry` discovery output.
4. Preserve the source path and manifest metadata.
5. Treat the JSONL streams as derived registry input, not Flynt-authored truth.
6. Allow future Flynt docs to cite Omegon IDs such as:
   - `evidence:tdd-savepoint:redgreen-123`
   - `surface:tool:tdd_savepoint_run`
   - `artifact:doxygen:index`

## Non-goals for the first Flynt adjustment

- Do not generate Omegon evidence.
- Do not validate every provider-specific status.
- Do not force `.omegon/evidence` files to be committed or ignored; that remains project policy.
- Do not replace existing Doxygen/Sphinx/rustdoc discovery. Those remain peer evidence/artifact sources.

## Acceptance criteria

- Opening a project with `.omegon/evidence/manifest.json` registers a known evidence source.
- The source reports its schema, provider list, and canonical stream paths.
- Missing optional streams do not fail project open; they produce warnings/degraded status.
- Flynt can surface at least counts or raw IDs from `records.jsonl` and `surfaces.jsonl` when present.
