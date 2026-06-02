use std::path::{Path, PathBuf};
use std::sync::Arc;

use flynt_core::project_registry::ProjectRegistrySnapshot;
use flynt_store::project::Project;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRegistrySnapshotSummary {
    pub path: PathBuf,
    pub schema: String,
    pub document_count: usize,
    pub visual_artifact_count: usize,
    pub raw_asset_count: usize,
    pub external_ref_count: usize,
    pub task_count: usize,
    pub board_count: usize,
    pub spec_count: usize,
    pub diagnostic_count: usize,
    pub validation_diagnostic_count: usize,
    pub edge_count: usize,
}

pub fn refresh_snapshot(
    project_root: &Path,
    project: &Project,
) -> anyhow::Result<ProjectRegistrySnapshot> {
    crate::bootstrap::refresh_project_registry_snapshot(project_root, project)
        .ok_or_else(|| anyhow::anyhow!("project registry snapshot refresh failed"))
}

pub fn refresh_snapshot_for_project(
    project_root: PathBuf,
    project: Arc<Project>,
) -> anyhow::Result<ProjectRegistrySnapshot> {
    refresh_snapshot(&project_root, &project)
}

pub fn snapshot_summary(project_root: &Path) -> anyhow::Result<ProjectRegistrySnapshotSummary> {
    let path = ProjectRegistrySnapshot::snapshot_path(project_root);
    let snapshot = ProjectRegistrySnapshot::load_from_project(project_root)?
        .ok_or_else(|| anyhow::anyhow!("snapshot does not exist at {}", path.display()))?;
    Ok(summary_from_snapshot(path, &snapshot))
}

pub fn summary_from_snapshot(
    path: PathBuf,
    snapshot: &ProjectRegistrySnapshot,
) -> ProjectRegistrySnapshotSummary {
    let diagnostics = snapshot.validate();
    ProjectRegistrySnapshotSummary {
        path,
        schema: snapshot.schema.clone(),
        document_count: snapshot.source_summary.document_count,
        visual_artifact_count: snapshot.source_summary.visual_artifact_count,
        raw_asset_count: snapshot.source_summary.raw_asset_count,
        external_ref_count: snapshot.source_summary.external_ref_count,
        task_count: snapshot.source_summary.task_count,
        board_count: snapshot.source_summary.board_count,
        spec_count: snapshot.source_summary.spec_count,
        diagnostic_count: snapshot.source_summary.diagnostic_count,
        validation_diagnostic_count: diagnostics.len(),
        edge_count: snapshot.source_summary.edge_count,
    }
}

pub fn log_snapshot_summary(summary: &ProjectRegistrySnapshotSummary) {
    tracing::info!(
        "Project Registry snapshot: path={}, schema={}, documents={}, artifacts={}, raw_assets={}, external_refs={}, tasks={}, boards={}, specs={}, diagnostics={}, validation_diagnostics={}, edges={}",
        summary.path.display(),
        summary.schema,
        summary.document_count,
        summary.visual_artifact_count,
        summary.raw_asset_count,
        summary.external_ref_count,
        summary.task_count,
        summary.board_count,
        summary.spec_count,
        summary.diagnostic_count,
        summary.validation_diagnostic_count,
        summary.edge_count
    );
}
