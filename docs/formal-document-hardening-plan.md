# Formal Document Trust-Boundary Hardening Plan

## Purpose

This plan captures the required design affordances and implementation sequence before the Formal Document `.typ` renderer becomes a broadly operator-facing GUI feature.

The current architecture is directionally correct: `.typ` is canonical, `FormalDocumentBuildService` owns Flynt invariants, and Typst compilation is behind `TypstEngine`. The remaining risk is not rendering quality; it is **operator trust**. Flynt must not imply that a document is reproducible, safe, or policy-compliant until package, plugin, font, network, output-promotion, and build-state boundaries are hardened.

## Design affordances required before GUI exposure

### 1. Trust status must be visible and precise

The GUI must not show a generic green "safe" state after static preflight. Static preflight is incomplete by design.

Required labels:

| State | Meaning |
|---|---|
| `Not built` | No manifest/preview exists. |
| `Stale preview` | Source hash differs from last successful manifest. |
| `Static preflight clear` | Flynt found no blockers without executing Typst; this is not a complete audit. |
| `Review required` | Package/plugin/network/font policy requires operator action. |
| `Building` | Typst build in progress; last-good preview remains visible. |
| `Build failed` | Build failed; last-good preview remains visible. |
| `Built` | Current source hash matches manifest and build succeeded. |
| `Policy verified` | Build-time/post-build policy checks passed for the artifacts in the manifest. |

Do not use "safe", "trusted", or "reproducible" as badges until post-build checks verify all required hashes and policy fields.

### 2. Preview must distinguish source freshness from artifact freshness

The preview pane must show:

- source path;
- last built timestamp;
- source hash match/mismatch;
- engine kind/version;
- whether preview is last-good or current;
- whether PDF was built;
- whether build used system fonts or bundled/project-only fonts.

After a failed build, the right pane must explicitly say:

```text
Showing last successful preview. Current source failed to build.
```

### 3. Policy review must be actionable

`review_required` must give the operator concrete actions, not just diagnostics.

Required action categories:

- approve plugin hash for this project;
- revoke plugin approval;
- inspect package lock entry;
- materialize/download package after explicit approval;
- switch to offline-only mode;
- switch system fonts on/off;
- open manifest;
- open package/plugin policy files.

A diagnostic must include enough context for the action:

```json
{
  "code": "typst_plugin_unapproved",
  "path": ".flynt/typst/packages/.../plugin.wasm",
  "sha256": "...",
  "package": "@preview/example:1.0.0",
  "action": "approve_plugin_hash"
}
```

### 4. Network behavior must be explicit

Manual Recompile must not silently fetch packages under default settings.

The GUI must expose the current network/package policy:

```text
Packages: Ask before download
Network during compile: Disabled unless explicitly approved
```

If CLI Typst cannot be forced offline, Flynt must prevent compile when unresolved packages exist under default/offline modes. The operator must approve package materialization before compile.

### 5. Plugin approval must be hash-based and revocable

Plugin approval UI must show:

- SHA-256;
- path;
- package/source if known;
- first seen timestamp;
- approving operator/source;
- documents that most recently used it, if known;
- revoke button.

Approval is project-scoped. Never global by default.

### 6. Package lock must be integrity-enforcing

A package lock entry without hash enforcement is misleading. Builds must fail/review-block when a locked package's directory hash differs.

The lock must distinguish:

- project-local package;
- cache materialized package;
- Typst Universe package;
- unknown/manual source.

### 7. Build manifest must avoid overclaiming

Manifest is an audit artifact, not proof of reproducibility unless all fields are populated and verified.

Minimum manifest fields before production GUI exposure:

- source hash;
- engine kind/version/path or bundled id;
- output preview/PDF paths;
- world policy;
- package list and hashes where available;
- plugin list and hashes;
- font list and hashes where available;
- dependency/assets list;
- diagnostics;
- build duration;
- build id;
- whether post-build policy verification passed.

### 8. Output promotion must be atomic

Builds must write to a staging location and promote only on success:

```text
reports/<slug>/
  manifest.json
  preview/
  document.pdf
  .staging/<build-id>/
```

A failed build must never delete or overwrite the last successful preview.

### 9. Concurrent builds must be serialized or rejected

Per document/output directory, only one build may promote artifacts at a time.

If another build is active:

- queue it; or
- reject with `already_building`.

Do not let slower build A overwrite newer build B.

### 10. SVG preview must be isolated

Typst-generated SVG preview must render in a constrained context:

- no script execution;
- no external resource loading;
- project-local resources only;
- prefer iframe/webview isolation or sanitizer boundary.

## Implementation plan

### Phase 1 — Policy correctness in core

1. Enforce package lock hashes.
   - If package exists and lock hash differs, emit `typst_package_hash_mismatch` error.
   - If imported package missing and no lock/materialized package exists, emit `typst_package_missing`.
   - Add tests for matching hash, mismatch, and missing lock entry.

2. Refine plugin scan severity.
   - Unapproved plugin in imported package: error.
   - Unapproved plugin elsewhere in package path: warning or informational until build-time evidence exists.
   - Add tests distinguishing imported package plugin from unrelated package plugin.

3. Add post-build policy validation.
   - Parse deps/assets after compile.
   - Compare discovered deps/assets/packages/plugins against policy and lock.
   - If violation appears after compile, mark build failed/review-required and do not promote outputs.

4. Add package/plugin policy result types.
   - Include action hints for GUI.
   - Avoid string-only diagnostics for approval workflows.

### Phase 2 — Build artifact safety

1. Stage outputs under `.staging/<build-id>`.
2. Promote manifest/preview/PDF atomically after successful compile and post-build checks.
3. Preserve previous successful manifest/preview on failure.
4. Add build id to manifest.
5. Add per-output-dir build lock or rejection mechanism.

### Phase 3 — Durable approvals and locks

1. Add read/write helpers for:

```text
.flynt/typst/package-lock.json
.flynt/typst/plugin-approvals.json
```

2. Add agent tools:

```text
formal_document_approve_plugin
formal_document_revoke_plugin
formal_document_package_lock_status
formal_document_refresh_package_lock
```

3. Add tests:
   - approval unblocks build;
   - changed plugin bytes re-block;
   - malformed JSON produces clear error;
   - revoked plugin blocks again;
   - package hash mismatch blocks.

### Phase 4 — Release fixture gate

Add:

```text
scripts/check-formal-document-fixtures.sh
```

Requirements:

- fail if Typst binary is unavailable;
- print Typst version;
- run real fixture suite;
- assert SVG preview outputs;
- assert PDF output for at least one fixture;
- assert compile-error fixture preserves last-good preview;
- assert deps parsing captures assets;
- assert system fonts remain disabled by default.

Normal unit tests may skip real Typst. Release fixture gate must not skip.

### Phase 5 — Settings and GUI surfacing

1. Add settings projection backed by `formal_document_doctor`.
2. Add `.typ` Formal Document tab:
   - source editor;
   - Recompile button;
   - right-pane last-good SVG preview;
   - stale/current/failure badges;
   - diagnostics/policy panel;
   - manifest inspector.
3. Add trust/cache settings:
   - package cache status;
   - plugin approval list;
   - revoke approval;
   - clear cache;
   - engine version/status.

## First implementation targets

Start with these in order:

1. Package lock hash enforcement in `preflight_typst_policy`.
2. Tests for matching/mismatched package lock hash.
3. Plugin approval file tests in `flynt-agent`.
4. Output staging in `FormalDocumentBuildService`.
5. Release fixture script.

## Non-goals for the hardening phase

- No live Typst preview.
- No embedded Rust Typst engine yet.
- No automatic package downloads.
- No global plugin approvals.
- No broad GUI polish before trust-boundary behavior is correct.
