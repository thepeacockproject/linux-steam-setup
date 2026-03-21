use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use super::config::{Config, is_legacy_install, legacy_install_candidates};
use super::game_detect;

/// Information about a detected legacy install.
#[derive(Debug, Clone)]
pub struct LegacyInstall {
    pub path: PathBuf,
    pub has_peacock: bool,
    pub has_node: bool,
    pub has_userdata: bool,
    pub peacock_version: Option<String>,
}

/// Detect any existing legacy `linux-steam-setup` installs.
pub fn detect_legacy() -> Option<LegacyInstall> {
    for candidate in legacy_install_candidates() {
        if is_legacy_install(&candidate) {
            return inspect_path(&candidate);
        }
    }
    None
}

/// Inspect an arbitrary directory and build a `LegacyInstall` if it
/// contains a `Peacock/` sub-directory (does NOT require `start.sh`).
pub fn inspect_path(dir: &Path) -> Option<LegacyInstall> {
    let peacock_dir = dir.join("Peacock");
    if !peacock_dir.is_dir() {
        return None;
    }
    let has_peacock = peacock_dir.join("chunk0.js").exists();
    let has_node = dir.join("node").join("bin").join("node").exists();
    let has_userdata = peacock_dir.join("userdata").is_dir();
    let peacock_version = detect_legacy_peacock_version(&peacock_dir);

    Some(LegacyInstall {
        path: dir.to_path_buf(),
        has_peacock,
        has_node,
        has_userdata,
        peacock_version,
    })
}

/// Try to detect what Peacock version is installed in a legacy setup.
fn detect_legacy_peacock_version(peacock_dir: &Path) -> Option<String> {
    // Check if there's a version identifier in the Peacock directory
    // Peacock's chunk0.js or package.json might contain version info
    let package_json = peacock_dir.join("package.json");
    if package_json.exists()
        && let Ok(contents) = std::fs::read_to_string(&package_json)
        && let Ok(data) = serde_json::from_str::<serde_json::Value>(&contents)
        && let Some(version) = data.get("version").and_then(|v| v.as_str())
    {
        return Some(format!("v{version}"));
    }
    None
}

/// Migrate a legacy install to the new location.
pub fn migrate(legacy: &LegacyInstall, config: &mut Config) -> Result<MigrationResult> {
    let mut result = MigrationResult::default();

    std::fs::create_dir_all(&config.install_dir)
        .context("Failed to create new install directory")?;

    // Migrate Peacock
    if legacy.has_peacock {
        let src = legacy.path.join("Peacock");
        let dest = config.peacock_dir();

        if dest.exists() {
            // Back up existing userdata if present
            let existing_userdata = dest.join("userdata");
            if existing_userdata.is_dir() {
                let backup = config.install_dir.join(".userdata_backup_migration");
                copy_dir_recursive(&existing_userdata, &backup)?;
            }
            std::fs::remove_dir_all(&dest)?;
        }

        copy_dir_recursive(&src, &dest).context("Failed to copy Peacock directory")?;
        result.peacock_migrated = true;

        if let Some(version) = &legacy.peacock_version {
            config.peacock_version = Some(version.clone());
        }
    }

    // Migrate Node
    if legacy.has_node {
        let src = legacy.path.join("node");
        let dest = config.node_dir();

        if dest.exists() {
            std::fs::remove_dir_all(&dest)?;
        }

        copy_dir_recursive(&src, &dest).context("Failed to copy Node.js directory")?;
        result.node_migrated = true;

        // Detect node version from migrated binary
        let node_bin = dest.join("bin").join("node");
        if node_bin.exists()
            && let Ok(output) = std::process::Command::new(&node_bin)
                .arg("--version")
                .output()
            && output.status.success()
        {
            config.node_version = Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
        }
    }

    // Replace any previous service definition and restore prior service state.
    if let Err(e) = super::service::replace(config).map(|service_result| {
        result.service_migrated = true;
        result.service_replaced = service_result.had_existing_service;
        result.legacy_service_replaced = service_result.replaced_legacy_service;
        result.service_enabled_restored = service_result.restored_enabled;
        result.service_restarted = service_result.restored_running;
    }) {
        result.service_error = Some(format!("Failed to install new service: {e}"));
    }

    // Clean up legacy PeacockPatcher.exe and WineLaunch.bat from game directories
    let game_installs = game_detect::detect_all();
    for install in &game_installs {
        let patcher = install.game_dir.join("PeacockPatcher.exe");
        let bat = install.game_dir.join("WineLaunch.bat");
        if patcher.exists() {
            let _ = std::fs::remove_file(&patcher);
            result.legacy_files_cleaned += 1;
        }
        if bat.exists() {
            let _ = std::fs::remove_file(&bat);
            result.legacy_files_cleaned += 1;
        }
    }

    // Save updated config
    config
        .save()
        .context("Failed to save config after migration")?;

    result.success = true;
    Ok(result)
}

/// Optionally remove the old legacy directory.
pub fn remove_legacy_dir(legacy: &LegacyInstall) -> Result<()> {
    if legacy.path.exists() {
        std::fs::remove_dir_all(&legacy.path)
            .context("Failed to remove legacy install directory")?;
    }
    Ok(())
}

#[derive(Debug, Clone, Default)]
pub struct MigrationResult {
    pub success: bool,
    pub peacock_migrated: bool,
    pub node_migrated: bool,
    pub service_migrated: bool,
    pub service_replaced: bool,
    pub legacy_service_replaced: bool,
    pub service_enabled_restored: bool,
    pub service_restarted: bool,
    pub service_error: Option<String>,
    pub legacy_files_cleaned: usize,
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
