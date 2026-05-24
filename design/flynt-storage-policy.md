+++
id = "flynt-storage-policy"
kind = "design_node"
title = "Flynt storage policy — portable metadata and local runtime state"
status = "active"
tags = ["storage", "project-model", "dogfooding", "release-0.11.0"]

[data]
issue_type = "design"
priority = 1
+++

# Flynt storage policy

## Problem

Opening a folder in Flynt must not create competing sources of truth inside that folder. The opened folder is user-owned content. Flynt may index it, render it, and optionally attach portable metadata to it, but runtime state and derived indexes should not appear beside the user's files by default.

Dogfooding this repository exposed the general problem: an implicit open created both `.flynt/` and `.flynt-local/`, plus agent lifecycle state. That is not a Flynt-repo special case; the same sprawl would be confusing in any source tree, customer bundle, research folder, or note vault.

## Definitions

- **Content root** — the directory the operator opened.
- **Portable metadata** — Flynt metadata intentionally meant to travel with the content root, such as project config, templates, shared lenses, or a project style guide.
- **Local runtime state** — device-specific state such as indexes, WAL files, UI state, capture queues, model/session choices, and caches.
- **Canonical project truth** — the user's actual project files: source, notes, docs, design records, specs, changelog, tests, and workflow files.

## Policy

- Opening a folder is read/index-only by default.
- Flynt must not create `.flynt-local/` in the content root for new opens.
- Local runtime state defaults to the platform app-data directory, keyed by the opened content root.
- `.flynt/` is reserved for portable metadata and should be created only by an explicit project-initialization action.
- Existing `.flynt/` metadata remains supported.
- Existing `.flynt-local/` paths remain supported when explicitly configured or encountered as legacy state, but new default paths should not point there.

## First-pass implementation for 0.11.0

The 0.11.0 starting point is intentionally narrow:

1. Stop default index database creation under `<content-root>/.flynt-local/`.
2. Resolve the default index database under external app data instead.
3. Preserve explicit absolute `local_runtime.flynt_index_db_path` overrides.
4. Preserve explicit absolute `local_runtime.local_state_root` overrides.
5. Keep existing `.flynt/config.toml` compatibility.
6. Add a settings toggle for writing a deterministic `.flynt/index.snapshot.jsonl` metadata snapshot when a project should carry portable Flynt index metadata.

This does not yet remove every `.flynt-local/` writer. Canvas capture assets, UI-state mirroring, and other runtime mirrors still need the same state-root abstraction. They should follow this policy in subsequent 0.11.x work.

## Non-goals

- Removing support for existing Flynt projects with `.flynt/` metadata.
- Forcing all portable metadata out of user projects.
- Designing a one-off exception for the Flynt source repository.
- Completing every runtime-state migration in the first pass.
