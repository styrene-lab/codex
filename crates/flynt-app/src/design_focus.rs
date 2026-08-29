use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const DESIGN_FOCUS_BRIDGE_JS: &str = r#"
(function(){
  if (window.__flyntDesignFocusBridgeInstalled) return;
  window.__flyntDesignFocusBridgeInstalled = true;
  window.addEventListener('message', function(e) {
    var d = e.data;
    if (!d || d.flyntDesignFocus !== true) return;
    dioxus.send(JSON.stringify(d));
  });
})();
"#;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignFocusBounds {
    pub x: i64,
    pub y: i64,
    pub w: i64,
    pub h: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignFocusEvent {
    pub event_type: String,
    pub focus_kind: String,
    pub cell_id: String,
    pub component: String,
    pub component_part: String,
    pub text: String,
}

impl DesignFocusEvent {
    pub fn label(&self) -> String {
        let component = self.component.trim();
        let part = self.component_part.trim();
        if !component.is_empty() && !part.is_empty() {
            format!("{}:{} ({})", component, part, self.cell_id)
        } else if !component.is_empty() {
            format!("{} ({})", component, self.cell_id)
        } else if !self.cell_id.is_empty() {
            format!("{} ({})", self.focus_kind, self.cell_id)
        } else {
            self.focus_kind.clone()
        }
    }

    pub fn to_state(&self, board_path: &Path) -> DesignFocusState {
        DesignFocusState {
            board_path: board_path.to_string_lossy().to_string(),
            cell_id: self.cell_id.clone(),
            focus_kind: self.focus_kind.clone(),
            component: non_empty(self.component.clone()),
            component_part: non_empty(self.component_part.clone()),
            text_excerpt: non_empty(self.text.clone()),
            bounds: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignFocusState {
    pub board_path: String,
    pub cell_id: String,
    pub focus_kind: String,
    pub component: Option<String>,
    pub component_part: Option<String>,
    pub text_excerpt: Option<String>,
    pub bounds: Option<DesignFocusBounds>,
}

pub fn write_active_focus(project_root: &Path, focus: &DesignFocusState) -> std::io::Result<()> {
    let dir = project_root.join(".flynt").join("local").join("flynt");
    std::fs::create_dir_all(&dir)?;
    let final_path = dir.join("design-focus.json");
    let tmp_path = dir.join("design-focus.json.tmp");
    let body = serde_json::to_vec_pretty(focus).map_err(std::io::Error::other)?;
    std::fs::write(&tmp_path, body)?;
    std::fs::rename(tmp_path, final_path)?;
    Ok(())
}

pub fn active_focus_path(project_root: &Path) -> PathBuf {
    project_root
        .join(".flynt")
        .join("local")
        .join("flynt")
        .join("design-focus.json")
}

fn non_empty(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn writes_active_focus_json() {
        let tmp = TempDir::new().unwrap();
        let focus = DesignFocusState {
            board_path: "boards/Test.board".into(),
            cell_id: "hero".into(),
            focus_kind: "component".into(),
            component: Some("Frame".into()),
            component_part: Some("root".into()),
            text_excerpt: Some("Hello".into()),
            bounds: Some(DesignFocusBounds {
                x: 1,
                y: 2,
                w: 3,
                h: 4,
            }),
        };
        write_active_focus(tmp.path(), &focus).unwrap();
        let body = std::fs::read_to_string(active_focus_path(tmp.path())).unwrap();
        assert!(body.contains("boards/Test.board"));
        assert!(body.contains("hero"));
        assert!(body.contains("Frame"));
    }

    #[test]
    fn event_label_prefers_component_and_cell() {
        let event = DesignFocusEvent {
            event_type: "select".into(),
            focus_kind: "component".into(),
            cell_id: "hero".into(),
            component: "Frame".into(),
            component_part: "root".into(),
            text: "Hello".into(),
        };
        assert_eq!(event.label(), "Frame:root (hero)");
    }

    #[test]
    fn event_converts_to_persisted_state() {
        let event = DesignFocusEvent {
            event_type: "hover".into(),
            focus_kind: "raw-cell".into(),
            cell_id: "raw".into(),
            component: "".into(),
            component_part: "root".into(),
            text: "Some text".into(),
        };
        let state = event.to_state(Path::new("boards/Test.board"));
        assert_eq!(state.board_path, "boards/Test.board");
        assert_eq!(state.cell_id, "raw");
        assert_eq!(state.component, None);
        assert_eq!(state.component_part.as_deref(), Some("root"));
        assert_eq!(state.text_excerpt.as_deref(), Some("Some text"));
    }
}
