use crate::models::DocumentId;
use crate::project_registry::{DocumentKind, ProjectRegistry, VisualArtifactId};
use crate::visual_artifacts::{RenderArtifact, VisualArtifactKind};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SidebarProjection {
    pub text_files: Vec<TextFileNavItem>,
    pub artifacts: ArtifactNavGroups,
    pub diagnostics: Vec<SidebarProjectionDiagnostic>,
}

impl SidebarProjection {
    pub fn from_registry(registry: &ProjectRegistry) -> Self {
        let mut text_files = registry
            .documents
            .documents
            .iter()
            .filter(|document| document.kind == DocumentKind::Note)
            .filter(|document| is_safe_text_file_path(&document.path))
            .map(|document| TextFileNavItem {
                id: Some(document.id.clone()),
                path: document.path.clone(),
                title: document.title.clone(),
                kind: TextFileKind::from_path(&document.path),
            })
            .collect::<Vec<_>>();
        text_files.sort_by(|a, b| a.path.cmp(&b.path));

        let mut artifacts = ArtifactNavGroups::default();
        for artifact in &registry.visual_artifacts.artifacts {
            let item = ArtifactNavItem {
                id: artifact.id.clone(),
                title: artifact.artifact.title.clone(),
                kind: ArtifactNavKind::from(artifact.artifact.kind),
                source_path: artifact.artifact.source_path.clone(),
                wrapper_path: artifact.artifact.wrapper_path.clone(),
                render_paths: artifact.artifact.renders.clone(),
            };
            match artifact.artifact.kind {
                VisualArtifactKind::DesignBoard => artifacts.boards.push(item),
                VisualArtifactKind::ExcalidrawDrawing => artifacts.drawings.push(item),
                VisualArtifactKind::D2Diagram => artifacts.diagrams.push(item),
                VisualArtifactKind::Flow => artifacts.flows.push(item),
            }
        }
        artifacts.sort();

        Self {
            text_files,
            artifacts,
            diagnostics: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextFileNavItem {
    pub id: Option<DocumentId>,
    pub path: PathBuf,
    pub title: String,
    pub kind: TextFileKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextFileKind {
    Markdown,
    PlainText,
    Toml,
    Yaml,
    Json,
    Csv,
}

impl TextFileKind {
    fn from_path(path: &Path) -> Self {
        match path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str()
        {
            "md" | "markdown" => Self::Markdown,
            "toml" => Self::Toml,
            "yaml" | "yml" => Self::Yaml,
            "json" => Self::Json,
            "csv" => Self::Csv,
            _ => Self::PlainText,
        }
    }
}

pub fn is_safe_text_file_path(path: &Path) -> bool {
    if !crate::project_registry::is_safe_project_relative_path(path) {
        return false;
    }
    let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
        return false;
    };
    matches!(
        extension.to_ascii_lowercase().as_str(),
        "md" | "markdown" | "txt" | "toml" | "yaml" | "yml" | "json" | "csv"
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ArtifactNavGroups {
    pub boards: Vec<ArtifactNavItem>,
    pub drawings: Vec<ArtifactNavItem>,
    pub diagrams: Vec<ArtifactNavItem>,
    pub flows: Vec<ArtifactNavItem>,
}

impl ArtifactNavGroups {
    fn sort(&mut self) {
        self.boards.sort_by(|a, b| a.title.cmp(&b.title));
        self.drawings.sort_by(|a, b| a.title.cmp(&b.title));
        self.diagrams.sort_by(|a, b| a.title.cmp(&b.title));
        self.flows.sort_by(|a, b| a.title.cmp(&b.title));
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactNavItem {
    pub id: VisualArtifactId,
    pub title: String,
    pub kind: ArtifactNavKind,
    pub source_path: PathBuf,
    pub wrapper_path: Option<PathBuf>,
    pub render_paths: Vec<RenderArtifact>,
}

impl ArtifactNavItem {
    pub fn reveal_paths(&self) -> Vec<PathBuf> {
        let mut paths = vec![self.source_path.clone()];
        if let Some(wrapper_path) = &self.wrapper_path {
            paths.push(wrapper_path.clone());
        }
        paths.extend(self.render_paths.iter().map(|render| render.path.clone()));
        paths.sort();
        paths.dedup();
        paths
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactNavKind {
    Board,
    Drawing,
    Diagram,
    Flow,
}

impl From<VisualArtifactKind> for ArtifactNavKind {
    fn from(kind: VisualArtifactKind) -> Self {
        match kind {
            VisualArtifactKind::DesignBoard => Self::Board,
            VisualArtifactKind::ExcalidrawDrawing => Self::Drawing,
            VisualArtifactKind::D2Diagram => Self::Diagram,
            VisualArtifactKind::Flow => Self::Flow,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SidebarProjectionDiagnostic {
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{DocumentId, Frontmatter, PublicationConfig};
    use crate::project_registry::{
        ArtifactMetadata, DocumentRecord, DocumentRegistry, ProjectRegistry, ProjectScope,
        VisualArtifactRecord, VisualArtifactRegistry,
    };
    use crate::visual_artifacts::{RenderArtifact, RenderFormat, RenderStatus, VisualArtifact};

    fn doc(path: &str, kind: DocumentKind) -> DocumentRecord {
        DocumentRecord {
            id: DocumentId::new(),
            path: PathBuf::from(path),
            title: Path::new(path)
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            frontmatter: Frontmatter::default(),
            aliases: Vec::new(),
            tags: Vec::new(),
            publication: PublicationConfig::default(),
            kind,
            outgoing: Vec::new(),
            backlinks: Vec::new(),
            raw_content: String::new(),
        }
    }

    fn board_artifact() -> VisualArtifactRecord {
        let artifact = VisualArtifact {
            kind: VisualArtifactKind::DesignBoard,
            title: "Demo".into(),
            source_path: PathBuf::from("boards/Demo.board"),
            wrapper_path: Some(PathBuf::from("boards/Demo.md")),
            renders: vec![RenderArtifact {
                format: RenderFormat::Png,
                path: PathBuf::from("boards/Demo.png"),
                status: RenderStatus::Present,
            }],
        };
        VisualArtifactRecord {
            id: VisualArtifactId::from_project_relative_source(
                VisualArtifactKind::DesignBoard,
                &artifact.source_path,
            ),
            artifact,
            surfaces: Vec::new(),
            metadata: ArtifactMetadata::default(),
        }
    }

    fn registry() -> ProjectRegistry {
        ProjectRegistry {
            scope: ProjectScope::default(),
            documents: DocumentRegistry {
                documents: vec![
                    doc("welcome.md", DocumentKind::Note),
                    doc("notes/data.csv", DocumentKind::Note),
                    doc("boards/Demo.md", DocumentKind::ArtifactWrapper),
                    doc("notes/image.png", DocumentKind::Note),
                ],
            },
            visual_artifacts: VisualArtifactRegistry {
                artifacts: vec![board_artifact()],
            },
            ..ProjectRegistry::default()
        }
    }

    #[test]
    fn projection_keeps_notes_and_plaintext_but_hides_wrappers() {
        let projection = SidebarProjection::from_registry(&registry());
        let paths = projection
            .text_files
            .iter()
            .map(|item| item.path.as_path())
            .collect::<Vec<_>>();

        assert!(paths.contains(&Path::new("welcome.md")));
        assert!(paths.contains(&Path::new("notes/data.csv")));
        assert!(!paths.contains(&Path::new("boards/Demo.md")));
        assert!(!paths.contains(&Path::new("notes/image.png")));
    }

    #[test]
    fn projection_groups_boards_as_artifacts_with_reveal_paths() {
        let projection = SidebarProjection::from_registry(&registry());
        assert_eq!(projection.artifacts.boards.len(), 1);
        let board = &projection.artifacts.boards[0];
        assert_eq!(board.title, "Demo");
        assert_eq!(board.kind, ArtifactNavKind::Board);
        assert_eq!(
            board.reveal_paths(),
            vec![
                PathBuf::from("boards/Demo.board"),
                PathBuf::from("boards/Demo.md"),
                PathBuf::from("boards/Demo.png"),
            ]
        );
    }
}
