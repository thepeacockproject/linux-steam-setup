use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;
use std::path::Path;

use super::download::{ProgressFn, download_file, fetch_json, unique_download_path};
use super::game_detect::GameInstall;

/// We list all releases (including pre-releases) and pick the first one,
/// because the most recent ZHMModSDK version may be tagged as a pre-release
/// and is required for the latest HITMAN build.
const GITHUB_RELEASES_URL: &str =
    "https://api.github.com/repos/OrfeasZ/ZHMModSDK/releases?per_page=1";

/// The specific asset to download — contains the SDK DLLs.
const SDK_ASSET_NAME: &str = "ZHMModSDK-Release.zip";

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

/// Query GitHub for the latest ZHMModSDK release tag (including pre-releases).
pub async fn fetch_latest_version(client: &Client) -> Result<String> {
    let releases: Vec<GitHubRelease> = fetch_json(client, GITHUB_RELEASES_URL).await?;
    let release = releases
        .into_iter()
        .next()
        .context("No ZHMModSDK releases found")?;
    Ok(release.tag_name)
}

/// Check if ZHMModSDK is installed in a game install's Retail directory.
pub fn is_installed(game_install: &GameInstall) -> bool {
    let retail_dir = game_install.game_dir.join("Retail");
    // The SDK places dinput8.dll inside Retail/
    retail_dir.join("dinput8.dll").exists()
}

/// Download and install ZHMModSDK into the game's Retail directory.
/// Per the official install guide for Steam Deck / Proton, we only need to
/// extract ZHMModSDK-Release.zip into the Retail folder. No Wine prefix
/// modifications are needed with Proton Experimental or newer.
pub async fn install_or_update(
    client: &Client,
    game_install: &GameInstall,
    on_progress: Option<ProgressFn>,
) -> Result<String> {
    let releases: Vec<GitHubRelease> = fetch_json(client, GITHUB_RELEASES_URL)
        .await
        .context("Failed to fetch ZHMModSDK releases")?;

    let release = releases
        .into_iter()
        .next()
        .context("No ZHMModSDK releases found")?;

    let asset = release
        .assets
        .iter()
        .find(|a| a.name == SDK_ASSET_NAME)
        .context(format!(
            "Release {} has no '{SDK_ASSET_NAME}' asset",
            release.tag_name
        ))?;

    let retail_dir = game_install.game_dir.join("Retail");
    if !retail_dir.is_dir() {
        std::fs::create_dir_all(&retail_dir).context("Failed to create Retail directory")?;
    }

    // Download ZIP to a temp file in the Retail dir
    let zip_path = unique_download_path(&retail_dir, "zhmmodsdk-download", "zip");
    download_file(client, &asset.browser_download_url, &zip_path, on_progress)
        .await
        .context("Failed to download ZHMModSDK")?;

    // Extract ZIP contents into Retail/
    extract_sdk_zip(&zip_path, &retail_dir).context("Failed to extract ZHMModSDK")?;

    // Clean up ZIP
    let _ = std::fs::remove_file(&zip_path);

    Ok(release.tag_name)
}

/// Remove ZHMModSDK from a game's Retail directory.
pub fn remove(game_install: &GameInstall) -> Result<()> {
    let retail_dir = game_install.game_dir.join("Retail");

    // Remove dinput8.dll (SDK loader)
    let dll_path = retail_dir.join("dinput8.dll");
    if dll_path.exists() {
        std::fs::remove_file(&dll_path).context("Failed to remove dinput8.dll")?;
    }

    // Remove zhmmodsdk.dll
    let sdk_dll = retail_dir.join("zhmmodsdk.dll");
    if sdk_dll.exists() {
        std::fs::remove_file(&sdk_dll).context("Failed to remove zhmmodsdk.dll")?;
    }

    // Remove mods directory inside Retail/
    let mods_dir = retail_dir.join("mods");
    if mods_dir.is_dir() {
        std::fs::remove_dir_all(&mods_dir).context("Failed to remove mods directory")?;
    }

    Ok(())
}

fn extract_sdk_zip(zip_path: &Path, retail_dir: &Path) -> Result<()> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let Some(name) = entry.enclosed_name().map(|n| n.to_path_buf()) else {
            continue;
        };

        let outpath = retail_dir.join(&name);

        // Security: ensure we don't write outside retail_dir
        if !outpath.starts_with(retail_dir) {
            continue;
        }

        if entry.is_dir() {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent)?;
            }
            // Overwrites built-in SDK files (including mods shipped with the SDK),
            // but never deletes user mods — only files present in the ZIP are touched.
            let mut outfile = std::fs::File::create(&outpath)?;
            std::io::copy(&mut entry, &mut outfile)?;
        }
    }

    Ok(())
}
