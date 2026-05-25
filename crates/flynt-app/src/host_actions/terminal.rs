//! terminal.create@1 HostAction parsing.

use crate::terminal::{TERMINAL_CREATE_V1, TerminalCreateParams};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalCreateReview {
    pub params: TerminalCreateParams,
    pub summary: String,
}

pub fn extract_terminal_create(raw: Option<&serde_json::Value>) -> Option<TerminalCreateReview> {
    let raw = raw?;
    let params_value = if raw.get("action").and_then(|v| v.as_str()) == Some(TERMINAL_CREATE_V1) {
        raw.get("params")?
    } else if raw.get("type").and_then(|v| v.as_str()) == Some(TERMINAL_CREATE_V1) {
        raw.get("params")?
    } else if raw.get("command").is_some() {
        raw
    } else {
        return None;
    };

    let params: TerminalCreateParams = serde_json::from_value(params_value.clone()).ok()?;
    if params.command.trim().is_empty() {
        return None;
    }
    Some(TerminalCreateReview {
        summary: summarize_terminal_create(&params),
        params,
    })
}

pub fn summarize_terminal_create(params: &TerminalCreateParams) -> String {
    let mut line = params.command.clone();
    for arg in &params.args {
        line.push(' ');
        line.push_str(arg);
    }
    let mut parts = vec![format!("command: {line}")];
    if let Some(cwd) = &params.cwd {
        parts.push(format!("cwd: {cwd}"));
    }
    if let Some(placement) = &params.placement {
        parts.push(format!("placement: {placement:?}"));
    }
    if let Some(reuse_key) = &params.reuse_key {
        parts.push(format!("reuse: {reuse_key}"));
    }
    parts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn detects_wrapped_terminal_create_action() {
        let raw = json!({
            "action": "terminal.create@1",
            "params": {
                "command": "cargo",
                "args": ["check", "-p", "flynt-app"],
                "placement": "bottom_pane",
                "reuse_key": "cargo-check"
            }
        });

        let review = extract_terminal_create(Some(&raw)).unwrap();
        assert_eq!(review.params.command, "cargo");
        assert_eq!(review.params.args, vec!["check", "-p", "flynt-app"]);
        assert!(review.summary.contains("cargo check -p flynt-app"));
    }

    #[test]
    fn detects_direct_terminal_create_params() {
        let raw = json!({ "command": "cargo", "args": ["test"] });
        let review = extract_terminal_create(Some(&raw)).unwrap();
        assert_eq!(review.params.command, "cargo");
    }

    #[test]
    fn rejects_shell_string_without_command_field() {
        let raw = json!({ "cmd": "cargo check -p flynt-app" });
        assert!(extract_terminal_create(Some(&raw)).is_none());
    }

    #[test]
    fn rejects_unrelated_raw_input() {
        let raw = json!({ "path": "README.md" });
        assert!(extract_terminal_create(Some(&raw)).is_none());
    }
}
