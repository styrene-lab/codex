+++
title = "Formal Document Typst Package and Plugin Policy"
tags = ["design","typst","formal-documents","security"]
+++

# Formal Document Typst Package and Plugin Policy

## Purpose

Flynt Formal Documents use `.typ` as canonical source and compile through `FormalDocumentBuildService`. Typst packages and WASM plugins are part of the normal Typst ecosystem, but they introduce second-order effects that are different from ordinary project-local files:

- packages may be fetched from the network;
- package updates can change rendering without source edits;
- packages can include or transitively reference plugins;
- plugins are executable WASM code;
- fonts/assets/packages/plugins affect reproducibility;
- cached ecosystem state can leak across projects if not scoped;
- CI/offline builds need deterministic behavior.

This policy defines the package/plugin trust and manifest model before adding package/plugin fixtures.


## Assessment findings addressed

This design is ready for a first implementation slice with three constraints:

1. **Preflight is intentionally conservative.** It scans source text, reachable local imports, package directories, and `.wasm` files; it does not claim to be a full Typst evaluator. Dynamic package/plugin discovery remains a build-time guard.
2. **Review-required is a first-class result.** Missing packages and unapproved plugins are not generic compile failures; they are policy decisions surfaced before compile.
3. **Hashing is the stable boundary.** Package directories and plugin files are hashed immediately before build/preflight. Cache path identity is advisory only.

The initial code slice should therefore implement pure policy primitives first: lock/approval structs, deterministic hashing, package import scanning, plugin scan, and policy preflight diagnostics. Package downloading/materialization and real plugin execution are later slices.

## Design principles

1. **No silent network** — manual Recompile must not fetch from the network unless policy explicitly allows it.
2. **Project-scoped trust** — approvals are scoped to the Flynt project, not global user state.
3. **Hash-based executable approval** — plugin approval is by content hash, not just package name/version.
4. **Reproducibility before convenience** — default builds prefer project/bundled state over system/global state.
5. **Manifest everything** — build output records the package/plugin/font/asset facts that shaped it.
6. **Fail closed for executable changes** — changed plugin bytes invalidate previous approval.
7. **Keep Typst semantics intact** — Flynt should not fork Typst package syntax or package resolution semantics unless needed for containment/policy.

## Policy modes

Existing core enums:

```rust
enum TypstPackageMode {
    OfflineOnly,
    AskBeforeDownload,
    AutoDownload,
}

enum TypstPluginMode {
    DenyAll,
    AskBeforeFirstHash,
    AllowApproved,
}
```

Recommended product defaults:

```text
package_mode = AskBeforeDownload
plugin_mode = AskBeforeFirstHash
font_mode = BundledAndProject
network = no silent network
```

### Package modes

| Mode | Behavior | Product use |
|---|---|---|
| `OfflineOnly` | Resolve only project-local/package-cache packages. Missing package is a diagnostic error. | CI, reproducible review, hardened mode |
| `AskBeforeDownload` | Missing external package produces a review-required result. Operator can approve download/materialization. | default GUI mode |
| `AutoDownload` | Missing external package may be downloaded automatically, but all fetched packages are recorded. | convenience/dev mode only |

### Plugin modes

| Mode | Behavior | Product use |
|---|---|---|
| `DenyAll` | Any plugin reference fails preflight/build. | hardened mode, untrusted projects |
| `AskBeforeFirstHash` | First observed plugin content hash requires project approval. Changed bytes require reapproval. | default GUI mode |
| `AllowApproved` | Approved hashes run; unapproved hashes fail. No interactive prompt. | CI/repro mode after approvals are committed or provisioned |

`AllowApproved` must not mean “allow any plugin from an approved package name.” Package identity is not sufficient for executable trust.

## Storage layout

Project-local policy and cache paths:

```text
.flynt/
  typst/
    packages/                 # project-local package path, reproducible inputs
    plugin-approvals.json     # project-scoped plugin approvals
    package-lock.json         # observed/frozen package identities and hashes
  cache/
    typst/
      packages/               # fetched package cache, safe to regenerate
```

The cache is an optimization. The lock/approvals are project policy state.

## Package lock

`package-lock.json` records the package set observed or approved for Formal Document builds.

Draft schema:

```json
{
  "version": 1,
  "packages": [
    {
      "namespace": "preview",
      "name": "codly",
      "version": "1.3.0",
      "source": "typst-universe",
      "path": ".flynt/cache/typst/packages/preview/codly/1.3.0",
      "sha256": "...",
      "approved_at": "2026-06-29T00:00:00Z",
      "approved_by": "operator",
      "license": null
    }
  ]
}
```

Hashing rule:

- For a package directory, compute a deterministic directory hash over normalized relative paths and file bytes.
- Ignore OS metadata, mtimes, and cache bookkeeping.
- Include plugin `.wasm` bytes in the package hash, but do not rely on the package hash as plugin execution approval.

Second-order effect: directory hashing makes lockfile churn possible if package caches include transient files. The package materializer must store only package contents under hashed paths or filter known transient metadata.

## Plugin approvals

`plugin-approvals.json` records executable WASM approvals.

Draft schema:

```json
{
  "version": 1,
  "approvals": [
    {
      "sha256": "...",
      "source": "typst-package",
      "package": {
        "namespace": "preview",
        "name": "example-plugin-package",
        "version": "0.2.0"
      },
      "path": ".flynt/cache/typst/packages/preview/example-plugin-package/0.2.0/plugin.wasm",
      "approved_at": "2026-06-29T00:00:00Z",
      "approved_by": "operator",
      "scope": "project",
      "reason": "Needed by formal document fixture/plugin-approved.typ"
    }
  ]
}
```

Approval key:

```text
plugin_sha256
```

Optional display metadata:

```text
package namespace/name/version
path
source
reason
```

Trust decision:

```text
if plugin_mode == DenyAll:
  reject any plugin
elif plugin hash is approved:
  allow
elif plugin_mode == AskBeforeFirstHash:
  return ReviewRequired diagnostic/action
else:
  reject unapproved plugin
```

Second-order effect: approving a plugin hash from one package allows the same bytes from another package inside the same project. That is acceptable because the executable content is identical, but the manifest must still record the actual package/path source used in the build.

## Build manifest additions

`FormalDocumentBuildManifest` already contains:

```rust
packages: Vec<TypstPackageUse>,
fonts: Vec<TypstFontUse>,
plugins: Vec<TypstPluginUse>,
assets: Vec<PathBuf>,
```

Package entries should include:

```json
{
  "namespace": "preview",
  "name": "codly",
  "version": "1.3.0",
  "source": "typst-universe|project-local|cache",
  "sha256": "..."
}
```

Plugin entries should include:

```json
{
  "path": ".flynt/cache/typst/packages/.../plugin.wasm",
  "sha256": "...",
  "approved": true,
  "source": "typst-universe|project-local|direct-project-file"
}
```

Manifest invariant:

- If a build executed a plugin, a `plugins[]` entry must exist.
- If `plugin_mode = DenyAll`, `plugins[]` must be empty on success.
- If `package_mode = OfflineOnly`, no build step may network-fetch packages.
- If a package is downloaded/materialized, the manifest and package lock must make the dependency visible.

## Build/preflight flow

### Preflight

Before invoking Typst for a package/plugin-heavy build, Flynt should run a preflight phase:

1. Parse or scan the `.typ` source and reachable project-local imports for `@namespace/name:version` package imports.
2. Check package lock/cache/project package path.
3. Decide whether missing packages require review, download, or failure.
4. Materialize approved packages into Flynt's package path/cache.
5. Scan materialized package contents for `.wasm` files.
6. Compare plugin hashes against `plugin-approvals.json`.
7. Return diagnostics/actions if approval or package download is required.

Preflight should not try to fully evaluate Typst. It is a policy gate, not a compiler replacement.

### Build

Build proceeds only when policy gates are satisfied:

1. Construct `FormalDocumentBuildRequest` with project-rooted package/cache/font paths.
2. Invoke `TypstEngine`.
3. Parse deps output.
4. Derive used package/font/plugin records from deps + package/cache scans.
5. Write manifest.
6. Preserve last successful preview on failure.

Second-order effect: Typst may discover packages/plugins during evaluation that preflight did not statically see. The engine must treat post-build deps/package/plugin discovery as authoritative and fail the build if it detects an unapproved plugin was used. With the CLI, this may require pre-scanning all materialized package directories before build and disabling network fetch in modes where Flynt cannot intercept new packages.

## Network policy

Default GUI behavior:

- Missing package in `AskBeforeDownload` returns a review-required diagnostic/action.
- Operator approves package download explicitly.
- Downloaded package is materialized into the Flynt cache/package path.
- Package identity/hash is recorded in `package-lock.json`.
- Build is retried.

CI/offline behavior:

- `OfflineOnly` fails if package is missing.
- No network access should be attempted.
- CI can seed `.flynt/typst/packages` or `.flynt/cache/typst/packages` before build.

Second-order effect: allowing Typst CLI to auto-download packages may bypass Flynt's prompt model. The CLI engine must either:

- run with cache/package paths arranged so all packages are already present before compile, or
- use `AutoDownload` only when the product explicitly accepts network fetch during build and records what appeared afterward.

## Package fixture plan

### `local-package.typ`

Goal: prove project-local packages work in `OfflineOnly` without network.

Fixture layout:

```text
fixtures/formal-documents/
  local-package.typ
  .flynt/typst/packages/local/flynt-fixture/0.1.0/package.typ
```

Typst source:

```typst
#import "@local/flynt-fixture:0.1.0": fixture-box

= Local Package Fixture

#fixture-box[Project-local package content]
```

Assertions:

- Build succeeds in offline mode.
- Manifest records package namespace/name/version/source/hash.
- Preview SVG exists.
- No network is attempted.

### `plugin-denied.typ`

Goal: prove plugin policy fails closed.

Options:

1. Use a minimal WASM plugin fixture if Typst supports direct fixture syntax reliably.
2. Use a local package containing `.wasm` and a source file that imports/uses it.
3. If producing a valid Typst WASM plugin is too heavy for first pass, create a policy preflight unit test that scans a materialized package containing `.wasm` and asserts `DenyAll` rejects it before Typst compilation.

Preferred first pass: policy preflight unit test, not real plugin execution. It validates Flynt policy without blocking on WASM authoring details.

### `plugin-approved.typ`

Goal: prove approval by hash.

First pass:

- same package/plugin fixture as denied test;
- compute hash;
- add approval entry;
- assert preflight allows;
- mutate one byte;
- assert approval no longer matches.

Later pass:

- real Typst plugin execution fixture once the minimal plugin build path is defined.

## Security considerations

### Executable content

Typst plugins are WASM, which is safer than native execution but still executable code. Risks include CPU/memory denial of service, parser bugs, or malicious document/package behavior within the allowed WASM host functions.

Mitigations:

- approve by hash;
- record plugin usage in manifest;
- deny unknown hashes by default;
- consider compile timeouts for CLI process;
- consider resource limits for plugin-heavy builds later.

### Path containment

Packages, plugin files, bibliography files, images, and fonts must stay inside allowed roots:

- project root;
- Flynt-managed cache/package paths;
- Flynt-bundled resources.

System fonts are opt-in. Arbitrary absolute paths in documents should not be accepted as normal Formal Document dependencies.

### Cache poisoning

A cache entry may be replaced between approval and build.

Mitigations:

- hash package/plugin bytes immediately before build;
- compare against lock/approval;
- record actual hashes in manifest;
- fail if approved hash does not match current bytes.

### Review fatigue

Prompting for every package/plugin creates bad operator behavior.

Mitigations:

- packages: approve namespace/name/version once per project lock;
- plugins: approve hash once per project;
- display concise package/plugin provenance;
- group approvals for one build in a single review panel.

## Product/UI implications

Formal Document surface should expose a policy/status panel:

```text
Packages: 2 locked, 1 missing approval
Plugins: 1 unapproved hash
Fonts: bundled + project only
Network: blocked until approved
```

Review-required actions:

- Approve package download/materialization.
- Deny package and keep build blocked.
- Approve plugin hash for project.
- Deny plugin and keep build blocked.
- Switch to offline/hardened mode.

The GUI should not present package/plugin policy failures as generic Typst compile errors. They are Flynt policy decisions.

## Implementation order

1. Add package/plugin policy structs:
   - `TypstPackageLock`
   - `TypstPackageLockEntry`
   - `TypstPluginApprovals`
   - `TypstPluginApproval`
   - `TypstPolicyDecision`
2. Add deterministic directory hashing and file hashing helpers.
3. Add package import scanner for `@namespace/name:version` in `.typ` source and reachable local imports.
4. Add package/plugin preflight function:
   - missing package;
   - denied plugin;
   - unapproved plugin;
   - approved plugin;
   - mutated plugin bytes.
5. Add `local-package.typ` fixture and offline test.
6. Add policy-level `plugin-denied` and `plugin-approved` tests.
7. Extend `FormalDocumentBuildManifest` population from preflight/deps results.
8. Wire review-required diagnostics/actions to agent/GUI commands.

## Open questions

- Should package locks be committed by default or treated like generated local policy? Recommendation: project package locks should be commit-worthy for reproducibility; cache contents should not be committed.
- Should plugin approvals be committed? Recommendation: yes for team reproducibility only if the team accepts code-execution policy in repo; otherwise allow local approval overlays. This needs an explicit UI choice.
- Can Typst CLI be forced fully offline? If not, Flynt must ensure missing packages are resolved before compile and use environment/network policy to prevent surprise fetches.
- How much of Typst's package resolver should Flynt reimplement? Recommendation: only enough to identify imports, materialized packages, hashes, and approvals; do not fork package resolution semantics unless necessary.

## References

- `docs/formal-document-typst-settings.md`

## Hardening plan

Trust-boundary findings and the implementation sequence are tracked in `docs/formal-document-hardening-plan.md`.
