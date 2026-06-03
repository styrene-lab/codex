use crate::armory_resolution::is_skill_package;
use anyhow::{Context, bail};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledSkillPackage {
    pub id: String,
    pub destination: PathBuf,
}

pub fn skill_id_from_package(path: &Path) -> anyhow::Result<String> {
    if !is_skill_package(path) {
        bail!("not an Armory skill package: expected plugin.toml and SKILL.md");
    }
    if let Some(name) = path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
    {
        Ok(name.to_string())
    } else {
        bail!("skill package path has no valid directory name")
    }
}

pub fn install_user_skill_package(
    src: &Path,
    omegon_home: &Path,
) -> anyhow::Result<InstalledSkillPackage> {
    let id = skill_id_from_package(src)?;
    let destination = omegon_home.join("armory/skills").join(&id);
    copy_skill_package(src, &destination)?;
    Ok(InstalledSkillPackage { id, destination })
}

fn copy_skill_package(src: &Path, destination: &Path) -> anyhow::Result<()> {
    if destination.exists() {
        std::fs::remove_dir_all(destination)
            .with_context(|| format!("remove existing skill package {}", destination.display()))?;
    }
    std::fs::create_dir_all(destination)
        .with_context(|| format!("create skill package destination {}", destination.display()))?;
    copy_dir_contents(src, destination)
}

fn copy_dir_contents(src: &Path, dst: &Path) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(src).with_context(|| format!("read {}", src.display()))? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            std::fs::create_dir_all(&dst_path)
                .with_context(|| format!("create {}", dst_path.display()))?;
            copy_dir_contents(&src_path, &dst_path)?;
        } else if file_type.is_file() {
            std::fs::copy(&src_path, &dst_path).with_context(|| {
                format!("copy {} to {}", src_path.display(), dst_path.display())
            })?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_skill(root: &Path, name: &str) -> PathBuf {
        let dir = root.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("plugin.toml"), "[plugin]\ntype = \"skill\"\n").unwrap();
        std::fs::write(dir.join("SKILL.md"), "# Skill\n").unwrap();
        dir
    }

    #[test]
    fn rejects_non_skill_package() {
        let tmp = TempDir::new().unwrap();
        let err = skill_id_from_package(tmp.path()).unwrap_err().to_string();
        assert!(err.contains("not an Armory skill package"));
    }

    #[test]
    fn installs_skill_to_user_armory() {
        let tmp = TempDir::new().unwrap();
        let src = create_skill(&tmp.path().join("src"), "my-skill");
        std::fs::create_dir_all(src.join("examples")).unwrap();
        std::fs::write(src.join("examples/demo.md"), "demo").unwrap();

        let installed = install_user_skill_package(&src, &tmp.path().join("home")).unwrap();
        assert_eq!(installed.id, "my-skill");
        assert!(installed.destination.join("plugin.toml").exists());
        assert!(installed.destination.join("SKILL.md").exists());
        assert!(installed.destination.join("examples/demo.md").exists());
    }
}
