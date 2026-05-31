use std::path::Path;

pub fn render_excalidraw_preview_html(project_root: &Path, source_path: &Path) -> String {
    let source_abs = project_root.join(source_path);
    let svg_abs = source_abs.with_extension("svg");
    let title = source_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("drawing.excalidraw");
    let escaped_title = escape_html(title);

    if svg_abs.exists() {
        return match std::fs::read_to_string(&svg_abs) {
            Ok(svg) => format!(
                "<div class=\"excalidraw-artifact-preview\" data-drawing=\"{escaped_title}\"><div class=\"excalidraw-artifact-preview-frame\">{svg}</div></div>"
            ),
            Err(_) => preview_placeholder(&escaped_title, "SVG render could not be read"),
        };
    }

    if source_abs.exists() {
        preview_placeholder(&escaped_title, "No SVG preview yet. Use Edit, then Export SVG.")
    } else {
        preview_placeholder(&escaped_title, "Drawing source file is missing.")
    }
}

fn preview_placeholder(title: &str, message: &str) -> String {
    format!(
        "<div class=\"excalidraw-embed-placeholder\" data-drawing=\"{title}\"><strong>{title}</strong><br><span>{}</span></div>",
        escape_html(message)
    )
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn preview_uses_svg_sidecar_when_present() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("drawings")).unwrap();
        std::fs::write(tmp.path().join("drawings/sketch.excalidraw"), "{}").unwrap();
        std::fs::write(tmp.path().join("drawings/sketch.svg"), "<svg></svg>").unwrap();

        let html = render_excalidraw_preview_html(
            tmp.path(),
            &PathBuf::from("drawings/sketch.excalidraw"),
        );

        assert!(html.contains("<svg></svg>"));
        assert!(html.contains("excalidraw-artifact-preview-frame"));
        assert!(!html.contains("project://"));
    }

    #[test]
    fn preview_placeholder_is_not_a_clickable_project_link() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("drawings")).unwrap();
        std::fs::write(tmp.path().join("drawings/sketch.excalidraw"), "{}").unwrap();

        let html = render_excalidraw_preview_html(
            tmp.path(),
            &PathBuf::from("drawings/sketch.excalidraw"),
        );

        assert!(html.contains("No SVG preview yet"));
        assert!(!html.contains("project://"));
        assert!(!html.contains("<a "));
    }
}
