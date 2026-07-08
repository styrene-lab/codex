+++
name = "typst"
description = "Harness-agnostic operating guide for helping Typst-ignorant operators maintain, render, audit, and publish existing Typst document sets."
version = "0.1.0"
tags = ["typst", "documents", "publishing", "pdf", "typesetting"]
aliases = ["formal-documents", "publication", "typesetting"]
activation = "domain_detected"
profile = ["docs", "coding"]
triggers = [
  "typst",
  ".typ",
  "formal document",
  "typeset",
  "compile pdf",
  "publication",
  "bibliography",
  "citation",
]
+++

# Typst Skill

Use this skill when the operator needs to work with an existing Typst document set and may know little or nothing about Typst. The skill is intentionally harness-agnostic: it names capabilities and command shapes rather than assuming a specific agent, IDE, or product surface.

Typst is a markup-based typesetting system. Treat `.typ` files as the canonical publication source. Markdown, notes, and generated intermediates may feed a Typst workflow, but final layout and publication semantics belong to Typst.

---

## Operating stance

1. **Protect the source.** Read before editing. Keep edits small. Preserve local macros, templates, package imports, and house style unless the operator asks for a redesign.
2. **Compile early.** A syntactically plausible Typst edit is not complete until the document compiles or the remaining diagnostic is explicitly reported.
3. **Use the project's world.** A Typst document is shaped by source root, packages, fonts, bibliographies, images, inputs, and compiler version. Do not assume global machine state is acceptable.
4. **Keep artifacts separate.** Source lives in `.typ` and adjacent assets. Outputs (`.pdf`, `.svg`, `.png`, dependency reports, manifests) should be generated into a build/output directory unless the project already has a convention.
5. **Do not silently fetch or execute.** Package downloads and WASM plugins can change trust and reproducibility. Surface those decisions before proceeding in hardened or shared projects.
6. **Explain Typst to the operator at the point of need.** Avoid a tutorial dump. When a diagnostic mentions `#let`, `#show`, `#import`, `where`, math mode, or bibliography syntax, translate only the concept needed to unblock the task.

---

## First response workflow

When asked to work on a Typst document set:

1. **Inventory the project.** Locate `.typ` entrypoints, shared templates, fonts, bibliography files, images, package config/locks, scripts, and CI jobs.
2. **Identify the build contract.** Prefer existing `just`, `make`, `npm`, `cargo`, shell scripts, CI commands, or harness tools over ad-hoc `typst compile`.
3. **Probe tool availability.** Check versions of required binaries (`typst`, formatter, LSP/preview tools if relevant) before promising a render.
4. **Compile a minimal target.** Choose the primary entrypoint or the file the operator named. Build PDF first unless the task is specifically preview/SVG/PNG.
5. **Read diagnostics as work items.** Convert compiler output into concrete file/line fixes. Do not ask the operator to interpret Typst errors.
6. **After edits, recompile.** If formatting or lint tooling exists, run it before final compile.
7. **Report artifacts.** Tell the operator which source changed, which command ran, and where the output was written.

---

## Document lifecycle

### 1. Discover

Find and classify:

- **Entrypoints:** standalone `.typ` files that produce PDFs, often under `docs/`, `reports/`, `papers/`, `src/`, or project root.
- **Templates/macros:** files imported by entrypoints, often named `template.typ`, `style.typ`, `lib.typ`, `defs.typ`, `preamble.typ`, or package-like folders.
- **Assets:** images (`.svg`, `.png`, `.jpg`, `.pdf`), data files (`.csv`, `.json`), diagrams, and generated figures.
- **Bibliography:** `.bib`, `.yml`, `.yaml`, `.json` files consumed by `bibliography(...)` or packages.
- **Fonts:** project font folders; common names include `fonts/`, `typst/fonts/`, `.fonts/`, `.flynt/fonts/`.
- **Generated outputs:** `build/`, `dist/`, `out/`, `reports/`, PDFs next to sources, SVG/PNG preview pages.
- **Automation:** `justfile`, `Makefile`, `package.json`, CI workflows, shell scripts, editor settings.

Useful inventory commands:

```sh
find . -name '*.typ' -o -name '*.bib' -o -name '*.yml' -o -name '*.yaml' -o -path './fonts/*'
find . -maxdepth 3 \( -name justfile -o -name Makefile -o -name package.json -o -name '*.sh' \)
rg '^(#import|#include|#bibliography|#show|#set|#let)|bibliography\(' -n --glob '*.typ'
```

### 2. Plan

Before changing content or layout, determine:

- Which `.typ` file is the entrypoint?
- What output is required: PDF, SVG pages, PNG pages, HTML preview, or all?
- Are system fonts allowed, or must fonts be project-pinned?
- Are remote packages allowed, already cached, or forbidden?
- Are WASM plugins used? If yes, do they require approval in this environment?
- Does the project require deterministic output metadata such as a fixed creation timestamp?
- Should bibliography/citation style be preserved exactly?

For non-trivial document work, make a task list by document section or artifact, not by tool invocation. Example:

```text
1. Compile baseline and capture current diagnostics.
2. Fix template import/path failures.
3. Update content in sections 2–4 without changing macro APIs.
4. Regenerate PDF and compare page count/known figures.
5. Record build command and output path.
```

### 3. Edit

Typst basics that matter during edits:

- Markup text is plain text. Headings use `=`, `==`, `===`.
- Code mode starts with `#`, e.g. `#let name = value`, `#set page(...)`, `#show heading: ...`.
- Math mode uses `$ ... $`; display math often uses a block form or separated math content.
- Imports are explicit: `#import "template.typ": name` or `#import "@preview/pkg:version": item`.
- Includes insert another file's content: `#include "chapter.typ"`.
- Labels use `<label>` and references often use `@label`.
- Comments use `// line` and `/* block */`.

Editing rules:

- Preserve macro names and call signatures unless you update every call site.
- Prefer adding small helper macros near existing helpers instead of duplicating layout code in chapters.
- Keep content edits separate from template/layout edits where possible.
- Do not reformat whole files unless a formatter is part of the project contract.
- When replacing images or data, verify relative paths from the Typst root/entrypoint.
- When touching citations, compile and inspect bibliography diagnostics; citation failures often render as missing references rather than hard failures.

### 4. Render

Use existing project commands first. If none exist, default command shapes are:

```sh
# PDF
mkdir -p build
typst compile --root . main.typ build/main.pdf

# SVG preview pages
typst compile --root . --format svg main.typ 'build/preview/page-{0p}-of-{t}.svg'

# PNG pages, useful for visual diff/review if supported by installed Typst
typst compile --root . --format png main.typ 'build/preview/page-{0p}.png'

# Watch during interactive editing
typst watch --root . main.typ build/main.pdf

# Dependency report, when supported by installed Typst
typst compile --root . --deps build/deps.json --deps-format json main.typ build/main.pdf
```

For reproducible or productized builds, add world flags intentionally:

```sh
typst compile \
  --root . \
  --package-path .typst/packages \
  --package-cache-path .cache/typst/packages \
  --font-path fonts \
  --ignore-system-fonts \
  --creation-timestamp 0 \
  --diagnostic-format short \
  main.typ build/main.pdf
```

Not every Typst version supports every flag. Probe with `typst compile --help` and degrade explicitly.

### 5. Review

After a successful compile, check:

- output file exists and was freshly written;
- page count did not change unexpectedly;
- warnings are understood, not ignored;
- missing references/citations are resolved;
- image and font substitutions did not silently alter layout;
- final artifacts are not committed unless project convention says outputs are versioned;
- build command is recorded in docs, CI, or the final report if the operator will repeat it.

### 6. Publish / handoff

For deliverables, produce:

- source patch summary;
- exact command(s) run;
- Typst/compiler version;
- output artifact paths;
- unresolved diagnostics or policy decisions;
- dependency/package/font assumptions.

If publishing externally, prefer a clean output folder and avoid embedding local absolute paths in manifests/logs.

---

## Toolchain and dependency map

### Required core binary

| Tool | Purpose | Typical commands | Notes |
|---|---|---|---|
| `typst` | Compile, watch, inspect fonts/help/version | `typst --version`, `typst compile`, `typst watch`, `typst fonts` | Required for local rendering. Pin version in CI/release workflows where output stability matters. |

### Strongly recommended tools

| Tool | Purpose | Typical commands | Notes |
|---|---|---|---|
| `tinymist` | Typst language server, editor integration, preview/export workflows | `tinymist --version`, `tinymist lsp`, `tinymist preview` | Active Typst language service with LSP, preview, and formatting-related capabilities. Useful in VS Code, Neovim, Zed, and agent harnesses that can speak LSP. |
| `typstyle` | Formatter for Typst source | `typstyle file.typ`, `typstyle -i file.typ` | Current formatter effort. Prefer over older `typstfmt` unless the project already standardized on `typstfmt`. |
| `hayagriva` | Bibliography processing library/CLI ecosystem around Typst citations | `hayagriva --help` | Typst itself handles bibliography rendering, but Hayagriva is useful for validating/converting bibliography data in some workflows. |

### Legacy / project-specific tools

| Tool | Purpose | Caution |
|---|---|---|
| `typstfmt` | Older Typst formatter | The upstream README states the last supported Typst version was 0.10 and points users toward `typstyle`; use only when a project pins it. |
| `typst-preview` / editor preview extensions | Live preview | Many workflows have moved toward Tinymist-backed preview. Respect existing editor/project setup. |
| `pandoc`, `quarto`, custom scripts | Markdown/LaTeX/data-to-Typst generation | Treat generated `.typ` as either canonical or disposable according to project docs. Do not edit generated files without finding the source. |
| `pdfinfo`, `mutool`, `qpdf`, `pdftotext` | Artifact inspection | Optional but useful for page count, metadata, text extraction, and CI checks. |
| `dvisvgm`, LaTeX tools | Legacy pipeline support | Not normally required for Typst; only use if the project scripts require them. |

### Package and asset dependencies

Typst documents may depend on:

- Typst Universe packages imported as `@preview/name:version` or other namespaces;
- local packages under a project package path;
- project images and generated figures;
- bibliography databases;
- custom fonts;
- data files read by Typst functions;
- WASM plugins provided by packages.

A robust harness should expose or emulate these operations:

```text
probe_tool(name) -> version/path/availability
run_command(args, cwd, timeout) -> stdout/stderr/status
read_file(path) / write_file(path) / exact_edit(path)
find_files(globs)
render_or_open_artifact(path)
hash_file(path) / hash_directory(path)
approve_or_reject_policy_action(action)
record_manifest(build facts)
```

Flynt-specific names such as `formal_document_doctor`, `formal_document_preflight`, `formal_document_build`, and `formal_document_state` are one product's implementation of these generic capabilities. A Copilot, CLI, IDE, or CI harness can apply the same lifecycle using shell commands and files.

---

## Policy and reproducibility

Use this stricter posture by default for shared, audited, or publication workflows:

```text
Engine: pinned Typst binary or declared system Typst version
Root: project root
Packages: no silent downloads
Package cache: project-scoped or CI-scoped
Plugins: deny or approve by content hash
Fonts: bundled/project fonts only unless system fonts are explicitly allowed
Network: off unless approving package materialization
Outputs: generated under build/dist/reports, not mixed with source
Diagnostics: captured as build artifacts
```

### Package handling

When a document imports a package:

```typst
#import "@preview/tablex:0.0.8": tablex
```

Do not assume the package is already available. Determine whether the project uses:

- Typst's default package cache;
- a checked-in/project-local package directory;
- a lockfile or manifest;
- CI cache restore;
- an approval flow for download/materialization.

If the operator is Typst-ignorant, explain package decisions as: "This document needs a third-party Typst package. We can use the already cached copy, fetch it, vendor it into the project, or remove the dependency. Fetching changes the build inputs."

### Plugin handling

WASM plugins are executable build inputs. If a package or source references a `.wasm` plugin:

- identify the file/package;
- hash the bytes;
- ask for approval in any harness that supports policy gates;
- record the hash and reason;
- reapproval is required if bytes change.

Do not describe a build as reproducible merely because it compiled. Reproducibility requires pinned compiler, package contents, plugin hashes, fonts, inputs, and creation timestamp policy.

### Fonts

Font availability changes layout. Prefer project fonts for deliverables.

Commands:

```sh
typst fonts
typst compile --font-path fonts --ignore-system-fonts --root . main.typ build/main.pdf
```

If missing glyphs or font substitutions appear, resolve by adding a project font, changing the template's font stack, or explicitly allowing system fonts.

---

## Diagnostics playbook

Common Typst failures and agent response:

| Symptom | Likely cause | Response |
|---|---|---|
| `file not found` | Wrong root or relative path | Re-run with correct `--root`; inspect `#import`, `#include`, image paths. |
| `unknown variable` | Missing import, renamed macro, scope issue | Search definitions; preserve API or update all call sites. |
| `unknown function` | Package/template not imported or version mismatch | Check `#import`; verify package version and template exports. |
| `expected ...` syntax error | Markup/code/math mode confusion | Inspect nearby `#`, `{}`, `[]`, `()`, `$`; make the smallest fix. |
| Missing citation/reference | Bibliography label mismatch or label not emitted | Search citation key/label; compile after fixing. |
| Font/glyph warning | Font unavailable or lacks glyph | Check `typst fonts`; add project font or adjust font family. |
| Package download needed | Cache miss for `@preview` import | Surface policy decision: fetch/cache/vendor/remove dependency. |
| Plugin denied/unapproved | WASM build input not approved | Hash and request/record approval or remove plugin dependency. |
| Output unchanged after edit | Edited non-entry/generated file or build failed silently | Identify entrypoint and source graph; clean output and rebuild. |

Always include the relevant file and line when reporting diagnostics. If Typst only gives short/human diagnostics, quote the exact diagnostic and map it to the source manually.

---

## Existing work to leverage

These ecosystem projects are worth using rather than rebuilding:

- **Typst CLI** — official compiler and local command-line workflow. It supports local compilation and watch-mode and exposes key controls such as root, fonts, package paths/caches, inputs, creation timestamps, dependency output, and output formats in recent versions.
- **Typst documentation and package universe** — canonical language reference, tutorial material, and package discovery. Use docs for syntax questions rather than guessing from LaTeX or Markdown habits.
- **Tinymist** — active integrated Typst language service. It provides LSP capabilities, preview support, and editor extensions. Prefer it for editor-grade features instead of custom parsing.
- **typstyle** — active formatter for Typst source. Use when the project wants automatic formatting.
- **typstfmt** — older formatter; useful only for legacy projects pinned to it. Its own README says its last supported Typst version was 0.10 and recommends typstyle.
- **Hayagriva** — bibliography ecosystem used by Typst; leverage it for bibliography data validation/conversion where appropriate.
- **PDF inspection tools** (`pdfinfo`, `mutool`, `qpdf`, `pdftotext`) — useful for CI checks and agent verification without visual inspection.

When researching new tools, check project activity, Typst version support, installation method, license, and whether it is a library, CLI, editor extension, or hosted service.

---

## Harness integration contract

A harness that wants good Typst support should expose these affordances to agents:

### Discovery tools

- list Typst entrypoints and dependency graph;
- identify imports/includes/packages/assets/bibliography/fonts;
- distinguish generated `.typ` files from canonical source;
- read project build scripts and CI.

### Toolchain tools

- locate binaries and versions (`typst`, `tinymist`, `typstyle`, optional PDF tools);
- install or suggest install plans without executing unapproved host changes;
- run bounded commands with timeouts and captured output;
- run watch/preview commands in an interactive/background session when operator interaction is expected.

### Policy tools

- set root/package/font/network/plugin policy;
- materialize packages only after approval;
- hash packages/plugins/fonts;
- store and revoke plugin approvals;
- write/read build manifests;
- keep project-scoped cache separate from global state.

### Rendering tools

- compile PDF;
- compile SVG/PNG pages for preview;
- preserve last-good preview after failed build;
- isolate rendered SVG/HTML from script/external resource execution;
- surface diagnostics and stale/current state distinctly.

### Review tools

- compare source hash to manifest;
- inspect PDF metadata/page count/text;
- optionally visual-diff rendered pages;
- record commands, versions, dependencies, and outputs.

---

## Minimal command recipes

### Doctor

```sh
set -eu
typst --version
typst compile --help | sed -n '1,120p'
typst fonts | sed -n '1,80p'
find . -name '*.typ' | sort | sed -n '1,80p'
```

### Baseline compile

```sh
set -eu
entry="${1:-main.typ}"
out="build/${entry%.typ}.pdf"
mkdir -p "$(dirname "$out")"
typst compile --root . --diagnostic-format short "$entry" "$out"
ls -lh "$out"
```

### Reproducible-ish compile

```sh
set -eu
entry="${1:-main.typ}"
mkdir -p build .cache/typst/packages
typst compile \
  --root . \
  --package-cache-path .cache/typst/packages \
  --font-path fonts \
  --ignore-system-fonts \
  --creation-timestamp 0 \
  --diagnostic-format short \
  "$entry" "build/${entry%.typ}.pdf"
```

### Format changed files with typstyle

```sh
set -eu
git diff --name-only -- '*.typ' | while IFS= read -r f; do
  [ -n "$f" ] && typstyle -i "$f"
done
```

### Capture dependencies

```sh
set -eu
entry="${1:-main.typ}"
mkdir -p build
typst compile --root . --deps build/deps.json --deps-format json "$entry" "build/${entry%.typ}.pdf"
```

---

## Anti-patterns

- Treating Typst as Markdown with better math. Typst has its own markup/code/math modes.
- Editing generated `.typ` without finding the generator/source.
- Shelling out with no `--root` in projects with nested documents.
- Relying on system fonts for release artifacts without recording that decision.
- Silently downloading packages during a supposedly reproducible build.
- Approving plugins by package name only; approve executable bytes by hash.
- Committing bulky generated PDFs/previews unless the project intentionally versions outputs.
- Reformatting an entire document set during a content-only change.
- Declaring success because PDF exists while diagnostics still show missing references/citations.

---

## Operator explanations

Use short translations for Typst-ignorant operators:

- **Entrypoint:** "the `.typ` file we compile into the PDF."
- **Template:** "shared Typst code that defines page layout, headings, fonts, and reusable blocks."
- **Package:** "third-party Typst code imported by the document. It affects rendering and may need fetching or vendoring."
- **Plugin:** "a WebAssembly executable used during compilation. It needs explicit trust handling."
- **Root:** "the directory Typst is allowed to read from and the base for resolving files."
- **System fonts:** "fonts installed on this machine; convenient but less reproducible than project fonts."
- **Manifest:** "a record of the compiler, source hash, dependencies, and outputs that shaped this build."
