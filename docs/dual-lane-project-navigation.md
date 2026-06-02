---
id: dual-lane-project-navigation
title: "Dual-lane project navigation"
status: exploring
tags: [navigation, project-registry, sidebar, notes, artifacts]
open_questions:
  - "[assumption] Ordinary markdown notes can be distinguished from artifact wrappers using ProjectRegistry DocumentKind::Note vs DocumentKind::ArtifactWrapper without losing user-authored markdown that lives near artifacts."
  - "[assumption] The primary sidebar should keep a visible low-level Notes/Files tree rather than replacing it with a fully semantic navigator."
  - "Should the artifact lane be a separate sidebar section under the same file explorer, or a separate tab/pane beside Notes, Tasks, Graph, and Settings?"
  - "What escape hatch should expose wrappers, sources, sidecars, and generated files: a global 'Show internals' toggle, an Advanced/Raw Files section, or per-artifact context actions only?"
dependencies: []
related: []
---

# Dual-lane project navigation

## Overview

Design Flynt's navigation model as two complementary lanes: a low-level plaintext/filesystem note browser that preserves Obsidian/Zed-style file agency, and a semantic artifact navigator that presents boards, drawings, diagrams, flows, tasks, specs, and raw/generated assets as domain objects rather than leaking wrapper/source/sidecar files into the note tree.

## Decisions

### Preserve low-level note/file agency

**Status:** accepted

**Rationale:** Flynt must keep the Obsidian/Zed affordance of creating notes, creating folders, moving/renaming/deleting ordinary markdown files, and seeing the actual plaintext filesystem organization. Semantic navigation must not erase that layer.

### Treat visual artifacts as multi-file domain objects

**Status:** accepted

**Rationale:** Boards, drawings, diagrams, and flows are not ordinary notes even when they have markdown wrappers. Their user-facing navigation entry should be the semantic artifact; wrapper/source/sidecar files should be accessible through artifact actions or advanced raw mode.

### Make .board the native Flynt design surface

**Status:** accepted

**Rationale:** Flynt should use boards/*.board and design_board_* tools for current design work. Alternative canvas terminology should not appear in current navigation or creation flows.

### Bridge semantic artifacts back to filesystem via reveal actions

**Status:** accepted

**Rationale:** Semantic artifact navigation should not hide file truth. Every semantic-domain artifact entry should expose context actions such as Reveal in Filesystem Explorer, Reveal Source, and Reveal Wrapper/Sidecars where applicable, so operators can jump from the domain object back to its concrete project-relative files without making wrappers/source files first-class note entries.

### Use separate notes filesystem and artifact semantic trees

**Status:** accepted

**Rationale:** The primary sidebar should contain two adjacent tree systems. The Notes/Text Files section reflects the real filesystem hierarchy for ordinary plaintext files and should mimic best-in-class Obsidian/Zed file browser affordances. The Artifacts section is also tree-like, but registry-derived and semantic: it groups boards, drawings, diagrams, flows, and related domain artifacts without exposing wrapper/source/sidecar implementation files as ordinary notes.

### Expose artifact backing files through context reveal actions only

**Status:** accepted

**Rationale:** There should be no global raw-files tree in the default navigation model. Power users can right-click semantic artifacts and use reveal actions such as Reveal in Filesystem Explorer, Reveal Source, Reveal Wrapper, and Reveal Generated Outputs. This preserves low-level agency without turning implementation files into first-class navigation entries.

### Notes tree hides artifact wrappers and generated sidecars by default

**Status:** accepted

**Rationale:** The Notes/Text Files tree is still a real filesystem hierarchy, but scoped to ordinary plaintext documents. Markdown wrappers for boards/drawings and generated render sidecars should be filtered out because they are implementation files for semantic artifacts, not notes the operator should edit through normal note workflows.

### Use lane-specific creation affordances

**Status:** accepted

**Rationale:** Notes/Text Files creation should use standard file-browser affordances with separate New File and New Folder actions. Artifact creation should use per-type New + affordances, e.g. Boards +, Drawings +, Diagrams +, and Flows +, because each artifact type owns a semantic multi-file creation command and should not be conflated with plain file creation.

### Resolve user-authored artifact links to semantic artifact nodes

**Status:** accepted

**Rationale:** Ordinary notes should be able to link to artifacts by human-readable identity, e.g. [[Demo]] or [[qrypt-system-architecture]], and have those resolve to semantic artifact nodes when no ordinary note target wins. Wrapper embed links such as ![[Demo.board]] are implementation edges and should be hidden from ordinary backlinks/outgoing link diagnostics by default.

### Notes/Text Files lane includes ordinary markdown and safe plaintext files

**Status:** accepted

**Rationale:** The low-level notes/filesystem lane should preserve plaintext agency beyond markdown. Ordinary markdown remains the primary note type, but safe plaintext files such as .txt, .toml, .yaml, .json, and .csv may appear as raw text files. Flynt can later add richer editor/viewer surfaces for common plaintext formats without removing raw source access.

### Tasks and specs use their own semantic sections

**Status:** accepted

**Rationale:** Artifacts should mean visual/file-backed domain objects such as boards, drawings, diagrams, and flows. Tasks and OpenSpec changes are separate interaction domains and should appear as their own semantic sidebar sections rather than being folded into Artifacts.

### Wrapper source editing is explicit-only

**Status:** accepted

**Rationale:** Artifact wrappers are protected from normal note editing and autosave. Power users may access wrapper source through explicit context actions such as Reveal Wrapper Source, with clear protected/advanced affordances. Normal note workflows must not overwrite wrappers.

### Sidebar projection uses live registry discovery, not persisted snapshots

**Status:** accepted

**Rationale:** The persisted ProjectRegistry snapshot is diagnostic/generated state, not UI truth. Sidebar semantic projection should use live project/store discovery or an in-memory ProjectRegistry rebuilt from current project state. If discovery fails, the app should degrade gracefully rather than using stale persisted snapshots as authoritative navigation data.

## Open Questions

- [assumption] Ordinary markdown notes can be distinguished from artifact wrappers using ProjectRegistry DocumentKind::Note vs DocumentKind::ArtifactWrapper without losing user-authored markdown that lives near artifacts.
- [assumption] The primary sidebar should keep a visible low-level Notes/Files tree rather than replacing it with a fully semantic navigator.
- Should the artifact lane be a separate sidebar section under the same file explorer, or a separate tab/pane beside Notes, Tasks, Graph, and Settings?
- What escape hatch should expose wrappers, sources, sidecars, and generated files: a global 'Show internals' toggle, an Advanced/Raw Files section, or per-artifact context actions only?
