---
title: Project Registry Persistence and Sync Design
status: exploring
tags: [project-registry, persistence, sync, plaintext]
---

# Project Registry Persistence and Sync Design

## Decision boundary

`ProjectRegistry` is primarily a derived index over plaintext project files. Persistence must not turn it into the authoritative source of project truth.

The canonical project remains:

- markdown documents and frontmatter
- visual artifact source files
- wrappers and render sidecars
- task/spec/design files
- open/plain/web-native assets
- external references as text

Persistent registry data is allowed only where it improves performance, diagnostics, collaboration, or preserves state that cannot be derived from source files.

## Persistence classes

### 1. Derived cache — local, disposable

Location:

```text
.flynt/cache/project-registry.json
```

or platform app-cache storage if `.flynt/cache` is not desired in repo-visible space.

Properties:

- safe to delete
- safe to rebuild
- not authoritative
- normally gitignored / not synced
- may include absolute paths and timestamps only if marked local/runtime
- can store expensive computed indexes

Examples:

- resolved wikilink graph
- visual artifact discovery output
- evidence stream counts and IDs
- raw asset inventory
- render freshness status
- thumbnail/cache metadata

### 2. Sync-safe snapshot — plaintext, optional, derived

Location candidate:

```text
.flynt/registry/project-registry.snapshot.json
```

Properties:

- deterministic ordering
- project-relative paths only
- no absolute paths
- no machine-specific timestamps
- safe to delete and rebuild
- useful for code review, diagnostics, and agent context
- should be regenerated, not manually edited
- should not be required for the app to open

This snapshot may be committed only if the project chooses to commit generated registry diagnostics. Default should likely be uncommitted until the workflow proves useful.

### 3. Durable user/project state — authoritative only for non-derivable intent

Location candidates:

```text
.flynt/project.toml
.flynt/registry/user-state.toml
.flynt/registry/artifact-state.toml
```

Properties:

- plaintext
- project-relative paths
- explicitly user-authored or user-approved
- small and stable
- suitable for git/iCloud sync

Examples that may warrant true persistence:

- user-created aliases not stored in document frontmatter
- pinned graph layouts or saved views
- artifact display preferences that are not encoded in source/wrapper
- manual artifact titles when no wrapper/document title exists
- user-confirmed external reference labels
- per-project registry feature flags
- durable artifact lineage after rename/move, if implemented

## What warrants real persistence?

Only persist information that is either:

1. **Non-derivable user intent** — cannot be reconstructed from plaintext source files.
2. **Expensive but harmless cache** — can be rebuilt, but storing improves UX. This must remain disposable.
3. **Cross-device collaboration state** — useful across machines and not derivable, e.g. shared saved graph layouts.
4. **Migration/lineage state** — records semantic continuity across file moves/renames when path-based IDs would otherwise treat the artifact as new.

Do **not** persist:

- duplicate document content
- duplicate source artifact content
- absolute project roots
- local render timestamps as sync truth
- machine-specific paths
- raw proprietary binary blobs
- registry state that conflicts with markdown/frontmatter/source files

## Recommended first persistence target

Do not persist the full `ProjectRegistry` yet.

First add a **snapshot serializer** that produces a sync-safe diagnostic view from an in-memory registry:

```rust
pub struct ProjectRegistrySnapshot {
    pub schema: String,
    pub generated_by: String,
    pub documents: Vec<DocumentSnapshot>,
    pub visual_artifacts: Vec<VisualArtifactSnapshot>,
    pub evidence_sources: Vec<EvidenceSourceSnapshot>,
    pub edges: Vec<ProjectEdge>,
}
```

This should deliberately omit:

- `ProjectScope.project_root`
- absolute paths
- local-only cache metadata
- full frontmatter blobs if duplicated elsewhere

Initial use:

- debugging
- tests
- agent context
- graph/sync diagnostics

No automatic write-on-every-change yet.

## Snapshot rules

- Project-relative paths only.
- Deterministic sort by stable ids/paths.
- Include schema version.
- Include source fingerprints only if cheap and project-relative.
- Do not include generated-at wall clock time by default; timestamps create noisy diffs.
- Do not block app startup if snapshot is missing/stale.
- Snapshot staleness should be advisory only.

## Cache rules

If Flynt later adds a local cache:

- put it under ignored cache storage
- allow full rebuild
- version it separately from sync-safe snapshots
- include runtime-only values freely if the file is not synced
- never use cache data to overwrite source files without revalidating against current disk state

## Sync interactions

### Git

Git should sync canonical plaintext/source files. Registry snapshots, if committed, are review aids and diagnostics, not truth.

Merge conflicts in snapshots should be solved by regenerating them.

### iCloud/Dropbox/Syncthing

Sync tools may update files asynchronously. The registry must tolerate:

- missing render sidecars
- stale wrappers
- partially written JSONL evidence streams
- temporarily inconsistent markdown/link state

Discovery should degrade with warnings, not fail the project.

### Multi-clone machines

Two checkouts of the same repo have different absolute paths. Therefore:

- project-relative identity is mandatory
- absolute roots are runtime-only
- `project_id` may help diagnostics but must not replace path/root scoping

## Relationship to Omegon evidence maps

`.omegon/evidence/*` is an input source, not Flynt-owned persistence.

Flynt may index:

- manifest schema
- stream presence
- stream IDs
- counts
- evidence edges later

Flynt should not rewrite Omegon evidence streams unless an explicit contract is designed.

## Migration path

1. Keep current in-memory `ProjectRegistry::discover`.
2. Add `ProjectRegistrySnapshot::from_registry`.
3. Add tests proving snapshots omit absolute roots.
4. Add an explicit command/tool to write a snapshot.
5. Decide later whether any snapshot should be auto-written.
6. Separately design durable user/project state for non-derivable preferences.

## Open questions

- Should sync-safe snapshots live under `.flynt/registry/` or be app-cache only until explicitly enabled?
- Should snapshot files be included in `.gitignore` by default?
- Do saved graph layouts belong in `.flynt/project.toml`, document frontmatter, or a registry state file?
- How should artifact rename lineage be represented without creating a fragile database?
