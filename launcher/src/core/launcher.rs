use reqwest::Client;

use super::download::fetch_json;
use serde::Deserialize;

const LAUNCHER_RELEASES_URL: &str =
    "https://api.github.com/repos/thepeacockproject/linux-steam-setup/releases/latest";

/// The version baked in at build time from Cargo.toml (updated by CI from the git tag).
pub const LAUNCHER_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
}

/// Information about available launcher versions.
#[derive(Debug, Clone)]
pub struct LauncherStatus {
    pub current_version: String,
    pub latest_version: Option<String>,
}

impl LauncherStatus {
    pub fn update_available(&self) -> bool {
        match &self.latest_version {
            // Strip the leading 'v' from the tag (e.g. "v1.2.3" → "1.2.3") before comparing.
            Some(latest) => latest.trim_start_matches('v') != self.current_version,
            None => false,
        }
    }
}

/// Check the current vs latest launcher version.
pub async fn check_update(client: &Client) -> LauncherStatus {
    let latest_version = fetch_latest_version(client).await.ok();
    LauncherStatus {
        current_version: LAUNCHER_VERSION.to_string(),
        latest_version,
    }
}

async fn fetch_latest_version(client: &Client) -> anyhow::Result<String> {
    let release: GitHubRelease = fetch_json(client, LAUNCHER_RELEASES_URL).await?;
    Ok(release.tag_name)
}

const RELEASES_PAGE_URL: &str =
    "https://github.com/thepeacockproject/linux-steam-setup/releases/latest";

/// Open the launcher releases page in the default browser using `xdg-open`.
pub fn open_download_page() {
    let _ = std::process::Command::new("xdg-open")
        .arg(RELEASES_PAGE_URL)
        .spawn();
}
