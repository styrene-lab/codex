use crate::models::{Document, DocumentId, Frontmatter, PublicationConfig, WikiLink};
use crate::store::ProjectStore;
use crate::visual_artifacts::{
    discover_d2_artifacts, discover_design_board_artifacts, discover_excalidraw_artifacts,
    RenderArtifact, VisualArtifact, VisualArtifactKind,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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


impl ProjectRegistry {
    pub fn discover(project_root: PathBuf, store: &dyn ProjectStore) -> anyhow::Result<Self> {
        let scope = ProjectScope {
            project_root: project_root.clone(),
            project_id: None,
            sync_identity: None,
        };
        let documents = DocumentRegistry::from_store(store)?;
        let visual_artifacts = VisualArtifactRegistry::discover(&project_root);
        let edges = build_project_edges(&documents, &visual_artifacts);
        Ok(Self {
            scope,
            documents,
            visual_artifacts,
            external_refs: ExternalRefRegistry::default(),
            raw_assets: RawAssetRegistry::default(),
            task_refs: TaskRegistryView::default(),
            spec_refs: SpecRegistryView::default(),
            edges,
        })
    }
}

impl DocumentRegistry {
    pub fn from_store(store: &dyn ProjectStore) -> anyhow::Result<Self> {
        let mut documents = Vec::new();
        for meta in store.list_documents()? {
            if let Some(doc) = store.get_document(&meta.id)? {
                let backlinks = store
                    .get_backlinks(&meta.id)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|backlink| backlink.id)
                    .collect();
                documents.push(DocumentRecord::from_document(doc, backlinks));
            }
        }
        documents.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(Self { documents })
    }

    pub fn by_path(&self, path: &std::path::Path) -> Option<&DocumentRecord> {
        self.documents.iter().find(|doc| doc.path == path)
    }
}

impl DocumentRecord {
    pub fn from_document(doc: Document, backlinks: Vec<DocumentId>) -> Self {
        let kind = classify_document_kind(&doc);
        Self {
            id: doc.id,
            path: doc.path,
            title: doc.title,
            aliases: doc.frontmatter.aliases.clone(),
            tags: doc.frontmatter.tags.clone(),
            publication: doc.frontmatter.publication.clone(),
            frontmatter: doc.frontmatter,
            kind,
            outgoing: doc.outgoing_links,
            backlinks,
        }
    }
}

impl VisualArtifactRegistry {
    pub fn discover(project_root: &std::path::Path) -> Self {
        let mut artifacts = Vec::new();
        artifacts.extend(discover_d2_artifacts(project_root).into_iter().map(VisualArtifactRecord::from));
        artifacts.extend(
            discover_excalidraw_artifacts(project_root)
                .into_iter()
                .map(VisualArtifactRecord::from),
        );
        artifacts.extend(
            discover_design_board_artifacts(project_root)
                .into_iter()
                .map(VisualArtifactRecord::from),
        );
        artifacts.sort_by(|a, b| a.id.0.cmp(&b.id.0));
        Self { artifacts }
    }

    pub fn by_source(&self, path: &std::path::Path) -> Option<&VisualArtifactRecord> {
        self.artifacts.iter().find(|record| record.artifact.source_path == path)
    }

    pub fn by_wrapper(&self, path: &std::path::Path) -> Option<&VisualArtifactRecord> {
        self.artifacts
            .iter()
            .find(|record| record.artifact.wrapper_path.as_deref() == Some(path))
    }
}

fn classify_document_kind(doc: &Document) -> DocumentKind {
    if doc.frontmatter.tags.iter().any(|tag| tag == "drawing" || tag == "design_board") {
        return DocumentKind::ArtifactWrapper;
    }
    match doc.frontmatter.kind.as_deref() {
        Some("task") => DocumentKind::Task,
        Some("spec") | Some("openspec_scenario") => DocumentKind::Spec,
        Some("design_node") => DocumentKind::DesignNode,
        Some("generated_index") => DocumentKind::GeneratedIndex,
        _ => DocumentKind::Note,
    }
}

fn build_project_edges(
    documents: &DocumentRegistry,
    visual_artifacts: &VisualArtifactRegistry,
) -> Vec<ProjectEdge> {
    let mut edges = Vec::new();
    for artifact in &visual_artifacts.artifacts {
        if let Some(wrapper_path) = &artifact.artifact.wrapper_path {
            if let Some(wrapper_doc) = documents.by_path(wrapper_path) {
                edges.push(ProjectEdge {
                    from: ProjectNodeRef::Document(wrapper_doc.id.clone()),
                    to: ProjectNodeRef::VisualArtifact(artifact.id.clone()),
                    relation: ProjectRelation::Wraps,
                    source: EdgeSource::ArtifactDiscovery,
                });
            }
        }
        for render in &artifact.artifact.renders {
            edges.push(ProjectEdge {
                from: ProjectNodeRef::VisualArtifact(artifact.id.clone()),
                to: ProjectNodeRef::ProjectPath(render.path.clone()),
                relation: ProjectRelation::RendersTo,
                source: EdgeSource::ArtifactDiscovery,
            });
        }
    }

    for doc in &documents.documents {
        for link in &doc.outgoing {
            edges.push(ProjectEdge {
                from: ProjectNodeRef::Document(doc.id.clone()),
                to: ProjectNodeRef::ProjectPath(PathBuf::from(&link.target)),
                relation: ProjectRelation::LinksTo,
                source: EdgeSource::MarkdownWikilink {
                    path: doc.path.clone(),
                },
            });
        }
    }
    edges
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ProjectScope {
    /// Absolute canonical path of the open Flynt project/vault/repo root.
    /// Runtime boundary only; do not serialize into committed snapshots unless
    /// explicitly marked as local diagnostics.
    pub project_root: PathBuf,
    pub project_id: Option<String>,
    pub sync_identity: Option<ProjectSyncIdentity>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ProjectSyncIdentity {
    pub vcs_kind: Option<String>,
    pub remote_url: Option<String>,
    pub branch: Option<String>,
    pub worktree_root: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DocumentRegistry {
    pub documents: Vec<DocumentRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentRecord {
    pub id: DocumentId,
    pub path: PathBuf,
    pub title: String,
    pub frontmatter: Frontmatter,
    pub aliases: Vec<String>,
    pub tags: Vec<String>,
    pub publication: PublicationConfig,
    pub kind: DocumentKind,
    pub outgoing: Vec<WikiLink>,
    pub backlinks: Vec<DocumentId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DocumentKind {
    #[default]
    Note,
    Task,
    Spec,
    DesignNode,
    ArtifactWrapper,
    GeneratedIndex,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct VisualArtifactRegistry {
    pub artifacts: Vec<VisualArtifactRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualArtifactRecord {
    pub id: VisualArtifactId,
    pub artifact: VisualArtifact,
    pub surfaces: Vec<ArtifactSurfaceCapability>,
    pub metadata: ArtifactMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct VisualArtifactId(pub String);

impl VisualArtifactId {
    pub fn from_project_relative_source(kind: VisualArtifactKind, source_path: &std::path::Path) -> Self {
        let prefix = match kind {
            VisualArtifactKind::D2Diagram => "d2",
            VisualArtifactKind::ExcalidrawDrawing => "excalidraw",
            VisualArtifactKind::DesignBoard => "design-board",
            VisualArtifactKind::Flow => "flow",
        };
        Self(format!("{prefix}:{}", normalize_project_path(source_path)))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArtifactSurfaceCapability {
    Preview,
    Edit,
    RevealSource,
    Render(crate::visual_artifacts::RenderFormat),
    Inspect,
    ShowConsumers,
    ShowDependencies,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ArtifactMetadata {
    pub tags: Vec<String>,
    pub title_source: TitleSource,
    pub mime_type: Option<String>,
    pub publication: ArtifactPublication,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TitleSource {
    WrapperFrontmatter,
    #[default]
    SourceFilename,
    EmbeddedLabel,
    ExternalTitle,
    Generated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactPublication {
    #[default]
    Private,
    Public,
    Unlisted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RawAssetRegistry {
    pub assets: Vec<RawAssetRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawAssetRecord {
    pub id: RawAssetId,
    pub path: PathBuf,
    pub media_type: String,
    pub role: RawAssetRole,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RawAssetId(pub String);

impl RawAssetId {
    pub fn from_project_relative_path(path: &std::path::Path) -> Self {
        Self(format!("asset:{}", normalize_project_path(path)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawAssetRole {
    RenderSidecar,
    Image,
    Stylesheet,
    Script,
    Data,
    ImportExportOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ExternalRefRegistry {
    pub refs: Vec<ExternalRefRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalRefRecord {
    pub id: ExternalRefId,
    pub target: ExternalTarget,
    pub label: Option<String>,
    pub provider: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ExternalRefId(pub String);

impl ExternalRefId {
    pub fn from_uri(uri: &str) -> Self {
        Self(format!("external-uri:{uri}"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExternalTarget {
    Uri(String),
    ExternalFile(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TaskRegistryView {
    pub task_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SpecRegistryView {
    pub spec_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectEdge {
    pub from: ProjectNodeRef,
    pub to: ProjectNodeRef,
    pub relation: ProjectRelation,
    pub source: EdgeSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProjectNodeRef {
    Document(DocumentId),
    VisualArtifact(VisualArtifactId),
    RawAsset(RawAssetId),
    ExternalRef(ExternalRefId),
    Task(String),
    Spec(String),
    ProjectPath(PathBuf),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeSource {
    MarkdownWikilink { path: PathBuf },
    MarkdownEmbed { path: PathBuf },
    Frontmatter { path: PathBuf, field: String },
    ArtifactDiscovery,
    TaskMembership,
    SpecLifecycle,
    Generated,
}

pub fn normalize_project_path(path: &std::path::Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

impl From<VisualArtifact> for VisualArtifactRecord {
    fn from(artifact: VisualArtifact) -> Self {
        let id = VisualArtifactId::from_project_relative_source(artifact.kind, &artifact.source_path);
        let surfaces = default_surfaces_for_artifact(artifact.kind, &artifact.renders);
        Self {
            id,
            artifact,
            surfaces,
            metadata: ArtifactMetadata::default(),
        }
    }
}

fn default_surfaces_for_artifact(
    kind: VisualArtifactKind,
    renders: &[RenderArtifact],
) -> Vec<ArtifactSurfaceCapability> {
    let mut surfaces = vec![
        ArtifactSurfaceCapability::Preview,
        ArtifactSurfaceCapability::RevealSource,
        ArtifactSurfaceCapability::Inspect,
        ArtifactSurfaceCapability::ShowConsumers,
        ArtifactSurfaceCapability::ShowDependencies,
    ];
    match kind {
        VisualArtifactKind::D2Diagram
        | VisualArtifactKind::ExcalidrawDrawing
        | VisualArtifactKind::DesignBoard
        | VisualArtifactKind::Flow => surfaces.push(ArtifactSurfaceCapability::Edit),
    }
    for render in renders {
        surfaces.push(ArtifactSurfaceCapability::Render(render.format));
    }
    surfaces
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::visual_artifacts::{RenderFormat, RenderStatus};

    #[test]
    fn artifact_ids_are_project_relative_and_kind_prefixed() {
        let id = VisualArtifactId::from_project_relative_source(
            VisualArtifactKind::ExcalidrawDrawing,
            std::path::Path::new("drawings/sketch.excalidraw"),
        );
        assert_eq!(id.0, "excalidraw:drawings/sketch.excalidraw");
    }

    #[test]
    fn visual_artifact_record_derives_default_surfaces() {
        let artifact = VisualArtifact {
            kind: VisualArtifactKind::D2Diagram,
            title: "system.d2".into(),
            source_path: PathBuf::from("diagrams/system.d2"),
            wrapper_path: None,
            renders: vec![RenderArtifact {
                format: RenderFormat::Svg,
                path: PathBuf::from("diagrams/system.svg"),
                status: RenderStatus::Current,
            }],
        };

        let record = VisualArtifactRecord::from(artifact);
        assert_eq!(record.id.0, "d2:diagrams/system.d2");
        assert!(record.surfaces.contains(&ArtifactSurfaceCapability::Preview));
        assert!(record.surfaces.contains(&ArtifactSurfaceCapability::Edit));
        assert!(record
            .surfaces
            .contains(&ArtifactSurfaceCapability::Render(RenderFormat::Svg)));
    }

    #[test]
    fn project_registry_serializes_as_plain_json() {
        let registry = ProjectRegistry::default();
        let json = serde_json::to_string_pretty(&registry).unwrap();
        assert!(json.contains("documents"));
        assert!(json.contains("visual_artifacts"));
        assert!(json.contains("edges"));
    }
}
