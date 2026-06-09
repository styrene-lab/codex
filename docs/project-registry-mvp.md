# Project Registry MVP

The Project Registry is a generated index of the open Flynt project. It is designed to make project relationships inspectable without replacing the underlying plaintext/store model.

## Persistence

Flynt writes a generated snapshot at:

```text
.flynt/local/registry/project-registry.snapshot.json
```

This file is derived state. It is safe to delete and rebuild. The snapshot must not be treated as authoritative project truth.

## Source of truth

The source of truth remains:

- Markdown/plaintext documents and frontmatter
- visual artifact source files such as `.excalidraw`, `.d2`, and design-board files
- task/board data from the existing project store
- OpenSpec files under `openspec/`
- external evidence outputs as bounded, provider-owned inputs

Flynt is intended to be a superset of Obsidian-style plaintext portability. Registry features must not require compromising plaintext compatibility, project-relative paths, or the ability to inspect/edit project data outside Flynt.

## Snapshot contents

The snapshot currently includes generated tables for:

- documents
- visual artifacts
- raw assets
- external references
- evidence sources
- tasks
- boards
- OpenSpec changes
- diagnostics
- graph edges among those nodes

The snapshot omits runtime-only scope such as the absolute project root.

## Refresh surfaces

The snapshot refresh operation is exposed through a centralized command path:

- Command Palette: `Project Registry: Refresh Snapshot`
- Command Palette: `Project Registry: Dump Snapshot Summary to Log`
- Settings diagnostics: `Project Registry` actions

All UI affordances should call the shared project registry command helpers rather than duplicating execution logic.

## Startup behavior

On project open, Flynt refreshes the snapshot in the background after reindexing. Failure to build, validate, or persist the snapshot logs diagnostics but does not block project open.

## Validation rules

Snapshots are validation-gated before write. The current validation checks include:

- safe project-relative paths
- no absolute path leakage through graph path nodes
- graph edge endpoints exist in snapshot node tables
- duplicate node IDs are reported
- task references to boards/documents/external refs are structurally checked

## Non-goals for the MVP

The snapshot does not yet drive core app behavior. Do not source these from the persisted snapshot yet:

- sidebar tree
- graph view
- task state
- OpenSpec lifecycle state
- visual artifact rendering/editing behavior

Those can be considered later after a separate design pass.

## Compatibility promise

Any future registry-backed feature must preserve these constraints:

1. Plaintext remains usable without Flynt.
2. Generated indexes are disposable.
3. Project state should be project-relative and portable.
4. Proprietary formats may be referenced/imported/exported, but must not become required semantic storage.
5. Command execution should be centralized so command palette, settings, sidebar, and future UI affordances share one implementation path.
