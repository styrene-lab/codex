use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Versioned action type for terminal creation.
pub const TERMINAL_CREATE_V1: &str = "terminal.create@1";

/// App-agnostic terminal creation request.
///
/// Mirrors Omegon's `terminal.create@1` shape so Flynt and Auspex can share a
/// terminal substrate without inventing a parallel action contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalCreateParams {
    /// Executable to launch. Hosts must apply manifest/runtime policy before execution.
    pub command: String,
    /// Argument vector. No shell-string variant exists in v1.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    /// Optional working directory request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    /// Optional environment additions. Host policy decides which keys may pass through.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    /// Optional human-readable terminal title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Optional advisory placement request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placement: Option<TerminalPlacement>,
    /// Optional origin-scoped reuse key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reuse_key: Option<String>,
}

impl TerminalCreateParams {
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            args: Vec::new(),
            cwd: None,
            env: BTreeMap::new(),
            title: None,
            placement: None,
            reuse_key: None,
        }
    }

    pub fn with_args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.args = args.into_iter().map(Into::into).collect();
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalPlacement {
    Default,
    SidePane,
    BottomPane,
    NewTab,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalCreateResult {
    pub terminal_id: String,
    pub backend: String,
    pub actual_placement: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalStatus {
    Running,
    Exited(String),
    Failed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_create_params_match_omegon_wire_shape() {
        let params = TerminalCreateParams {
            command: "cargo".to_string(),
            args: vec![
                "check".to_string(),
                "-p".to_string(),
                "flynt-app".to_string(),
            ],
            cwd: Some("${workspace}".to_string()),
            env: BTreeMap::from([("RUST_LOG".to_string(), "info".to_string())]),
            title: Some("Validate Flynt".to_string()),
            placement: Some(TerminalPlacement::BottomPane),
            reuse_key: Some("cargo-check".to_string()),
        };

        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["command"], "cargo");
        assert_eq!(json["args"][0], "check");
        assert_eq!(json["placement"], "bottom_pane");
        assert_eq!(json["reuse_key"], "cargo-check");

        let parsed: TerminalCreateParams = serde_json::from_value(json).unwrap();
        assert_eq!(parsed, params);
    }

    #[test]
    fn terminal_create_result_match_omegon_wire_shape() {
        let result = TerminalCreateResult {
            terminal_id: "term-1".to_string(),
            backend: "portable_pty".to_string(),
            actual_placement: "bottom_pane".to_string(),
            warnings: vec!["placement degraded".to_string()],
        };

        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["terminal_id"], "term-1");
        assert_eq!(json["backend"], "portable_pty");

        let parsed: TerminalCreateResult = serde_json::from_value(json).unwrap();
        assert_eq!(parsed, result);
    }
}
