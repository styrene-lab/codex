+++
title = "Formal Document Typst Settings"
tags = ["design","typst","formal-documents","settings"]
+++

# Formal Document Typst Settings

## Decision

Flynt Settings for the Typst internal engine expose **policy and observability**, not arbitrary Typst compiler knobs. `.typ` files own document content and layout. The Formal Document build service owns reproducibility, trust, storage, and fallback behavior.

Product rule:

> Settings govern the Typst world; `.typ` files govern the document.

## Required settings surface

### Engine

Expose active engine and availability:

```text
Engine:
  ● Bundled Typst
  ○ System Typst              advanced/dev fallback
  ○ Disabled
```

Required status:

- active engine kind;
- bundled Typst version;
- engine path when CLI-backed;
- system Typst probe status;
- last probe error;
- `Probe Engine` action.

Default: `Bundled Typst`.

System Typst fallback must be explicit. It must never silently replace the bundled engine because that weakens reproducibility.

### Build behavior

Expose the formal-document compile posture:

```text
Recompile: Manual only
Build PDF by default: off
Output folder: reports/
Keep last successful preview after failure: on
```

`Keep last successful preview after failure` is an invariant more than a preference; show it for operator understanding, but do not make it easy to disable.

Auto-recompile-on-save is future/advanced only. Manual recompile remains the default because builds may involve package/plugin/network policy.

### Packages

Expose package resolution policy:

```text
Package resolution:
  ○ Offline only
  ● Ask before download
  ○ Auto-download packages
```

Required status/actions:

- package path: `.flynt/typst/packages`;
- package cache: `.flynt/cache/typst/packages`;
- `View Package Lock`;
- `Open Package Cache`;
- `Clear Package Cache`.

Default: `Ask before download`.

Package downloads affect reproducibility and must be reflected in `.flynt/typst/package-lock.json` and build manifests.

### Plugins

Expose WASM plugin execution policy:

```text
Plugin execution:
  ○ Deny all plugins
  ● Ask before first plugin hash
  ○ Allow approved plugins
```

Required status/actions:

- approved plugin count;
- `Review Plugin Approvals`;
- `Revoke All Plugin Approvals`.

Plugin approval identity is content hash first:

```text
sha256:<hash>
source package/path
approved_at
reason
```

Default: `Ask before first plugin hash`.

If plugin bytes change, approval is invalidated even if the package name/version/path are unchanged.

### Fonts

Expose font reproducibility policy:

```text
Fonts:
  ● Bundled + project fonts only
  ○ Bundled + project + system fonts
```

Required status/actions:

- project font paths:
  - `fonts/`
  - `typst/fonts/`
  - `.flynt/fonts/`
- `Open Project Fonts Folder`;
- `Scan Fonts`.

Default: `Bundled + project fonts only`.

System fonts are opt-in because they can make layout and glyph availability machine-dependent.

### Network

Expose the build network posture separately from package policy:

```text
Network during Formal Document builds:
  ● Never without approval
  ○ Allow package downloads according to package policy
  ○ Offline only
```

No build should silently network-fetch packages or dependencies. A build may return `review_required` when package policy needs operator action.

### Reproducibility

Expose reproducible build controls in advanced settings:

```text
Creation timestamp: unset | <unix epoch>
Use fixed creation timestamp: off by default
```

Build manifests still record actual `built_at`. Fixed creation timestamp is for deterministic output artifacts, not for audit timestamps.

### Diagnostics

Expose operational tools:

```text
[Run Typst Doctor]
[Run Fixture Smoke Test]
[Open Last Build Manifest]
[Copy Engine Diagnostics]
```

Doctor should report:

- active engine;
- engine version;
- engine path;
- package path/cache existence;
- font paths;
- plugin approval count;
- minimal compile capability;
- SVG output capability;
- PDF output capability.

## Recommended Settings layout

```text
Settings
└── Formal Documents
    ├── Engine
    │   ├── Active engine: Bundled Typst
    │   ├── Bundled version: 0.15.0
    │   ├── System Typst: not found / found
    │   └── [Probe Engine]
    │
    ├── Build Behavior
    │   ├── Recompile: Manual only
    │   ├── Build PDF by default: off
    │   ├── Output folder: reports/
    │   └── [Clear Generated Previews]
    │
    ├── Packages
    │   ├── Mode: Ask before download
    │   ├── Package path: .flynt/typst/packages
    │   ├── Cache path: .flynt/cache/typst/packages
    │   ├── [View Package Lock]
    │   └── [Clear Package Cache]
    │
    ├── Plugins
    │   ├── Mode: Ask before first plugin hash
    │   ├── Approved plugins: N
    │   ├── [Review Approvals]
    │   └── [Revoke All]
    │
    ├── Fonts
    │   ├── Mode: Bundled + project fonts only
    │   ├── Project font paths: fonts/, typst/fonts/, .flynt/fonts/
    │   └── [Scan Fonts]
    │
    └── Diagnostics
        ├── [Run Typst Doctor]
        ├── [Run Fixture Smoke Test]
        ├── [Open Last Build Manifest]
        └── [Copy Diagnostics]
```

## Do not expose initially

Avoid generic escape hatches that bypass Flynt policy:

- raw extra Typst CLI args;
- arbitrary global `--input` key/value injection;
- arbitrary package cache paths outside the project;
- arbitrary font paths outside the project;
- diagnostic format selection;
- page/layout settings that belong in `.typ`;
- PDF/SVG internals that are not product-level choices.

If advanced users need custom Typst behavior, they should express it in `.typ` source or project-local config that Flynt can inspect and record.

## Mandatory first public exposure checklist

Before exposing Formal Documents as a visible feature, Settings must show at least:

1. active engine and version;
2. package mode;
3. plugin mode and approval review;
4. font mode;
5. network/download policy;
6. output/cache path visibility;
7. Typst Doctor / smoke test.

## Implementation implication

The core settings model should serialize to a project-visible policy object that can populate `TypstWorldPolicy` and the package/plugin preflight inputs. Document-specific details stay in the `.typ` source and build manifest.
