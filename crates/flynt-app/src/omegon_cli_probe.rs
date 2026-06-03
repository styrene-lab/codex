use serde::{Deserialize, Serialize};
use std::{path::PathBuf, process::Stdio, time::Duration};
use tokio::process::Command;

use crate::omegon_cli_contract::OmegonCliContract;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OmegonCliProbeStatus {
    Compatible,
    Unknown,
    Incompatible,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OmegonCliProbeResult {
    pub binary: PathBuf,
    pub expected_contract_version: u32,
    pub status: OmegonCliProbeStatus,
    pub version: Option<String>,
    pub details: Vec<String>,
}

impl OmegonCliProbeResult {
    pub fn compatible(binary: PathBuf, version: Option<String>) -> Self {
        Self {
            binary,
            expected_contract_version: OmegonCliContract::current().version,
            status: OmegonCliProbeStatus::Compatible,
            version,
            details: Vec::new(),
        }
    }

    pub fn unknown(binary: PathBuf, details: Vec<String>) -> Self {
        Self {
            binary,
            expected_contract_version: OmegonCliContract::current().version,
            status: OmegonCliProbeStatus::Unknown,
            version: None,
            details,
        }
    }

    pub fn incompatible(binary: PathBuf, details: Vec<String>) -> Self {
        Self {
            binary,
            expected_contract_version: OmegonCliContract::current().version,
            status: OmegonCliProbeStatus::Incompatible,
            version: None,
            details,
        }
    }
}

pub async fn probe_omegon_cli(binary: PathBuf) -> OmegonCliProbeResult {
    let version = run_probe_command(&binary, &["--version"]).await;
    let acp_help = run_probe_command(&binary, &["acp", "--help"]).await;

    let version_text = match version {
        Ok(output) => first_nonempty_line(&output),
        Err(error) => {
            return OmegonCliProbeResult::incompatible(
                binary,
                vec![format!("failed to execute omegon --version: {error}")],
            );
        }
    };

    match acp_help {
        Ok(help) => {
            let mut details = Vec::new();
            if !help.contains("--cwd") {
                details.push("omegon acp --help does not advertise --cwd".into());
            }
            if !help.contains("--agent") {
                details.push("omegon acp --help does not advertise --agent".into());
            }
            if !help.contains("-y") && !help.contains("--yes") {
                details.push("omegon acp --help does not advertise consent flag -y/--yes".into());
            }

            if details.is_empty() {
                OmegonCliProbeResult::compatible(binary, version_text)
            } else {
                OmegonCliProbeResult {
                    binary,
                    expected_contract_version: OmegonCliContract::current().version,
                    status: OmegonCliProbeStatus::Unknown,
                    version: version_text,
                    details,
                }
            }
        }
        Err(error) => OmegonCliProbeResult {
            binary,
            expected_contract_version: OmegonCliContract::current().version,
            status: OmegonCliProbeStatus::Unknown,
            version: version_text,
            details: vec![format!("failed to execute omegon acp --help: {error}")],
        },
    }
}

async fn run_probe_command(binary: &PathBuf, args: &[&str]) -> anyhow::Result<String> {
    let child = Command::new(binary)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;

    let output = tokio::time::timeout(Duration::from_secs(5), child.wait_with_output()).await??;
    let mut text = String::new();
    text.push_str(&String::from_utf8_lossy(&output.stdout));
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    if output.status.success() {
        Ok(text)
    } else {
        Err(anyhow::anyhow!(
            "command exited with {}: {}",
            output.status,
            text.trim()
        ))
    }
}

fn first_nonempty_line(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(ToString::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_nonempty_line_skips_blanks() {
        assert_eq!(
            first_nonempty_line("\n\n omegon 0.1\n"),
            Some("omegon 0.1".into())
        );
        assert_eq!(first_nonempty_line("\n\t\n"), None);
    }

    #[test]
    fn probe_result_records_expected_contract_version() {
        let result = OmegonCliProbeResult::unknown("omegon".into(), vec!["missing".into()]);
        assert_eq!(
            result.expected_contract_version,
            OmegonCliContract::current().version
        );
        assert_eq!(result.status, OmegonCliProbeStatus::Unknown);
    }
}
