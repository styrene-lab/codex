//! Pure resolution of `FlyntLinkAction` targets against existing Flynt
//! project/entity data. Platform-neutral: no navigation, no project
//! switching, no side effects. `flynt-app`/`flynt-mobile` execute the
//! resolved targets through their own route/command layer.

use crate::models::{Document, DocumentId, Task, TaskId};
use crate::native_invocation::DocumentReference;
use crate::store::ProjectStore;
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ResolutionError {
    #[error("unknown project: {0}")]
    UnknownProject(String),
    #[error("document not found")]
    DocumentNotFound,
    #[error("task not found")]
    TaskNotFound,
    #[error("project store error: {0}")]
    Store(#[from] anyhow::Error),
}

/// A directory that can resolve a link's opaque `project_id` string to a
/// project root path. Implemented by the host app over its own known-projects
/// registry — `flynt-core` does not know about that registry's shape.
pub trait KnownProjectDirectory {
    fn resolve_project_root(&self, project_id: &str) -> Option<PathBuf>;
}

pub fn resolve_project_root(
    project_id: &str,
    directory: &dyn KnownProjectDirectory,
) -> Result<PathBuf, ResolutionError> {
    directory
        .resolve_project_root(project_id)
        .ok_or_else(|| ResolutionError::UnknownProject(project_id.to_string()))
}

pub fn resolve_document(
    document: &DocumentReference,
    store: &dyn ProjectStore,
) -> Result<Document, ResolutionError> {
    let found = match document {
        DocumentReference::Id(id) => store.get_document(&DocumentId(*id))?,
        DocumentReference::RelativePath(path) => store.get_document_by_path(Path::new(path))?,
    };
    found.ok_or(ResolutionError::DocumentNotFound)
}

pub fn resolve_task(task_id: Uuid, store: &dyn ProjectStore) -> Result<Task, ResolutionError> {
    store
        .get_task(&TaskId(task_id))?
        .ok_or(ResolutionError::TaskNotFound)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datum::EntityKind;
    use crate::models::{
        Board, BoardId, DecayRate, DocumentMeta, Frontmatter, Priority, SearchResult, TaskStatus,
    };
    use crate::store::{DocumentMetadataFilter, TaskFilter};
    use chrono::Utc;

    #[derive(Default)]
    struct TestStore {
        docs: Vec<Document>,
        tasks: Vec<Task>,
    }

    impl ProjectStore for TestStore {
        fn get_document(&self, id: &DocumentId) -> anyhow::Result<Option<Document>> {
            Ok(self.docs.iter().find(|doc| &doc.id == id).cloned())
        }
        fn get_document_by_path(&self, path: &Path) -> anyhow::Result<Option<Document>> {
            Ok(self.docs.iter().find(|doc| doc.path == path).cloned())
        }
        fn find_document_by_slug(&self, _slug: &str) -> anyhow::Result<Option<DocumentMeta>> {
            Ok(None)
        }
        fn list_documents(&self) -> anyhow::Result<Vec<DocumentMeta>> {
            Ok(Vec::new())
        }
        fn list_documents_by_metadata(
            &self,
            _filter: &DocumentMetadataFilter,
        ) -> anyhow::Result<Vec<DocumentMeta>> {
            Ok(Vec::new())
        }
        fn save_document(&self, _doc: &Document) -> anyhow::Result<()> {
            Ok(())
        }
        fn delete_document(&self, _id: &DocumentId) -> anyhow::Result<()> {
            Ok(())
        }
        fn search_documents(&self, _query: &str) -> anyhow::Result<Vec<SearchResult>> {
            Ok(Vec::new())
        }
        fn get_backlinks(&self, _id: &DocumentId) -> anyhow::Result<Vec<DocumentMeta>> {
            Ok(Vec::new())
        }
        fn list_entities_by_kind(&self, _kind: &EntityKind) -> anyhow::Result<Vec<DocumentMeta>> {
            Ok(Vec::new())
        }
        fn get_task(&self, id: &TaskId) -> anyhow::Result<Option<Task>> {
            Ok(self.tasks.iter().find(|task| &task.id == id).cloned())
        }
        fn list_tasks(&self, _filter: &TaskFilter) -> anyhow::Result<Vec<Task>> {
            Ok(Vec::new())
        }
        fn save_task(&self, _task: &Task) -> anyhow::Result<()> {
            Ok(())
        }
        fn delete_task(&self, _id: &TaskId) -> anyhow::Result<()> {
            Ok(())
        }
        fn update_task(
            &self,
            _id: &TaskId,
            _patch: &flynt_models::TaskPatch,
        ) -> anyhow::Result<bool> {
            Ok(false)
        }
        fn get_board(&self, _id: &BoardId) -> anyhow::Result<Option<Board>> {
            Ok(None)
        }
        fn list_boards(&self) -> anyhow::Result<Vec<Board>> {
            Ok(Vec::new())
        }
        fn save_board(&self, _board: &Board) -> anyhow::Result<()> {
            Ok(())
        }
        fn delete_board(&self, _id: &BoardId) -> anyhow::Result<()> {
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

    fn sample_document(id: Uuid, path: &str) -> Document {
        let now = Utc::now();
        Document {
            id: DocumentId(id),
            path: PathBuf::from(path),
            title: path.to_string(),
            content: String::new(),
            frontmatter: Frontmatter::default(),
            outgoing_links: Vec::new(),
            created_at: now,
            updated_at: now,
            entity: None,
        }
    }

    fn sample_task(id: Uuid) -> Task {
        let now = Utc::now();
        Task {
            id: TaskId(id),
            board_id: BoardId(Uuid::new_v4()),
            column: "Backlog".into(),
            title: "Sample".into(),
            description: String::new(),
            priority: Priority::Medium,
            status: TaskStatus::Todo,
            tags: Vec::new(),
            document_refs: Vec::new(),
            external_refs: Vec::new(),
            due_date: None,
            position: 0,
            created_at: now,
            updated_at: now,
            decay: DecayRate::default(),
            last_touched_at: None,
            design_node_id: None,
            openspec_change: None,
            engagement_id: None,
            execution: None,
        }
    }

    #[test]
    fn resolves_document_by_id() {
        let id = Uuid::new_v4();
        let store = TestStore {
            docs: vec![sample_document(id, "notes/today.md")],
            tasks: Vec::new(),
        };
        let doc = resolve_document(&DocumentReference::Id(id), &store).unwrap();
        assert_eq!(doc.path, PathBuf::from("notes/today.md"));
    }

    #[test]
    fn resolves_document_by_relative_path() {
        let id = Uuid::new_v4();
        let store = TestStore {
            docs: vec![sample_document(id, "notes/today.md")],
            tasks: Vec::new(),
        };
        let doc = resolve_document(
            &DocumentReference::RelativePath("notes/today.md".into()),
            &store,
        )
        .unwrap();
        assert_eq!(doc.id, DocumentId(id));
    }

    #[test]
    fn missing_document_is_not_found() {
        let store = TestStore::default();
        let err = resolve_document(&DocumentReference::Id(Uuid::new_v4()), &store).unwrap_err();
        assert!(matches!(err, ResolutionError::DocumentNotFound));
    }

    #[test]
    fn resolves_and_misses_tasks() {
        let id = Uuid::new_v4();
        let store = TestStore {
            docs: Vec::new(),
            tasks: vec![sample_task(id)],
        };
        assert_eq!(resolve_task(id, &store).unwrap().id, TaskId(id));
        assert!(matches!(
            resolve_task(Uuid::new_v4(), &store).unwrap_err(),
            ResolutionError::TaskNotFound
        ));
    }

    struct FixedDirectory(Vec<(Uuid, PathBuf)>);
    impl KnownProjectDirectory for FixedDirectory {
        fn resolve_project_root(&self, project_id: &str) -> Option<PathBuf> {
            let target = Uuid::parse_str(project_id).ok()?;
            self.0
                .iter()
                .find(|(id, _)| *id == target)
                .map(|(_, root)| root.clone())
        }
    }

    #[test]
    fn resolves_known_project_root() {
        let id = Uuid::new_v4();
        let directory = FixedDirectory(vec![(id, PathBuf::from("/projects/example"))]);
        let root = resolve_project_root(&id.to_string(), &directory).unwrap();
        assert_eq!(root, PathBuf::from("/projects/example"));
    }

    #[test]
    fn unknown_or_malformed_project_id_fails() {
        let directory = FixedDirectory(Vec::new());
        assert!(matches!(
            resolve_project_root(&Uuid::new_v4().to_string(), &directory).unwrap_err(),
            ResolutionError::UnknownProject(_)
        ));
        assert!(matches!(
            resolve_project_root("not-a-uuid", &directory).unwrap_err(),
            ResolutionError::UnknownProject(_)
        ));
    }
}
