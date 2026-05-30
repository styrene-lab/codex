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
        (Some(source_time), Some(render_time)) if render_time >= source_time => RenderStatus::Current,
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
    formats.iter().copied().map(|format| sibling_render(source, format)).collect()
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
        assert_eq!(render_status(&source, &tmp.path().join("diagram.svg")), RenderStatus::Missing);
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
}
