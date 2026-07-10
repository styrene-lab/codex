+++
title = "Report Markdown Surface"
tags = ["design","reports","typst","markdown"]
+++

# Report Markdown Surface

## Overview

Flynt now distinguishes plain Markdown notes from **Report Markdown**: Markdown-authored documents intended for formal presentation, distribution, and Typst-backed PDF rendering.

This is not "Typst Markdown" as a canonical language name. Typst is not Markdown. Report Markdown is a Flynt document surface that keeps `.md` as the authoring substrate while opting into compiled-document semantics and a Typst presentation backend.

## Document classes

| Class | Source | Purpose | Semantics |
|---|---|---|---|
| Plain Markdown | `.md` without report frontmatter | notes, knowledge capture, lightweight docs | conservative Markdown; `$...$` remains literal for report compilation |
| Report Markdown | `.md` with `kind = "report"` or `[report]` | professional reports, briefs, formal publications | Typst-aware report semantics, preflight diagnostics, generated Typst/PDF |
| Native Typst | `.typ` | maximum layout/typesetting control | Typst owns the source; Flynt indexes/previews/builds but does not pretend it is Markdown |

## Frontmatter contract

Minimal Report Markdown:

```toml
+++
title = "Architecture Brief"
kind = "report"

[report]
backend = "typst"
profile = "technical-brief"
math = "literal"
unicode = "diagnose"
raw_typst = "deny"
+++
```

Supported v0 fields:

```toml
[report]
backend = "typst"              # currently the only backend
profile = "technical-brief"    # presentation profile/template selector
math = "literal"               # off | literal | typst | latex_warn
unicode = "diagnose"           # literal | diagnose | strict
raw_typst = "deny"             # deny | allow | trusted_only
toc = true
number_headings = true
```

## Semantics

Plain Markdown can still be exported through the report compiler, but it remains conservative:

- dollar signs are literal;
- raw Typst fences render as code unless explicitly allowed by a Report Markdown profile;
- Typst math is not inferred;
- diagnostics tell the operator when a plain note contains constructs that may deserve Report Markdown conversion.

Report Markdown opts into presentation semantics:

- `typst-math` fences become Typst display math;
- `typst` fences are raw Typst only when `[report].raw_typst = "allow"`;
- `report` fences carry report directives such as `page-break`;
- non-ASCII characters can emit diagnostics for font/glyph preview;
- future extensions can add figures, citations, cross-references, and appendices as first-class Report IR nodes.

## Supported v0 fenced blocks

### Typst math

````markdown
```typst-math
sum_(i=1)^n i = n(n+1)/2
```
````

### Raw Typst

Raw Typst is denied by default and rendered as code. A report must explicitly allow it:

```toml
[report]
raw_typst = "allow"
```

````markdown
```typst
#pagebreak()
```
````

### Report directive

````markdown
```report
page-break
```
````

## Compilation pipeline

```text
Plain Markdown / Report Markdown / Native Typst
  → Flynt source classification
  → Report config + diagnostics
  → Flynt Report IR
  → generated report.typ
  → optional typst compile → report.pdf
  → manifest.json
```

The Report IR is authoritative for Flynt's semantic interpretation. The generated Typst source is the audit boundary. The final PDF is authoritative for presentation.

## Design decisions

- Report Markdown is a first-class surface, not a global Markdown renderer preference.
- Plain notes are never silently upgraded into Typst/math semantics.
- Typst-specific behavior is explicit in frontmatter and diagnostics.
- Native `.typ` remains an escape hatch for documents that need full Typst control.
- Flynt owns project/report semantics; Typst owns final typesetting and export.

## Initial implementation

The initial core model lives in `crates/flynt-core/src/report.rs` and includes:

- `ReportConfig` for source mode, backend, profile, math mode, Unicode mode, raw Typst policy, table-of-contents, and heading numbering;
- `ReportSourceMode` to distinguish plain Markdown, Report Markdown, and native Typst;
- `ReportDiagnostic` and `SourceSpan` for preflight and future live-render affordances;
- Report IR block variants for headings, paragraphs, code, Typst math, raw Typst, report directives, and thematic breaks;
- Typst bundle rendering plus optional `typst compile` execution;
- manifest output with source mode, profile, and diagnostics.
