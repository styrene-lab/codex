use flynt_core::omegon_deployment::{
    FLYNT_DEPLOYMENT_CONTRACT_VERSION, FLYNT_DEPLOYMENT_EXTENSION, FLYNT_DEPLOYMENT_MEMORY_SCOPE,
    FLYNT_DEPLOYMENT_PROFILE, FLYNT_SURFACE_GUIDE_VERSION, OmegonDeploymentManifest,
};
use serde_json::Value;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeploymentStatus {
    Ok,
    Warning,
    Blocked,
    Unknown,
}

impl DeploymentStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ok => "Ready",
            Self::Warning => "Warning",
            Self::Blocked => "Blocked",
            Self::Unknown => "Unknown",
        }
    }

    pub fn class(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Warning => "warning",
            Self::Blocked => "blocked",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeploymentManifestSource {
    Loaded,
    Generated,
    MissingDefault,
    Invalid { error: String },
}

impl DeploymentManifestSource {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Loaded => "loaded",
            Self::Generated => "generated",
            Self::MissingDefault => "default",
            Self::Invalid { .. } => "invalid",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedDeploymentManifest {
    pub manifest: OmegonDeploymentManifest,
    pub source: DeploymentManifestSource,
}

impl LoadedDeploymentManifest {
    pub fn loaded(manifest: OmegonDeploymentManifest) -> Self {
        Self {
            manifest,
            source: DeploymentManifestSource::Loaded,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeploymentDiagnostic {
    pub status: DeploymentStatus,
    pub summary: String,
    pub details: Vec<String>,
}

pub fn classify_loaded_deployment(
    loaded: &LoadedDeploymentManifest,
    extension_initialize: Option<&Value>,
    project_root: &Path,
) -> DeploymentDiagnostic {
    classify_deployment(
        &loaded.manifest,
        extension_initialize,
        project_root,
        &loaded.source,
    )
}

pub fn classify_deployment(
    manifest: &OmegonDeploymentManifest,
    extension_initialize: Option<&Value>,
    project_root: &Path,
    source: &DeploymentManifestSource,
) -> DeploymentDiagnostic {
    let mut status = match source {
        DeploymentManifestSource::Loaded => DeploymentStatus::Ok,
        DeploymentManifestSource::Generated | DeploymentManifestSource::MissingDefault => {
            DeploymentStatus::Warning
        }
        DeploymentManifestSource::Invalid { .. } => DeploymentStatus::Blocked,
    };
    let mut details = Vec::new();

    match source {
        DeploymentManifestSource::Loaded => {}
        DeploymentManifestSource::Generated => {
            details.push("deployment manifest was synthesized from Flynt defaults".into());
        }
        DeploymentManifestSource::MissingDefault => {
            details.push("deployment manifest is missing; using in-memory Flynt defaults".into());
        }
        DeploymentManifestSource::Invalid { error } => {
            details.push(format!("deployment manifest is invalid: {error}"));
        }
    }

    if manifest.deployment.profile != FLYNT_DEPLOYMENT_PROFILE {
        status = DeploymentStatus::Blocked;
        details.push(format!(
            "expected profile {FLYNT_DEPLOYMENT_PROFILE}, got {}",
            manifest.deployment.profile
        ));
    }
    if manifest.deployment.memory_scope != FLYNT_DEPLOYMENT_MEMORY_SCOPE {
        status = DeploymentStatus::Blocked;
        details.push(format!(
            "expected project memory, got {}",
            manifest.deployment.memory_scope
        ));
    }
    if !manifest
        .activation
        .extensions
        .iter()
        .any(|extension| extension == FLYNT_DEPLOYMENT_EXTENSION)
    {
        status = DeploymentStatus::Blocked;
        details.push("flynt extension is not activated by the deployment manifest".into());
    }
    if manifest.deployment.capability_contract_version != FLYNT_DEPLOYMENT_CONTRACT_VERSION {
        status = DeploymentStatus::Blocked;
        details.push(format!(
            "capability contract mismatch: expected {}, got {}",
            FLYNT_DEPLOYMENT_CONTRACT_VERSION, manifest.deployment.capability_contract_version
        ));
    }
    if manifest.deployment.surface_guide_version != FLYNT_SURFACE_GUIDE_VERSION {
        status = DeploymentStatus::Blocked;
        details.push(format!(
            "surface guide mismatch: expected {}, got {}",
            FLYNT_SURFACE_GUIDE_VERSION, manifest.deployment.surface_guide_version
        ));
    }

    match extension_initialize {
        Some(init) => {
            let info = flynt_extension_info(init);
            if info["required_profile"].as_str() != Some(FLYNT_DEPLOYMENT_PROFILE) {
                status = DeploymentStatus::Blocked;
                details
                    .push("flynt extension did not report the required flynt-agent profile".into());
            }
            if info["project_root"].as_str() != Some(project_root.to_string_lossy().as_ref()) {
                status = DeploymentStatus::Blocked;
                details.push(
                    "flynt extension project root does not match the open Flynt project".into(),
                );
            }
            if info["capability_contract_version"].as_u64()
                != Some(FLYNT_DEPLOYMENT_CONTRACT_VERSION as u64)
            {
                status = DeploymentStatus::Blocked;
                details.push("flynt extension capability contract is incompatible".into());
            }
            if info["surface_guide_version"].as_u64() != Some(FLYNT_SURFACE_GUIDE_VERSION as u64) {
                status = DeploymentStatus::Blocked;
                details.push("flynt extension surface guide version is incompatible".into());
            }
        }
        None => {
            if status == DeploymentStatus::Ok {
                status = DeploymentStatus::Unknown;
            }
            details.push("flynt extension initialize metadata has not been observed yet".into());
            details.push("actual active profile and memory scope have not been verified".into());
        }
    }

    let summary = match status {
        DeploymentStatus::Ok => "Flynt ACP ready — scoped flynt-agent deployment is valid".into(),
        DeploymentStatus::Warning => {
            "Flynt ACP warning — deployment exists but provenance is incomplete".into()
        }
        DeploymentStatus::Blocked => {
            "Flynt ACP blocked — scoped deployment contract is violated".into()
        }
        DeploymentStatus::Unknown => {
            "Flynt ACP unknown — extension handshake has not completed".into()
        }
    };

    DeploymentDiagnostic {
        status,
        summary,
        details,
    }
}

fn flynt_extension_info(metadata: &Value) -> &Value {
    metadata
        .get("extension_info")
        .or_else(|| metadata.get("metadata").and_then(|value| value.get("extension_info")))
        .or_else(|| metadata.get("flynt").and_then(|value| value.get("extension_info")))
        .or_else(|| {
            metadata
                .get("omegon/extensions")
                .and_then(|value| value.get("flynt"))
                .and_then(|value| value.get("extension_info"))
        })
        .or_else(|| {
            metadata
                .get("_meta")
                .and_then(|value| value.get("flynt"))
                .and_then(|value| value.get("extension_info"))
        })
        .unwrap_or(&Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    fn init(root: &Path) -> Value {
        json!({
            "extension_info": {
                "required_profile": FLYNT_DEPLOYMENT_PROFILE,
                "project_root": root.to_string_lossy(),
                "capability_contract_version": FLYNT_DEPLOYMENT_CONTRACT_VERSION,
                "surface_guide_version": FLYNT_SURFACE_GUIDE_VERSION
            }
        })
    }

    #[test]
    fn matching_manifest_and_extension_is_ok() {
        let tmp = TempDir::new().unwrap();
        let manifest = OmegonDeploymentManifest::default();
        let diagnostic = classify_deployment(
            &manifest,
            Some(&init(tmp.path())),
            tmp.path(),
            &DeploymentManifestSource::Loaded,
        );
        assert_eq!(diagnostic.status, DeploymentStatus::Ok);
    }

    #[test]
    fn missing_extension_metadata_is_unknown() {
        let tmp = TempDir::new().unwrap();
        let manifest = OmegonDeploymentManifest::default();
        let diagnostic = classify_deployment(
            &manifest,
            None,
            tmp.path(),
            &DeploymentManifestSource::Loaded,
        );
        assert_eq!(diagnostic.status, DeploymentStatus::Unknown);
    }

    #[test]
    fn wrapped_extension_metadata_is_ok() {
        let tmp = TempDir::new().unwrap();
        let manifest = OmegonDeploymentManifest::default();
        let diagnostic = classify_deployment(
            &manifest,
            Some(&json!({ "metadata": init(tmp.path()) })),
            tmp.path(),
            &DeploymentManifestSource::Loaded,
        );
        assert_eq!(diagnostic.status, DeploymentStatus::Ok);
    }

    #[test]
    fn acp_flynt_alias_metadata_is_ok() {
        let tmp = TempDir::new().unwrap();
        let manifest = OmegonDeploymentManifest::default();
        let diagnostic = classify_deployment(
            &manifest,
            Some(&json!({ "flynt": init(tmp.path()) })),
            tmp.path(),
            &DeploymentManifestSource::Loaded,
        );
        assert_eq!(diagnostic.status, DeploymentStatus::Ok);
    }

    #[test]
    fn acp_namespaced_extension_metadata_is_ok() {
        let tmp = TempDir::new().unwrap();
        let manifest = OmegonDeploymentManifest::default();
        let diagnostic = classify_deployment(
            &manifest,
            Some(&json!({ "omegon/extensions": { "flynt": init(tmp.path()) } })),
            tmp.path(),
            &DeploymentManifestSource::Loaded,
        );
        assert_eq!(diagnostic.status, DeploymentStatus::Ok);
    }

    #[test]
    fn profile_mismatch_blocks() {
        let tmp = TempDir::new().unwrap();
        let mut manifest = OmegonDeploymentManifest::default();
        manifest.deployment.profile = "default".into();
        let diagnostic = classify_deployment(
            &manifest,
            Some(&init(tmp.path())),
            tmp.path(),
            &DeploymentManifestSource::Loaded,
        );
        assert_eq!(diagnostic.status, DeploymentStatus::Blocked);
    }

    #[test]
    fn project_root_mismatch_blocks() {
        let tmp = TempDir::new().unwrap();
        let other = TempDir::new().unwrap();
        let manifest = OmegonDeploymentManifest::default();
        let diagnostic = classify_deployment(
            &manifest,
            Some(&init(other.path())),
            tmp.path(),
            &DeploymentManifestSource::Loaded,
        );
        assert_eq!(diagnostic.status, DeploymentStatus::Blocked);
    }

    #[test]
    fn invalid_manifest_source_blocks() {
        let tmp = TempDir::new().unwrap();
        let manifest = OmegonDeploymentManifest::default();
        let diagnostic = classify_deployment(
            &manifest,
            Some(&init(tmp.path())),
            tmp.path(),
            &DeploymentManifestSource::Invalid {
                error: "bad toml".into(),
            },
        );
        assert_eq!(diagnostic.status, DeploymentStatus::Blocked);
        assert!(
            diagnostic
                .details
                .iter()
                .any(|detail| detail.contains("bad toml"))
        );
    }
}
