//! Adapters from `flynt-core`'s platform-neutral native invocation
//! resolution onto `flynt-app`'s local types (`KnownProject`, `Route`).
//! Still pure — no navigation or project-switch side effects. Those belong
//! to the Apple lifecycle adapters that execute a resolved target.

use crate::bootstrap::KnownProject;
use crate::state::Route;
use flynt_core::native_invocation::FlyntView;
use flynt_core::native_invocation_resolve::KnownProjectDirectory;
use std::path::PathBuf;
use uuid::Uuid;

/// Adapts a slice of `KnownProject` into `flynt-core`'s resolution trait.
/// A wrapper is required because `impl ForeignTrait for [LocalType]` is not
/// allowed under Rust's orphan rules (the slice type itself is foreign).
pub struct KnownProjects<'a>(pub &'a [KnownProject]);

impl KnownProjectDirectory for KnownProjects<'_> {
    fn resolve_project_root(&self, project_id: &str) -> Option<PathBuf> {
        let target = Uuid::parse_str(project_id).ok()?;
        self.0
            .iter()
            .find(|project| project.project_id == Some(target))
            .map(|project| project.root.clone())
    }
}

pub fn route_for_view(view: FlyntView) -> Route {
    match view {
        FlyntView::Notes => Route::Notes,
        FlyntView::Tasks => Route::Kanban,
        FlyntView::Graph => Route::Graph,
        FlyntView::Search => Route::Search,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flynt_core::native_invocation_resolve::resolve_project_root;

    #[test]
    fn maps_every_view_to_a_route() {
        assert_eq!(route_for_view(FlyntView::Notes), Route::Notes);
        assert_eq!(route_for_view(FlyntView::Tasks), Route::Kanban);
        assert_eq!(route_for_view(FlyntView::Graph), Route::Graph);
        assert_eq!(route_for_view(FlyntView::Search), Route::Search);
    }

    #[test]
    fn resolves_known_project_by_id() {
        let id = Uuid::new_v4();
        let known = vec![KnownProject {
            name: "Example".into(),
            root: PathBuf::from("/projects/example"),
            project_id: Some(id),
        }];
        let root = resolve_project_root(&id.to_string(), &KnownProjects(&known)).unwrap();
        assert_eq!(root, PathBuf::from("/projects/example"));
    }

    #[test]
    fn unknown_project_id_fails() {
        let known = vec![KnownProject {
            name: "Example".into(),
            root: PathBuf::from("/projects/example"),
            project_id: Some(Uuid::new_v4()),
        }];
        assert!(resolve_project_root(&Uuid::new_v4().to_string(), &KnownProjects(&known)).is_err());
    }
}
