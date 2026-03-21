use anyhow::{Context, Result, bail};
use reqwest::Client;
use serde::Deserialize;
use std::path::Path;

use super::config::Config;
use super::download::{ProgressFn, download_file, fetch_json, unique_download_path};

const GITHUB_RELEASES_URL: &str =
    "https://api.github.com/repos/thepeacockproject/Peacock/releases/latest";

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

/// Information about available Peacock versions.
#[derive(Debug, Clone)]
pub struct PeacockStatus {
    pub installed_version: Option<String>,
    pub latest_version: Option<String>,
}

impl PeacockStatus {
    #[allow(dead_code)]
    pub fn update_available(&self) -> bool {
        match (&self.installed_version, &self.latest_version) {
            (Some(installed), Some(latest)) => installed != latest,
            (None, Some(_)) => true,
            _ => false,
        }
    }

    #[allow(dead_code)]
    pub fn is_installed(&self) -> bool {
        self.installed_version.is_some()
    }
}

/// Check the installed vs latest Peacock versions.
/// If no version is tracked in config but the Peacock directory exists on disk
/// (e.g. after migrating an old install), report as "unknown" version.
pub async fn check_status(client: &Client, config: &Config) -> PeacockStatus {
    let installed_version = match &config.peacock_version {
        Some(v) => Some(v.clone()),
        None if config.is_peacock_installed() => Some("unknown".into()),
        None => None,
    };
    let latest_version = fetch_latest_version(client).await.ok();

    PeacockStatus {
        installed_version,
        latest_version,
    }
}

/// Query GitHub API for the latest Peacock release tag.
pub async fn fetch_latest_version(client: &Client) -> Result<String> {
    let release: GitHubRelease = fetch_json(client, GITHUB_RELEASES_URL).await?;
    Ok(release.tag_name)
}

/// Download and install (or update) Peacock.
/// On update, preserves `userdata/` directory.
pub async fn install_or_update(
    client: &Client,
    config: &mut Config,
    on_progress: Option<ProgressFn>,
) -> Result<String> {
    let release: GitHubRelease = fetch_json(client, GITHUB_RELEASES_URL)
        .await
        .context("Failed to fetch latest Peacock release")?;

    let asset =
        find_linux_asset(&release).context("No Linux build found in the latest Peacock release")?;

    let peacock_dir = config.peacock_dir();
    let install_dir = &config.install_dir;

    // Create install dir if needed
    std::fs::create_dir_all(install_dir).context("Failed to create install directory")?;

    // Back up user data into a single staging directory
    let backup_dir = install_dir.join(".peacock_backup");
    if backup_dir.exists() {
        std::fs::remove_dir_all(&backup_dir)?;
    }
    std::fs::create_dir_all(&backup_dir).context("Failed to create backup directory")?;

    let backup_dirs = ["userdata", "plugins", "logs", "contracts", "contractSessions"];
    for name in &backup_dirs {
        let src = peacock_dir.join(name);
        if src.is_dir() {
            copy_dir_recursive(&src, &backup_dir.join(name))
                .with_context(|| format!("Failed to back up {name}"))?;
        }
    }

    let backup_files = ["options.ini"];
    for name in &backup_files {
        let src = peacock_dir.join(name);
        if src.is_file() {
            std::fs::copy(&src, backup_dir.join(name))
                .with_context(|| format!("Failed to back up {name}"))?;
        }
    }

    // Remove old Peacock directory
    if peacock_dir.exists() {
        std::fs::remove_dir_all(&peacock_dir).context("Failed to remove old Peacock directory")?;
    }

    // Download ZIP to temp location
    let zip_path = unique_download_path(install_dir, "peacock-download", "zip");
    download_file(client, &asset.browser_download_url, &zip_path, on_progress)
        .await
        .context("Failed to download Peacock")?;

    // Extract ZIP
    extract_zip(&zip_path, install_dir).context("Failed to extract Peacock archive")?;

    // The ZIP extracts to a subfolder like "Peacock-vX.Y.Z-linux"
    // Find it and rename to "Peacock"
    let extracted_folder = find_extracted_folder(install_dir, &release.tag_name)?;
    if extracted_folder != peacock_dir {
        std::fs::rename(&extracted_folder, &peacock_dir)
            .context("Failed to rename extracted Peacock folder")?;
    }

    // Clean up ZIP
    let _ = std::fs::remove_file(&zip_path);

    // Restore backed-up data into the new Peacock directory
    if backup_dir.exists() {
        for name in &backup_dirs {
            let src = backup_dir.join(name);
            if src.is_dir() {
                let dest = config.peacock_dir().join(name);
                if dest.exists() {
                    std::fs::remove_dir_all(&dest)?;
                }
                std::fs::rename(&src, &dest)
                    .with_context(|| format!("Failed to restore {name}"))?;
            }
        }
        for name in &backup_files {
            let src = backup_dir.join(name);
            if src.is_file() {
                std::fs::copy(&src, config.peacock_dir().join(name))
                    .with_context(|| format!("Failed to restore {name}"))?;
            }
        }
        let _ = std::fs::remove_dir_all(&backup_dir);
    }

    // Update config
    config.peacock_version = Some(release.tag_name.clone());
    config.save().context("Failed to save config")?;

    Ok(release.tag_name)
}

fn find_linux_asset(release: &GitHubRelease) -> Option<&GitHubAsset> {
    release
        .assets
        .iter()
        .find(|a| a.name.contains("linux") && a.name.ends_with(".zip"))
}

fn find_extracted_folder(install_dir: &Path, tag: &str) -> Result<std::path::PathBuf> {
    // Expected pattern: "Peacock-{tag}-linux"
    let expected = format!("Peacock-{tag}-linux");
    let candidate = install_dir.join(&expected);
    if candidate.is_dir() {
        return Ok(candidate);
    }

    // Fallback: look for any directory starting with "Peacock-"
    for entry in std::fs::read_dir(install_dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if entry.file_type()?.is_dir() && name_str.starts_with("Peacock-") {
            return Ok(entry.path());
        }
    }

    // Maybe the ZIP extracts directly as "Peacock"
    let peacock = install_dir.join("Peacock");
    if peacock.is_dir() {
        return Ok(peacock);
    }

    bail!("Could not find extracted Peacock directory (expected '{expected}')")
}

fn extract_zip(zip_path: &Path, dest: &Path) -> Result<()> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let Some(outpath) = entry.enclosed_name().map(|n| dest.join(n)) else {
            continue; // skip entries with unsafe paths
        };

        if entry.is_dir() {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut outfile = std::fs::File::create(&outpath)?;
            std::io::copy(&mut entry, &mut outfile)?;

            // Preserve executable permissions on Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = entry.unix_mode() {
                    std::fs::set_permissions(&outpath, std::fs::Permissions::from_mode(mode))?;
                }
            }
        }
    }

    Ok(())
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let dest_path = dest.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &dest_path)?;
        } else {
            std::fs::copy(entry.path(), &dest_path)?;
        }
    }
    Ok(())
}
