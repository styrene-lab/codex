//! UI state mirror — writes the panel/tab/view state the embedded Omegon
//! agent needs to answer "what am I looking at?".
//!
//! Shape on disk: `<userspace project runtime>/ui-state.json`. The flynt-agent
//! extension reads this file via its `get_ui_state` tool. Atomic write via
//! tempfile + rename so the agent never sees a half-written document.

use flynt_core::store::ProjectStore;
use flynt_store::project::Project;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::state::{Route, TabState};

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
struct DocumentRef {
    id: String,
    title: String,
    /// Path relative to project root. Pass directly to `get_document`.
    path: String,
    /// What kind of document this is from the agent's perspective. Lets
    /// design_board-aware tools (design_board_active) skip the body-parse hop. Values:
    /// "design-board", "drawing", or "note" (the default).
    document_type: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct UiStateSnapshot<'a> {
    /// The document the user is currently looking at, if any.
    active_document: Option<DocumentRef>,
    /// All open document tabs in tab-bar order.
    open_documents: Vec<DocumentRef>,
    /// Which top-level view is shown (notes, kanban, graph, settings, search, welcome).
    current_view: &'static str,
    /// Absolute path of the project root the user is in.
    project_root: &'a str,
    /// ISO-8601 UTC timestamp of this snapshot.
    updated_at: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
struct UiStateSnapshotStable {
    active_document: Option<DocumentRef>,
    open_documents: Vec<DocumentRef>,
    current_view: String,
    project_root: String,
}

impl<'a> From<&UiStateSnapshot<'a>> for UiStateSnapshotStable {
    fn from(snapshot: &UiStateSnapshot<'a>) -> Self {
        Self {
            active_document: snapshot.active_document.clone(),
            open_documents: snapshot.open_documents.clone(),
            current_view: snapshot.current_view.to_string(),
            project_root: snapshot.project_root.to_string(),
        }
    }
}

fn route_label(route: &Route) -> &'static str {
    match route {
        Route::Welcome => "welcome",
        Route::Notes => "notes",
        Route::Design => "design",
        Route::Search => "search",
        Route::Lenses => "lenses",
        Route::Kanban => "kanban",
        Route::Graph => "graph",
        Route::Omegon => "omegon",
        Route::TerminalLab => "terminal",
    }
}

fn resolve_doc_ref(
    project: &Project,
    id: &flynt_core::models::DocumentId,
    title: &str,
) -> Option<DocumentRef> {
    let doc = project.store.get_document(id).ok().flatten()?;
    let path_str = doc.path.to_string_lossy().to_string();
    Some(DocumentRef {
        id: id.0.to_string(),
        title: title.to_string(),
        document_type: classify_document(project, &doc.path).to_string(),
        path: path_str,
    })
}

fn classify_document(project: &Project, rel_path: &Path) -> &'static str {
    // Cheap classification: read the body and look for a single-line embed
    // (design_board or drawing wrapper). Also recovers drawing wrappers from
    // frontmatter + sibling data file, matching NotesView dispatch.
    let abs = project.root.join(rel_path);
    let Ok(content) = std::fs::read_to_string(&abs) else {
        return "note";
    };
    let body = if let Some(rest) = content.strip_prefix("+++\n") {
        rest.find("\n+++")
            .map(|end| rest[end + 4..].trim())
            .unwrap_or(content.trim())
    } else {
        content.trim()
    };
    let lines: Vec<&str> = body.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.len() == 1 {
        let line = lines[0].trim();
        if line.starts_with("![[") {
            if line.ends_with(".board]]") {
                return "design-board";
            }
            if line.ends_with(".excalidraw]]") {
                return "drawing";
            }
        }
    }
    if crate::views::excalidraw::frontmatter_has_drawing_tag(&content) {
        if let Some(stem) = rel_path.file_stem() {
            let sibling = rel_path
                .parent()
                .unwrap_or(std::path::Path::new(""))
                .join(format!("{}.excalidraw", stem.to_string_lossy()));
            if project.root.join(sibling).exists() {
                return "drawing";
            }
        }
    }
    "note"
}

/// Write the UI state snapshot to `<userspace project runtime>/ui-state.json`.
/// Errors are logged but never propagated — UI state is best-effort and must
/// not block the editor on a slow disk.
pub fn write_snapshot(project: &Project, runtime_root: &Path, tabs: &TabState, route: &Route) {
    let active_document = tabs
        .tabs
        .get(tabs.active)
        .and_then(|(id, title)| resolve_doc_ref(project, id, title));

    let open_documents: Vec<DocumentRef> = tabs
        .tabs
        .iter()
        .filter_map(|(id, title)| resolve_doc_ref(project, id, title))
        .collect();

    let project_root_buf = project.root.to_string_lossy();
    let snapshot = UiStateSnapshot {
        active_document,
        open_documents,
        current_view: route_label(route),
        project_root: project_root_buf.as_ref(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    if let Err(e) = write_atomic(runtime_root, &snapshot) {
        tracing::warn!("ui-state write failed: {e}");
    }
}

fn write_atomic(runtime_root: &Path, snapshot: &UiStateSnapshot<'_>) -> std::io::Result<()> {
    std::fs::create_dir_all(runtime_root)?;
    let final_path = runtime_root.join("ui-state.json");
    let tmp_path = runtime_root.join("ui-state.json.tmp");
    let stable = UiStateSnapshotStable::from(snapshot);
    if let Ok(existing) = std::fs::read(&final_path) {
        if let Ok(existing_snapshot) = serde_json::from_slice::<UiStateSnapshotStable>(&existing) {
            if existing_snapshot == stable {
                return Ok(());
            }
        }
    }

    let body = serde_json::to_vec_pretty(snapshot)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::write(&tmp_path, &body)?;
    std::fs::rename(&tmp_path, &final_path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use flynt_store::project::Project;

    #[test]
    fn writes_ui_state_to_runtime_root_not_project_local_state() {
        let project_tmp = tempfile::TempDir::new().unwrap();
        let runtime_tmp = tempfile::TempDir::new().unwrap();
        let project = Project::open(project_tmp.path()).unwrap();
        let tabs = TabState::default();

        write_snapshot(&project, runtime_tmp.path(), &tabs, &Route::Notes);

        assert!(runtime_tmp.path().join("ui-state.json").exists());
        assert!(!project_tmp.path().join(".flynt/local").exists());
    }

    #[test]
    fn identical_ui_state_snapshot_is_noop() {
        let project_tmp = tempfile::TempDir::new().unwrap();
        let runtime_tmp = tempfile::TempDir::new().unwrap();
        let project = Project::open(project_tmp.path()).unwrap();
        let tabs = TabState::default();

        write_snapshot(&project, runtime_tmp.path(), &tabs, &Route::Notes);
        let path = runtime_tmp.path().join("ui-state.json");
        let first = std::fs::read_to_string(&path).unwrap();
        write_snapshot(&project, runtime_tmp.path(), &tabs, &Route::Notes);
        let second = std::fs::read_to_string(&path).unwrap();

        assert_eq!(first, second);
    }
}
