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

## Core model

```rust
pub struct VisualArtifactRegistry {
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
