use flynt_core::omegon_deployment::OmegonDeploymentManifest;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArmoryArtifactSource {
    ProjectOverride,
    UserArmory,
    DevCheckout,
    BundledFallback,
    Missing,
}

impl ArmoryArtifactSource {
    pub fn label(&self) -> &'static str {
        match self {
            Self::ProjectOverride => "project",
            Self::UserArmory => "user-armory",
            Self::DevCheckout => "dev-checkout",
            Self::BundledFallback => "bundled",
            Self::Missing => "missing",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArmorySkillResolution {
    pub name: String,
    pub source: ArmoryArtifactSource,
    pub path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArmoryResolutionReport {
    pub skills: Vec<ArmorySkillResolution>,
}

impl ArmoryResolutionReport {
    pub fn missing_required_skills(&self) -> Vec<String> {
        self.skills
            .iter()
            .filter(|skill| skill.source == ArmoryArtifactSource::Missing)
            .map(|skill| skill.name.clone())
            .collect()
    }
}

pub fn resolve_deployment_skills(
    manifest: &OmegonDeploymentManifest,
    project_root: &Path,
    omegon_home: &Path,
    bundled_root: Option<&Path>,
) -> ArmoryResolutionReport {
    resolve_deployment_skills_with_dev_root(
        manifest,
        project_root,
        omegon_home,
        bundled_root,
        default_dev_armory_root().as_deref(),
    )
}

pub fn resolve_deployment_skills_with_dev_root(
    manifest: &OmegonDeploymentManifest,
    project_root: &Path,
    omegon_home: &Path,
    bundled_root: Option<&Path>,
    dev_armory_root: Option<&Path>,
) -> ArmoryResolutionReport {
    let skills = manifest
        .activation
        .skills
        .iter()
        .map(|skill| {
            resolve_skill(
                skill,
                project_root,
                omegon_home,
                bundled_root,
                dev_armory_root,
            )
        })
        .collect();
    ArmoryResolutionReport { skills }
}

fn default_dev_armory_root() -> Option<PathBuf> {
    std::env::var("FLYNT_ARMORY_DEV_ROOT")
        .ok()
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
        .or_else(|| {
            dirs::home_dir()
                .map(|home| home.join("workspace/styrene-labs/omegon-armory"))
                .filter(|path| path.is_dir())
        })
}

fn resolve_skill(
    name: &str,
    project_root: &Path,
    omegon_home: &Path,
    bundled_root: Option<&Path>,
    dev_armory_root: Option<&Path>,
) -> ArmorySkillResolution {
    let candidates = [
        (
            ArmoryArtifactSource::ProjectOverride,
            project_root.join(".flynt/omegon/skills").join(name),
        ),
        (
            ArmoryArtifactSource::UserArmory,
            omegon_home.join("armory/skills").join(name),
        ),
    ];

    for (source, path) in candidates {
        if is_skill_package(&path) {
            return ArmorySkillResolution {
                name: name.into(),
                source,
                path: Some(path),
            };
        }
    }

    if let Some(root) = dev_armory_root {
        let path = root.join("skills").join(name);
        if is_skill_package(&path) {
            return ArmorySkillResolution {
                name: name.into(),
                source: ArmoryArtifactSource::DevCheckout,
                path: Some(path),
            };
        }
    }

    if let Some(root) = bundled_root {
        let path = root.join("skills").join(name);
        if is_skill_package(&path) {
            return ArmorySkillResolution {
                name: name.into(),
                source: ArmoryArtifactSource::BundledFallback,
                path: Some(path),
            };
        }
    }

    ArmorySkillResolution {
        name: name.into(),
        source: ArmoryArtifactSource::Missing,
        path: None,
    }
}

pub fn is_skill_package(path: &Path) -> bool {
    path.join("plugin.toml").is_file() && path.join("SKILL.md").is_file()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_skill(root: &Path, name: &str) {
        let dir = root.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("plugin.toml"), "[plugin]\ntype = \"skill\"\n").unwrap();
        std::fs::write(dir.join("SKILL.md"), "# Skill\n").unwrap();
    }

    #[test]
    fn project_override_wins_over_user_armory() {
        let tmp = TempDir::new().unwrap();
        let project = tmp.path().join("project");
        let home = tmp.path().join("home");
        create_skill(&project.join(".flynt/omegon/skills"), "d2-authoring");
        create_skill(&home.join("armory/skills"), "d2-authoring");

        let got = resolve_skill("d2-authoring", &project, &home, None, None);
        assert_eq!(got.source, ArmoryArtifactSource::ProjectOverride);
    }

    #[test]
    fn user_armory_wins_over_bundled_fallback() {
        let tmp = TempDir::new().unwrap();
        let project = tmp.path().join("project");
        let home = tmp.path().join("home");
        let bundled = tmp.path().join("bundled");
        create_skill(&home.join("armory/skills"), "d2-authoring");
        create_skill(&bundled.join("skills"), "d2-authoring");

        let got = resolve_skill("d2-authoring", &project, &home, Some(&bundled), None);
        assert_eq!(got.source, ArmoryArtifactSource::UserArmory);
    }

    #[test]
    fn dev_checkout_wins_over_bundled_fallback() {
        let tmp = TempDir::new().unwrap();
        let project = tmp.path().join("project");
        let home = tmp.path().join("home");
        let dev = tmp.path().join("omegon-armory");
        let bundled = tmp.path().join("bundled");
        create_skill(&dev.join("skills"), "d2-authoring");
        create_skill(&bundled.join("skills"), "d2-authoring");

        let got = resolve_skill("d2-authoring", &project, &home, Some(&bundled), Some(&dev));
        assert_eq!(got.source, ArmoryArtifactSource::DevCheckout);
    }

    #[test]
    fn reports_missing_required_skills() {
        let tmp = TempDir::new().unwrap();
        let mut manifest = OmegonDeploymentManifest::default();
        manifest.activation.skills = vec!["d2-authoring".into()];

        let report =
            resolve_deployment_skills_with_dev_root(&manifest, tmp.path(), tmp.path(), None, None);
        assert_eq!(report.missing_required_skills(), vec!["d2-authoring"]);
    }
}
