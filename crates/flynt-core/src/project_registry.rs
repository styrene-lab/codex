use crate::external_ref::parse_ref;
use crate::models::{
    Board, Document, DocumentId, Frontmatter, PublicationConfig, Task, TaskStatus, WikiLink,
};
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
        let raw_assets = RawAssetRegistry::discover(&project_root, &visual_artifacts);
        let task_refs = TaskRegistryView::from_store(store)?;
        let external_refs = ExternalRefRegistry::discover(&documents, &task_refs);
        let spec_refs = SpecRegistryView::discover(&project_root);
        let edges = build_project_edges(
            &documents,
            &visual_artifacts,
            &external_refs,
            &task_refs,
            &spec_refs,
        );
        Ok(Self {
            scope,
            documents,
            visual_artifacts,
            external_refs,
            evidence: EvidenceRegistry::discover(&project_root),
            diagnostics,
            raw_assets,
            task_refs,
            spec_refs,
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
            raw_content: doc.content,
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
    external_refs: &ExternalRefRegistry,
    task_refs: &TaskRegistryView,
    spec_refs: &SpecRegistryView,
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

    for external_ref in &external_refs.refs {
        for source_doc in &external_ref.document_sources {
            edges.push(ProjectEdge {
                from: ProjectNodeRef::Document(source_doc.clone()),
                to: ProjectNodeRef::ExternalRef(external_ref.id.clone()),
                relation: ProjectRelation::References,
                source: EdgeSource::Generated,
            });
        }
    }

    for spec in &spec_refs.changes {
        for path in spec.project_paths() {
            edges.push(ProjectEdge {
                from: ProjectNodeRef::Spec(spec.name.clone()),
                to: ProjectNodeRef::ProjectPath(path),
                relation: ProjectRelation::References,
                source: EdgeSource::SpecLifecycle,
            });
        }
    }

    for task in &task_refs.tasks {
        if let Some(change) = &task.openspec_change {
            edges.push(ProjectEdge {
                from: ProjectNodeRef::Task(task.id.clone()),
                to: ProjectNodeRef::Spec(change.clone()),
                relation: ProjectRelation::Implements,
                source: EdgeSource::SpecLifecycle,
            });
        }
        edges.push(ProjectEdge {
            from: ProjectNodeRef::Task(task.id.clone()),
            to: ProjectNodeRef::Board(task.board_id.clone()),
            relation: ProjectRelation::BelongsTo,
            source: EdgeSource::TaskMembership,
        });
        for document_id in &task.document_refs {
            edges.push(ProjectEdge {
                from: ProjectNodeRef::Task(task.id.clone()),
                to: ProjectNodeRef::Document(document_id.clone()),
                relation: ProjectRelation::References,
                source: EdgeSource::TaskMembership,
            });
        }
        for external_ref in &task.external_refs {
            let id = ExternalRefId::from_uri(external_ref);
            edges.push(ProjectEdge {
                from: ProjectNodeRef::Task(task.id.clone()),
                to: ProjectNodeRef::ExternalRef(id),
                relation: ProjectRelation::References,
                source: EdgeSource::TaskMembership,
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
    pub raw_assets: Vec<RawAssetRecord>,
    pub external_refs: Vec<ExternalRefRecord>,
    pub tasks: Vec<TaskRecord>,
    pub boards: Vec<BoardRecord>,
    pub specs: Vec<SpecChangeRecord>,
    pub diagnostics: Vec<ProjectRegistryDiagnostic>,
    pub edges: Vec<ProjectEdge>,
}

impl ProjectRegistrySnapshot {
    pub const SCHEMA: &'static str = "flynt-project-registry-snapshot/v1";

    pub fn validate(&self) -> Vec<ProjectRegistryDiagnostic> {
        let mut diagnostics = validate_snapshot_paths(self);
        diagnostics.extend(validate_snapshot_graph(self));
        diagnostics
    }

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

        let mut raw_assets = registry.raw_assets.assets.clone();
        raw_assets.sort_by(|a, b| a.id.cmp(&b.id));

        let mut external_refs = registry.external_refs.refs.clone();
        external_refs.sort_by(|a, b| a.id.cmp(&b.id));

        let mut tasks = registry.task_refs.tasks.clone();
        tasks.sort_by(|a, b| a.id.cmp(&b.id));

        let mut boards = registry.task_refs.boards.clone();
        boards.sort_by(|a, b| a.id.cmp(&b.id));

        let mut specs = registry.spec_refs.changes.clone();
        specs.sort_by(|a, b| a.name.cmp(&b.name));

        let mut diagnostics = registry.diagnostics.clone();
        diagnostics.sort_by_key(stable_diagnostic_key);

        let mut edges = registry.edges.clone();
        edges.sort_by_key(stable_edge_key);

        edges.retain(snapshot_edge_is_safe);

        Self {
            schema: Self::SCHEMA.to_string(),
            generated_by: "flynt".to_string(),
            documents,
            visual_artifacts,
            evidence_sources,
            raw_assets,
            external_refs,
            tasks,
            boards,
            specs,
            diagnostics,
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

fn stable_diagnostic_key(diagnostic: &ProjectRegistryDiagnostic) -> String {
    format!(
        "{:?}|{:?}|{:?}|{}",
        diagnostic.severity, diagnostic.kind, diagnostic.path, diagnostic.message
    )
}

fn snapshot_edge_is_safe(edge: &ProjectEdge) -> bool {
    snapshot_node_is_safe(&edge.from) && snapshot_node_is_safe(&edge.to)
}

fn validate_snapshot_paths(snapshot: &ProjectRegistrySnapshot) -> Vec<ProjectRegistryDiagnostic> {
    let mut diagnostics = Vec::new();
    for document in &snapshot.documents {
        push_unsafe_path_diagnostic(
            &mut diagnostics,
            &document.path,
            "snapshot document path is unsafe",
        );
    }
    for artifact in &snapshot.visual_artifacts {
        push_unsafe_path_diagnostic(
            &mut diagnostics,
            &artifact.source_path,
            "snapshot visual artifact source path is unsafe",
        );
        if let Some(wrapper_path) = &artifact.wrapper_path {
            push_unsafe_path_diagnostic(
                &mut diagnostics,
                wrapper_path,
                "snapshot visual artifact wrapper path is unsafe",
            );
        }
        for render in &artifact.renders {
            push_unsafe_path_diagnostic(
                &mut diagnostics,
                &render.path,
                "snapshot visual artifact render path is unsafe",
            );
        }
    }
    for evidence in &snapshot.evidence_sources {
        push_unsafe_path_diagnostic(
            &mut diagnostics,
            &evidence.root_path,
            "snapshot evidence root path is unsafe",
        );
        push_unsafe_path_diagnostic(
            &mut diagnostics,
            &evidence.manifest_path,
            "snapshot evidence manifest path is unsafe",
        );
        for stream in &evidence.streams {
            push_unsafe_path_diagnostic(
                &mut diagnostics,
                &stream.path,
                "snapshot evidence stream path is unsafe",
            );
        }
    }
    for asset in &snapshot.raw_assets {
        push_unsafe_path_diagnostic(
            &mut diagnostics,
            &asset.path,
            "snapshot raw asset path is unsafe",
        );
    }
    for spec in &snapshot.specs {
        push_unsafe_path_diagnostic(
            &mut diagnostics,
            &spec.path,
            "snapshot spec change path is unsafe",
        );
        if let Some(path) = &spec.proposal_path {
            push_unsafe_path_diagnostic(
                &mut diagnostics,
                path,
                "snapshot spec proposal path is unsafe",
            );
        }
        if let Some(path) = &spec.design_path {
            push_unsafe_path_diagnostic(
                &mut diagnostics,
                path,
                "snapshot spec design path is unsafe",
            );
        }
        if let Some(path) = &spec.tasks_path {
            push_unsafe_path_diagnostic(
                &mut diagnostics,
                path,
                "snapshot spec tasks path is unsafe",
            );
        }
        for path in &spec.spec_paths {
            push_unsafe_path_diagnostic(
                &mut diagnostics,
                path,
                "snapshot spec file path is unsafe",
            );
        }
    }
    for diagnostic in &snapshot.diagnostics {
        if let Some(path) = &diagnostic.path {
            push_unsafe_path_diagnostic(
                &mut diagnostics,
                path,
                "snapshot diagnostic path is unsafe",
            );
        }
    }
    diagnostics
}

fn push_unsafe_path_diagnostic(
    diagnostics: &mut Vec<ProjectRegistryDiagnostic>,
    path: &std::path::Path,
    message: impl Into<String>,
) {
    if !is_safe_project_relative_path(path) {
        diagnostics.push(ProjectRegistryDiagnostic {
            severity: RegistryDiagnosticSeverity::Error,
            kind: RegistryDiagnosticKind::UnsafePath,
            path: Some(path.to_path_buf()),
            message: message.into(),
        });
    }
}

fn snapshot_node_is_safe(node: &ProjectNodeRef) -> bool {
    match node {
        ProjectNodeRef::ProjectPath(path) => is_safe_project_relative_path(path),
        _ => true,
    }
}

fn validate_snapshot_graph(snapshot: &ProjectRegistrySnapshot) -> Vec<ProjectRegistryDiagnostic> {
    let document_ids = snapshot
        .documents
        .iter()
        .map(|doc| doc.id.clone())
        .collect::<std::collections::HashSet<_>>();
    let artifact_ids = snapshot
        .visual_artifacts
        .iter()
        .map(|artifact| VisualArtifactId(artifact.id.clone()))
        .collect::<std::collections::HashSet<_>>();
    let raw_asset_ids = snapshot
        .raw_assets
        .iter()
        .map(|asset| asset.id.clone())
        .collect::<std::collections::HashSet<_>>();
    let external_ref_ids = snapshot
        .external_refs
        .iter()
        .map(|external_ref| external_ref.id.clone())
        .collect::<std::collections::HashSet<_>>();
    let task_ids = snapshot
        .tasks
        .iter()
        .map(|task| task.id.clone())
        .collect::<std::collections::HashSet<_>>();
    let board_ids = snapshot
        .boards
        .iter()
        .map(|board| board.id.clone())
        .collect::<std::collections::HashSet<_>>();
    let spec_ids = snapshot
        .specs
        .iter()
        .map(|spec| spec.name.clone())
        .collect::<std::collections::HashSet<_>>();

    let mut diagnostics = Vec::new();
    for edge in &snapshot.edges {
        validate_snapshot_node(
            edge,
            &edge.from,
            &document_ids,
            &artifact_ids,
            &raw_asset_ids,
            &external_ref_ids,
            &task_ids,
            &board_ids,
            &spec_ids,
            &mut diagnostics,
        );
        validate_snapshot_node(
            edge,
            &edge.to,
            &document_ids,
            &artifact_ids,
            &raw_asset_ids,
            &external_ref_ids,
            &task_ids,
            &board_ids,
            &spec_ids,
            &mut diagnostics,
        );
    }
    diagnostics
}

#[allow(clippy::too_many_arguments)]
fn validate_snapshot_node(
    edge: &ProjectEdge,
    node: &ProjectNodeRef,
    document_ids: &std::collections::HashSet<DocumentId>,
    artifact_ids: &std::collections::HashSet<VisualArtifactId>,
    raw_asset_ids: &std::collections::HashSet<RawAssetId>,
    external_ref_ids: &std::collections::HashSet<ExternalRefId>,
    task_ids: &std::collections::HashSet<String>,
    board_ids: &std::collections::HashSet<String>,
    spec_ids: &std::collections::HashSet<String>,
    diagnostics: &mut Vec<ProjectRegistryDiagnostic>,
) {
    let missing = match node {
        ProjectNodeRef::Document(id) => !document_ids.contains(id),
        ProjectNodeRef::VisualArtifact(id) => !artifact_ids.contains(id),
        ProjectNodeRef::RawAsset(id) => !raw_asset_ids.contains(id),
        ProjectNodeRef::ExternalRef(id) => !external_ref_ids.contains(id),
        ProjectNodeRef::Task(id) => !task_ids.contains(id),
        ProjectNodeRef::Board(id) => !board_ids.contains(id),
        ProjectNodeRef::Spec(id) => !spec_ids.contains(id),
        ProjectNodeRef::ProjectPath(path) => {
            if !is_safe_project_relative_path(path) {
                diagnostics.push(ProjectRegistryDiagnostic {
                    severity: RegistryDiagnosticSeverity::Error,
                    kind: RegistryDiagnosticKind::UnsafePath,
                    path: Some(path.clone()),
                    message: "snapshot edge contains unsafe project path".into(),
                });
            }
            false
        }
    };
    if missing {
        diagnostics.push(ProjectRegistryDiagnostic {
            severity: RegistryDiagnosticSeverity::Error,
            kind: RegistryDiagnosticKind::DanglingGraphEndpoint,
            path: None,
            message: format!("snapshot edge has missing endpoint {node:?} in edge {edge:?}"),
        });
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
    DanglingGraphEndpoint,
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
    #[serde(skip)]
    pub raw_content: String,
}

impl DocumentRecord {
    fn content(&self) -> &str {
        &self.raw_content
    }
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

impl RawAssetRegistry {
    pub fn discover(
        project_root: &std::path::Path,
        visual_artifacts: &VisualArtifactRegistry,
    ) -> Self {
        let mut assets = Vec::new();
        collect_raw_assets(project_root, project_root, visual_artifacts, &mut assets);
        assets.sort_by(|a, b| a.id.0.cmp(&b.id.0));
        assets.dedup_by(|a, b| a.id == b.id);
        Self { assets }
    }
}

fn collect_raw_assets(
    project_root: &std::path::Path,
    dir: &std::path::Path,
    visual_artifacts: &VisualArtifactRegistry,
    assets: &mut Vec<RawAssetRecord>,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            if should_skip_registry_dir(project_root, &path) {
                continue;
            }
            collect_raw_assets(project_root, &path, visual_artifacts, assets);
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let Ok(rel) = path.strip_prefix(project_root) else {
            continue;
        };
        let rel = rel.to_path_buf();
        if !is_safe_project_relative_path(&rel) || is_visual_artifact_source(&rel, visual_artifacts)
        {
            continue;
        }
        let Some((media_type, role)) = classify_raw_asset(&rel, visual_artifacts) else {
            continue;
        };
        assets.push(RawAssetRecord {
            id: RawAssetId::from_project_relative_path(&rel),
            path: rel,
            media_type: media_type.to_string(),
            role,
        });
    }
}

fn should_skip_registry_dir(project_root: &std::path::Path, path: &std::path::Path) -> bool {
    let Ok(rel) = path.strip_prefix(project_root) else {
        return true;
    };
    matches!(
        rel.components()
            .next()
            .and_then(|component| component.as_os_str().to_str()),
        Some(".git") | Some("target") | Some("node_modules") | Some(".venv")
    )
}

fn is_visual_artifact_source(
    path: &std::path::Path,
    visual_artifacts: &VisualArtifactRegistry,
) -> bool {
    visual_artifacts.by_source(path).is_some() || visual_artifacts.by_wrapper(path).is_some()
}

fn classify_raw_asset(
    path: &std::path::Path,
    visual_artifacts: &VisualArtifactRegistry,
) -> Option<(&'static str, RawAssetRole)> {
    if visual_artifacts
        .artifacts
        .iter()
        .flat_map(|artifact| artifact.artifact.renders.iter())
        .any(|render| render.path == path)
    {
        return Some((media_type_for_path(path)?, RawAssetRole::RenderSidecar));
    }

    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "svg" => Some(("image/svg+xml", RawAssetRole::Image)),
        "png" => Some(("image/png", RawAssetRole::Image)),
        "jpg" | "jpeg" => Some(("image/jpeg", RawAssetRole::Image)),
        "gif" => Some(("image/gif", RawAssetRole::Image)),
        "webp" => Some(("image/webp", RawAssetRole::Image)),
        "css" => Some(("text/css", RawAssetRole::Stylesheet)),
        "html" | "htm" => Some(("text/html", RawAssetRole::Markup)),
        "js" | "mjs" => Some(("text/javascript", RawAssetRole::Script)),
        "json" => Some(("application/json", RawAssetRole::Data)),
        "toml" => Some(("application/toml", RawAssetRole::Data)),
        "yaml" | "yml" => Some(("application/yaml", RawAssetRole::Data)),
        "csv" => Some(("text/csv", RawAssetRole::Data)),
        "pdf" | "docx" | "xlsx" | "pptx" => {
            Some(("application/octet-stream", RawAssetRole::ImportExportOnly))
        }
        _ => None,
    }
}

fn media_type_for_path(path: &std::path::Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "svg" => Some("image/svg+xml"),
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "html" | "htm" => Some("text/html"),
        _ => Some("application/octet-stream"),
    }
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
    Markup,
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
    pub document_sources: Vec<DocumentId>,
    pub task_sources: Vec<String>,
}

impl ExternalRefRegistry {
    pub fn discover(documents: &DocumentRegistry, task_refs: &TaskRegistryView) -> Self {
        let mut refs: Vec<ExternalRefRecord> = Vec::new();
        for doc in &documents.documents {
            for url in extract_urls(&doc.content()) {
                upsert_external_ref(&mut refs, &url, Some(&doc.id), None);
            }
        }
        for task in &task_refs.tasks {
            for external_ref in &task.external_refs {
                upsert_external_ref(&mut refs, external_ref, None, Some(&task.id));
            }
        }
        refs.sort_by(|a, b| a.id.0.cmp(&b.id.0));
        Self { refs }
    }
}

fn upsert_external_ref(
    refs: &mut Vec<ExternalRefRecord>,
    url: &str,
    document_source: Option<&DocumentId>,
    task_source: Option<&str>,
) {
    let parsed = parse_ref(url);
    let id = ExternalRefId::from_uri(&parsed.url);
    if let Some(existing) = refs.iter_mut().find(|record| record.id == id) {
        if let Some(document_source) = document_source {
            if !existing.document_sources.contains(document_source) {
                existing.document_sources.push(document_source.clone());
            }
        }
        if let Some(task_source) = task_source {
            let task_source = task_source.to_string();
            if !existing.task_sources.contains(&task_source) {
                existing.task_sources.push(task_source);
            }
        }
        return;
    }
    refs.push(ExternalRefRecord {
        id,
        target: ExternalTarget::Uri(parsed.url),
        label: Some(parsed.label),
        provider: Some(parsed.provider.name().to_string()),
        document_sources: document_source.into_iter().cloned().collect(),
        task_sources: task_source.into_iter().map(str::to_string).collect(),
    });
}

fn extract_urls(content: &str) -> Vec<String> {
    content
        .split_whitespace()
        .filter_map(|token| {
            let cleaned = token
                .trim_matches(|ch: char| {
                    matches!(
                        ch,
                        '<' | '>' | '(' | ')' | '[' | ']' | '"' | '\'' | ',' | ';'
                    )
                })
                .trim_end_matches(|ch: char| matches!(ch, '.' | ',' | ';' | ')' | ']'));
            if cleaned.starts_with("https://") || cleaned.starts_with("http://") {
                Some(cleaned.to_string())
            } else {
                None
            }
        })
        .collect()
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
    pub tasks: Vec<TaskRecord>,
    pub boards: Vec<BoardRecord>,
}

impl TaskRegistryView {
    pub fn from_store(store: &dyn ProjectStore) -> anyhow::Result<Self> {
        let mut tasks = store
            .list_tasks(&crate::store::TaskFilter::default())?
            .into_iter()
            .map(TaskRecord::from)
            .collect::<Vec<_>>();
        tasks.sort_by(|a, b| a.id.cmp(&b.id));

        let mut boards = store
            .list_boards()?
            .into_iter()
            .map(BoardRecord::from)
            .collect::<Vec<_>>();
        boards.sort_by(|a, b| a.id.cmp(&b.id));

        Ok(Self { tasks, boards })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskRecord {
    pub id: String,
    pub title: String,
    pub status: TaskStatus,
    pub board_id: String,
    pub column: String,
    pub document_refs: Vec<DocumentId>,
    pub external_refs: Vec<String>,
    pub tags: Vec<String>,
    pub openspec_change: Option<String>,
    pub evidence: TaskEvidenceSummary,
}

impl From<Task> for TaskRecord {
    fn from(task: Task) -> Self {
        Self {
            id: task.id.0.to_string(),
            title: task.title,
            status: task.status,
            board_id: task.board_id.0.to_string(),
            column: task.column,
            document_refs: task.document_refs,
            external_refs: task.external_refs,
            tags: task.tags,
            openspec_change: task.openspec_change,
            evidence: TaskEvidenceSummary::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoardRecord {
    pub id: String,
    pub name: String,
    pub columns: Vec<String>,
}

impl From<Board> for BoardRecord {
    fn from(board: Board) -> Self {
        Self {
            id: board.id.0.to_string(),
            name: board.name,
            columns: board
                .columns
                .into_iter()
                .map(|column| column.name)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TaskEvidenceSummary {
    pub state: TaskEvidenceState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskEvidenceState {
    #[default]
    NotRequired,
    Missing,
    Partial,
    Satisfied,
    Stale,
    Conflicting,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SpecRegistryView {
    pub changes: Vec<SpecChangeRecord>,
}

impl SpecRegistryView {
    pub fn discover(project_root: &std::path::Path) -> Self {
        let changes_root = project_root.join("openspec/changes");
        let mut changes = Vec::new();
        let Ok(entries) = std::fs::read_dir(&changes_root) else {
            return Self { changes };
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if !file_type.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            let path = PathBuf::from("openspec/changes").join(&name);
            if !is_safe_project_relative_path(&path) {
                continue;
            }
            changes.push(SpecChangeRecord::from_change_dir(project_root, name, path));
        }
        changes.sort_by(|a, b| a.name.cmp(&b.name));
        Self { changes }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecChangeRecord {
    pub name: String,
    pub path: PathBuf,
    pub proposal_path: Option<PathBuf>,
    pub design_path: Option<PathBuf>,
    pub tasks_path: Option<PathBuf>,
    pub spec_paths: Vec<PathBuf>,
}

impl SpecChangeRecord {
    fn from_change_dir(project_root: &std::path::Path, name: String, path: PathBuf) -> Self {
        let proposal_path = existing_relative_file(project_root, path.join("proposal.md"));
        let design_path = existing_relative_file(project_root, path.join("design.md"));
        let tasks_path = existing_relative_file(project_root, path.join("tasks.md"));
        let mut spec_paths = Vec::new();
        collect_spec_files(project_root, &path.join("specs"), &mut spec_paths);
        spec_paths.sort();
        Self {
            name,
            path,
            proposal_path,
            design_path,
            tasks_path,
            spec_paths,
        }
    }

    fn project_paths(&self) -> Vec<PathBuf> {
        let mut paths = vec![self.path.clone()];
        paths.extend(self.proposal_path.clone());
        paths.extend(self.design_path.clone());
        paths.extend(self.tasks_path.clone());
        paths.extend(self.spec_paths.clone());
        paths
    }
}

fn existing_relative_file(project_root: &std::path::Path, relative: PathBuf) -> Option<PathBuf> {
    if !is_safe_project_relative_path(&relative) {
        return None;
    }
    project_root.join(&relative).is_file().then_some(relative)
}

fn collect_spec_files(
    project_root: &std::path::Path,
    relative_dir: &std::path::Path,
    out: &mut Vec<PathBuf>,
) {
    if !is_safe_project_relative_path(relative_dir) {
        return;
    }
    let Ok(entries) = std::fs::read_dir(project_root.join(relative_dir)) else {
        return;
    };
    for entry in entries.flatten() {
        let path = relative_dir.join(entry.file_name());
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            collect_spec_files(project_root, &path, out);
        } else if file_type.is_file()
            && path.extension().and_then(|ext| ext.to_str()) == Some("md")
            && is_safe_project_relative_path(&path)
        {
            out.push(path);
        }
    }
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
    Board(String),
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
        boards: Vec<Board>,
        tasks: Vec<Task>,
    }

    #[cfg(test)]
    impl TestStore {
        fn with_docs(docs: Vec<Document>) -> Self {
            Self {
                docs,
                boards: Vec::new(),
                tasks: Vec::new(),
            }
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
            Ok(self.tasks.clone())
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
            Ok(self.boards.clone())
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
        std::fs::create_dir_all(tmp.path().join("drawings/rendered")).unwrap();
        std::fs::write(tmp.path().join("drawings/rendered/map.svg"), "<svg/>").unwrap();

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
    fn spec_registry_discovers_openspec_changes_and_task_edges() {
        let tmp = tempfile::TempDir::new().unwrap();
        let change = tmp.path().join("openspec/changes/add-registry/specs/core");
        std::fs::create_dir_all(&change).unwrap();
        std::fs::write(
            tmp.path().join("openspec/changes/add-registry/proposal.md"),
            "proposal",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("openspec/changes/add-registry/design.md"),
            "design",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("openspec/changes/add-registry/tasks.md"),
            "tasks",
        )
        .unwrap();
        std::fs::write(change.join("registry.md"), "spec").unwrap();

        let board = Board::minimalist("Spec Work");
        let mut task = Task::new(board.id.clone(), "Active", "Implement registry");
        task.openspec_change = Some("add-registry".into());
        let store = TestStore {
            docs: Vec::new(),
            boards: vec![board],
            tasks: vec![task],
        };

        let registry = ProjectRegistry::discover(tmp.path().to_path_buf(), &store).unwrap();
        assert_eq!(registry.spec_refs.changes.len(), 1);
        let spec = &registry.spec_refs.changes[0];
        assert_eq!(spec.name, "add-registry");
        assert_eq!(
            spec.spec_paths,
            vec![PathBuf::from(
                "openspec/changes/add-registry/specs/core/registry.md"
            )]
        );
        assert!(
            registry
                .edges
                .iter()
                .any(|edge| edge.relation == ProjectRelation::Implements
                    && matches!(edge.to, ProjectNodeRef::Spec(ref name) if name == "add-registry"))
        );
        let snapshot = ProjectRegistrySnapshot::from_registry(&registry);
        assert_eq!(snapshot.specs.len(), 1);
        assert!(snapshot.validate().is_empty());
    }

    #[test]
    fn task_external_refs_are_registered_and_snapshot_validates() {
        let tmp = tempfile::TempDir::new().unwrap();
        let board = Board::minimalist("Agent Work");
        let mut task = Task::new(board.id.clone(), "Active", "Fix bug");
        task.external_refs = vec!["https://github.com/styrene-labs/flynt/issues/456".into()];
        let store = TestStore {
            docs: Vec::new(),
            boards: vec![board],
            tasks: vec![task],
        };

        let registry = ProjectRegistry::discover(tmp.path().to_path_buf(), &store).unwrap();
        assert_eq!(registry.external_refs.refs.len(), 1);
        assert!(registry.external_refs.refs[0].document_sources.is_empty());
        assert_eq!(registry.external_refs.refs[0].task_sources.len(), 1);

        let snapshot = ProjectRegistrySnapshot::from_registry(&registry);
        assert!(snapshot.validate().is_empty());
    }

    #[test]
    fn task_registry_projects_tasks_boards_and_membership_edges() {
        let tmp = tempfile::TempDir::new().unwrap();
        let board = Board::minimalist("Agent Work");
        let mut task = Task::new(board.id.clone(), "Active", "Do the work");
        task.tags = vec!["agent".into()];
        let store = TestStore {
            docs: Vec::new(),
            boards: vec![board.clone()],
            tasks: vec![task.clone()],
        };

        let registry = ProjectRegistry::discover(tmp.path().to_path_buf(), &store).unwrap();
        assert_eq!(registry.task_refs.boards.len(), 1);
        assert_eq!(registry.task_refs.tasks.len(), 1);
        assert_eq!(registry.task_refs.tasks[0].title, "Do the work");
        assert_eq!(
            registry.task_refs.tasks[0].evidence.state,
            TaskEvidenceState::NotRequired
        );
        assert!(registry.edges.iter().any(|edge| {
            edge.relation == ProjectRelation::BelongsTo
                && matches!(edge.from, ProjectNodeRef::Task(_))
                && matches!(edge.to, ProjectNodeRef::Board(_))
        }));
    }

    #[test]
    fn external_ref_registry_extracts_document_urls_and_edges() {
        let tmp = tempfile::TempDir::new().unwrap();
        let note = test_doc(
            "notes/issue.md",
            "Issue",
            "Track https://github.com/styrene-labs/flynt/issues/123.",
            Frontmatter::default(),
            Vec::new(),
        );
        let store = TestStore::with_docs(vec![note]);
        let registry = ProjectRegistry::discover(tmp.path().to_path_buf(), &store).unwrap();
        assert_eq!(registry.external_refs.refs.len(), 1);
        let ext = &registry.external_refs.refs[0];
        assert_eq!(ext.document_sources.len(), 1);
        assert!(ext.task_sources.is_empty());
        assert_eq!(ext.provider.as_deref(), Some("GitHub"));
        assert!(ext.label.as_deref().unwrap_or_default().contains("#123"));
        assert!(registry.edges.iter().any(|edge| {
            edge.relation == ProjectRelation::References
                && matches!(edge.to, ProjectNodeRef::ExternalRef(_))
        }));
    }

    #[test]
    fn raw_asset_registry_discovers_open_assets_and_marks_render_sidecars() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("drawings")).unwrap();
        std::fs::create_dir_all(tmp.path().join("assets")).unwrap();
        std::fs::write(tmp.path().join("drawings/map.excalidraw"), "{}").unwrap();
        std::fs::write(tmp.path().join("drawings/map.md"), "![[map.excalidraw]]").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        std::fs::create_dir_all(tmp.path().join("drawings/rendered")).unwrap();
        std::fs::write(tmp.path().join("drawings/rendered/map.svg"), "<svg/>").unwrap();
        std::fs::write(tmp.path().join("assets/logo.svg"), "<svg/>").unwrap();
        std::fs::write(tmp.path().join("assets/theme.css"), "body {}").unwrap();
        std::fs::write(tmp.path().join("assets/data.json"), "{}").unwrap();
        std::fs::write(tmp.path().join("assets/deck.pptx"), "opaque").unwrap();

        let artifacts = VisualArtifactRegistry::discover(tmp.path());
        let assets = RawAssetRegistry::discover(tmp.path(), &artifacts);

        assert!(
            assets
                .assets
                .iter()
                .any(|asset| asset.path == PathBuf::from("assets/logo.svg")
                    && asset.role == RawAssetRole::Image)
        );
        assert!(
            assets
                .assets
                .iter()
                .any(|asset| asset.path == PathBuf::from("assets/theme.css")
                    && asset.role == RawAssetRole::Stylesheet)
        );
        assert!(
            assets
                .assets
                .iter()
                .any(|asset| asset.path == PathBuf::from("assets/data.json")
                    && asset.role == RawAssetRole::Data)
        );
        assert!(
            assets
                .assets
                .iter()
                .any(|asset| asset.path == PathBuf::from("assets/deck.pptx")
                    && asset.role == RawAssetRole::ImportExportOnly)
        );
        assert!(assets.assets.iter().any(|asset| asset.path
            == PathBuf::from("drawings/rendered/map.svg")
            && asset.role == RawAssetRole::RenderSidecar));
        assert!(
            !assets
                .assets
                .iter()
                .any(|asset| asset.path == PathBuf::from("drawings/map.excalidraw"))
        );
        assert!(
            !assets
                .assets
                .iter()
                .any(|asset| asset.path == PathBuf::from("drawings/map.md"))
        );
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
    fn snapshot_validation_reports_unsafe_paths_across_node_tables() {
        let mut snapshot = ProjectRegistrySnapshot::from_registry(&ProjectRegistry::default());
        snapshot.documents.push(DocumentSnapshot {
            id: DocumentId::new(),
            path: PathBuf::from("/tmp/leak.md"),
            title: "Leak".into(),
            kind: DocumentKind::Note,
            aliases: Vec::new(),
            tags: Vec::new(),
        });
        snapshot.visual_artifacts.push(VisualArtifactSnapshot {
            id: "d2:/tmp/leak.d2".into(),
            kind: VisualArtifactKind::D2Diagram,
            title: "Leak".into(),
            source_path: PathBuf::from("/tmp/leak.d2"),
            wrapper_path: Some(PathBuf::from("../leak.md")),
            renders: Vec::new(),
            surfaces: Vec::new(),
        });
        snapshot.raw_assets.push(RawAssetRecord {
            id: RawAssetId::from_project_relative_path(std::path::Path::new("../secret.png")),
            path: PathBuf::from("../secret.png"),
            media_type: "image/png".into(),
            role: RawAssetRole::Image,
        });
        snapshot.specs.push(SpecChangeRecord {
            name: "leak".into(),
            path: PathBuf::from("/tmp/openspec/changes/leak"),
            proposal_path: Some(PathBuf::from("../proposal.md")),
            design_path: None,
            tasks_path: None,
            spec_paths: Vec::new(),
        });

        let diagnostics = snapshot.validate();
        assert!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.kind == RegistryDiagnosticKind::UnsafePath)
                .count()
                >= 4
        );
    }

    #[test]
    fn snapshot_validation_reports_dangling_external_ref_edges() {
        let mut snapshot = ProjectRegistrySnapshot::from_registry(&ProjectRegistry::default());
        snapshot.edges.push(ProjectEdge {
            from: ProjectNodeRef::Task("missing-task".into()),
            to: ProjectNodeRef::ExternalRef(ExternalRefId::from_uri("https://example.com/missing")),
            relation: ProjectRelation::References,
            source: EdgeSource::Generated,
        });
        let diagnostics = snapshot.validate();
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.kind == RegistryDiagnosticKind::DanglingGraphEndpoint)
        );
    }

    #[test]
    fn project_registry_serializes_as_plain_json() {
        let registry = ProjectRegistry::default();
        let json = serde_json::to_string_pretty(&registry).unwrap();
        assert!(json.contains("documents"));
        assert!(json.contains("visual_artifacts"));
        assert!(json.contains("evidence"));
        assert!(json.contains("raw_assets"));
        assert!(json.contains("external_refs"));
        assert!(json.contains("task_refs"));
        assert!(json.contains("diagnostics"));
        assert!(json.contains("edges"));

        let snapshot_json =
            serde_json::to_string_pretty(&ProjectRegistrySnapshot::from_registry(&registry))
                .unwrap();
        assert!(snapshot_json.contains("evidence_sources"));
        assert!(snapshot_json.contains("raw_assets"));
        assert!(snapshot_json.contains("external_refs"));
        assert!(snapshot_json.contains("tasks"));
        assert!(snapshot_json.contains("boards"));
        assert!(snapshot_json.contains("diagnostics"));
        assert!(snapshot_json.contains("specs"));
    }
}
