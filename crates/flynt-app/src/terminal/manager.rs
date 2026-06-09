use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, anyhow};
use uuid::Uuid;

use super::types::{TerminalCreateParams, TerminalCreateResult, TerminalStatus};
use super::view::{AlacrittyTerminalSession, TerminalSnapshot};

const BACKEND: &str = "portable_pty+alacritty_terminal";

#[derive(Clone)]
pub struct TerminalManager {
    inner: Arc<Mutex<TerminalManagerInner>>,
    fallback_cwd: PathBuf,
    rows: usize,
    cols: usize,
}

struct TerminalManagerInner {
    sessions: HashMap<String, TerminalSessionRecord>,
}

struct TerminalSessionRecord {
    id: String,
    params: TerminalCreateParams,
    session: AlacrittyTerminalSession,
    rows: usize,
    cols: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalSessionInfo {
    pub terminal_id: String,
    pub title: String,
    pub command_line: String,
    pub status: TerminalStatus,
}

impl TerminalManager {
    pub fn new(fallback_cwd: impl Into<PathBuf>, rows: usize, cols: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(TerminalManagerInner {
                sessions: HashMap::new(),
            })),
            fallback_cwd: fallback_cwd.into(),
            rows,
            cols,
        }
    }

    pub fn create(&self, params: TerminalCreateParams) -> Result<TerminalCreateResult> {
        self.validate_create(&params)?;
        let terminal_id = self.terminal_id_for(&params);
        let actual_placement = params
            .placement
            .as_ref()
            .map(|placement| serde_json::to_value(placement).unwrap_or_default())
            .and_then(|value| value.as_str().map(str::to_string))
            .unwrap_or_else(|| "default".to_string());

        let mut inner = self.inner.lock().unwrap();
        if let Some(record) = inner.sessions.get(&terminal_id) {
            return Ok(TerminalCreateResult {
                terminal_id: record.id.clone(),
                backend: BACKEND.to_string(),
                actual_placement,
                warnings: vec!["reused existing terminal session".to_string()],
            });
        }

        let session = AlacrittyTerminalSession::spawn_with_params(
            params.clone(),
            Some(Path::new(&self.fallback_cwd)),
            self.rows,
            self.cols,
        )
        .with_context(|| {
            format!(
                "failed to create terminal '{}'; argv spawn failed",
                params.command
            )
        })?;

        inner.sessions.insert(
            terminal_id.clone(),
            TerminalSessionRecord {
                id: terminal_id.clone(),
                params,
                session,
                rows: self.rows,
                cols: self.cols,
            },
        );

        Ok(TerminalCreateResult {
            terminal_id,
            backend: BACKEND.to_string(),
            actual_placement,
            warnings: Vec::new(),
        })
    }

    pub fn send_input(&self, terminal_id: &str, input: &str) -> Result<()> {
        let inner = self.inner.lock().unwrap();
        let record = inner
            .sessions
            .get(terminal_id)
            .ok_or_else(|| anyhow!("terminal '{terminal_id}' was not found"))?;
        record.session.write_input(input);
        Ok(())
    }

    pub fn scroll_lines(&self, terminal_id: &str, lines: i32) -> Result<TerminalSnapshot> {
        let mut inner = self.inner.lock().unwrap();
        let record = inner
            .sessions
            .get_mut(terminal_id)
            .ok_or_else(|| anyhow!("terminal '{terminal_id}' was not found"))?;
        record.session.poll();
        record.session.scroll_lines(lines);
        Ok(record.session.snapshot(record.rows, record.cols))
    }

    pub fn poll_snapshot(&self, terminal_id: &str) -> Result<TerminalSnapshot> {
        let mut inner = self.inner.lock().unwrap();
        let record = inner
            .sessions
            .get_mut(terminal_id)
            .ok_or_else(|| anyhow!("terminal '{terminal_id}' was not found"))?;
        record.session.poll();
        Ok(record.session.snapshot(record.rows, record.cols))
    }

    pub fn resize(&self, terminal_id: &str, rows: usize, cols: usize) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        let record = inner
            .sessions
            .get_mut(terminal_id)
            .ok_or_else(|| anyhow!("terminal '{terminal_id}' was not found"))?;
        record.rows = rows.max(1);
        record.cols = cols.max(1);
        record.session.resize(record.rows, record.cols)
    }

    pub fn status(&self, terminal_id: &str) -> Result<TerminalStatus> {
        let inner = self.inner.lock().unwrap();
        let record = inner
            .sessions
            .get(terminal_id)
            .ok_or_else(|| anyhow!("terminal '{terminal_id}' was not found"))?;
        Ok(record.session.try_wait_status())
    }

    pub fn kill(&self, terminal_id: &str) -> Result<()> {
        let inner = self.inner.lock().unwrap();
        let record = inner
            .sessions
            .get(terminal_id)
            .ok_or_else(|| anyhow!("terminal '{terminal_id}' was not found"))?;
        record.session.kill()
    }

    pub fn release(&self, terminal_id: &str) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        let record = inner
            .sessions
            .remove(terminal_id)
            .ok_or_else(|| anyhow!("terminal '{terminal_id}' was not found"))?;
        // Releasing is a UI lifecycle operation. Ensure any still-running child is not
        // orphaned, but ignore kill failures for already-exited terminals.
        let _ = record.session.kill();
        Ok(())
    }

    pub fn release_all_exited(&self) -> usize {
        let mut inner = self.inner.lock().unwrap();
        let before = inner.sessions.len();
        inner.sessions.retain(|_, record| {
            matches!(record.session.try_wait_status(), TerminalStatus::Running)
        });
        before - inner.sessions.len()
    }

    pub fn list(&self) -> Vec<TerminalSessionInfo> {
        let inner = self.inner.lock().unwrap();
        inner
            .sessions
            .values()
            .map(|record| TerminalSessionInfo {
                terminal_id: record.id.clone(),
                title: record
                    .params
                    .title
                    .clone()
                    .unwrap_or_else(|| record.params.command.clone()),
                command_line: command_line(&record.params),
                status: record.session.try_wait_status(),
            })
            .collect()
    }

    fn validate_create(&self, params: &TerminalCreateParams) -> Result<()> {
        if params.command.trim().is_empty() {
            return Err(anyhow!("terminal command is required"));
        }
        if params.command.contains('\0') {
            return Err(anyhow!("terminal command must not contain NUL bytes"));
        }
        Ok(())
    }

    fn terminal_id_for(&self, params: &TerminalCreateParams) -> String {
        if let Some(reuse_key) = params
            .reuse_key
            .as_deref()
            .filter(|key| !key.trim().is_empty())
        {
            stable_terminal_id(reuse_key)
        } else {
            format!("term-{}", Uuid::new_v4())
        }
    }
}

fn stable_terminal_id(reuse_key: &str) -> String {
    let sanitized: String = reuse_key
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_lowercase();
    if sanitized.is_empty() {
        format!("term-{}", Uuid::new_v4())
    } else {
        format!("term-{sanitized}")
    }
}

fn command_line(params: &TerminalCreateParams) -> String {
    let mut parts = vec![params.command.clone()];
    parts.extend(params.args.clone());
    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_id_uses_reuse_key() {
        let manager = TerminalManager::new(".", 24, 80);
        let params = TerminalCreateParams::new("cargo").with_args(["check"]);
        assert!(manager.terminal_id_for(&params).starts_with("term-"));

        let mut reused = TerminalCreateParams::new("cargo").with_args(["check"]);
        reused.reuse_key = Some("cargo check".to_string());
        assert_eq!(manager.terminal_id_for(&reused), "term-cargo-check");
    }

    #[test]
    fn rejects_empty_and_nul_commands() {
        let manager = TerminalManager::new(".", 24, 80);
        assert!(
            manager
                .validate_create(&TerminalCreateParams::new(" "))
                .is_err()
        );
        assert!(
            manager
                .validate_create(&TerminalCreateParams::new("bad\0cmd"))
                .is_err()
        );
        assert!(
            manager
                .validate_create(&TerminalCreateParams::new("/bin/sh"))
                .is_ok()
        );
        assert!(
            manager
                .validate_create(&TerminalCreateParams::new("cargo"))
                .is_ok()
        );
    }
}
