# ADR 0001: Formal Document Typst Engine Strategy

## Status

Accepted

## Context

Flynt Formal Documents now use `.typ` as the canonical formal-document source format. Markdown notes can be formalized into `.typ`, but the formal document renderer should not treat Markdown as the publication source.

Typst is implemented in Rust and publishes crates (`typst`, `typst-cli`, `typst-svg`, `typst-pdf`, `typst-kit`, etc.), so an embedded Rust engine is technically feasible. However, using Typst as a library means Flynt must own or adapt Typst's world integration: source resolution, root containment, package resolution, package cache behavior, font discovery, plugin policy, bibliography/image loading, diagnostics, dependency tracking, and export.

The Typst CLI already exposes the controls Flynt needs for the first real engine:

- `--root`
- `--font-path`
- `--ignore-system-fonts`
- `--package-path`
- `--package-cache-path`
- `--input`
- `--creation-timestamp`
- `--deps` / `--deps-format json`
- SVG/PDF output via `typst compile`

Real Typst 0.15.0 testing showed that `typst compile` does **not** support `--diagnostic-format json`; accepted values are `human` and `short`. Flynt therefore uses `--diagnostic-format short` for the bundled CLI engine and keeps structured source-span diagnostics as a future embedded-engine motivation.

## Decision

Flynt will use the `TypstEngine` trait as the stable internal boundary and ship the first real implementation as a bundled Typst CLI engine.

Implementation ladder:

```text
TypstEngine
├── StubTypstEngine          deterministic contract tests only
├── BundledCliTypstEngine    first real production path
├── SystemTypstEngine        advanced/dev fallback
└── EmbeddedTypstEngine      later direct Rust integration
```

The long-term preferred shape remains an embedded Rust engine, but only after the GUI, manifest, artifact, policy, and fixture contracts are stable enough to compare embedded output against the CLI backend.

## Rationale

### Why bundled CLI first

- It matches Typst's supported product boundary and common user workflow.
- It provides full ecosystem behavior immediately: packages, cache paths, fonts, inputs, deps, SVG, and PDF.
- It isolates compiler failures from the GUI process.
- It gives straightforward fixture-based upgrade testing across pinned Typst versions.
- It avoids prematurely reimplementing Typst's world integration inside Flynt.

### Why keep the embedded route open

An embedded engine may later provide:

- richer diagnostics and source spans;
- better incremental rebuilds;
- persistent compiler/cache state;
- direct SVG/PDF bytes without process staging;
- deeper Flynt project-world integration.

Those advantages are real, but they depend on correctly implementing the Typst world boundary. That is a later implementation, not a prerequisite for the first production renderer.

## Consequences

- Flynt must bundle or otherwise resolve a pinned Typst CLI binary per release.
- Typst version updates are compatibility-sensitive and require the Formal Document fixture suite to pass before adoption.
- The GUI and agent tools must call `FormalDocumentBuildService`; they must not shell out to Typst directly.
- The build manifest must record engine kind/version/path, world policy, source hash, outputs, diagnostics, assets, package/font/plugin metadata where known, and duration.
- Diagnostic span quality is limited while using the CLI because Typst 0.15.0 compile only exposes `human`/`short` diagnostics.
- The `TypstEngine` trait keeps the door open for `EmbeddedTypstEngine` without changing GUI or agent contracts.

## Required validation for Typst updates

A Typst bump is accepted only when the Formal Document fixture suite validates Flynt's contracts, not merely when Typst compiles one file.

Minimum fixture set:

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
- Source hash appears in the manifest.
- Engine version appears in the manifest.
- Package/font/plugin policy appears in the manifest.
- Dependency/assets list is recorded.
- Failed builds preserve the last successful preview.
- System fonts remain disabled unless explicitly enabled.
- Package cache/local package resolution works in offline mode once seeded.
- Plugin approval is hash-based and rejects changed plugin bytes.

## Current implementation evidence

As of this ADR:

- `crates/flynt-core/src/formal_document.rs` contains `FormalDocumentBuildService`, `TypstEngine`, `StubTypstEngine`, and preliminary `BundledCliTypstEngine`.
- Real Typst CLI 0.15.0 was installed locally under `.flynt/typst-toolchain/bin/typst` for validation.
- The real minimal compile fixture passes when that binary is on `PATH`.
- `cargo test -p flynt-core formal_document::tests -- --nocapture` passes with the local Typst binary on `PATH`.
- `cargo check -p flynt-core` passes.

## References

- `docs/formal-document-typst-settings.md`

- `docs/formal-document-rendering.md`
- `docs/formal-document-typst-engine.md`
- `crates/flynt-core/src/formal_document.rs`

## Package and plugin policy

Package/plugin trust, lockfile, approval, fixture, and second-order-effect design is defined in `docs/formal-document-typst-policy.md`.

## Hardening plan

Trust-boundary findings and the implementation sequence are tracked in `docs/formal-document-hardening-plan.md`.
