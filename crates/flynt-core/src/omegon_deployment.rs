use serde::{Deserialize, Serialize};

pub const FLYNT_DEPLOYMENT_PROFILE: &str = "flynt-agent";
pub const FLYNT_DEPLOYMENT_EXTENSION: &str = "flynt";
pub const FLYNT_DEPLOYMENT_MEMORY_SCOPE: &str = "project";
pub const FLYNT_DEPLOYMENT_CONTRACT_VERSION: u32 = 1;
pub const FLYNT_SURFACE_GUIDE_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OmegonDeploymentManifest {
    pub deployment: DeploymentSection,
    pub activation: ActivationSection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeploymentSection {
    pub id: String,
    pub host: String,
    pub profile: String,
    pub memory_scope: String,
    pub capability_contract_version: u32,
    pub surface_guide_version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivationSection {
    pub extensions: Vec<String>,
    pub skills: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub optional_skills: Vec<String>,
}

impl Default for OmegonDeploymentManifest {
    fn default() -> Self {
        Self {
            deployment: DeploymentSection {
                id: "flynt-project".into(),
                host: "flynt-app".into(),
                profile: FLYNT_DEPLOYMENT_PROFILE.into(),
                memory_scope: FLYNT_DEPLOYMENT_MEMORY_SCOPE.into(),
                capability_contract_version: FLYNT_DEPLOYMENT_CONTRACT_VERSION,
                surface_guide_version: FLYNT_SURFACE_GUIDE_VERSION,
            },
            activation: ActivationSection {
                extensions: vec![FLYNT_DEPLOYMENT_EXTENSION.into()],
                skills: vec![
                    "vault".into(),
                    "d2-authoring".into(),
                    "excalidraw-authoring".into(),
                    "flynt-design".into(),
                ],
                optional_skills: vec!["git".into(), "openspec".into(), "security".into()],
            },
        }
    }
}

impl OmegonDeploymentManifest {
    pub fn to_toml_pretty(&self) -> anyhow::Result<String> {
        Ok(toml::to_string_pretty(self)?)
    }

    pub fn from_toml(content: &str) -> anyhow::Result<Self> {
        Ok(toml::from_str(content)?)
    }
}

pub fn merge_with_default_required_activation(
    mut manifest: OmegonDeploymentManifest,
) -> OmegonDeploymentManifest {
    let default = OmegonDeploymentManifest::default();
    for skill in default.activation.skills {
        if !manifest.activation.skills.iter().any(|existing| existing == &skill) {
            manifest.activation.skills.push(skill);
        }
    }
    for extension in default.activation.extensions {
        if !manifest.activation.extensions.iter().any(|existing| existing == &extension) {
            manifest.activation.extensions.push(extension);
        }
    }
    for skill in default.activation.optional_skills {
        if !manifest.activation.optional_skills.iter().any(|existing| existing == &skill) {
            manifest.activation.optional_skills.push(skill);
        }
    }
    manifest
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_manifest_pins_flynt_agent_scope() {
        let manifest = OmegonDeploymentManifest::default();
        assert_eq!(manifest.deployment.profile, FLYNT_DEPLOYMENT_PROFILE);
        assert_eq!(manifest.deployment.memory_scope, FLYNT_DEPLOYMENT_MEMORY_SCOPE);
        assert_eq!(manifest.deployment.host, "flynt-app");
        assert_eq!(manifest.activation.extensions, vec![FLYNT_DEPLOYMENT_EXTENSION]);
        assert!(manifest.activation.skills.contains(&"d2-authoring".to_string()));
        assert!(manifest.activation.skills.contains(&"excalidraw-authoring".to_string()));
    }

    #[test]
    fn manifest_round_trips_as_toml() {
        let manifest = OmegonDeploymentManifest::default();
        let toml = manifest.to_toml_pretty().unwrap();
        assert!(toml.contains("profile = \"flynt-agent\""));
        assert!(toml.contains("memory_scope = \"project\""));
        assert!(toml.contains("extensions = [\"flynt\"]"));
        let parsed = OmegonDeploymentManifest::from_toml(&toml).unwrap();
        assert_eq!(parsed, manifest);
    }

    #[test]
    fn merge_adds_new_required_skills_to_existing_manifest() {
        let mut manifest = OmegonDeploymentManifest::default();
        manifest.activation.skills = vec!["vault".into(), "flynt-design".into()];
        manifest.activation.optional_skills = vec!["git".into()];

        let merged = merge_with_default_required_activation(manifest);
        assert!(merged.activation.skills.contains(&"vault".to_string()));
        assert!(merged.activation.skills.contains(&"flynt-design".to_string()));
        assert!(merged.activation.skills.contains(&"d2-authoring".to_string()));
        assert!(merged.activation.skills.contains(&"excalidraw-authoring".to_string()));
        assert!(merged.activation.optional_skills.contains(&"security".to_string()));
    }

}
