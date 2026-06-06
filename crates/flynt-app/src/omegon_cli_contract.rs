use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OmegonCliContract {
    pub version: u32,
}

impl OmegonCliContract {
    pub const fn current() -> Self {
        Self { version: 1 }
    }

    pub fn acp_args(&self, cwd: &Path, agent_id: Option<&str>) -> Vec<String> {
        let mut args = vec![
            "acp".to_string(),
            "--cwd".to_string(),
            cwd.to_string_lossy().to_string(),
            "-y".to_string(),
            "--log-level".to_string(),
            "error".to_string(),
        ];
        if let Some(agent_id) = agent_id.filter(|id| !id.is_empty()) {
            args.push("--agent".to_string());
            args.push(agent_id.to_string());
        }
        args
    }

    pub fn auth_login_args(&self, provider: &str) -> Vec<String> {
        vec!["auth".into(), "login".into(), provider.into()]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliProbe {
    pub binary: PathBuf,
    pub expected_contract_version: u32,
}

impl CliProbe {
    pub fn new(binary: PathBuf) -> Self {
        Self {
            binary,
            expected_contract_version: OmegonCliContract::current().version,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acp_args_include_project_scope_and_consent() {
        let args = OmegonCliContract::current().acp_args(Path::new("/tmp/project"), None);
        assert_eq!(
            args,
            vec!["acp", "--cwd", "/tmp/project", "-y", "--log-level", "error"]
        );
    }

    #[test]
    fn acp_args_include_agent_when_selected() {
        let args =
            OmegonCliContract::current().acp_args(Path::new("/tmp/project"), Some("flynt-agent"));
        assert_eq!(
            args,
            vec![
                "acp",
                "--cwd",
                "/tmp/project",
                "-y",
                "--log-level",
                "error",
                "--agent",
                "flynt-agent"
            ]
        );
    }

    #[test]
    fn auth_login_args_are_centralized() {
        assert_eq!(
            OmegonCliContract::current().auth_login_args("anthropic"),
            vec!["auth", "login", "anthropic"]
        );
    }
}
