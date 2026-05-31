---
title: Visual Artifact Registry
status: decided
tags: [visual-artifacts, registry, architecture]
---

# Visual Artifact Registry

## Decision

Flynt should introduce a typed `VisualArtifactRegistry` as the source of truth for visual artifact identity, raw sources, document wrappers, renders, consumers, and supported surfaces.

Markdown wrappers remain the standardized document interface for tabs/search/sidebar/agents/publication, but wrappers are not the artifact source of truth. They are adapters into the artifact registry.

## Problem

Today artifact state is inferred repeatedly from filesystem conventions and wrapper bodies:

```text
drawings/foo.excalidraw
        + drawings/foo.md
        + drawings/foo.svg
        + sidebar discovery
        + NotesView wrapper detection
        + UI-state classification
```

That works for simple 1:1 drawings, but it breaks down when:

- one note embeds multiple artifacts
- one artifact has multiple rendered outputs
- a wrapper is missing or corrupted
- preview/edit/source are different surfaces of the same artifact
- external URIs are consumed by diagrams or notes
- HTML/CSS components need to be treated as reusable visual artifacts


## Project-root boundary and sync semantics

A `VisualArtifactRegistry` is scoped to exactly one open Flynt project/vault/repo root. It must never merge or compare artifact records across different roots unless an explicit cross-project feature is later designed.

```rust
pub struct VisualArtifactRegistry {
    pub scope: ArtifactRegistryScope,
    pub artifacts: Vec<VisualArtifactRecord>,
    pub edges: Vec<VisualArtifactEdge>,
}

pub struct ArtifactRegistryScope {
    /// Absolute canonical path of the open Flynt project/vault/repo root.
    /// Used only as the runtime boundary, not as durable synced identity.
    pub project_root: PathBuf,

    /// Stable local project identifier when available from Flynt project metadata.
    pub project_id: Option<String>,

    /// Optional VCS/workspace identity for sync diagnostics, never for path resolution.
    pub sync_identity: Option<ArtifactSyncIdentity>,
}

pub struct ArtifactSyncIdentity {
    pub vcs_kind: Option<String>,
    pub remote_url: Option<String>,
    pub branch: Option<String>,
    pub worktree_root: Option<PathBuf>,
}
```

### Path rules

Registry records store project-relative paths for durable artifact identity:

```text
source.path = drawings/foo.excalidraw
wrapper.path = drawings/foo.md
render.path = drawings/foo.svg
```

Absolute paths are allowed only in `ArtifactRegistryScope.project_root` and transient runtime caches. They must not be written into synced metadata, wrappers, notes, or artifact records intended to survive checkout relocation.

All path resolution must follow:

```text
absolute_path = registry.scope.project_root.join(project_relative_path)
```

No `..` segments, absolute source paths, symlink escapes, or paths outside `project_root` are valid registry entries. External references must use `ArtifactSource::ExternalUri`, not fake project paths.

### Sync implications

The registry is a derived index over files in the open project root. For initial implementation it should be rebuilt from project files and the document store, not synced as an authoritative database.

This matters because:

- multiple clones of the same repo have different absolute roots
- one machine may have stale render sidecars until export/render runs
- wrapper repair must create files inside the current project root only
- artifact ids must remain stable across clones if paths are unchanged
- source, wrapper, and render sidecars are normal sync participants
- external URI artifacts are references, not copied project files

### Artifact id stability

Initial deterministic id rule:

```text
artifact_id = kind + ":" + normalized_project_relative_source
```

Examples:

```text
d2:diagrams/system.d2
excalidraw:drawings/foo.excalidraw
image:assets/logo.svg
external-uri:https://example.com/spec
component:components/button.component.html
```

This keeps ids portable across machines while remaining bounded to one project. If a file moves, it is a new artifact unless a future move/rename tracker records lineage.

### Multi-root non-goal

Do not let one running registry span:

- multiple Flynt projects
- nested git worktrees as independent roots
- user-global asset libraries
- external URI caches shared across projects

Those may become separate registries or federated registry views later. The first registry is strictly one open root.

## Core model

```rust
pub struct VisualArtifactRegistry {
    pub scope: ArtifactRegistryScope,
    pub artifacts: Vec<VisualArtifactRecord>,
    pub edges: Vec<VisualArtifactEdge>,
}

pub struct VisualArtifactRecord {
    pub id: VisualArtifactId,
    pub kind: VisualArtifactKind,
    pub title: String,
    pub source: ArtifactSource,
    pub wrapper: Option<ArtifactWrapper>,
    pub renders: Vec<RenderArtifact>,
    pub surfaces: Vec<ArtifactSurfaceCapability>,
    pub metadata: ArtifactMetadata,
}

pub struct VisualArtifactId(pub String);

pub enum ArtifactSource {
    ProjectFile { path: PathBuf },
    ExternalUri { uri: String },
    EmbeddedInline { owner: ArtifactOwner, fragment_id: String },
    Generated { generator: String, output_path: Option<PathBuf> },
}

pub struct ArtifactWrapper {
    pub document_id: Option<DocumentId>,
    pub path: PathBuf,
    pub wrapper_kind: ArtifactWrapperKind,
}

pub enum ArtifactWrapperKind {
    CanonicalOneToOne,
    ComposedDocumentEmbed,
    VirtualGenerated,
}

pub struct RenderArtifact {
    pub format: RenderFormat,
    pub path: PathBuf,
    pub status: RenderStatus,
    pub produced_by: Option<String>,
}

pub enum ArtifactSurfaceCapability {
    Preview,
    Edit,
    RevealSource,
    Render(RenderFormat),
    Inspect,
    ShowConsumers,
    ShowDependencies,
}

pub struct ArtifactMetadata {
    pub tags: Vec<String>,
    pub title_source: TitleSource,
    pub mime_type: Option<String>,
    pub publication: ArtifactPublication,
}

pub enum TitleSource {
    WrapperFrontmatter,
    SourceFilename,
    EmbeddedLabel,
    ExternalTitle,
    Generated,
}
```

## Relationship model

```rust
pub struct VisualArtifactEdge {
    pub from: ArtifactEndpoint,
    pub to: ArtifactEndpoint,
    pub relation: ArtifactRelation,
}

pub enum ArtifactEndpoint {
    Artifact(VisualArtifactId),
    Document(DocumentId),
    ProjectPath(PathBuf),
    ExternalUri(String),
}

pub enum ArtifactRelation {
    Wraps,
    Embeds,
    Consumes,
    RendersTo,
    DerivedFrom,
    LinksTo,
    DependsOn,
}
```

Key distinction:

```text
wrapper.md WRAPS foo.excalidraw
architecture.md EMBEDS foo.excalidraw
board.board CONSUMES foo.excalidraw
foo.excalidraw RENDERS_TO foo.svg
foo.excalidraw LINKS_TO https://example.com
```

Wrappers and composed notes may both use markdown embed syntax, but the registry records different relations.

## Registry query surface

Initial API shape:

```rust
impl VisualArtifactRegistry {
    pub fn discover(project_root: &Path, store: &dyn ProjectStore) -> Self;

    pub fn artifact_by_id(&self, id: &VisualArtifactId) -> Option<&VisualArtifactRecord>;
    pub fn artifact_by_source(&self, path: &Path) -> Option<&VisualArtifactRecord>;
    pub fn artifact_by_wrapper(&self, path: &Path) -> Option<&VisualArtifactRecord>;

    pub fn wrappers_for(&self, id: &VisualArtifactId) -> Vec<&ArtifactWrapper>;
    pub fn consumers_of(&self, id: &VisualArtifactId) -> Vec<&VisualArtifactEdge>;
    pub fn dependencies_of(&self, id: &VisualArtifactId) -> Vec<&VisualArtifactEdge>;

    pub fn default_surface_for(&self, id: &VisualArtifactId) -> Option<ArtifactSurfaceCapability>;
    pub fn surface_for_action(
        &self,
        id: &VisualArtifactId,
        action: ArtifactActionKind,
    ) -> Option<ArtifactSurfaceCapability>;
}
```

## Initial artifact primitive support map

| Primitive | Source kind | Wrapper? | Preview | Edit | Render outputs | Relations |
|---|---|---:|---|---|---|---|
| D2 diagram | `ProjectFile(.d2)` | optional `.md` | SVG/PNG sidecar | source text editor | SVG, PNG | embeds external URIs, links to project refs |
| Excalidraw drawing | `ProjectFile(.excalidraw)` | canonical `.md` | SVG/PNG sidecar | Excalidraw editor | SVG, PNG | embeds images, links external URIs |
| Image | `ProjectFile(.png/.jpg/.jpeg/.gif/.webp/.svg)` | optional | image element | external/default file editor initially | derived thumbnails later | embedded by notes/boards/drawings |
| External URI | `ExternalUri` | no | link card/screenshot later | external browser | screenshot/card cache later | linked by notes/diagrams/drawings |
| HTML/CSS component | `ProjectFile(.component.html/.component.css)` or `EmbeddedInline` | optional | sandboxed preview | source/component editor | PNG/HTML snapshot | consumed by design boards/docs |
| Design Board | `ProjectFile(.board)` | canonical `.md` | board renderer | board editor | HTML, PNG | consumes D2/Excalidraw/images/components |
| Flow graph | `ProjectFile(.flow)` | optional/canonical later | flow renderer | flow editor | SVG/PNG later | links components/artifacts |

## Immediate support phases

### Phase 1 — In-memory registry façade

Build `VisualArtifactRegistry` from existing discovery functions without changing persistence.

- include D2, Excalidraw, Design Board
- add image discovery for common image formats
- record wrapper-vs-embed distinction
- expose lookup helpers used by sidebar and surface resolver

### Phase 2 — Surface resolver consumes registry

Replace ad hoc path/body detection in `visual_artifact_surface` with registry lookups:

```text
active document path -> registry.artifact_by_wrapper(path) -> surface
```

Normal notes with multiple embeds stay notes; their embeds become `Embeds` edges.

### Phase 3 — Action executor consumes registry

`ArtifactActionKind::Open/Edit/RevealSource/Render/Inspect` should route through registry records, not path conventions.

### Phase 4 — Add image and external URI primitives

Index image files as visual artifacts and external links as URI artifacts where they are consumed by visual surfaces.

### Phase 5 — Add HTML/CSS component primitive

Treat reusable components as visual artifacts with sandboxed previews and board consumption edges.

## Non-goals for first implementation

- no database schema migration yet
- no full content-addressed blob store yet
- no automatic external URI screenshotting yet
- no headless Excalidraw render service beyond existing sidecar export

## Design constraints

- Raw artifact JSON/binary/source files must not be indexed as prose notes.
- A canonical wrapper is an adapter into the document system, not the source of truth.
- Multiple notes may embed the same artifact.
- One note may embed many artifacts.
- `Open` should prefer preview/read surfaces.
- `Edit` should prefer artifact-native editors.
- `RevealSource` should expose raw source.
- Registry discovery must be deterministic and cheap enough for sidebar refresh.
