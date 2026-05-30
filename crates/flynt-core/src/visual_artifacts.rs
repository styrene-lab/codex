use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VisualArtifactKind {
    D2Diagram,
    ExcalidrawDrawing,
    DesignBoard,
    Flow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RenderFormat {
    Svg,
    Png,
    Html,
}

impl RenderFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Svg => "svg",
            Self::Png => "png",
            Self::Html => "html",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Svg => "SVG",
            Self::Png => "PNG",
            Self::Html => "HTML",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RenderStatus {
    Missing,
    Current,
    Stale,
    Present,
}

impl RenderStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Current => "current",
            Self::Stale => "stale",
            Self::Present => "present",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderArtifact {
    pub format: RenderFormat,
    pub path: PathBuf,
    pub status: RenderStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualArtifact {
    pub kind: VisualArtifactKind,
    pub title: String,
    pub source_path: PathBuf,
    pub wrapper_path: Option<PathBuf>,
    pub renders: Vec<RenderArtifact>,
}

pub fn render_status(source: &Path, render: &Path) -> RenderStatus {
    if !render.exists() {
        return RenderStatus::Missing;
    }
    match (modified(source), modified(render)) {
        (Some(source_time), Some(render_time)) if render_time >= source_time => {
            RenderStatus::Current
        }
        (Some(_), Some(_)) => RenderStatus::Stale,
        _ => RenderStatus::Present,
    }
}

pub fn sibling_render(source: &Path, format: RenderFormat) -> RenderArtifact {
    let path = source.with_extension(format.extension());
    RenderArtifact {
        format,
        status: render_status(source, &path),
        path,
    }
}

pub fn sibling_renders(source: &Path, formats: &[RenderFormat]) -> Vec<RenderArtifact> {
    formats
        .iter()
        .copied()
        .map(|format| sibling_render(source, format))
        .collect()
}

/// Discover D2 diagram artifacts under `<project>/diagrams`.
pub fn discover_d2_artifacts(project_root: &Path) -> Vec<VisualArtifact> {
    let diagrams_root = project_root.join("diagrams");
    let wrappers = discover_d2_wrappers(project_root);
    let mut sources = Vec::new();
    collect_d2_sources(&diagrams_root, &mut sources);
    sources.sort();

    sources
        .into_iter()
        .filter_map(|source| {
            let rel_source = source.strip_prefix(project_root).ok()?.to_path_buf();
            let title = source.file_name()?.to_string_lossy().into_owned();
            let wrapper_path = wrappers.iter().find_map(|(target, wrapper)| {
                if target == &rel_source || project_root.join(target) == source {
                    Some(wrapper.clone())
                } else {
                    None
                }
            });
            Some(VisualArtifact {
                kind: VisualArtifactKind::D2Diagram,
                title,
                source_path: rel_source,
                wrapper_path,
                renders: d2_renders(project_root, &source),
            })
        })
        .collect()
}

fn d2_renders(project_root: &Path, source: &Path) -> Vec<RenderArtifact> {
    [RenderFormat::Svg, RenderFormat::Png]
        .into_iter()
        .map(|format| d2_render(project_root, source, format))
        .collect()
}

fn d2_render(project_root: &Path, source: &Path, format: RenderFormat) -> RenderArtifact {
    let sibling = source.with_extension(format.extension());
    let canonical = canonical_d2_render_path(project_root, source, format);
    let path = if sibling.exists() { sibling } else { canonical };
    RenderArtifact {
        format,
        status: render_status(source, &path),
        path: path
            .strip_prefix(project_root)
            .unwrap_or(&path)
            .to_path_buf(),
    }
}

fn canonical_d2_render_path(project_root: &Path, source: &Path, format: RenderFormat) -> PathBuf {
    let stem = source.file_stem().unwrap_or_default();
    project_root
        .join("diagrams")
        .join("rendered")
        .join(stem)
        .with_extension(format.extension())
}

fn collect_d2_sources(dir: &Path, sources: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_d2_sources(&path, sources);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("d2") {
            sources.push(path);
        }
    }
}

fn discover_d2_wrappers(project_root: &Path) -> Vec<(PathBuf, PathBuf)> {
    let mut wrappers = Vec::new();
    collect_d2_wrappers(project_root, project_root, &mut wrappers);
    wrappers
}

fn collect_d2_wrappers(project_root: &Path, dir: &Path, wrappers: &mut Vec<(PathBuf, PathBuf)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_d2_wrappers(project_root, &path, wrappers);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            let Ok(body) = std::fs::read_to_string(&path) else {
                continue;
            };
            if let Some(target) = single_embed_target(body.trim()) {
                if target.extension().and_then(|ext| ext.to_str()) == Some("d2") {
                    if let Ok(wrapper) = path.strip_prefix(project_root) {
                        wrappers.push((target, wrapper.to_path_buf()));
                    }
                }
            }
        }
    }
}

fn single_embed_target(body: &str) -> Option<PathBuf> {
    let inner = body.strip_prefix("![[")?.strip_suffix("]]")?;
    if inner.contains('\n') || inner.is_empty() {
        return None;
    }
    Some(PathBuf::from(inner))
}

fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::thread;
    use std::time::Duration;
    use tempfile::TempDir;

    #[test]
    fn missing_render_is_missing() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("diagram.d2");
        fs::write(&source, "a -> b").unwrap();
        assert_eq!(
            render_status(&source, &tmp.path().join("diagram.svg")),
            RenderStatus::Missing
        );
    }

    #[test]
    fn newer_render_is_current() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("diagram.d2");
        let render = tmp.path().join("diagram.svg");
        fs::write(&source, "a -> b").unwrap();
        thread::sleep(Duration::from_millis(5));
        fs::write(&render, "<svg/>").unwrap();
        assert_eq!(render_status(&source, &render), RenderStatus::Current);
    }

    #[test]
    fn older_render_is_stale() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("diagram.d2");
        let render = tmp.path().join("diagram.svg");
        fs::write(&render, "<svg/>").unwrap();
        thread::sleep(Duration::from_millis(5));
        fs::write(&source, "a -> b").unwrap();
        assert_eq!(render_status(&source, &render), RenderStatus::Stale);
    }

    #[test]
    fn sibling_render_uses_format_extension() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("diagram.d2");
        fs::write(&source, "a -> b").unwrap();
        let render = sibling_render(&source, RenderFormat::Png);
        assert_eq!(render.path, tmp.path().join("diagram.png"));
        assert_eq!(render.format, RenderFormat::Png);
        assert_eq!(render.status, RenderStatus::Missing);
    }

    #[test]
    fn discovers_nested_d2_and_ignores_render_outputs() {
        let tmp = TempDir::new().unwrap();
        let nested = tmp.path().join("diagrams/system");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("flow.d2"), "a -> b").unwrap();
        fs::write(nested.join("flow.svg"), "<svg/>").unwrap();
        fs::write(nested.join("flow.png"), "png").unwrap();

        let artifacts = discover_d2_artifacts(tmp.path());
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].kind, VisualArtifactKind::D2Diagram);
        assert_eq!(
            artifacts[0].source_path,
            PathBuf::from("diagrams/system/flow.d2")
        );
    }

    #[test]
    fn d2_discovery_prefers_existing_sibling_render() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("diagrams");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("flow.d2"), "a -> b").unwrap();
        thread::sleep(Duration::from_millis(5));
        fs::write(dir.join("flow.svg"), "<svg/>").unwrap();

        let artifact = discover_d2_artifacts(tmp.path()).pop().unwrap();
        let svg = artifact
            .renders
            .iter()
            .find(|render| render.format == RenderFormat::Svg)
            .unwrap();
        assert_eq!(svg.path, PathBuf::from("diagrams/flow.svg"));
        assert_eq!(svg.status, RenderStatus::Current);
    }

    #[test]
    fn d2_discovery_uses_rendered_directory_when_sibling_absent() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("diagrams");
        fs::create_dir_all(dir.join("rendered")).unwrap();
        fs::write(dir.join("flow.d2"), "a -> b").unwrap();
        fs::write(dir.join("rendered/flow.png"), "png").unwrap();

        let artifact = discover_d2_artifacts(tmp.path()).pop().unwrap();
        let png = artifact
            .renders
            .iter()
            .find(|render| render.format == RenderFormat::Png)
            .unwrap();
        assert_eq!(png.path, PathBuf::from("diagrams/rendered/flow.png"));
        assert_ne!(png.status, RenderStatus::Missing);
    }

    #[test]
    fn d2_discovery_pairs_single_embed_wrapper() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("diagrams")).unwrap();
        fs::write(tmp.path().join("diagrams/flow.d2"), "a -> b").unwrap();
        fs::write(tmp.path().join("flow.md"), "![[diagrams/flow.d2]]").unwrap();

        let artifact = discover_d2_artifacts(tmp.path()).pop().unwrap();
        assert_eq!(artifact.wrapper_path, Some(PathBuf::from("flow.md")));
    }
}
