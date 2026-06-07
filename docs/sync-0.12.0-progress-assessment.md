---
title: Sync 0.12.0 Progress Assessment
date: 2026-06-07
status: assessed
tags: [sync, release, icloud, git, idempotency]
---

# Sync 0.12.0 Progress Assessment

This note captures the current sync-hardening state after the userspace/project-state separation and open-idle audits.

## Core policy decision

Flynt now treats the vault as portable project truth and userspace as runtime/cache/UI state.

### Kept in the vault

Project-scoped, portable, intentional data:

- `.flynt/config.toml`
- `.flynt/lenses/*.toml`
- `.flynt/templates/*.md` when user-customized or explicitly installed
- Markdown notes/tasks/docs
- `.excalidraw` source files when intentionally edited
- design board/project artifacts that are project content

### Moved out of the vault or no longer written on idle open

Machine/runtime/generated state:

- `.flynt-local/flynt/ui-state.json`
- `.flynt-local/flynt/assets/*.json`
- `.flynt/forge-sync.db`
- `.flynt/registry/project-registry.snapshot.json` unless `track_index_snapshot=true`
- `.flynt/omegon.toml` unless operator explicitly persists deployment config
- `.flynt/operator-settings.json`
- `.omegon/`
- `ai/`
- generated SVG exports unless explicitly produced/tracked

## Accepted exception

For Git repos, Flynt may append an idempotent managed `.gitignore` block on open:

```gitignore
# Flynt local/generated state
.flynt-local/
.codex-local/
.omegon/
ai/
.flynt/forge-sync.db
.flynt/operator-settings.json
.flynt/omegon.toml
.flynt/registry/project-registry.snapshot.json
drawings/*.svg
.DS_Store
*.tmp
*.swp
*~
```

This is intentionally allowed for Git safety. It is the only accepted existing-folder idle-open mutation.

## Implemented hardening

### Open-idle own-goals removed

Existing-folder idle open no longer creates or mutates:

- Markdown/frontmatter content
- `.flynt/config.toml`
- `.flynt/forge-sync.db`
- `.flynt/omegon.toml`
- `.flynt/registry/project-registry.snapshot.json`
- `.flynt/templates/*.md`
- `.flynt-local/*` inside the vault
- `.omegon/*`
- `ai/*`

### Userspace runtime migration

- UI state now writes under the userspace per-project runtime root.
- Forge sync DB now writes under the userspace per-project runtime root.

### Frontmatter policy

- Existing folders/vaults default `write_frontmatter=false` until portable metadata or managed scopes are explicitly enabled.
- New Flynt-managed project creation uses `Project::open_managed()` and may materialize portable metadata intentionally.

### Registry/deployment/template writes

- Registry snapshot refresh only writes project snapshot when `track_index_snapshot=true`.
- Omegon deployment manifest is loaded from defaults on idle open; `.flynt/omegon.toml` is not created unless persisted intentionally.
- Default templates are no longer materialized into `.flynt/templates/` on idle open.

### Drawing no-op behavior

The drawing no-op audit passed. Opening `drawings/Test.md` and waiting produced no forbidden mutations:

- no `.excalidraw` rewrite
- no `.md` rewrite
- no `.svg` creation/modification
- no `.flynt/*` creation
- no `.flynt-local/*` creation in vault
- no `.omegon/*`
- no `ai/*`

Only the accepted `.gitignore` safety block was added.

### Git safety behavior

- Auto-sync halts on conflict.
- Git pull is fast-forward-only for 0.12.0.
- Divergent histories return a blocker/conflict without mutating the worktree.
- Preflight blocks detached HEAD, merge/rebase/cherry-pick states, index conflicts, and missing remote refs.

## Audit evidence

### Existing folder open-idle

Fixture:

```text
target/sync-audit/post-userspace-existing-folder
```

Result:

```text
Added: .gitignore
Modified: none
Removed: none
```

Repeated open including local-state paths:

```text
Added: none
Modified: none
Removed: none
```

### Drawing no-op

Fixture:

```text
target/sync-audit/drawing-noop-vault
```

Result:

```text
Added: .gitignore
Modified: none
Removed: none
```

## Remaining validation before release claim

### iCloud

1. One-Mac iCloud open-idle audit with local-state included.
2. Two-Mac iCloud single-writer validation.
3. Two-Mac iCloud same-file conflict characterization.

Release wording should remain conservative:

> Flynt is idempotent in iCloud-backed folders and avoids self-churn. iCloud conflict copies remain provider/manual.

### Git

1. Two-clone fast-forward A → B and B → A validation.
2. Divergence blocker validation in real clones.
3. Conflict/unsafe-state UI surfacing.

Release wording should remain conservative:

> Git sync supports clean fast-forward workflows and blocks unsafe/divergent states. Manual resolution is required for divergence/conflicts.

## 0.12.0 release recommendation

Sync can be included in 0.12.0 as guarded/internal-beta functionality if the release notes and UI copy are conservative:

- iCloud: supported as a folder-backed storage location; no merge claims.
- Git: fast-forward/manual-sync safe path; divergence is blocked, not merged.
- Auto-sync: keep off or guarded unless diagnostics are clean.
- Existing-folder open: non-destructive except the managed Git `.gitignore` safety block.

Do not claim automatic multi-writer conflict resolution in 0.12.0.
