use flynt_core::omegon_deployment::OmegonDeploymentManifest;

pub fn activate_skill(manifest: &mut OmegonDeploymentManifest, skill_id: &str) -> bool {
    let skill_id = skill_id.trim();
    if skill_id.is_empty()
        || manifest
            .activation
            .skills
            .iter()
            .any(|skill| skill == skill_id)
    {
        return false;
    }
    manifest.activation.skills.push(skill_id.to_string());
    true
}

pub fn deactivate_skill(manifest: &mut OmegonDeploymentManifest, skill_id: &str) -> bool {
    let skill_id = skill_id.trim();
    let default = OmegonDeploymentManifest::default();
    if default
        .activation
        .skills
        .iter()
        .any(|skill| skill == skill_id)
    {
        return false;
    }
    let before = manifest.activation.skills.len();
    manifest.activation.skills.retain(|skill| skill != skill_id);
    before != manifest.activation.skills.len()
}

pub fn save_manifest(
    omegon: &crate::bootstrap::OmegonRuntimeContext,
    manifest: &OmegonDeploymentManifest,
) -> anyhow::Result<()> {
    if let Some(parent) = omegon.deployment_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&omegon.deployment_path, manifest.to_toml_pretty()?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activates_missing_skill_once() {
        let mut manifest = OmegonDeploymentManifest::default();
        assert!(activate_skill(&mut manifest, "custom-skill"));
        assert!(!activate_skill(&mut manifest, "custom-skill"));
        assert_eq!(
            manifest
                .activation
                .skills
                .iter()
                .filter(|skill| *skill == "custom-skill")
                .count(),
            1
        );
    }

    #[test]
    fn deactivates_non_required_skill() {
        let mut manifest = OmegonDeploymentManifest::default();
        activate_skill(&mut manifest, "custom-skill");
        assert!(deactivate_skill(&mut manifest, "custom-skill"));
        assert!(
            !manifest
                .activation
                .skills
                .contains(&"custom-skill".to_string())
        );
    }

    #[test]
    fn does_not_deactivate_required_default_skill() {
        let mut manifest = OmegonDeploymentManifest::default();
        assert!(!deactivate_skill(&mut manifest, "vault"));
        assert!(manifest.activation.skills.contains(&"vault".to_string()));
    }
}
