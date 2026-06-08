# Changelog

## 0.12.0 — 2026-06-08

### Added
- **Omegon plan/task ACP projections** — Flynt can read Omegon `_plans/*`, `_tasks/*`, runtime capability, task source, revision, binding, and external-import contract surfaces.
- **Omegon task promotion flow** — Kanban tasks can be promoted into Omegon review context, with local pending drafts and explicit session/repo/conflict binding state chips.
- **QA surface fixture coverage** — compact vault fixtures exercise notes, design boards, drawings, flows, task ingestion, and promotion states.

### Fixed
- **Task markdown ingestion** — startup indexing and file watcher reindex now materialize `kind = "task"` markdown into the Kanban task table without rewriting operator-authored files.
- **Surface artifact context** — note Context/Links resolves `.board`, `.excalidraw`, and `.flow` visual artifacts instead of marking existing surface embeds as missing notes.
- **Flow failure visibility** — invalid `.flow` files now surface parse/read errors instead of silently appearing as an empty starter canvas.
- **Omegon provider warnings** — unknown or GPT display model names no longer fall through to Ollama warnings.
- **Kanban readability** — task chips and agent tool-call status rows use higher-contrast styling.
- **macOS bundle icon packaging** — release bundling preserves the `.icns` app icon in the copied app bundle.

## 0.11.2 — 2026-05-27

### Added
- **ACP HostAction terminal review** — Flynt now advertises ACP terminal host capability, receives Omegon `terminal.create@1` approval requests, renders review cards in the agent rail, and opens approved sessions through Flynt's `TerminalManager`.
- **Native Flynt terminal substrate** — added a reusable portable-pty + alacritty terminal manager with session listing, snapshot rendering, keyboard input, kill/release, resize, and exited-session cleanup.
- **Terminal HostAction assessment vault** — added an end-to-end assessment workspace with a PDF fixture for reader-driven `terminal.create@1` validation.

### Fixed
- **Omegon binary resolution** — Flynt now prefers the active user Omegon binary before stale channel-version installs unless explicitly pinned.
- **ACP reconnect stability** — reconnect no longer panics when session status UI holds a signal read borrow.
- **Terminal usability** — Terminal Lab no longer auto-runs diagnostics noise, captures Tab for TUIs, resizes the PTY to the rendered viewport, clears exited sessions, and respects the operator's configured `$SHELL` path.

## 0.11.1 — 2026-05-24

### Fixed
- Updated the source-template idempotence test to match the fourth default template added in 0.11.0.
- Raised the lipstyk PR diff threshold for the existing feature branch so CI gates the release branch consistently while legacy findings remain visible in SARIF.

## 0.11.0 — 2026-05-22

### Added
- **Research workspace groundwork** — source-note templates and Flynt agent surface guidance now establish the first source-backed research workspace path.
- **Portable analysis bundle design** — documented provenance-preserving bundles with source manifests, access scope, authorization notes, artifacts, and analysis outputs.
- **Source task/canvas projection design** — documented how source-backed artifacts project into task and canvas workflows without duplicating source truth.
- **Eidolon embedded viewer integration design** — defined the boundary for reviewing captured/source-backed evidence through an embedded viewer.
- **Omegon 0.23 ACP alignment** — Flynt now preserves Omegon-owned profile defaults on ACP session startup and only replays explicit operator-selected config overrides.
- **Storage policy first pass** — Flynt now documents the portable-metadata/local-runtime-state boundary, defaults new index databases outside the opened content root instead of under `.flynt-local/`, and exposes an opt-in tracked JSONL index snapshot for repos that should carry portable metadata.

### Fixed
- **Flynt surface guide execution test** — the agent extension test now calls the executable `execute_flynt_surface_guide` RPC while still advertising the user-facing `flynt_surface_guide` tool.

## 0.10.8 — 2026-05-20

### Fixed
- **Embedded Omegon streaming performance** — agent text/thought deltas are now
  batched before updating the chat rail, reducing Dioxus re-renders and WebView
  repaint pressure while responses stream.
- **Agent chat scroll smoothness** — sticky-bottom scrolling is coalesced through
  `requestAnimationFrame` instead of forcing layout on every DOM mutation.
- **WSL responsiveness while the agent is busy** — the actively streaming
  assistant message renders as plain text until completion, then switches to the
  existing markdown renderer once the response is idle.
- **Streaming order preservation** — adversarial review found that separate text
  and thought buffers could reorder interleaved ACP deltas at flush boundaries;
  batching now preserves delta order while still coalescing adjacent chunks.

## 0.10.5 — 2026-05-16

### Added
- **Semantic Excalidraw authoring tools** — integrated agents can now create,
  validate, render, inspect, and patch drawings through `DrawingSpec`
  components/connections instead of hand-generating raw Excalidraw JSON.
- **Drawing sidecars** — agent-authored Excalidraw diagrams persist a
  `drawings/<name>.drawing.json` sidecar so future edits can patch semantic
  components deterministically.

### Fixed
- **Excalidraw autosave blanking** — SVG auto-export no longer unmounts the
  visible editor while a drawing is open.
- **Release asset packaging** — direct macOS release bundles now retain full
  vendored visual assets as unhashed fallbacks, and stable release tags must
  match the workspace version.

## 0.9.0 — 2026-05-07

UX and perf rewrite of the sidebar + agent rail, Dioxus 0.7.9 upgrade, and a two-pass adversarial review.

### Added
- **Resizable sidebar** with projects pinned at top and collapse-to-icons mode
- **File-tree sidebar rewrite** with multi-level folder nesting and persistent panel widths
- **Collapsible graph filters** + fuzzy tag search
- **Drag-divider for agent panel resize** + textarea auto-grow on long prompts
- **Inline session status** in the agent rail with provider auth warnings (expired / missing credentials surfaced before the next prompt fails)
- **Auth-expired status badge** — visible signal when `/login` is needed

### Changed
- **Dioxus 0.6 → 0.7.9** across all view crates
- **Render cache for instant tab switching** — VDOM no longer re-diffs the full notes view on every tab change (was the root of the 1.4s click delay)

### Fixed
- **1.4s sidebar-click delay** caused by a redundant route write in the click handler
- Adversarial review pass 1: divider race, JSON escape in serialized state, eval leak in markdown render
- Adversarial review pass 2: hook stability, deterministic sort keys, provider matching, polling interval correctness

## 0.7.0 — 2026-05-06

First release under the new name. **Codyx is now Flynt.** Binaries, bundle IDs, asset paths, and the project site (https://flynt.styrene.io) all migrated. Existing projects and configuration continue to work unchanged.

### Added
- **Embedded Omegon agent configuration GUI** — configure the agent surface (model, tools, project scope) directly from the desktop app, no separate config file editing
- **Git login flow** — first-class git authentication for sync, including SSH key selection and credential helper integration
- **Tracing instrumentation** — structured `tracing` spans across project, sync, and ACP layers for diagnostics

### Changed
- **Rebrand: Codyx → Flynt** across crates (`flynt-models`, `flynt-store`, `flynt-app`, `flynt-agent`, `flynt-mobile`), binaries, desktop entries, and asset paths
- **Bundle ID** remains `io.styrene.flynt` (unchanged from 0.5.0)
- **Homebrew formula** moved to `Flynt` class on `styrene-lab/homebrew-tap`

### Fixed
- **ACP** — corrections to the Agent Control Protocol surface uncovered while wiring the embedded Omegon GUI

## 0.5.0 — 2026-04-23

### Added
- **iOS Share Extension** — save links, text, and images from any iOS app into your Flynt project via the system Share Sheet
- **Obsidian-style live preview** — CM6 editor now hides markdown syntax (headings, bold, links, tables) and reveals on cursor focus, using the Lezer syntax tree instead of regex
- **Table widget rendering** — markdown tables render as styled HTML tables; click into them to edit raw markdown
- **Frontmatter hiding** — TOML frontmatter is collapsed when cursor is outside it
- **Wikilink click navigation** — Cmd+click on `[[wikilinks]]` navigates to the target note
- **Context menu** — right-click in the editor for formatting options (bold, italic, headings, code blocks, tables, etc.)
- **Rename Save button** — document rename now has an explicit Save button alongside Enter/Cancel
- **TestFlight distribution** — both macOS and iOS builds upload to TestFlight via CI
- **App Store Connect integration** — `just testflight` builds and uploads both platforms

### Changed
- **Bundle ID** — migrated from `com.black-meridian.flynt` to `io.styrene.flynt`
- **CM6 bundle** — rebuilt with live preview extensions (Lezer-based syntax hiding, StateField block decorations)
- **Version** now pulled from `Cargo.toml` automatically in Justfile
- **macOS TestFlight** builds include App Sandbox, JIT, and network entitlements
- **iOS Info.plist** patching uses actool partial plist merge (fixes App Store validation)

### Fixed
- **IndexingConfig** — existing repos opened as projects no longer get frontmatter injected into source files
- **App Store icon validation** — actool's generated CFBundleIcons plist is now merged properly instead of hand-crafted
- **macOS sandbox crash** — added `network.server` entitlement for Dioxus edit socket
- **Quarantine xattr** — stripped from provisioning profiles before packaging
- **IPA packaging** — uses `xcodebuild -exportArchive` instead of manual zip for correct bundle structure

## 0.4.0 — 2026-04-20

### Added
- Context menus with cut/copy/paste and formatting
- Wikilink rendering and click-to-navigate in CM6 editor
- Graph edge improvements and force layout tuning
- Sticky notes toolbar

## 0.3.0 — 2026-04-18

### Added
- Sticky topbar with compact title
- Excalidraw drawing integration (8MB lazy-loaded bundle)
- Query blocks (TABLE, LIST, TASK inline queries)
- Daily notes with templates

## 0.2.0 — 2026-04-15

### Added
- Kanban boards with task decay model
- Git sync (auto-commit, push, pull)
- Knowledge graph with D3 force layout
- Search with FTS5

## 0.1.0 — 2026-04-10

### Added
- Initial release
- Markdown notes with TOML frontmatter
- Wikilinks and backlinks
- SQLite index with full-text search
- macOS and Linux desktop builds
