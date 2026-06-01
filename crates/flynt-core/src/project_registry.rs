use crate::models::{Document, DocumentId, Frontmatter, PublicationConfig, WikiLink};
use crate::store::ProjectStore;
use crate::visual_artifacts::{
    RenderArtifact, VisualArtifact, VisualArtifactKind, discover_d2_artifacts,
    discover_design_board_artifacts, discover_excalidraw_artifacts,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectRegistry {
    pub scope: ProjectScope,
    pub documents: DocumentRegistry,
    pub visual_artifacts: VisualArtifactRegistry,
    pub external_refs: ExternalRefRegistry,
    pub evidence: EvidenceRegistry,
    pub diagnostics: Vec<ProjectRegistryDiagnostic>,
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
        let visual_artifacts = VisualArtifactRegistry::discover(&project_root);
        let documents = DocumentRegistry::from_store(store, &visual_artifacts)?;
        let diagnostics = collect_registry_diagnostics(&documents, &visual_artifacts);
        let edges = build_project_edges(&documents, &visual_artifacts);
        Ok(Self {
            scope,
            documents,
            visual_artifacts,
            external_refs: ExternalRefRegistry::default(),
            evidence: EvidenceRegistry::discover(&project_root),
            diagnostics,
            raw_assets: RawAssetRegistry::default(),
            task_refs: TaskRegistryView::default(),
            spec_refs: SpecRegistryView::default(),
            edges,
        })
    }
}

impl DocumentRegistry {
    pub fn from_store(
        store: &dyn ProjectStore,
        visual_artifacts: &VisualArtifactRegistry,
    ) -> anyhow::Result<Self> {
        let mut documents = Vec::new();
        for meta in store.list_documents()? {
            if let Some(doc) = store.get_document(&meta.id)? {
                let backlinks = store
                    .get_backlinks(&meta.id)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|backlink| backlink.id)
                    .collect();
                documents.push(DocumentRecord::from_document(
                    doc,
                    backlinks,
                    visual_artifacts,
                ));
            }
        }
        documents.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(Self { documents })
    }

    pub fn by_path(&self, path: &std::path::Path) -> Option<&DocumentRecord> {
        self.documents.iter().find(|doc| doc.path == path)
    }

    pub fn resolve_link_target(&self, target: &str) -> Option<&DocumentRecord> {
        let target = target.split('#').next().unwrap_or(target).trim();
        if target.is_empty() {
            return None;
        }
        let normalized = normalize_slug(target);
        self.documents.iter().find(|doc| {
            normalize_slug(&doc.title) == normalized
                || normalize_slug(
                    doc.path
                        .file_stem()
                        .and_then(|stem| stem.to_str())
                        .unwrap_or(""),
                ) == normalized
                || normalize_slug(doc.path.to_string_lossy().trim_end_matches(".md")) == normalized
        })
    }
}

impl DocumentRecord {
    pub fn from_document(
        doc: Document,
        backlinks: Vec<DocumentId>,
        visual_artifacts: &VisualArtifactRegistry,
    ) -> Self {
        let kind = classify_document_kind(&doc, visual_artifacts);
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
        artifacts.extend(
            discover_d2_artifacts(project_root)
                .into_iter()
                .map(VisualArtifactRecord::from),
        );
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
        self.artifacts
            .iter()
            .find(|record| record.artifact.source_path == path)
    }

    pub fn by_wrapper(&self, path: &std::path::Path) -> Option<&VisualArtifactRecord> {
        self.artifacts
            .iter()
            .find(|record| record.artifact.wrapper_path.as_deref() == Some(path))
    }
}

fn classify_document_kind(
    doc: &Document,
    visual_artifacts: &VisualArtifactRegistry,
) -> DocumentKind {
    if visual_artifacts.by_wrapper(&doc.path).is_some() {
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
            let (to, relation) =
                if let Some(target_doc) = documents.resolve_link_target(&link.target) {
                    (
                        ProjectNodeRef::Document(target_doc.id.clone()),
                        ProjectRelation::LinksTo,
                    )
                } else {
                    (
                        ProjectNodeRef::ProjectPath(PathBuf::from(&link.target)),
                        ProjectRelation::UnresolvedReference,
                    )
                };
            edges.push(ProjectEdge {
                from: ProjectNodeRef::Document(doc.id.clone()),
                to,
                relation,
                source: EdgeSource::MarkdownWikilink {
                    path: doc.path.clone(),
                },
            });
        }
    }
    edges
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectRegistrySnapshot {
    pub schema: String,
    pub generated_by: String,
    pub documents: Vec<DocumentSnapshot>,
    pub visual_artifacts: Vec<VisualArtifactSnapshot>,
    pub evidence_sources: Vec<EvidenceSourceSnapshot>,
    pub edges: Vec<ProjectEdge>,
}

impl ProjectRegistrySnapshot {
    pub const SCHEMA: &'static str = "flynt-project-registry-snapshot/v1";

    pub fn from_registry(registry: &ProjectRegistry) -> Self {
        let mut documents = registry
            .documents
            .documents
            .iter()
            .map(DocumentSnapshot::from)
            .collect::<Vec<_>>();
        documents.sort_by(|a, b| a.path.cmp(&b.path));

        let mut visual_artifacts = registry
            .visual_artifacts
            .artifacts
            .iter()
            .map(VisualArtifactSnapshot::from)
            .collect::<Vec<_>>();
        visual_artifacts.sort_by(|a, b| a.id.cmp(&b.id));

        let mut evidence_sources = registry
            .evidence
            .sources
            .iter()
            .map(EvidenceSourceSnapshot::from)
            .collect::<Vec<_>>();
        evidence_sources.sort_by(|a, b| a.manifest_path.cmp(&b.manifest_path));

        let mut edges = registry.edges.clone();
        edges.sort_by_key(stable_edge_key);

        edges.retain(snapshot_edge_is_safe);

        Self {
            schema: Self::SCHEMA.to_string(),
            generated_by: "flynt".to_string(),
            documents,
            visual_artifacts,
            evidence_sources,
            edges,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSnapshot {
    pub id: DocumentId,
    pub path: PathBuf,
    pub title: String,
    pub kind: DocumentKind,
    pub aliases: Vec<String>,
    pub tags: Vec<String>,
}

impl From<&DocumentRecord> for DocumentSnapshot {
    fn from(record: &DocumentRecord) -> Self {
        Self {
            id: record.id.clone(),
            path: record.path.clone(),
            title: record.title.clone(),
            kind: record.kind,
            aliases: record.aliases.clone(),
            tags: record.tags.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualArtifactSnapshot {
    pub id: String,
    pub kind: VisualArtifactKind,
    pub title: String,
    pub source_path: PathBuf,
    pub wrapper_path: Option<PathBuf>,
    pub renders: Vec<RenderArtifact>,
    pub surfaces: Vec<ArtifactSurfaceCapability>,
}

impl From<&VisualArtifactRecord> for VisualArtifactSnapshot {
    fn from(record: &VisualArtifactRecord) -> Self {
        Self {
            id: record.id.0.clone(),
            kind: record.artifact.kind,
            title: record.artifact.title.clone(),
            source_path: record.artifact.source_path.clone(),
            wrapper_path: record.artifact.wrapper_path.clone(),
            renders: record.artifact.renders.clone(),
            surfaces: record.surfaces.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceSourceSnapshot {
    pub kind: EvidenceSourceKind,
    pub root_path: PathBuf,
    pub manifest_path: PathBuf,
    pub schema: Option<String>,
    pub streams: Vec<EvidenceStreamRecord>,
    pub warnings: Vec<String>,
}

impl From<&EvidenceSourceRecord> for EvidenceSourceSnapshot {
    fn from(record: &EvidenceSourceRecord) -> Self {
        let mut streams = record.streams.clone();
        streams.sort_by_key(|stream| (stream.kind, stream.path.clone()));
        Self {
            kind: record.kind,
            root_path: record.root_path.clone(),
            manifest_path: record.manifest_path.clone(),
            schema: record.schema.clone(),
            streams,
            warnings: record.warnings.clone(),
        }
    }
}

fn stable_edge_key(edge: &ProjectEdge) -> String {
    format!("{:?}|{:?}|{:?}", edge.from, edge.relation, edge.to)
}

fn snapshot_edge_is_safe(edge: &ProjectEdge) -> bool {
    snapshot_node_is_safe(&edge.from) && snapshot_node_is_safe(&edge.to)
}

fn snapshot_node_is_safe(node: &ProjectNodeRef) -> bool {
    match node {
        ProjectNodeRef::ProjectPath(path) => is_safe_project_relative_path(path),
        _ => true,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectRegistryDiagnostic {
    pub severity: RegistryDiagnosticSeverity,
    pub kind: RegistryDiagnosticKind,
    pub path: Option<PathBuf>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegistryDiagnosticSeverity {
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegistryDiagnosticKind {
    UnsafePath,
    MissingDocumentForWrapper,
    MissingVisualArtifactSource,
}

fn collect_registry_diagnostics(
    documents: &DocumentRegistry,
    visual_artifacts: &VisualArtifactRegistry,
) -> Vec<ProjectRegistryDiagnostic> {
    let mut diagnostics = Vec::new();
    for artifact in &visual_artifacts.artifacts {
        if !is_safe_project_relative_path(&artifact.artifact.source_path) {
            diagnostics.push(ProjectRegistryDiagnostic {
                severity: RegistryDiagnosticSeverity::Error,
                kind: RegistryDiagnosticKind::UnsafePath,
                path: Some(artifact.artifact.source_path.clone()),
                message: "visual artifact source path is not project-relative".into(),
            });
        }
        if let Some(wrapper_path) = &artifact.artifact.wrapper_path {
            if documents.by_path(wrapper_path).is_none() {
                diagnostics.push(ProjectRegistryDiagnostic {
                    severity: RegistryDiagnosticSeverity::Warning,
                    kind: RegistryDiagnosticKind::MissingDocumentForWrapper,
                    path: Some(wrapper_path.clone()),
                    message: "artifact wrapper exists on disk but is not indexed as a document"
                        .into(),
                });
            }
        }
    }
    diagnostics
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
    pub fn from_project_relative_source(
        kind: VisualArtifactKind,
        source_path: &std::path::Path,
    ) -> Self {
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
pub struct EvidenceRegistry {
    pub sources: Vec<EvidenceSourceRecord>,
}

impl EvidenceRegistry {
    pub fn discover(project_root: &std::path::Path) -> Self {
        let mut sources = Vec::new();
        if let Some(source) = EvidenceSourceRecord::discover_omegon_evidence_map(project_root) {
            sources.push(source);
        }
        Self { sources }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceSourceRecord {
    pub kind: EvidenceSourceKind,
    pub root_path: PathBuf,
    pub manifest_path: PathBuf,
    pub schema: Option<String>,
    pub streams: Vec<EvidenceStreamRecord>,
    pub warnings: Vec<String>,
}

impl EvidenceSourceRecord {
    pub fn discover_omegon_evidence_map(project_root: &std::path::Path) -> Option<Self> {
        let root_path = PathBuf::from(".omegon/evidence");
        let manifest_path = root_path.join("manifest.json");
        let manifest_abs = project_root.join(&manifest_path);
        if !manifest_abs.exists() {
            return None;
        }

        let mut warnings = Vec::new();
        let manifest = match std::fs::read_to_string(&manifest_abs)
            .ok()
            .and_then(|raw| serde_json::from_str::<OmegonEvidenceManifest>(&raw).ok())
        {
            Some(manifest) => manifest,
            None => {
                warnings.push("manifest.json could not be parsed".to_string());
                OmegonEvidenceManifest::default()
            }
        };

        let files = manifest.files.unwrap_or_default();
        let streams = [
            (
                EvidenceStreamKind::Records,
                files.records.unwrap_or_else(|| "records.jsonl".into()),
            ),
            (
                EvidenceStreamKind::Surfaces,
                files.surfaces.unwrap_or_else(|| "surfaces.jsonl".into()),
            ),
            (
                EvidenceStreamKind::Edges,
                files.edges.unwrap_or_else(|| "edges.jsonl".into()),
            ),
            (
                EvidenceStreamKind::Artifacts,
                files.artifacts.unwrap_or_else(|| "artifacts.jsonl".into()),
            ),
        ]
        .into_iter()
        .map(|(kind, rel)| {
            EvidenceStreamRecord::from_relative_path(project_root, &root_path, kind, rel)
        })
        .collect();

        Some(Self {
            kind: EvidenceSourceKind::OmegonEvidenceMap,
            root_path,
            manifest_path,
            schema: manifest.schema,
            streams,
            warnings,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceSourceKind {
    OmegonEvidenceMap,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceStreamRecord {
    pub kind: EvidenceStreamKind,
    pub path: PathBuf,
    pub status: EvidenceStreamStatus,
    pub stats: EvidenceStreamStats,
    pub ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct EvidenceStreamStats {
    pub line_count: usize,
    pub parsed_count: usize,
    pub malformed_count: usize,
}

impl EvidenceStreamRecord {
    fn from_relative_path(
        project_root: &std::path::Path,
        source_root: &std::path::Path,
        kind: EvidenceStreamKind,
        relative_path: String,
    ) -> Self {
        let path = source_root.join(relative_path);
        if !is_safe_project_relative_path(&path) {
            return Self {
                kind,
                path,
                status: EvidenceStreamStatus::InvalidPath,
                stats: EvidenceStreamStats::default(),
                ids: Vec::new(),
            };
        }
        let abs = project_root.join(&path);
        if !abs.exists() {
            return Self {
                kind,
                path,
                status: EvidenceStreamStatus::Missing,
                stats: EvidenceStreamStats::default(),
                ids: Vec::new(),
            };
        }
        let raw = std::fs::read_to_string(&abs).unwrap_or_default();
        let (stats, ids) = inspect_jsonl_stream(&raw);
        Self {
            kind,
            path,
            status: EvidenceStreamStatus::Present,
            stats,
            ids,
        }
    }
}

fn inspect_jsonl_stream(raw: &str) -> (EvidenceStreamStats, Vec<String>) {
    let mut stats = EvidenceStreamStats::default();
    let mut ids = Vec::new();
    for line in raw.lines().map(str::trim).filter(|line| !line.is_empty()) {
        stats.line_count += 1;
        match serde_json::from_str::<serde_json::Value>(line) {
            Ok(value) => {
                stats.parsed_count += 1;
                if let Some(id) = value.get("id").and_then(|id| id.as_str()) {
                    ids.push(id.to_string());
                }
            }
            Err(_) => stats.malformed_count += 1,
        }
    }
    (stats, ids)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStreamKind {
    Records,
    Surfaces,
    Edges,
    Artifacts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStreamStatus {
    Present,
    Missing,
    InvalidPath,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct OmegonEvidenceManifest {
    schema: Option<String>,
    files: Option<OmegonEvidenceManifestFiles>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct OmegonEvidenceManifestFiles {
    records: Option<String>,
    surfaces: Option<String>,
    edges: Option<String>,
    artifacts: Option<String>,
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
    UnresolvedReference,
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

fn normalize_slug(value: &str) -> String {
    value
        .trim()
        .trim_end_matches(".md")
        .chars()
        .flat_map(char::to_lowercase)
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

pub fn is_safe_project_relative_path(path: &std::path::Path) -> bool {
    use std::path::Component;
    !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
}

impl From<VisualArtifact> for VisualArtifactRecord {
    fn from(artifact: VisualArtifact) -> Self {
        let id =
            VisualArtifactId::from_project_relative_source(artifact.kind, &artifact.source_path);
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
    use crate::models::DocumentMeta;
    use crate::visual_artifacts::{RenderFormat, RenderStatus};

    #[cfg(test)]
    #[derive(Default)]
    struct TestStore {
        docs: Vec<Document>,
    }

    #[cfg(test)]
    impl TestStore {
        fn with_docs(docs: Vec<Document>) -> Self {
            Self { docs }
        }
    }

    #[cfg(test)]
    impl ProjectStore for TestStore {
        fn get_document(&self, id: &DocumentId) -> anyhow::Result<Option<Document>> {
            Ok(self.docs.iter().find(|doc| &doc.id == id).cloned())
        }

        fn get_document_by_path(&self, path: &std::path::Path) -> anyhow::Result<Option<Document>> {
            Ok(self.docs.iter().find(|doc| doc.path == path).cloned())
        }

        fn find_document_by_slug(&self, slug: &str) -> anyhow::Result<Option<DocumentMeta>> {
            Ok(self
                .docs
                .iter()
                .find(|doc| {
                    doc.title.eq_ignore_ascii_case(slug)
                        || doc.path.to_string_lossy().contains(slug)
                })
                .map(document_meta))
        }

        fn list_documents(&self) -> anyhow::Result<Vec<DocumentMeta>> {
            Ok(self.docs.iter().map(document_meta).collect())
        }

        fn list_documents_by_metadata(
            &self,
            _filter: &crate::store::DocumentMetadataFilter,
        ) -> anyhow::Result<Vec<DocumentMeta>> {
            Ok(Vec::new())
        }

        fn save_document(&self, _doc: &Document) -> anyhow::Result<()> {
            Ok(())
        }
        fn delete_document(&self, _id: &DocumentId) -> anyhow::Result<()> {
            Ok(())
        }
        fn search_documents(
            &self,
            _query: &str,
        ) -> anyhow::Result<Vec<crate::models::SearchResult>> {
            Ok(Vec::new())
        }
        fn get_backlinks(&self, id: &DocumentId) -> anyhow::Result<Vec<DocumentMeta>> {
            Ok(self
                .docs
                .iter()
                .filter(|doc| {
                    doc.outgoing_links.iter().any(|link| {
                        self.docs.iter().any(|target| {
                            &target.id == id && target.path.to_string_lossy().contains(&link.target)
                        })
                    })
                })
                .map(document_meta)
                .collect())
        }
        fn list_entities_by_kind(
            &self,
            _kind: &crate::datum::EntityKind,
        ) -> anyhow::Result<Vec<DocumentMeta>> {
            Ok(Vec::new())
        }
        fn get_task(
            &self,
            _id: &crate::models::TaskId,
        ) -> anyhow::Result<Option<crate::models::Task>> {
            Ok(None)
        }
        fn list_tasks(
            &self,
            _filter: &crate::store::TaskFilter,
        ) -> anyhow::Result<Vec<crate::models::Task>> {
            Ok(Vec::new())
        }
        fn save_task(&self, _task: &crate::models::Task) -> anyhow::Result<()> {
            Ok(())
        }
        fn delete_task(&self, _id: &crate::models::TaskId) -> anyhow::Result<()> {
            Ok(())
        }
        fn update_task(
            &self,
            _id: &crate::models::TaskId,
            _patch: &flynt_models::TaskPatch,
        ) -> anyhow::Result<bool> {
            Ok(false)
        }
        fn get_board(
            &self,
            _id: &crate::models::BoardId,
        ) -> anyhow::Result<Option<crate::models::Board>> {
            Ok(None)
        }
        fn list_boards(&self) -> anyhow::Result<Vec<crate::models::Board>> {
            Ok(Vec::new())
        }
        fn save_board(&self, _board: &crate::models::Board) -> anyhow::Result<()> {
            Ok(())
        }
        fn delete_board(&self, _id: &crate::models::BoardId) -> anyhow::Result<()> {
            Ok(())
        }
        fn get_engagement(
            &self,
            _id: &flynt_models::engagement::EngagementId,
        ) -> anyhow::Result<Option<flynt_models::engagement::Engagement>> {
            Ok(None)
        }
        fn list_engagements(&self) -> anyhow::Result<Vec<flynt_models::engagement::Engagement>> {
            Ok(Vec::new())
        }
        fn save_engagement(
            &self,
            _engagement: &flynt_models::engagement::Engagement,
        ) -> anyhow::Result<()> {
            Ok(())
        }
        fn delete_engagement(
            &self,
            _id: &flynt_models::engagement::EngagementId,
        ) -> anyhow::Result<bool> {
            Ok(false)
        }
    }

    #[cfg(test)]
    fn document_meta(doc: &Document) -> crate::models::DocumentMeta {
        crate::models::DocumentMeta {
            id: doc.id.clone(),
            path: doc.path.clone(),
            title: doc.title.clone(),
            tags: doc.frontmatter.tags.clone(),
            metadata: Default::default(),
            entity_kind: None,
            updated_at: doc.updated_at,
        }
    }

    #[cfg(test)]
    fn test_doc(
        path: &str,
        title: &str,
        content: &str,
        frontmatter: Frontmatter,
        outgoing_links: Vec<WikiLink>,
    ) -> Document {
        let now = chrono::Utc::now();
        Document {
            id: DocumentId::new(),
            path: PathBuf::from(path),
            title: title.to_string(),
            content: content.to_string(),
            frontmatter,
            outgoing_links,
            created_at: now,
            updated_at: now,
            entity: None,
        }
    }

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
        assert!(
            record
                .surfaces
                .contains(&ArtifactSurfaceCapability::Preview)
        );
        assert!(record.surfaces.contains(&ArtifactSurfaceCapability::Edit));
        assert!(
            record
                .surfaces
                .contains(&ArtifactSurfaceCapability::Render(RenderFormat::Svg))
        );
    }

    #[test]
    fn evidence_registry_discovers_omegon_manifest_and_stream_counts() {
        let tmp = tempfile::TempDir::new().unwrap();
        let evidence_dir = tmp.path().join(".omegon/evidence");
        std::fs::create_dir_all(&evidence_dir).unwrap();
        std::fs::write(
            evidence_dir.join("manifest.json"),
            r#"{"schema":"omegon-evidence-manifest/v1","files":{"records":"records.jsonl","surfaces":"surfaces.jsonl","edges":"edges.jsonl","artifacts":"artifacts.jsonl"}}"#,
        )
        .unwrap();
        std::fs::write(
            evidence_dir.join("records.jsonl"),
            "{\"id\":\"a\"}\n{\"id\":\"b\"}\n",
        )
        .unwrap();
        std::fs::write(evidence_dir.join("surfaces.jsonl"), "{\"id\":\"s\"}\n").unwrap();

        let registry = EvidenceRegistry::discover(tmp.path());
        assert_eq!(registry.sources.len(), 1);
        let source = &registry.sources[0];
        assert_eq!(source.kind, EvidenceSourceKind::OmegonEvidenceMap);
        assert_eq!(
            source.schema.as_deref(),
            Some("omegon-evidence-manifest/v1")
        );
        let records = source
            .streams
            .iter()
            .find(|stream| stream.kind == EvidenceStreamKind::Records)
            .unwrap();
        assert_eq!(records.status, EvidenceStreamStatus::Present);
        assert_eq!(records.stats.line_count, 2);
        assert_eq!(records.stats.parsed_count, 2);
        assert_eq!(records.stats.malformed_count, 0);
        assert_eq!(records.ids, vec!["a", "b"]);
        let surfaces = source
            .streams
            .iter()
            .find(|stream| stream.kind == EvidenceStreamKind::Surfaces)
            .unwrap();
        assert_eq!(surfaces.ids, vec!["s"]);
        let edges = source
            .streams
            .iter()
            .find(|stream| stream.kind == EvidenceStreamKind::Edges)
            .unwrap();
        assert_eq!(edges.status, EvidenceStreamStatus::Missing);
    }

    #[test]
    fn evidence_stream_rejects_paths_that_escape_project_root() {
        let tmp = tempfile::TempDir::new().unwrap();
        let stream = EvidenceStreamRecord::from_relative_path(
            tmp.path(),
            std::path::Path::new(".omegon/evidence"),
            EvidenceStreamKind::Records,
            "../records.jsonl".to_string(),
        );
        assert_eq!(stream.status, EvidenceStreamStatus::InvalidPath);
        assert_eq!(stream.stats, EvidenceStreamStats::default());
        assert!(stream.ids.is_empty());
    }

    #[test]
    fn evidence_registry_absent_without_omegon_manifest() {
        let tmp = tempfile::TempDir::new().unwrap();
        let registry = EvidenceRegistry::discover(tmp.path());
        assert!(registry.sources.is_empty());
    }

    #[test]
    fn startup_registry_builds_wrappers_renders_evidence_and_snapshot_without_absolute_paths() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("drawings")).unwrap();
        std::fs::write(tmp.path().join("drawings/map.excalidraw"), "{}").unwrap();
        std::fs::write(tmp.path().join("drawings/map.md"), "![[map.excalidraw]]").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        std::fs::write(tmp.path().join("drawings/map.svg"), "<svg/>").unwrap();

        let mut wrapper_frontmatter = Frontmatter::default();
        wrapper_frontmatter.tags = vec!["drawing".into()];
        let wrapper = test_doc(
            "drawings/map.md",
            "Map",
            "![[map.excalidraw]]",
            wrapper_frontmatter,
            vec![WikiLink {
                target: "map.excalidraw".into(),
                display: None,
                anchor: None,
            }],
        );
        let note = test_doc(
            "notes/architecture.md",
            "Architecture",
            "See [[drawings/map]]",
            Frontmatter::default(),
            vec![WikiLink {
                target: "drawings/map".into(),
                display: None,
                anchor: None,
            }],
        );
        let store = TestStore::with_docs(vec![note, wrapper]);

        let registry = ProjectRegistry::discover(tmp.path().to_path_buf(), &store).unwrap();
        assert_eq!(registry.documents.documents.len(), 2);
        assert_eq!(registry.visual_artifacts.artifacts.len(), 1);
        assert_eq!(
            registry
                .documents
                .by_path(std::path::Path::new("drawings/map.md"))
                .unwrap()
                .kind,
            DocumentKind::ArtifactWrapper
        );
        assert!(
            registry
                .edges
                .iter()
                .any(|edge| edge.relation == ProjectRelation::Wraps)
        );
        assert!(
            registry
                .edges
                .iter()
                .any(|edge| edge.relation == ProjectRelation::RendersTo)
        );
        assert!(
            registry
                .edges
                .iter()
                .any(|edge| edge.relation == ProjectRelation::LinksTo)
        );

        let snapshot = ProjectRegistrySnapshot::from_registry(&registry);
        let json = serde_json::to_string_pretty(&snapshot).unwrap();
        assert!(!json.contains(tmp.path().to_string_lossy().as_ref()));
        assert!(json.contains("drawings/map.excalidraw"));
    }

    #[test]
    fn unresolved_wikilinks_remain_visible_as_unresolved_edges() {
        let tmp = tempfile::TempDir::new().unwrap();
        let note = test_doc(
            "notes/architecture.md",
            "Architecture",
            "See [[missing thing]]",
            Frontmatter::default(),
            vec![WikiLink {
                target: "missing thing".into(),
                display: None,
                anchor: None,
            }],
        );
        let store = TestStore::with_docs(vec![note]);
        let registry = ProjectRegistry::discover(tmp.path().to_path_buf(), &store).unwrap();
        assert!(registry.edges.iter().any(|edge| {
            edge.relation == ProjectRelation::UnresolvedReference
                && matches!(&edge.to, ProjectNodeRef::ProjectPath(path) if path == std::path::Path::new("missing thing"))
        }));
    }

    #[test]
    fn snapshot_omits_runtime_scope_and_sorts_records() {
        let registry = ProjectRegistry {
            scope: ProjectScope {
                project_root: PathBuf::from("/tmp/not-portable"),
                project_id: Some("local".into()),
                sync_identity: None,
            },
            documents: DocumentRegistry::default(),
            visual_artifacts: VisualArtifactRegistry {
                artifacts: vec![
                    VisualArtifactRecord::from(VisualArtifact {
                        kind: VisualArtifactKind::D2Diagram,
                        title: "b".into(),
                        source_path: PathBuf::from("diagrams/b.d2"),
                        wrapper_path: None,
                        renders: Vec::new(),
                    }),
                    VisualArtifactRecord::from(VisualArtifact {
                        kind: VisualArtifactKind::D2Diagram,
                        title: "a".into(),
                        source_path: PathBuf::from("diagrams/a.d2"),
                        wrapper_path: None,
                        renders: Vec::new(),
                    }),
                ],
            },
            external_refs: ExternalRefRegistry::default(),
            evidence: EvidenceRegistry::default(),
            diagnostics: Vec::new(),
            raw_assets: RawAssetRegistry::default(),
            task_refs: TaskRegistryView::default(),
            spec_refs: SpecRegistryView::default(),
            edges: Vec::new(),
        };

        let snapshot = ProjectRegistrySnapshot::from_registry(&registry);
        assert_eq!(snapshot.schema, ProjectRegistrySnapshot::SCHEMA);
        assert_eq!(snapshot.visual_artifacts[0].id, "d2:diagrams/a.d2");
        assert_eq!(snapshot.visual_artifacts[1].id, "d2:diagrams/b.d2");
        let json = serde_json::to_string_pretty(&snapshot).unwrap();
        assert!(!json.contains("/tmp/not-portable"));
        assert!(!json.contains("project_root"));
    }

    #[test]
    fn project_registry_serializes_as_plain_json() {
        let registry = ProjectRegistry::default();
        let json = serde_json::to_string_pretty(&registry).unwrap();
        assert!(json.contains("documents"));
        assert!(json.contains("visual_artifacts"));
        assert!(json.contains("evidence"));
        assert!(json.contains("edges"));
    }
}
