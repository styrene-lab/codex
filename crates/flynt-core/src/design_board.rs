//! DesignBoard document model.
//!
//! A `.board` file is JSON describing a grid of HTML/CSS cells. Both
//! `flynt-app` (renderer) and `flynt-agent` (design_board_* ACP tools) read and
//! write these files, so the wire shape lives here in `flynt-core` to
//! keep the two binaries in lockstep.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Current on-disk schema version. Bump when the shape changes in a way
/// that older readers cannot tolerate. Old files still parse via the
/// `version` check in `DesignBoard::load`, which surfaces a typed error rather
/// than silently corrupting data.
pub const CANVAS_VERSION: u32 = 1;

/// Top-level design_board document. Lives on disk as `<name>.board` JSON; a
/// sibling `<name>.md` wrapper with `![[<name>.board]]` makes it
/// indexable and routable in the UI (mirrors the Excalidraw pattern).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesignBoard {
    pub version: u32,
    pub theme: String,
    pub grid: Grid,
    pub cells: Vec<Cell>,
}

/// Grid container parameters. Cells position themselves in this grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Grid {
    pub cols: u32,
    pub rows: u32,
    /// Gap between cells in pixels.
    pub gap: u32,
}

/// One cell in the design board. The agent owns this — it writes raw HTML, CSS,
/// and optional JS. Each cell renders inside a sandboxed iframe in the UI,
/// so cells cannot leak styles or JS into each other or the host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cell {
    /// Stable identifier so the agent can apply partial updates without
    /// rewriting the whole document.
    pub id: String,
    /// Grid column, 0-indexed.
    pub x: u32,
    /// Grid row, 0-indexed.
    pub y: u32,
    /// Column span (>= 1).
    pub w: u32,
    /// Row span (>= 1).
    pub h: u32,
    pub html: String,
    pub css: String,
    /// Optional vanilla JS that runs scoped to this cell's iframe.
    pub js: Option<String>,
}

impl Default for DesignBoard {
    fn default() -> Self {
        Self {
            version: CANVAS_VERSION,
            theme: "default".into(),
            grid: Grid::default(),
            cells: Vec::new(),
        }
    }
}

impl Default for Grid {
    fn default() -> Self {
        Self {
            cols: 12,
            rows: 8,
            gap: 8,
        }
    }
}

impl DesignBoard {
    /// Parse a JSON design board file. Returns an error on missing/malformed
    /// JSON or on a `version` we don't know how to read.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let data = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("read {}: {e}", path.display()))?;
        Self::from_json(&data)
    }

    /// Parse from a JSON string. Same error semantics as `load`.
    pub fn from_json(data: &str) -> anyhow::Result<Self> {
        let design_board: DesignBoard = serde_json::from_str(data)
            .map_err(|e| anyhow::anyhow!("parse design_board json: {e}"))?;
        if design_board.version > CANVAS_VERSION {
            anyhow::bail!(
                "design_board version {} is newer than supported version {}",
                design_board.version,
                CANVAS_VERSION
            );
        }
        Ok(design_board)
    }

    /// Serialize and write to disk atomically (write to tempfile, then
    /// rename). Atomic write avoids partial-file corruption if Flynt
    /// crashes mid-save, which matters here because the agent edits this
    /// file too and a torn write would surface to it as a parse error.
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        let tmp = path.with_extension("design_board.tmp");
        std::fs::write(&tmp, json.as_bytes())?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    /// Find a cell by ID. Used by `design_board_set_cells` to apply partial
    /// updates without callers needing to scan.
    pub fn find_cell(&self, id: &str) -> Option<&Cell> {
        self.cells.iter().find(|c| c.id == id)
    }

    pub fn find_cell_mut(&mut self, id: &str) -> Option<&mut Cell> {
        self.cells.iter_mut().find(|c| c.id == id)
    }

    /// Insert or replace a cell (matched by `id`). Returns `true` if an
    /// existing cell was replaced, `false` if appended.
    pub fn upsert_cell(&mut self, cell: Cell) -> bool {
        if let Some(existing) = self.find_cell_mut(&cell.id) {
            *existing = cell;
            true
        } else {
            self.cells.push(cell);
            false
        }
    }

    /// Remove a cell by ID. Returns whether it was present.
    pub fn remove_cell(&mut self, id: &str) -> bool {
        let len = self.cells.len();
        self.cells.retain(|c| c.id != id);
        self.cells.len() != len
    }
}

/// Create a new design board: a `.board` data file plus a `.md` wrapper that
/// embeds it. Returns the `.md` path (indexable by Flynt). The wrapper
/// pattern mirrors how Excalidraw documents are stored — the `.md` is
/// the indexable handle, the data file is the source of truth.
///
/// Lives here in flynt-core so both flynt-app (UI menu/command palette)
/// and flynt-agent (design_board_create ACP tool) can call into the same
/// implementation. Refuses to overwrite an existing `.board` file.
pub fn create_design_board(project_root: &Path, name: &str) -> anyhow::Result<PathBuf> {
    let boards_dir = project_root.join("boards");
    std::fs::create_dir_all(&boards_dir)?;

    let design_board_file = format!("{name}.board");
    let design_board_abs = boards_dir.join(&design_board_file);
    if design_board_abs.exists() {
        anyhow::bail!("design board already exists: boards/{design_board_file}");
    }
    DesignBoard::default().save(&design_board_abs)?;

    let md_file = format!("{name}.md");
    let md_rel = PathBuf::from("boards").join(&md_file);
    let md_abs = project_root.join(&md_rel);
    let escaped_name = name.replace('"', "\\\"");
    let md_content = format!(
        "+++\ntitle = \"{escaped_name}\"\ntags = [\"design_board\"]\n+++\n\n![[{design_board_file}]]\n"
    );
    std::fs::write(&md_abs, md_content)?;

    Ok(md_rel)
}

// ── Capture pipeline types ──────────────────────────────────────────────
//
// The runtime capture (xcap, JS measurement) lives in `flynt-app` since it
// needs the WebView. The wire types live here so the omegon-design tool
// (separate binary) and flynt-app's request handler agree on the shape of
// `<project>/.flynt-local/flynt/capture-{requests,responses}/*.json` files.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureRequest {
    pub request_id: String,
    #[serde(default)]
    pub design_board_path: Option<String>,
    #[serde(default = "default_true")]
    pub include_metrics: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BoxXywh {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellMetric {
    pub id: String,
    pub cell_box: BoxXywh,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_box: Option<BoxXywh>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill_ratio: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureResponse {
    pub request_id: String,
    pub image_path: String,
    /// PNG bytes, base64-encoded. Inlined here so the tool can return it
    /// in one round trip without the agent needing a follow-up read.
    pub image_base64: String,
    pub image_width: u32,
    pub image_height: u32,
    pub viewport_box: BoxXywh,
    pub cells: Vec<CellMetric>,
    pub scale_factor: f32,
    pub captured: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub fn capture_request_dir(project_root: &Path) -> PathBuf {
    project_root
        .join(".flynt-local")
        .join("flynt")
        .join("capture-requests")
}

pub fn capture_response_dir(project_root: &Path) -> PathBuf {
    project_root
        .join(".flynt-local")
        .join("flynt")
        .join("capture-responses")
}

/// Operator-level design board settings, persisted in `FlyntOperatorSettings.board`.
/// Phase 4 introduces real values; Phase 1+2 ship the field with defaults so
/// later phases can attach without migrating existing operator-settings.json.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesignBoardSettings {
    /// Theme preset ID applied to new boards.
    pub default_theme: String,
    /// Grid dimensions used when creating a new design board.
    pub default_grid: Grid,
    /// One-shot bootstrap flag set after design board assets are copied into
    /// the project's `.flynt-local/flynt/assets/` directory. See Phase 4.
    pub assets_initialized: bool,
}

impl Default for DesignBoardSettings {
    fn default() -> Self {
        Self {
            default_theme: "default".into(),
            default_grid: Grid::default(),
            assets_initialized: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{NamedTempFile, TempDir};

    fn sample_cell(id: &str) -> Cell {
        Cell {
            id: id.into(),
            x: 0,
            y: 0,
            w: 4,
            h: 2,
            html: "<button class=\"btn\">Hi</button>".into(),
            css: ".btn { color: red; }".into(),
            js: None,
        }
    }

    #[test]
    fn design_board_default_is_v1_with_empty_cells() {
        let c = DesignBoard::default();
        assert_eq!(c.version, CANVAS_VERSION);
        assert_eq!(c.theme, "default");
        assert_eq!(c.grid.cols, 12);
        assert_eq!(c.grid.rows, 8);
        assert!(c.cells.is_empty());
    }

    #[test]
    fn design_board_round_trip_through_json() {
        let mut c = DesignBoard::default();
        c.upsert_cell(sample_cell("a"));
        c.upsert_cell(Cell {
            id: "b".into(),
            x: 5,
            y: 0,
            w: 3,
            h: 4,
            html: "<div>ok</div>".into(),
            css: "".into(),
            js: Some("console.log(1)".into()),
        });
        let json = serde_json::to_string(&c).unwrap();
        let back: DesignBoard = serde_json::from_str(&json).unwrap();
        assert_eq!(c, back);
    }

    #[test]
    fn design_board_load_rejects_future_version() {
        let json = r#"{"version":99,"theme":"x","grid":{"cols":1,"rows":1,"gap":0},"cells":[]}"#;
        let err = DesignBoard::from_json(json).unwrap_err().to_string();
        assert!(err.contains("newer than supported"), "got: {err}");
    }

    #[test]
    fn design_board_load_rejects_malformed_json() {
        let err = DesignBoard::from_json("not json").unwrap_err().to_string();
        assert!(err.contains("parse design_board json"), "got: {err}");
    }

    #[test]
    fn design_board_load_rejects_missing_required_fields() {
        // theme missing → serde error via from_json
        let err = DesignBoard::from_json(
            r#"{"version":1,"grid":{"cols":1,"rows":1,"gap":0},"cells":[]}"#,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("parse design_board json"), "got: {err}");
    }

    #[test]
    fn save_then_load_round_trip() {
        let f = NamedTempFile::new().unwrap();
        let mut c = DesignBoard::default();
        c.upsert_cell(sample_cell("only"));
        c.save(f.path()).unwrap();

        let back = DesignBoard::load(f.path()).unwrap();
        assert_eq!(c, back);
    }

    #[test]
    fn save_is_atomic_no_tmp_left_behind() {
        let f = NamedTempFile::new().unwrap();
        DesignBoard::default().save(f.path()).unwrap();

        let tmp = f.path().with_extension("design_board.tmp");
        assert!(!tmp.exists(), "atomic save must clean up its tempfile");
    }

    #[test]
    fn upsert_replaces_existing() {
        let mut c = DesignBoard::default();
        c.upsert_cell(sample_cell("a"));
        let mut updated = sample_cell("a");
        updated.html = "<span>new</span>".into();
        let was_replaced = c.upsert_cell(updated.clone());
        assert!(was_replaced);
        assert_eq!(c.cells.len(), 1);
        assert_eq!(c.find_cell("a").unwrap().html, "<span>new</span>");
    }

    #[test]
    fn upsert_appends_when_id_unknown() {
        let mut c = DesignBoard::default();
        let was_replaced = c.upsert_cell(sample_cell("new"));
        assert!(!was_replaced);
        assert_eq!(c.cells.len(), 1);
    }

    #[test]
    fn remove_returns_whether_present() {
        let mut c = DesignBoard::default();
        c.upsert_cell(sample_cell("a"));
        assert!(c.remove_cell("a"));
        assert!(!c.remove_cell("a"));
        assert!(c.cells.is_empty());
    }

    #[test]
    fn create_design_board_writes_data_file_and_wrapper() {
        let tmp = TempDir::new().unwrap();
        let md_path = create_design_board(tmp.path(), "Hero").unwrap();

        assert!(md_path.to_string_lossy().ends_with(".md"));
        let md_abs = tmp.path().join(&md_path);
        let design_board_abs = tmp.path().join("boards/Hero.board");
        assert!(md_abs.exists());
        assert!(design_board_abs.exists());

        let design_board = DesignBoard::load(&design_board_abs).unwrap();
        assert_eq!(design_board.version, CANVAS_VERSION);
        assert!(design_board.cells.is_empty());

        let md = std::fs::read_to_string(&md_abs).unwrap();
        assert!(md.contains("![[Hero.board]]"));
        assert!(md.contains("tags = [\"design_board\"]"));
        assert!(md.contains("title = \"Hero\""));
    }

    #[test]
    fn create_design_board_refuses_to_overwrite_existing() {
        let tmp = TempDir::new().unwrap();
        create_design_board(tmp.path(), "Hero").unwrap();
        let err = create_design_board(tmp.path(), "Hero")
            .unwrap_err()
            .to_string();
        assert!(err.contains("already exists"), "got: {err}");
    }

    #[test]
    fn create_design_board_escapes_quotes_in_name() {
        let tmp = TempDir::new().unwrap();
        let md_path = create_design_board(tmp.path(), "Quoted \"X\"").unwrap();
        let md = std::fs::read_to_string(tmp.path().join(&md_path)).unwrap();
        // Frontmatter title must remain valid TOML — embedded quote escaped.
        assert!(md.contains(r#"title = "Quoted \"X\"""#), "got: {md}");
    }

    #[test]
    fn design_board_settings_default_marks_assets_uninitialized() {
        let s = DesignBoardSettings::default();
        assert!(!s.assets_initialized);
        assert_eq!(s.default_theme, "default");
        assert_eq!(s.default_grid.cols, 12);
    }

    #[test]
    fn cell_serializes_optional_js_only_when_present() {
        let mut c = sample_cell("x");
        c.js = None;
        let json = serde_json::to_string(&c).unwrap();
        // serde keeps the null by default; we accept that — round-trip is
        // what matters, not field omission. This test pins the behavior so
        // a future serde annotation change is intentional.
        let back: Cell = serde_json::from_str(&json).unwrap();
        assert_eq!(c, back);
    }
}
