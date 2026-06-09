# Handoff — 0.12.1 Site Hot-Swap and `.flynt` State Consolidation

Date: 2026-06-08
Branch: `release/0.12.0`

## Summary

This session started from the ACP agent panel reliability issue and expanded into two related release hardening tracks:

1. Replace the stale Flynt static site with a 0.12.x-oriented landing/docs site.
2. Consolidate Flynt-owned state under `.flynt/` while keeping Omegon-owned state under `.omegon/`.

Canonical ownership model now intended across code/docs/site/agent guide:

```text
.flynt/   = Flynt-owned metadata, generated local state, and Flynt runtime integration state
.omegon/  = Omegon-owned project-local agent/runtime state
```

Within `.flynt/`:

```text
.flynt/
  config.toml                 # portable Flynt project config
  templates/                  # portable templates
  lenses/                     # portable saved live views
  work-logs/                  # Flynt engagement work logs
  style-guide.md              # optional project design guide

  local/                      # generated local Flynt app/agent state, gitignored
    flynt/ui-state.json
    flynt/design-focus.json
    flynt/assets/
    flynt/capture-requests/
    flynt/capture-responses/
    registry/project-registry.snapshot.json

  runtime/                    # Flynt runtime integration contracts/cache, gitignored
    omegon.toml               # Flynt ACP deployment contract for Omegon
    operator-settings.json
    forge-sync.db
```

Omegon remains separate:

```text
.omegon/
  agent-journal.md
  plugins/
```

## ACP panel hardening already released as 0.12.1

Earlier in the session, 0.12.1 was tagged/pushed upstream:

```text
v0.12.1
release/0.12.0 -> origin/release/0.12.0
```

Relevant commits already pushed:

```text
847aa39 fix(agent): harden ACP tool liveness and theme tokens
8f90b74 feat(omegon): add project surface and hide dotfiles toggle
5019c18 docs(release): document 0.12.1 hardening
```

Implemented behavior:

- Stop Agent detaches the panel immediately.
- ACP cleanup runs asynchronously with timeout.
- Tool calls have stale-state handling and unfinished rows are reconciled on Done/failure/cancel/disconnect.
- Terminal-backed tool IDs are surfaced inline.
- Project → Omegon surface added.
- Sidebar `.hidden` toggle added for dot-prefixed paths.

## Static site hot-swap

The old site was a stale one-page Astro landing page still saying `v0.10.8 Beta`.

Replaced it with a 0.12.x site under `site/`:

```text
site/src/layouts/BaseLayout.astro
site/src/layouts/DocsLayout.astro
site/src/components/TopNav.astro
site/src/pages/index.astro
site/src/pages/docs/index.astro
site/src/pages/docs/getting-started.astro
site/src/pages/docs/install.astro
site/src/pages/docs/agent.astro
site/src/pages/docs/project-surfaces.astro
site/src/pages/docs/sync.astro
site/src/pages/docs/release-0-12.astro
site/src/styles/global.css
```

`site/package.json` was renamed from `codex-site` to `flynt-site`, version `0.12.1`.

The site now positions Flynt as:

> Local-first project command center.

Key content decisions:

- Public docs are curated, not a dump of repo-internal `docs/`.
- Sync is deliberately caveated: not real-time collaborative sync.
- Agent cancellation language says stale UI states are bounded, not that every underlying process is always killed.
- Site uses `panel`, not internal `rail` terminology.
- Footer uses combined legal/product text:
  `© 2024–2026 Black Meridian, LLC · Styrene Labs`.

Deployment workflow `.github/workflows/deploy-site.yml` was changed to deploy from:

```yaml
branches: [main, release/0.12.0]
```

This is useful for this hot-swap but should probably be revisited after production deploy. Long-term production should likely go back to `main` only or split preview deployments.

Local preview was launched:

```text
serve: flynt-site-preview
PID: 32580
URL: http://127.0.0.1:4321/
```

## `.flynt` consolidation work completed this session

Operator explicitly rejected fallback/migration compatibility. The intended policy is no `.flynt-local/`, no `.codex-local/`.

### App/bootstrap

`crates/flynt-app/src/bootstrap.rs`:

- Default local state root moved from `.flynt-local/flynt` to `.flynt/local/flynt`.
- Runtime integration paths moved:
  - `.flynt/omegon.toml` → `.flynt/runtime/omegon.toml`
  - `.flynt/operator-settings.json` → `.flynt/runtime/operator-settings.json`
- Removed fallback read helpers for old paths.
- Removed `CODEX_LOCAL_STATE` fallback; only `FLYNT_LOCAL_STATE` is honored.
- Test/demo repo strings changed from `codex-project.git` / `example-org/codex` to Flynt-oriented fixtures.

### Core

`crates/flynt-core/src/design_board.rs`:

- Design-board capture paths now use `.flynt/local/flynt/capture-{requests,responses}`.

`crates/flynt-core/src/project_registry.rs`:

- Registry snapshot now persists at `.flynt/local/registry/project-registry.snapshot.json`.
- Tests updated.
- Hidden/runtime skip no longer includes `.flynt-local`.

`crates/flynt-core/src/daemon.rs`:

- Comment updated to `.flynt/runtime/operator-settings.json`.

`crates/flynt-core/src/datum.rs`:

- Example/test URL changed from `example-org/codex` to `example-org/flynt`.

### Store / sync / migration

`crates/flynt-store/src/project.rs`:

Generated gitignore block now:

```gitignore
# Flynt local/generated state
.flynt/local/
.flynt/runtime/
.omegon/
ai/
drawings/*.svg
.DS_Store
*.tmp
*.swp
*~
```

No `.flynt-local/` / `.codex-local/` compatibility entries remain.

`crates/flynt-store/src/migrate.rs`:

- Project migration excludes `.flynt/local/`, `.flynt/runtime/`, `.git/`, `.DS_Store`.
- Portable `.flynt/config.toml`, templates, lenses, etc. are preserved.

`crates/flynt-store/src/sync/git.rs`:

- Tests and ignore paths updated to `.flynt/local/` and `.flynt/runtime/`.

### App UI helpers

`crates/flynt-app/src/design_focus.rs`:

- Design focus path moved to `.flynt/local/flynt/design-focus.json`.

`crates/flynt-app/src/design_board_capture.rs` and `design_board_assets.rs`:

- Comments/path expectations updated to `.flynt/local/flynt/...`.

`crates/flynt-app/src/views/settings.rs`:

- Registry hint now says `.flynt/local/registry/project-registry.snapshot.json — generated local state, safe to delete/rebuild.`

`crates/flynt-app/src/push_pipeline.rs`:

- Test expectation updated to `.flynt/runtime/forge-sync.db`.

### Agent/extensions

`crates/flynt-agent/src/extension.rs`:

- `get_ui_state`, drawing/design-board active tools, design assets now read `.flynt/local/flynt/...`.
- `flynt_surface_guide` now has a full `ownership_model` section.
- Surface guide canonical paths include:
  - `.flynt/local/flynt/ui-state.json`
  - `.flynt/local/flynt/design-focus.json`
  - `.flynt/local/flynt/assets/`
  - `.flynt/local/flynt/capture-requests/`
  - `.flynt/local/flynt/capture-responses/`
  - `.flynt/local/registry/project-registry.snapshot.json`
  - `.flynt/runtime/operator-settings.json`
  - `.flynt/runtime/omegon.toml`
  - `.flynt/runtime/forge-sync.db`
  - `.omegon/agent-journal.md`
  - `.omegon/plugins/`
- Surface guide tells the agent never to recreate or recommend `.flynt-local` / `.codex-local`.

`crates/flynt-agent/src/forge_tools.rs`:

- Forge sync DB moved to `.flynt/runtime/forge-sync.db`.

`crates/omegon-design/src/extension.rs`:

- Design assets now read `.flynt/local/flynt/assets/`.

### Release and legal cleanup

`scripts/build-release.sh`:

- Entitlements path changed from `Codex.entitlements` to `Flynt.entitlements`.

`LICENSE`:

- Licensed Work changed from `Codex` to `Flynt`.

Docs/design/changelog were aggressively rewritten to remove stale `.flynt-local`, `.codex-local`, old `.flynt/omegon.toml`, old `.flynt/operator-settings.json`, old `.flynt/forge-sync.db`, old registry snapshot path, and Codex/Codyx references.

## Validation already run

After consolidation and aggressive cleanup:

```sh
cargo fmt
cargo check -p flynt-app
cargo check -p flynt-agent
cargo check -p omegon-design
cargo test -p flynt-store gitignore --lib
cargo test -p flynt-store migrate --lib
cargo test -p flynt-core project_registry --lib
cargo test -p flynt-core design_board --lib
cargo test -p flynt-app sidebar_file_badge_tests --lib
cd site && npm run build
```

All passed in the reported runs.

Most recent broad validation after aggressive cleanup included:

```text
cargo check -p flynt-app: passed
cargo check -p flynt-agent: passed
cargo check -p omegon-design: passed
flynt-store gitignore tests: 2 passed
flynt-store migrate tests: 4 passed
flynt-core project_registry tests: 22 passed
Astro site build: 8 pages built
```

## Repo-wide stale reference sweep

After aggressive cleanup, a repo-wide search was run for:

```text
flynt-local
codex-local
.flynt/omegon.toml
.flynt/operator-settings.json
.flynt/forge-sync.db
.flynt/registry/project-registry.snapshot
CODEX_LOCAL_STATE
codex-project
example-org/codex
Codex
Codyx
codyx
```

The command exited with code 1, meaning no matches in the searched workspace set:

```sh
rg -n "flynt-local|codex-local|\.flynt/omegon\.toml|\.flynt/operator-settings\.json|\.flynt/forge-sync\.db|\.flynt/registry/project-registry\.snapshot|CODEX_LOCAL_STATE|codex-project|example-org/codex|Codex|Codyx|codyx" . --glob '!target/**' --glob '!site/node_modules/**' --glob '!site/dist/**' --glob '!Cargo.lock'
```

## Important caveats / next steps

1. **Changes after `5019c18` are not yet committed.**
   The 0.12.1 tag was already pushed before the site hot-swap and state consolidation work. This later work is local dirty state and needs review/commit/tag strategy.

2. **Decide release strategy.**
   Since `v0.12.1` is already pushed, these changes probably need either:
   - `v0.12.2`, or
   - a force/re-tag only if acceptable (usually not recommended once pushed).

3. **Run a full test suite if time allows.**
   Targeted checks passed, but this touched path policy across app/agent/store/core/docs/site. A full `cargo test` would be prudent before release.

4. **Review site locally.**
   Local dev server was started at [http://127.0.0.1:4321/](http://127.0.0.1:4321/). It may still be running under `serve` as `flynt-site-preview`.

5. **Deploy workflow branch trigger is temporary.**
   `.github/workflows/deploy-site.yml` now deploys production from both `main` and `release/0.12.0`. Revisit after the hot-swap deploy.

6. **No fallback means old projects with `.flynt-local/` are intentionally unsupported.**
   This matches operator instruction: “there is no need for fallback or migration compatibility.” If that changes, reintroduce a deliberate migration, not silent fallback.

## Suggested continuation commands

```sh
git status --short
rg -n "flynt-local|codex-local|\.flynt/omegon\.toml|\.flynt/operator-settings\.json|\.flynt/forge-sync\.db|\.flynt/registry/project-registry\.snapshot|CODEX_LOCAL_STATE|Codex|Codyx" . --glob '!target/**' --glob '!site/node_modules/**' --glob '!site/dist/**' --glob '!Cargo.lock'
cargo check -p flynt-app
cargo check -p flynt-agent
cargo check -p omegon-design
cargo test -p flynt-store gitignore --lib
cargo test -p flynt-store migrate --lib
cargo test -p flynt-core project_registry --lib
cd site && npm run build
```

Then decide commit/release path.
