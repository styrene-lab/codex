use flynt_core::models::Frontmatter;
use flynt_core::visual_artifacts::VisualArtifactKind;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VisualArtifactSurface {
    ExcalidrawPreview { source_path: PathBuf },
    ExcalidrawEditor { source_path: PathBuf },
    DesignBoard { source_path: PathBuf },
}

pub fn resolve_wrapper_surface(
    project_root: &Path,
    rel_path: &Path,
    body: &str,
    frontmatter: &Frontmatter,
) -> Option<VisualArtifactSurface> {
    resolve_excalidraw_surface(project_root, rel_path, body, frontmatter)
        .or_else(|| resolve_design_board_surface(project_root, rel_path, body))
}

fn resolve_excalidraw_surface(
    project_root: &Path,
    rel_path: &Path,
    body: &str,
    frontmatter: &Frontmatter,
) -> Option<VisualArtifactSurface> {
    let from_body = crate::views::excalidraw::excalidraw_embed_path(body);
    let from_recovery =
        if from_body.is_none() && frontmatter.tags.iter().any(|tag| tag == "drawing") {
            rel_path
                .file_stem()
                .map(|stem| format!("{}.excalidraw", stem.to_string_lossy()))
        } else {
            None
        };
    resolve_sibling_artifact(
        project_root,
        rel_path,
        from_body.or(from_recovery),
        VisualArtifactKind::ExcalidrawDrawing,
    )
}

fn resolve_design_board_surface(
    project_root: &Path,
    rel_path: &Path,
    body: &str,
) -> Option<VisualArtifactSurface> {
    let from_body = crate::views::design_board::design_board_embed_path(body);
    let from_recovery = if from_body.is_none()
        && crate::views::design_board::frontmatter_has_design_board_tag(body)
    {
        rel_path
            .file_stem()
            .map(|stem| format!("{}.board", stem.to_string_lossy()))
    } else {
        None
    };
    resolve_sibling_artifact(
        project_root,
        rel_path,
        from_body.or(from_recovery),
        VisualArtifactKind::DesignBoard,
    )
}

fn resolve_sibling_artifact(
    project_root: &Path,
    rel_path: &Path,
    file_name: Option<String>,
    kind: VisualArtifactKind,
) -> Option<VisualArtifactSurface> {
    let file_name = file_name?;
    let doc_dir = rel_path.parent().unwrap_or_else(|| Path::new(""));
    let source_path = doc_dir.join(file_name);
    if !project_root.join(&source_path).exists() {
        return None;
    }
    match kind {
        VisualArtifactKind::ExcalidrawDrawing => {
            Some(VisualArtifactSurface::ExcalidrawPreview { source_path })
        }
        VisualArtifactKind::DesignBoard => Some(VisualArtifactSurface::DesignBoard { source_path }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn frontmatter_with_tags(tags: &[&str]) -> Frontmatter {
        Frontmatter {
            tags: tags.iter().map(|tag| (*tag).to_string()).collect(),
            ..Frontmatter::default()
        }
    }

    #[test]
    fn resolves_excalidraw_wrapper_to_preview_surface() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("drawings")).unwrap();
        std::fs::write(tmp.path().join("drawings/sketch.excalidraw"), "{}").unwrap();

        let surface = resolve_wrapper_surface(
            tmp.path(),
            Path::new("drawings/sketch.md"),
            "![[sketch.excalidraw]]\n",
            &frontmatter_with_tags(&["drawing"]),
        );

        assert_eq!(
            surface,
            Some(VisualArtifactSurface::ExcalidrawPreview {
                source_path: PathBuf::from("drawings/sketch.excalidraw")
            })
        );
    }

    #[test]
    fn resolves_corrupt_drawing_wrapper_from_frontmatter_and_sibling() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("drawings")).unwrap();
        std::fs::write(tmp.path().join("drawings/sketch.excalidraw"), "{}").unwrap();

        let surface = resolve_wrapper_surface(
            tmp.path(),
            Path::new("drawings/sketch.md"),
            "body was overwritten",
            &frontmatter_with_tags(&["drawing"]),
        );

        assert_eq!(
            surface,
            Some(VisualArtifactSurface::ExcalidrawPreview {
                source_path: PathBuf::from("drawings/sketch.excalidraw")
            })
        );
    }

    #[test]
    fn resolves_design_board_wrapper_to_board_surface() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("boards")).unwrap();
        std::fs::write(tmp.path().join("boards/hero.board"), "{}").unwrap();

        let surface = resolve_wrapper_surface(
            tmp.path(),
            Path::new("boards/hero.md"),
            "![[hero.board]]\n",
            &frontmatter_with_tags(&["design_board"]),
        );

        assert_eq!(
            surface,
            Some(VisualArtifactSurface::DesignBoard {
                source_path: PathBuf::from("boards/hero.board")
            })
        );
    }
}
