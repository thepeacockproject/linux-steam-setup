use anyhow::{Context, Result};
use reqwest::Client;
use std::path::Path;

use super::config::Config;
use super::download::{ProgressFn, download_file, unique_download_path};

/// Information about available Node.js versions.
#[derive(Debug, Clone)]
pub struct NodeStatus {
    pub installed_version: Option<String>,
    pub required_version: Option<String>,
}

impl NodeStatus {
    pub fn update_needed(&self) -> bool {
        match (&self.installed_version, &self.required_version) {
            (Some(installed), Some(required)) => installed.trim() != required.trim(),
            (None, Some(_)) => true,
            _ => false,
        }
    }

    pub fn is_installed(&self) -> bool {
        self.installed_version.is_some()
    }
}

/// Check the installed vs required Node.js versions.
pub fn check_status(config: &Config) -> NodeStatus {
    let installed_version = detect_installed_version(config);
    let required_version = read_required_version(config);

    NodeStatus {
        installed_version,
        required_version,
    }
}

/// Read the installed node version by running `node --version`.
fn detect_installed_version(config: &Config) -> Option<String> {
    let node_bin = config.node_bin();
    if !node_bin.exists() {
        // Also check if system-wide node is available
        return None;
    }

    std::process::Command::new(&node_bin)
        .arg("--version")
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
            } else {
                None
            }
        })
}

/// Read the required Node.js version from Peacock's `.nvmrc` file.
fn read_required_version(config: &Config) -> Option<String> {
    let nvmrc_path = config.peacock_nvmrc();
    std::fs::read_to_string(nvmrc_path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Download and install the required Node.js version.
pub async fn install_or_update(
    client: &Client,
    config: &mut Config,
    on_progress: Option<ProgressFn>,
) -> Result<String> {
    let required = read_required_version(config)
        .context("Cannot determine required Node.js version — is Peacock installed?")?;

    let node_dir = config.node_dir();
    let install_dir = &config.install_dir;

    // Build download URL
    let url = format!("https://nodejs.org/dist/{required}/node-{required}-linux-x64.tar.gz",);

    // Download tar.gz
    let tarball_path = unique_download_path(install_dir, "node-download", "tar.gz");
    download_file(client, &url, &tarball_path, on_progress)
        .await
        .context("Failed to download Node.js")?;

    // Remove old node directory
    if node_dir.exists() {
        std::fs::remove_dir_all(&node_dir).context("Failed to remove old Node.js directory")?;
    }

    // Extract tar.gz, stripping the first path component
    std::fs::create_dir_all(&node_dir)?;
    extract_tarball(&tarball_path, &node_dir).context("Failed to extract Node.js archive")?;

    // Clean up tarball
    let _ = std::fs::remove_file(&tarball_path);

    // Update config
    config.node_version = Some(required.clone());
    config.save().context("Failed to save config")?;

    Ok(required)
}

/// Extract a `.tar.gz` file to `dest`, stripping the first path component.
fn extract_tarball(tarball: &Path, dest: &Path) -> Result<()> {
    let file = std::fs::File::open(tarball)?;
    let gz = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(gz);

    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;

        // Strip the first component (e.g. "node-v20.11.0-linux-x64/")
        let stripped: std::path::PathBuf = path.components().skip(1).collect();
        if stripped.as_os_str().is_empty() {
            continue;
        }

        let outpath = dest.join(&stripped);

        // Security: ensure we don't extract outside dest
        if !outpath.starts_with(dest) {
            continue;
        }

        if entry.header().entry_type().is_dir() {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent)?;
            }
            entry.unpack(&outpath)?;
        }
    }

    Ok(())
}
