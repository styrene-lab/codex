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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualArtifactRef {
    pub kind: VisualArtifactKind,
    pub source_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactActionRequest {
    pub target: VisualArtifactRef,
    pub action: ArtifactActionKind,
    #[serde(default)]
    pub policy: ArtifactActionPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArtifactActionKind {
    Open,
    Edit,
    RevealSource,
    ShowDependencies,
    ShowConsumers,
    Render(RenderFormat),
    Inspect,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactActionPolicy {
    pub repair_wrapper: bool,
    pub prefer_wrapper: bool,
    pub allow_destructive: bool,
}

impl Default for ArtifactActionPolicy {
    fn default() -> Self {
        Self {
            repair_wrapper: true,
            prefer_wrapper: true,
            allow_destructive: false,
        }
    }
}

impl ArtifactActionRequest {
    pub fn open(target: VisualArtifactRef) -> Self {
        Self {
            target,
            action: ArtifactActionKind::Open,
            policy: ArtifactActionPolicy::default(),
        }
    }

    pub fn edit(target: VisualArtifactRef) -> Self {
        Self {
            target,
            action: ArtifactActionKind::Edit,
            policy: ArtifactActionPolicy::default(),
        }
    }

    pub fn reveal_source(target: VisualArtifactRef) -> Self {
        Self {
            target,
            action: ArtifactActionKind::RevealSource,
            policy: ArtifactActionPolicy {
                prefer_wrapper: false,
                ..ArtifactActionPolicy::default()
            },
        }
    }
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
    let wrappers = discover_wrappers(project_root, "d2");
    let mut sources = Vec::new();
    collect_sources_with_extension(&diagrams_root, "d2", &mut sources);
    sources.sort();

    sources
        .into_iter()
        .filter_map(|source| {
            let rel_source = source.strip_prefix(project_root).ok()?.to_path_buf();
            let title = source.file_name()?.to_string_lossy().into_owned();
            let wrapper_path = wrappers.iter().find_map(|(target, wrapper)| {
                if target == &rel_source || source_matches_embed(project_root, &source, target) {
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

/// Discover Excalidraw drawing artifacts under the drawings directory.
pub fn discover_excalidraw_artifacts(project_root: &Path) -> Vec<VisualArtifact> {
    let drawings_root = project_root.join("drawings");
    let wrappers = discover_wrappers(project_root, "excalidraw");
    let mut sources = Vec::new();
    collect_sources_with_extension(&drawings_root, "excalidraw", &mut sources);
    sources.sort();

    sources
        .into_iter()
        .filter_map(|source| {
            let rel_source = source.strip_prefix(project_root).ok()?.to_path_buf();
            let title = source.file_name()?.to_string_lossy().into_owned();
            let wrapper_path = wrappers.iter().find_map(|(target, wrapper)| {
                if target == &rel_source || source_matches_embed(project_root, &source, target) {
                    Some(wrapper.clone())
                } else {
                    None
                }
            });
            Some(VisualArtifact {
                kind: VisualArtifactKind::ExcalidrawDrawing,
                title,
                source_path: rel_source,
                wrapper_path,
                renders: rendered_dir_renders(
                    project_root,
                    "drawings",
                    &source,
                    &[RenderFormat::Svg, RenderFormat::Png],
                ),
            })
        })
        .collect()
}

/// Discover Flynt Design Board artifacts under the boards directory.
pub fn discover_design_board_artifacts(project_root: &Path) -> Vec<VisualArtifact> {
    let boards_root = project_root.join("boards");
    let wrappers = discover_wrappers(project_root, "board");
    let mut sources = Vec::new();
    collect_sources_with_extension(&boards_root, "board", &mut sources);
    sources.sort();

    sources
        .into_iter()
        .filter_map(|source| {
            let rel_source = source.strip_prefix(project_root).ok()?.to_path_buf();
            let title = source.file_name()?.to_string_lossy().into_owned();
            let wrapper_path = wrappers.iter().find_map(|(target, wrapper)| {
                if target == &rel_source || source_matches_embed(project_root, &source, target) {
                    Some(wrapper.clone())
                } else {
                    None
                }
            });
            Some(VisualArtifact {
                kind: VisualArtifactKind::DesignBoard,
                title,
                source_path: rel_source,
                wrapper_path,
                renders: rendered_dir_renders(
                    project_root,
                    "boards",
                    &source,
                    &[RenderFormat::Html, RenderFormat::Png],
                ),
            })
        })
        .collect()
}

/// Discover node-flow artifacts under the flows directory.
pub fn discover_flow_artifacts(project_root: &Path) -> Vec<VisualArtifact> {
    let flows_root = project_root.join("flows");
    let mut sources = Vec::new();
    collect_sources_with_extension(&flows_root, "flow", &mut sources);
    sources.sort();

    sources
        .into_iter()
        .filter_map(|source| {
            let rel_source = source.strip_prefix(project_root).ok()?.to_path_buf();
            let title = source.file_name()?.to_string_lossy().into_owned();
            Some(VisualArtifact {
                kind: VisualArtifactKind::Flow,
                title,
                source_path: rel_source,
                wrapper_path: None,
                renders: Vec::new(),
            })
        })
        .collect()
}

pub fn discover_design_board_consumed_artifacts(
    project_root: &Path,
    board_source_path: &Path,
) -> Vec<VisualArtifactRef> {
    let abs = project_root.join(board_source_path);
    let Ok(board) = crate::design_board::DesignBoard::load(&abs) else {
        return Vec::new();
    };
    let mut refs = Vec::new();
    for cell in &board.cells {
        let Some((html, css, js)) = cell.raw_html_parts() else {
            continue;
        };
        collect_consumed_refs(project_root, board_source_path, html, &mut refs);
        collect_consumed_refs(project_root, board_source_path, css, &mut refs);
        if let Some(js) = js {
            collect_consumed_refs(project_root, board_source_path, js, &mut refs);
        }
    }
    refs.sort_by(|a, b| a.source_path.cmp(&b.source_path));
    refs.dedup();
    refs
}

fn collect_consumed_refs(
    project_root: &Path,
    board_source_path: &Path,
    text: &str,
    refs: &mut Vec<VisualArtifactRef>,
) {
    for target in embed_targets(text) {
        let kind = match target.extension().and_then(|ext| ext.to_str()) {
            Some("d2") => VisualArtifactKind::D2Diagram,
            Some("excalidraw") => VisualArtifactKind::ExcalidrawDrawing,
            _ => continue,
        };
        if let Some(source_path) = resolve_artifact_ref(project_root, board_source_path, &target) {
            refs.push(VisualArtifactRef { kind, source_path });
        }
    }
}

fn embed_targets(text: &str) -> Vec<PathBuf> {
    let mut targets = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("![[") {
        let after = &rest[start + 3..];
        let Some(end) = after.find("]]") else {
            break;
        };
        let inner = &after[..end];
        let target = inner.split('|').next().unwrap_or(inner).trim();
        if !target.is_empty() {
            targets.push(PathBuf::from(target));
        }
        rest = &after[end + 2..];
    }
    targets
}

fn resolve_artifact_ref(
    project_root: &Path,
    owner_source_path: &Path,
    target: &Path,
) -> Option<PathBuf> {
    let candidates = [
        owner_source_path
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .join(target),
        target.to_path_buf(),
        PathBuf::from("diagrams").join(target),
        PathBuf::from("drawings").join(target),
    ];
    candidates
        .into_iter()
        .find(|candidate| project_root.join(candidate).exists())
}

fn rendered_dir_renders(
    project_root: &Path,
    artifact_dir: &str,
    source: &Path,
    formats: &[RenderFormat],
) -> Vec<RenderArtifact> {
    formats
        .iter()
        .copied()
        .map(|format| rendered_dir_render(project_root, artifact_dir, source, format))
        .collect()
}

fn rendered_dir_render(
    project_root: &Path,
    artifact_dir: &str,
    source: &Path,
    format: RenderFormat,
) -> RenderArtifact {
    let stem = source.file_stem().unwrap_or_default();
    let path = project_root
        .join(artifact_dir)
        .join("rendered")
        .join(stem)
        .with_extension(format.extension());
    RenderArtifact {
        format,
        status: render_status(source, &path),
        path: path
            .strip_prefix(project_root)
            .unwrap_or(&path)
            .to_path_buf(),
    }
}

fn collect_sources_with_extension(dir: &Path, extension: &str, sources: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_sources_with_extension(&path, extension, sources);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some(extension) {
            sources.push(path);
        }
    }
}

fn discover_wrappers(project_root: &Path, extension: &str) -> Vec<(PathBuf, PathBuf)> {
    let mut wrappers = Vec::new();
    collect_wrappers(project_root, project_root, extension, &mut wrappers);
    wrappers
}

fn collect_wrappers(
    project_root: &Path,
    dir: &Path,
    extension: &str,
    wrappers: &mut Vec<(PathBuf, PathBuf)>,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_wrappers(project_root, &path, extension, wrappers);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            let Ok(body) = std::fs::read_to_string(&path) else {
                continue;
            };
            if let Some(target) =
                single_embed_target(content_without_frontmatter(body.trim()).trim())
            {
                if target.extension().and_then(|ext| ext.to_str()) == Some(extension) {
                    if let Ok(wrapper) = path.strip_prefix(project_root) {
                        wrappers.push((target, wrapper.to_path_buf()));
                    }
                }
            }
        }
    }
}

fn source_matches_embed(project_root: &Path, source: &Path, target: &Path) -> bool {
    project_root.join(target) == source
        || source
            .parent()
            .is_some_and(|parent| parent.join(target) == source)
}

fn content_without_frontmatter(content: &str) -> &str {
    let Some(rest) = content.strip_prefix("+++") else {
        return content;
    };
    let Some(end) = rest.find("\n+++") else {
        return content;
    };

    let after_closing_delimiter = end + "\n+++".len();
    rest.get(after_closing_delimiter..).unwrap_or("")
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
    fn content_without_frontmatter_handles_frontmatter_only_note() {
        let content = "+++\nid = \"af1853be-836c-4c2a-b8cd-d4968ec29b60\"\ntags = []\naliases = []\nimported_reference = false\n\n[publication]\nenabled = false\nvisibility = \"private\"\n+++";

        assert_eq!(content_without_frontmatter(content), "");
    }

    #[test]
    fn content_without_frontmatter_returns_body_after_closing_delimiter() {
        let content = "+++\ntitle = \"Map\"\n+++\n\n![[map.excalidraw]]\n";

        assert_eq!(content_without_frontmatter(content), "\n\n![[map.excalidraw]]\n");
    }

    #[test]
    fn content_without_frontmatter_leaves_regular_markdown_unchanged() {
        let content = "# Title\n\nBody";

        assert_eq!(content_without_frontmatter(content), content);
    }

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

    #[test]
    fn excalidraw_discovery_pairs_wrapper_and_render_exports() {
        let tmp = TempDir::new().unwrap();
        let drawings = tmp.path().join("drawings");
        fs::create_dir_all(drawings.join("rendered")).unwrap();
        fs::write(drawings.join("map.excalidraw"), "{}").unwrap();
        thread::sleep(Duration::from_millis(5));
        fs::write(drawings.join("rendered/map.svg"), "<svg/>").unwrap();
        fs::write(
            drawings.join("map.md"),
            "+++\ntitle = \"Map\"\ntags = [\"drawing\"]\n+++\n\n![[map.excalidraw]]\n",
        )
        .unwrap();

        let artifacts = discover_excalidraw_artifacts(tmp.path());
        assert_eq!(artifacts.len(), 1);
        let artifact = &artifacts[0];
        assert_eq!(artifact.kind, VisualArtifactKind::ExcalidrawDrawing);
        assert_eq!(
            artifact.source_path,
            PathBuf::from("drawings/map.excalidraw")
        );
        assert_eq!(
            artifact.wrapper_path,
            Some(PathBuf::from("drawings/map.md"))
        );
        let svg = artifact
            .renders
            .iter()
            .find(|render| render.format == RenderFormat::Svg)
            .unwrap();
        assert_eq!(svg.path, PathBuf::from("drawings/rendered/map.svg"));
        assert_eq!(svg.status, RenderStatus::Current);
    }

    #[test]
    fn excalidraw_discovery_ignores_render_outputs_as_primary_artifacts() {
        let tmp = TempDir::new().unwrap();
        let drawings = tmp.path().join("drawings");
        fs::create_dir_all(drawings.join("rendered")).unwrap();
        fs::write(drawings.join("rendered/map.svg"), "<svg/>").unwrap();
        fs::write(drawings.join("rendered/map.png"), "png").unwrap();

        assert!(discover_excalidraw_artifacts(tmp.path()).is_empty());
    }

    #[test]
    fn wrapper_pairing_supports_project_relative_and_sibling_embeds() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("drawings/nested")).unwrap();
        fs::write(tmp.path().join("drawings/nested/local.excalidraw"), "{}").unwrap();
        fs::write(
            tmp.path().join("drawings/nested/local.md"),
            "![[local.excalidraw]]",
        )
        .unwrap();
        fs::write(tmp.path().join("drawings/global.excalidraw"), "{}").unwrap();
        fs::write(
            tmp.path().join("global.md"),
            "![[drawings/global.excalidraw]]",
        )
        .unwrap();

        let artifacts = discover_excalidraw_artifacts(tmp.path());
        let local = artifacts
            .iter()
            .find(|artifact| {
                artifact.source_path == PathBuf::from("drawings/nested/local.excalidraw")
            })
            .unwrap();
        assert_eq!(
            local.wrapper_path,
            Some(PathBuf::from("drawings/nested/local.md"))
        );
        let global = artifacts
            .iter()
            .find(|artifact| artifact.source_path == PathBuf::from("drawings/global.excalidraw"))
            .unwrap();
        assert_eq!(global.wrapper_path, Some(PathBuf::from("global.md")));
    }

    #[test]
    fn design_board_discovery_pairs_wrapper_and_exports() {
        let tmp = TempDir::new().unwrap();
        let boards = tmp.path().join("boards");
        fs::create_dir_all(boards.join("rendered")).unwrap();
        fs::write(
            boards.join("hero.board"),
            r#"{"version":1,"theme":"default","grid":{"cols":12,"rows":8,"gap":8},"cells":[]}"#,
        )
        .unwrap();
        thread::sleep(Duration::from_millis(5));
        fs::write(boards.join("rendered/hero.html"), "<html></html>").unwrap();
        fs::write(
            boards.join("hero.md"),
            "+++\ntitle = \"Hero\"\ntags = [\"design_board\"]\n+++\n\n![[hero.board]]\n",
        )
        .unwrap();

        let artifacts = discover_design_board_artifacts(tmp.path());
        assert_eq!(artifacts.len(), 1);
        let artifact = &artifacts[0];
        assert_eq!(artifact.kind, VisualArtifactKind::DesignBoard);
        assert_eq!(artifact.source_path, PathBuf::from("boards/hero.board"));
        assert_eq!(artifact.wrapper_path, Some(PathBuf::from("boards/hero.md")));
        let html = artifact
            .renders
            .iter()
            .find(|render| render.format == RenderFormat::Html)
            .unwrap();
        assert_eq!(html.path, PathBuf::from("boards/rendered/hero.html"));
        assert_eq!(html.status, RenderStatus::Current);
    }

    #[test]
    fn design_board_consumed_artifacts_detects_d2_and_excalidraw_embeds() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("boards")).unwrap();
        fs::create_dir_all(tmp.path().join("diagrams")).unwrap();
        fs::create_dir_all(tmp.path().join("drawings")).unwrap();
        fs::write(tmp.path().join("diagrams/flow.d2"), "a -> b").unwrap();
        fs::write(tmp.path().join("drawings/sketch.excalidraw"), "{}").unwrap();
        fs::write(
            tmp.path().join("boards/hero.board"),
            r#"{"version":1,"theme":"default","grid":{"cols":12,"rows":8,"gap":8},"cells":[{"id":"a","x":0,"y":0,"w":1,"h":1,"content":{"kind":"html","html":"![[diagrams/flow.d2]] ![[drawings/sketch.excalidraw]]"}}]}"#,
        )
        .unwrap();

        let refs =
            discover_design_board_consumed_artifacts(tmp.path(), Path::new("boards/hero.board"));
        assert_eq!(refs.len(), 2);
        assert!(refs.contains(&VisualArtifactRef {
            kind: VisualArtifactKind::D2Diagram,
            source_path: PathBuf::from("diagrams/flow.d2")
        }));
        assert!(refs.contains(&VisualArtifactRef {
            kind: VisualArtifactKind::ExcalidrawDrawing,
            source_path: PathBuf::from("drawings/sketch.excalidraw")
        }));
    }

    #[test]
    fn discovers_flow_artifacts() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("flows")).unwrap();
        fs::write(tmp.path().join("flows/auth.flow"), "{}").unwrap();

        let artifacts = discover_flow_artifacts(tmp.path());
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].kind, VisualArtifactKind::Flow);
        assert_eq!(artifacts[0].source_path, PathBuf::from("flows/auth.flow"));
        assert!(artifacts[0].wrapper_path.is_none());
    }
}
