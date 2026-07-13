//! Cloud sync provider detection — finds locally-installed sync folders.
//!
//! Detects iCloud Drive, Google Drive, Dropbox, and OneDrive by checking
//! for their local sync directories. No API keys or authentication needed —
//! the provider's desktop client handles sync transparently.

use std::path::PathBuf;

/// A detected cloud sync provider.
#[derive(Debug, Clone, PartialEq)]
pub struct CloudProvider {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    /// The local sync root directory.
    pub sync_root: PathBuf,
}

/// Detect all available cloud sync providers on this machine.
pub fn detect_providers() -> Vec<CloudProvider> {
    let mut providers = Vec::new();

    if let Some(p) = detect_icloud() {
        providers.push(p);
    }
    if let Some(p) = detect_google_drive() {
        providers.push(p);
    }
    if let Some(p) = detect_dropbox() {
        providers.push(p);
    }
    if let Some(p) = detect_onedrive() {
        providers.push(p);
    }

    providers
}

/// Get the project path within a cloud provider's sync directory.
pub fn project_path_for_provider(provider: &CloudProvider, project_name: &str) -> PathBuf {
    provider.sync_root.join(project_name)
}

/// Create a project inside a cloud provider's sync directory.
pub fn create_cloud_project(
    provider: &CloudProvider,
    project_name: &str,
) -> anyhow::Result<PathBuf> {
    let project_root = project_path_for_provider(provider, project_name);
    if project_root.exists() {
        anyhow::bail!(
            "Project '{}' already exists in {}",
            project_name,
            provider.label
        );
    }
    std::fs::create_dir_all(project_root.join(".flynt"))?;

    let sync = if provider.id == "icloud" {
        flynt_core::models::SyncConfig::ICloud
    } else {
        // Filesystem sync providers replicate the project folder themselves.
        flynt_core::models::SyncConfig::None
    };
    let config = flynt_core::models::ProjectConfig {
        project_name: project_name.to_string(),
        sync,
        ..flynt_core::models::ProjectConfig::default()
    };
    std::fs::write(
        project_root.join(".flynt/config.toml"),
        toml::to_string_pretty(&config)?,
    )?;

    Ok(project_root)
}

// ── Provider detection ──────────────────────────────────────────────────────

fn detect_icloud() -> Option<CloudProvider> {
    #[cfg(target_os = "macos")]
    {
        let path = dirs::home_dir()?.join("Library/Mobile Documents/com~apple~CloudDocs");
        if path.is_dir() {
            return Some(CloudProvider {
                id: "icloud",
                label: "iCloud Drive",
                description: "Syncs automatically between Apple devices",
                sync_root: path,
            });
        }
    }
    None
}

fn detect_google_drive() -> Option<CloudProvider> {
    let home = dirs::home_dir()?;

    // macOS: Google Drive for Desktop
    #[cfg(target_os = "macos")]
    {
        let cloud_storage = home.join("Library/CloudStorage");
        if cloud_storage.is_dir()
            && let Ok(entries) = std::fs::read_dir(&cloud_storage)
        {
            for entry in entries.filter_map(|e| e.ok()) {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("GoogleDrive") {
                    let my_drive = entry.path().join("My Drive");
                    if my_drive.is_dir() {
                        return Some(CloudProvider {
                            id: "google-drive",
                            label: "Google Drive",
                            description: "15 GB free, syncs across all platforms",
                            sync_root: my_drive,
                        });
                    }
                }
            }
        }
    }

    // Linux: common mount points
    #[cfg(target_os = "linux")]
    {
        for candidate in [
            home.join("Google Drive"),
            home.join("google-drive"),
            PathBuf::from("/mnt/gdrive"),
        ] {
            if candidate.is_dir() {
                return Some(CloudProvider {
                    id: "google-drive",
                    label: "Google Drive",
                    description: "15 GB free, syncs across all platforms",
                    sync_root: candidate,
                });
            }
        }
    }

    None
}

fn detect_dropbox() -> Option<CloudProvider> {
    let home = dirs::home_dir()?;

    // Check common Dropbox locations
    for candidate in [
        home.join("Dropbox"),
        #[cfg(target_os = "macos")]
        home.join("Library/CloudStorage/Dropbox"),
    ] {
        if candidate.is_dir() {
            return Some(CloudProvider {
                id: "dropbox",
                label: "Dropbox",
                description: "2 GB free, widely supported",
                sync_root: candidate,
            });
        }
    }

    None
}

fn detect_onedrive() -> Option<CloudProvider> {
    let home = dirs::home_dir()?;

    // macOS: OneDrive desktop client
    #[cfg(target_os = "macos")]
    {
        let cloud_storage = home.join("Library/CloudStorage");
        if cloud_storage.is_dir()
            && let Ok(entries) = std::fs::read_dir(&cloud_storage)
        {
            for entry in entries.filter_map(|e| e.ok()) {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("OneDrive") {
                    return Some(CloudProvider {
                        id: "onedrive",
                        label: "OneDrive",
                        description: "5 GB free, included with Microsoft 365",
                        sync_root: entry.path(),
                    });
                }
            }
        }
    }

    // Linux / fallback
    for candidate in [home.join("OneDrive"), home.join("onedrive")] {
        if candidate.is_dir() {
            return Some(CloudProvider {
                id: "onedrive",
                label: "OneDrive",
                description: "5 GB free, included with Microsoft 365",
                sync_root: candidate,
            });
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_path_for_provider_appends_name() {
        let provider = CloudProvider {
            id: "test",
            label: "Test",
            description: "test provider",
            sync_root: PathBuf::from("/cloud/sync"),
        };
        let path = project_path_for_provider(&provider, "MyProject");
        assert_eq!(path, PathBuf::from("/cloud/sync/MyProject"));
    }

    #[test]
    fn create_cloud_project_creates_directory_and_config() {
        let tmp = tempfile::TempDir::new().unwrap();
        let provider = CloudProvider {
            id: "test",
            label: "Test Cloud",
            description: "test",
            sync_root: tmp.path().to_path_buf(),
        };

        let result = create_cloud_project(&provider, "TestProject").unwrap();
        assert!(result.exists());
        assert!(result.join(".flynt/config.toml").exists());

        let config = std::fs::read_to_string(result.join(".flynt/config.toml")).unwrap();
        assert!(config.contains("TestProject"));
        assert!(config.contains("backend = \"none\""));
        let parsed: flynt_core::models::ProjectConfig = toml::from_str(&config).unwrap();
        assert!(matches!(parsed.sync, flynt_core::models::SyncConfig::None));
    }

    #[test]
    fn create_icloud_cloud_project_writes_parseable_backend() {
        let tmp = tempfile::TempDir::new().unwrap();
        let provider = CloudProvider {
            id: "icloud",
            label: "iCloud Drive",
            description: "test",
            sync_root: tmp.path().to_path_buf(),
        };

        let result = create_cloud_project(&provider, "Synced").unwrap();
        let config = std::fs::read_to_string(result.join(".flynt/config.toml")).unwrap();
        assert!(config.contains("backend = \"i_cloud\""));
        let parsed: flynt_core::models::ProjectConfig = toml::from_str(&config).unwrap();
        assert!(matches!(
            parsed.sync,
            flynt_core::models::SyncConfig::ICloud
        ));
    }

    #[test]
    fn create_cloud_project_rejects_existing() {
        let tmp = tempfile::TempDir::new().unwrap();
        let provider = CloudProvider {
            id: "test",
            label: "Test",
            description: "test",
            sync_root: tmp.path().to_path_buf(),
        };

        create_cloud_project(&provider, "Existing").unwrap();
        let result = create_cloud_project(&provider, "Existing");
        assert!(result.is_err());
    }

    #[test]
    fn detect_providers_does_not_panic() {
        let providers = detect_providers();
        for p in &providers {
            assert!(!p.id.is_empty());
            assert!(!p.label.is_empty());
        }
    }
}
