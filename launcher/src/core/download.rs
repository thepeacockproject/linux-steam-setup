use anyhow::{Context, Result, bail};
use futures_util::StreamExt;
use reqwest::Client;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::AsyncWriteExt;

const MAX_RETRIES: u32 = 3;
const INITIAL_BACKOFF_MS: u64 = 500;

/// Progress callback: (bytes_downloaded, total_bytes).
/// `total_bytes` is 0 if Content-Length was not provided.
pub type ProgressFn = Arc<dyn Fn(u64, u64) + Send + Sync>;

pub fn unique_download_path(base_dir: &Path, base_name: &str, extension: &str) -> PathBuf {
    let timestamp = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_nanos(),
        Err(_) => 0,
    };
    let pid = std::process::id();

    base_dir.join(format!("{base_name}-{pid}-{timestamp}.{extension}"))
}

/// Download a file from `url` to `dest`, reporting progress via `on_progress`.
/// Uses a temp file + rename for atomicity. Retries transient errors.
pub async fn download_file(
    client: &Client,
    url: &str,
    dest: &Path,
    on_progress: Option<ProgressFn>,
) -> Result<PathBuf> {
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .context("Failed to create destination directory")?;
    }

    let tmp_path = dest.with_extension("tmp");
    let mut last_err = None;

    for attempt in 0..MAX_RETRIES {
        if attempt > 0 {
            let backoff = INITIAL_BACKOFF_MS * 2u64.pow(attempt - 1);
            tokio::time::sleep(std::time::Duration::from_millis(backoff)).await;
        }

        match download_once(client, url, &tmp_path, &on_progress).await {
            Ok(()) => {
                tokio::fs::rename(&tmp_path, dest)
                    .await
                    .context("Failed to rename temp file to destination")?;
                return Ok(dest.to_path_buf());
            }
            Err(e) => {
                // Clean up partial temp file
                let _ = tokio::fs::remove_file(&tmp_path).await;
                last_err = Some(e);
            }
        }
    }

    let err = last_err.context("BUG: download retries exhausted without capturing an error")?;
    bail!("Download failed after {MAX_RETRIES} attempts: {err}")
}

async fn download_once(
    client: &Client,
    url: &str,
    tmp_path: &Path,
    on_progress: &Option<ProgressFn>,
) -> Result<()> {
    let response = client
        .get(url)
        .header("User-Agent", "peacock-launcher")
        .send()
        .await
        .context("HTTP request failed")?
        .error_for_status()
        .context("Server returned an error")?;

    let total = response.content_length().unwrap_or(0);
    let mut stream = response.bytes_stream();
    let mut file = tokio::fs::File::create(tmp_path)
        .await
        .context("Failed to create temp file")?;
    let mut downloaded: u64 = 0;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("Error reading response body")?;
        file.write_all(&chunk)
            .await
            .context("Failed to write to temp file")?;
        downloaded += chunk.len() as u64;
        if let Some(cb) = on_progress {
            cb(downloaded, total);
        }
    }

    file.flush().await.context("Failed to flush temp file")?;

    // Verify size if Content-Length was provided
    if total > 0 && downloaded != total {
        bail!(
            "Size mismatch: expected {} bytes, got {} bytes",
            total,
            downloaded
        );
    }

    Ok(())
}

/// Fetch JSON from a URL (for GitHub API calls).
pub async fn fetch_json<T: serde::de::DeserializeOwned>(client: &Client, url: &str) -> Result<T> {
    let response = client
        .get(url)
        .header("User-Agent", "peacock-launcher")
        .header("Accept", "application/json")
        .send()
        .await
        .context("HTTP request failed")?
        .error_for_status()
        .context("Server returned an error")?;

    response.json::<T>().await.context("Failed to parse JSON")
}

/// Build a shared reqwest client with reasonable defaults.
pub fn build_client() -> Result<Client> {
    Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .connect_timeout(std::time::Duration::from_secs(30))
        .build()
        .context("Failed to build HTTP client")
}
