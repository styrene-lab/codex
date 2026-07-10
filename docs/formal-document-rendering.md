+++
title = "Formal Document Rendering"
tags = ["design","typst","formal-documents"]
+++

# Formal Document Rendering

## Decision

Flynt formal documents use `.typ` as the canonical source filetype. They do not render through the Markdown note renderer. Formal documents get a dedicated non-live split-pane rendering path with an explicit manual **Recompile** control.

## Product model

```text
Informal Note (.md)
  → Formalize Note
  → Formal Document (.typ)
  → Recompile
  → Preview artifact + optional PDF
```

Notes remain lightweight Markdown. Formal Documents are compiled Typst sources intended for publication-grade dissemination.

## GUI layout

```text
┌──────────────────────────────────────────────────────────────┐
│ Formal Document · documents/brief.typ                         │
│ [Recompile]  Status: Stale  Phase: idle  PDF: off  SVG preview│
├───────────────────────────────┬──────────────────────────────┤
│ Typst source editor            │ Last compiled preview         │
│                                │                              │
│ = Architecture Brief           │ preview/page-1.svg            │
│                                │                              │
│ $ E = m c^2 $                  │                              │
├───────────────────────────────┴──────────────────────────────┤
│ Diagnostics · Outline · Assets · Build manifest               │
└──────────────────────────────────────────────────────────────┘
```

The right pane shows the last successfully compiled artifact. It is not a live interpretation of the left pane.

## Build states

```rust
enum FormalDocumentBuildState {
    Missing,
    Clean,
    Dirty,
    Queued,
    Building,
    Succeeded,
    Failed,
}
```

| State | Meaning | Preview behavior |
|---|---|---|
| Missing | No artifact exists | Show first-build help state |
| Clean | Current source hash matches last successful manifest | Show current artifact |
| Dirty | Source changed since last successful build | Show last artifact with stale badge |
| Queued | Operator requested build | Keep current/last preview |
| Building | Typst build is running | Keep current/last preview and show progress |
| Succeeded | Build completed | Refresh preview artifact |
| Failed | Build failed | Keep last successful preview and show diagnostics |

Failed builds must not erase the last successful preview.

## Recompile action

Manual recompile performs:

1. Save the current `.typ` buffer.
2. Compute source hash.
3. Prepare Flynt-managed assets/context.
4. Invoke the Typst build service.
5. Capture diagnostics and timing.
6. Write build manifest.
7. Refresh preview only on success.

Pseudo-flow:

```text
on_recompile(path):
  save_source_buffer(path)
  mark_state(Queued)
  prepare_assets(path)
  mark_state(Building)
  result = typst_build(path)
  if result.ok:
      write_manifest(result)
      refresh_preview(result.preview_artifacts)
      mark_state(Clean)
  else:
      write_diagnostics(result)
      keep_last_successful_preview()
      mark_state(Failed)
```

## Progress affordance

Typst may not expose granular compile progress. Flynt should show honest phase progress, not fake page progress.

| Phase | Progress affordance |
|---|---|
| queued | 0% |
| preparing assets | 15% |
| generating Flynt context | 30% |
| compiling Typst | indeterminate/pulsing around 60% |
| writing outputs | 85% |
| refreshing preview | 95% |
| done | 100% |

## Build artifacts

Default bundle layout:

```text
reports/<slug>/
  manifest.json
  diagnostics.json
  document.pdf
  preview/
    page-1.svg
    page-2.svg
  assets/
    runtime-map.svg
    trust-boundary.svg
```

SVG preview is the first GUI target. PDF remains the dissemination artifact.

## Bundled CLI engine command shape

Preview builds render SVG pages into the bundle:

```text
typst compile
  --format svg
  --root <project-root>
  --diagnostic-format short
  --package-path <project/.flynt/typst/packages>
  --package-cache-path <project/.flynt/cache/typst/packages>
  --ignore-system-fonts
  --font-path <bundled-fonts>
  --font-path <project-fonts>...
  --input flynt_document=<source>
  --creation-timestamp <epoch>
  <source.typ>
  <output>/preview/page-{0p}-of-{t}.svg
```

PDF builds use the same world flags and write:

```text
<output>/document.pdf
```

Dependency scans use the same world flags and emit `deps.json` for manifest population. The first implementation may record the raw deps file as an asset; the next slice should parse it into manifest `assets`.

## Manifest contract

```json
{
  "kind": "formal_document_build",
  "source": "documents/architecture.typ",
  "source_sha256": "...",
  "built_at": "...",
  "typst_version": "...",
  "outputs": {
    "pdf": "reports/architecture/document.pdf",
    "preview": ["reports/architecture/preview/page-1.svg"]
  },
  "diagnostics": [],
  "assets": []
}
```

The GUI compares the current source hash against `source_sha256` to determine staleness. Timestamps are advisory only.

## Command/API shape

```json
{
  "path": "documents/architecture.typ",
  "preview": true,
  "pdf": false,
  "output_dir": "reports/architecture",
  "force": false
}
```

Success:

```json
{
  "state": "succeeded",
  "source": "documents/architecture.typ",
  "source_sha256": "...",
  "outputs": {
    "manifest": "reports/architecture/manifest.json",
    "preview": ["reports/architecture/preview/page-1.svg"],
    "pdf": null
  },
  "diagnostics": [],
  "duration_ms": 842
}
```

Failure:

```json
{
  "state": "failed",
  "source": "documents/architecture.typ",
  "outputs": {
    "last_successful_preview": ["reports/architecture/preview/page-1.svg"]
  },
  "diagnostics": [
    {
      "severity": "error",
      "message": "unknown variable: foo",
      "line": 42,
      "column": 9
    }
  ]
}
```

## Engine architecture

Formal Document rendering has one internal build path shared by the GUI, agent tools, future CLI commands, and tests:

```text
Formal Document surface
  → FormalDocumentBuildService
  → TypstEngine implementation
  → manifest + preview artifacts + optional PDF
```

The GUI never shells out to Typst directly. The build service owns Flynt invariants:

- `.typ` source validation;
- project-root containment;
- source hashing and staleness detection;
- previous successful manifest lookup;
- last-good-preview preservation;
- manifest writing;
- state calculation;
- normalized diagnostics for the GUI.

Typst engine implementations own compilation only. The initial implementation ladder is:

```text
TypstEngine
├── StubTypstEngine          deterministic contract tests only
├── BundledCliTypstEngine    first real implementation target
├── SystemTypstEngine        advanced/dev fallback
└── EmbeddedTypstEngine      later direct Rust integration
```

`BundledCliTypstEngine` is the first real target because Typst's CLI already exposes the flags Flynt needs for root containment, font policy, package paths, inputs, short diagnostics, dependency output, SVG preview, and PDF export. The embedded engine remains the preferred long-term shape once the library API contract is proven stable enough for Flynt's needs. This decision is captured in `docs/adr/0001-formal-document-typst-engine.md`.

## Dependency model

Flynt should treat Typst as an internal Formal Document build engine, not as an operator-managed prerequisite. The preferred long-term implementation is an embedded Rust service/library layer. The first real implementation should be a bundled CLI engine behind the same `TypstEngine` trait so the GUI and manifest contracts can stabilize before committing to Typst's internal crate APIs.

## Typst ecosystem policy

Flynt pins a stable Typst engine per Flynt release, but does not vendor Typst Universe wholesale. Formal Document builds run inside a Flynt-managed Typst world.

Default policy:

```text
Engine: Bundled Typst
Packages: Ask before download
Fonts: bundled + project fonts only
Plugins: ask before first plugin hash
Network: no silent network during manual Recompile
Root: project root
```

Advanced policy toggles may allow system Typst, system fonts, auto-downloaded packages, or hardened offline/plugin-deny builds, but those modes must be visible in the build manifest.

## Build world contract

Formal Document builds use the core types in `crates/flynt-core/src/formal_document.rs`:

```rust
struct FormalDocumentBuildRequest {
    source: PathBuf,
    output_dir: PathBuf,
    preview: bool,
    pdf: bool,
    force: bool,
    inputs: Vec<TypstInput>,
    world: TypstWorldPolicy,
}

struct TypstEngineOutput {
    diagnostics: Vec<ReportDiagnostic>,
    preview: Vec<PathBuf>,
    pdf: Option<PathBuf>,
    assets: Vec<PathBuf>,
    packages: Vec<TypstPackageUse>,
    fonts: Vec<TypstFontUse>,
    plugins: Vec<TypstPluginUse>,
}

trait TypstEngine {
    fn engine_info(&self) -> TypstEngineInfo;
    fn compile(&self, request: &FormalDocumentBuildRequest) -> Result<TypstEngineOutput>;
}
```

The world policy embedded in the request is:

```rust
struct TypstWorldPolicy {
    package_mode: TypstPackageMode,
    font_mode: TypstFontMode,
    plugin_mode: TypstPluginMode,
    engine_preference: TypstEnginePreference,
    project_root: PathBuf,
    package_path: PathBuf,
    package_cache_path: PathBuf,
    font_paths: Vec<PathBuf>,
    creation_timestamp: Option<i64>,
}
```

Typst packages are resolved from a Flynt-managed package path/cache and recorded by namespace, name, version, source, and hash where available. Project-local packages are supported for reproducible team documents.

Fonts are resolved from bundled and project font paths by default. System fonts are opt-in because they make output machine-dependent.

WASM plugins are supported as Typst ecosystem passthroughs, but first execution of a new plugin hash requires project-scoped approval.

## Typst upgrade testing

A Typst version bump is a compatibility-sensitive dependency update. Before a Flynt release adopts a new Typst stable version, the Formal Document test suite must validate the Flynt contract, not just whether Typst compiles one sample.

Required fixture coverage:

```text
minimal.typ
math.typ
unicode.typ
figures.typ
bibliography.typ
project-font.typ
local-package.typ
plugin-denied.typ
plugin-approved.typ
compile-error.typ
multi-page.typ
```

Required assertions:

- SVG preview artifacts are written to the expected bundle layout.
- Optional PDF output is written when requested.
- source hash appears in the manifest.
- bundled Typst version appears in the manifest.
- package/font/plugin policy appears in the manifest.
- dependency/assets list is recorded.
- diagnostics preserve source spans where Typst exposes them.
- failed builds keep the last successful preview slot and do not replace successful outputs.
- system fonts remain disabled unless the policy explicitly enables them.
- package cache/local package resolution works in offline mode once seeded.
- plugin approval is hash-based and rejects changed plugin bytes.

Initial core types for this contract live in `crates/flynt-core/src/formal_document.rs`. The current `StubTypstEngine` is intentionally only a deterministic bundle-contract test engine. `BundledCliTypstEngine` has preliminary CLI argument construction and invocation scaffolding behind the same `TypstEngine` trait; the implementation now includes Typst locator/version probing, short diagnostic parsing, permissive deps parsing, and a gated real-Typst minimal fixture. The next implementation slice should expand the fixture suite and wire build/preflight commands to the agent and GUI surfaces.

See also `docs/formal-document-typst-engine.md` for the focused engine design.


## Package and plugin policy

Package/plugin trust, lockfile, approval, fixture, and second-order-effect design is defined in `docs/formal-document-typst-policy.md`.

## References

- `docs/formal-document-typst-settings.md`

## Hardening plan

Trust-boundary findings and the implementation sequence are tracked in `docs/formal-document-hardening-plan.md`.
