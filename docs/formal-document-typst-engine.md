# Formal Document Typst Engine

## Purpose

Flynt Formal Documents use `.typ` as the canonical source. The engine is responsible for compiling that source into GUI preview artifacts and optional PDFs under Flynt's project/world policy.

Typst owns typesetting. Flynt owns:

- source/root containment;
- build policy;
- package/font/plugin passthrough;
- artifact layout;
- diagnostics normalization;
- last-good-preview behavior;
- manifest/audit data;
- GUI and agent command consistency.

## Engine boundary

All callers use one build path:

```text
GUI Recompile
Agent formal_document_build
Future CLI command
Tests
  → FormalDocumentBuildService
  → TypstEngine implementation
```

No UI surface should shell out to Typst directly.

## Engine implementations

```text
TypstEngine
├── StubTypstEngine          contract tests only
├── BundledCliTypstEngine    first real implementation target
├── SystemTypstEngine        advanced/dev fallback
└── EmbeddedTypstEngine      later direct Rust integration
```

The first practical implementation is `BundledCliTypstEngine` because the Typst CLI already exposes Flynt's required integration controls:

- `--root`
- `--font-path`
- `--ignore-system-fonts`
- `--package-path`
- `--package-cache-path`
- `--input`
- `--creation-timestamp`
- `--diagnostic-format short`
- `--deps` / `--deps-format json`

## CLI invocation shape

Preview build:

```text
typst compile
  --root <project-root>
  --diagnostic-format short
  --package-path <project/.flynt/typst/packages>
  --package-cache-path <project/.flynt/cache/typst/packages>
  --font-path <bundled-fonts>
  --font-path <project-fonts>...
  --ignore-system-fonts
  --input flynt_document=<source>
  --creation-timestamp <epoch>
  <source.typ>
  <output>/preview/page-{0p}-of-{t}.svg
```

PDF build:

```text
typst compile
  <same world flags>
  <source.typ>
  <output>/document.pdf
```

Dependency scan:

```text
typst compile
  <same world flags>
  --deps <output>/deps.json
  --deps-format json
  <source.typ>
  <output>/.deps-probe.pdf
```

## Build service responsibilities

`TypstEngine` performs engine-specific compilation. `FormalDocumentBuildService` performs Flynt policy/orchestration:

1. Validate `.typ` source path.
2. Validate project-root containment.
3. Compute source hash.
4. Load previous successful manifest if present.
5. Create bundle directories.
6. Invoke engine for requested outputs.
7. Preserve last successful preview on failure.
8. Write manifest and diagnostics.
9. Return a `FormalDocumentBuildResult` for GUI/agent use.

## World policy

Default policy remains reproducibility-first:

```text
Engine: bundled
Packages: ask before download
Fonts: bundled + project only
Plugins: ask before first hash
System fonts: disabled
Network: no silent network
Root: project root
```

The CLI command builder must make this policy visible in arguments. For example, `BundledAndProject` font mode emits `--ignore-system-fonts`; `BundledProjectAndSystem` omits it and must be recorded in the manifest.

## First implementation slice

The first code slice intentionally does not require Typst to be installed in tests. It adds:

- `FormalDocumentBuildService` to centralize orchestration;
- `BundledCliTypstEngine` scaffolding;
- deterministic CLI argument construction tests;
- contract tests proving the service writes the bundle shape through `StubTypstEngine`.

The current implementation can run the CLI when a Typst binary is available, parse short diagnostics/deps, and execute a gated real-Typst minimal fixture. The next slice should expand the fixture suite and wire the build service to agent/GUI commands.

## ADR

The engine strategy decision is recorded in `docs/adr/0001-formal-document-typst-engine.md`. In short: Flynt ships the first real engine as a bundled Typst CLI behind the `TypstEngine` trait, while preserving `EmbeddedTypstEngine` as the long-term path once GUI/manifest/fixture contracts are stable.

## Package and plugin policy

Package/plugin trust, lockfile, approval, fixture, and second-order-effect design is defined in `docs/formal-document-typst-policy.md`.

## References

- `docs/formal-document-typst-settings.md`

## Hardening plan

Trust-boundary findings and the implementation sequence are tracked in `docs/formal-document-hardening-plan.md`.
