---
title: Project Registry
status: exploring
tags: [project-registry, documents, graph, plaintext]
---

# Project Registry

## Decision

Flynt needs a top-level plaintext `ProjectRegistry` that represents the currently open project/vault/repo as structured semantic state: documents, visual artifacts, external references, raw plaintext assets, and graph relationships.

This registry is not a proprietary document management layer. Flynt's internal truth remains plaintext, local-first, syncable, and FOSS-friendly.

## Non-goal: proprietary office formats as first-class objects

Formats such as `.docx`, `.xlsx`, `.pptx`, proprietary design files, and other opaque/binary application formats should not become first-class internal Flynt artifacts.

They may be supported only as:

- import sources
- export targets
- external URI/file references
- attachments opened by the system default application
- publication/import pipeline inputs or outputs

They should not define Flynt's internal semantic model.

If a proprietary file is referenced from a note, model it as an external/file reference edge, not as a parsed document with Flynt-owned semantics.


## Obsidian-superset compatibility constraint

Flynt should be a superset of Obsidian-style plaintext vault behavior, not a replacement that breaks the underlying portability contract. Better structured features must not require unacceptable compromises to:

- plaintext markdown compatibility
- human-editable project files
- project-relative links and embeds
- local-first sync
- git-friendly diffs
- the ability to keep using other markdown/vault tools

Registry data may add structure, caching, diagnostics, and richer surfaces, but the canonical project should remain understandable and useful as a folder of plaintext files plus open/web-native assets.

Implications:

- Markdown notes remain real markdown notes.
- Wikilinks and embeds remain visible in text.
- Visual artifact wrappers are adapters, not opaque database handles.
- Derived registry snapshots must be safe to delete and rebuild.
- New features should degrade gracefully in tools that only understand markdown files.
- Binary/proprietary formats stay at import/export/default-opener boundaries.

## Core principle

Documents are the primary semantic substrate. Visual artifacts, external references, tasks, specs, and raw assets attach to or relate through documents and graph edges.

The project registry should preserve the same semantics the graph view exposes, but internally as typed structs that can be serialized to plaintext for syncing and inspection.

## Scope

A `ProjectRegistry` is scoped to one open Flynt project root, matching the Visual Artifact Registry boundary.

```rust
pub struct ProjectRegistry {
    pub scope: ProjectScope,
    pub documents: DocumentRegistry,
    pub visual_artifacts: VisualArtifactRegistry,
    pub external_refs: ExternalRefRegistry,
    pub raw_assets: RawAssetRegistry,
    pub task_refs: TaskRegistryView,
    pub spec_refs: SpecRegistryView,
    pub edges: Vec<ProjectEdge>,
}

pub struct ProjectScope {
    pub project_root: PathBuf,
    pub project_id: Option<String>,
    pub sync_identity: Option<ProjectSyncIdentity>,
}
```

`project_root` is the runtime boundary. Durable identities use project-relative paths, document ids, or stable external URIs.

## Document registry

The existing `ProjectStore`/document index already owns durable document identity. The `DocumentRegistry` is a structured registry view over that store, not a replacement for it.

```rust
pub struct DocumentRegistry {
    pub documents: Vec<DocumentRecord>,
}

pub struct DocumentRecord {
    pub id: DocumentId,
    pub path: PathBuf,
    pub title: String,
    pub frontmatter: Frontmatter,
    pub aliases: Vec<String>,
    pub tags: Vec<String>,
    pub publication: PublicationConfig,
    pub kind: DocumentKind,
    pub outgoing: Vec<DocumentLink>,
    pub backlinks: Vec<DocumentId>,
}

pub enum DocumentKind {
    Note,
    Task,
    Spec,
    DesignNode,
    ArtifactWrapper,
    GeneratedIndex,
}
```

Documents remain markdown/plaintext. Their semantics include frontmatter, wikilinks, embeds, publication state, aliases, tags, and typed relationships.


## Image and visual media dependency policy

Core artifact/image features must not require users to install system image libraries or external CLI tools. Default Flynt builds should work from Cargo alone on normal developer machines.

Accepted default stack direction:

- `image` for raster decode/encode compatibility
- `fast_image_resize` for thumbnails/resizing
- `resvg` / `usvg` / `tiny-skia` for self-contained SVG rasterization
- `svg` for simple SVG generation/manipulation

Avoid as default dependencies:

- ImageMagick / GraphicsMagick
- system `librsvg`, cairo, pango stacks
- required native `libwebp` bindings
- shelling out for normal preview/render/import/export behavior

Format policy:

- SVG is the canonical vector/open diagram image format.
- WebP is the preferred future raster cache/export target when pure-Rust encoding is reliable.
- PNG is an acceptable fallback raster sidecar/cache format until WebP is boring and system-lib-free.
- PNG/JPEG/GIF remain compatibility import/export/display formats, not preferred internal primitives.

Native/system-library bindings may exist only behind optional features and must not be required by the default app.

## Raw asset registry

`RawAssetRegistry` is for plaintext or open web-native assets that are useful inside Flynt but are not semantic documents.

```rust
pub struct RawAssetRegistry {
    pub assets: Vec<RawAssetRecord>,
}

pub struct RawAssetRecord {
    pub id: RawAssetId,
    pub path: PathBuf,
    pub media_type: String,
    pub role: RawAssetRole,
}

pub enum RawAssetRole {
    RenderSidecar,
    Image,
    Stylesheet,
    Script,
    Data,
    ImportExportOnly,
}
```

Examples:

- `.svg`, `.png`, `.jpg`, `.webp` as images or render sidecars
- `.css`, `.html`, `.js` as component/source assets where explicitly supported
- `.json`, `.toml`, `.yaml`, `.csv` as plaintext data when referenced or domain-modeled

Opaque/proprietary binaries should usually be `ImportExportOnly` or external file refs, not deep registry objects.

## External references

External URIs and external files are references, not internal project truth.

```rust
pub struct ExternalRefRegistry {
    pub refs: Vec<ExternalRefRecord>,
}

pub struct ExternalRefRecord {
    pub id: ExternalRefId,
    pub target: ExternalTarget,
    pub label: Option<String>,
    pub provider: Option<String>,
}

pub enum ExternalTarget {
    Uri(String),
    ExternalFile(PathBuf),
}
```

An external file path is outside `project_root`; Flynt may offer "open with system default app" but should not sync or internalize it without an explicit import step.

## Project graph relationships

The registry's edge list is the typed form of the project graph.

```rust
pub struct ProjectEdge {
    pub from: ProjectNodeRef,
    pub to: ProjectNodeRef,
    pub relation: ProjectRelation,
    pub source: EdgeSource,
}

pub enum ProjectNodeRef {
    Document(DocumentId),
    VisualArtifact(VisualArtifactId),
    RawAsset(RawAssetId),
    ExternalRef(ExternalRefId),
    Task(String),
    Spec(String),
    ProjectPath(PathBuf),
}

pub enum ProjectRelation {
    LinksTo,
    Embeds,
    Wraps,
    Consumes,
    RendersTo,
    DerivedFrom,
    DependsOn,
    BelongsTo,
    Implements,
    References,
}

pub enum EdgeSource {
    MarkdownWikilink { path: PathBuf },
    MarkdownEmbed { path: PathBuf },
    Frontmatter { path: PathBuf, field: String },
    ArtifactDiscovery,
    TaskMembership,
    SpecLifecycle,
    Generated,
}
```

This lets the graph view, sidebar, agent tools, publication pipeline, and sync diagnostics consume the same typed relationship model.

## Plaintext serialization

The registry should be derivable from source files, but a serialized snapshot can be useful for debugging, sync diagnostics, and agent context.

If serialized, use a plaintext format:

```text
.flynt/registry/project-registry.json
```

or later:

```text
.flynt/registry/project-registry.toml
```

Rules:

- snapshot is derived, not authoritative
- no absolute paths except optional runtime diagnostics that must not be committed
- deterministic ordering for stable diffs
- safe to delete and rebuild
- no proprietary binary blobs

## Relationship to existing systems

| Existing system | Registry role |
|---|---|
| `ProjectStore` | source of document records and document ids |
| Graph view | visualization of `ProjectEdge` relationships |
| `VisualArtifactRegistry` | subregistry for visual artifact records and artifact-specific surfaces |
| Task/board stores | provide task/board registry views and membership edges |
| OpenSpec/design tree | provide spec/design lifecycle nodes and edges |
| External ref parser | provides `ExternalRefRecord` and `LinksTo` edges |

## First implementation path

1. Define registry structs in `flynt-core` without persistence changes.
2. Build an in-memory `ProjectRegistry::discover(project)` façade from existing stores/discovery functions.
3. Use it to power graph/sidebar/surface lookup incrementally.
4. Add deterministic plaintext snapshot output for diagnostics only.
5. Only later decide whether any part should become persisted authoritative state.

## Constraints

- The registry is project-root scoped.
- Durable paths are project-relative.
- Documents remain plaintext markdown.
- Visual artifacts remain source files plus wrappers/renders.
- Proprietary formats are import/export/default-opener references only.
- The graph's semantic relationships should be represented as typed structs, not only UI state.
