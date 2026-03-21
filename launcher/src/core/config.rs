use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const CONFIG_DIR_NAME: &str = "peacock-linux";
const CONFIG_FILE_NAME: &str = "config.toml";
const DEFAULT_PORT: u16 = 3000;
const INSTALL_DIR_ENV: &str = "PEACOCK_INSTALL_DIR";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Root directory for Peacock + Node installs
    pub install_dir: PathBuf,
    /// Port Peacock listens on
    pub port: u16,
    /// Installed Peacock version (semver tag, e.g. "v7.0.0")
    #[serde(default)]
    pub peacock_version: Option<String>,
    /// Installed Node.js version (e.g. "v20.11.0")
    #[serde(default)]
    pub node_version: Option<String>,
    /// Installed ZHMModSDK version (e.g. "v3.1.0")
    #[serde(default)]
    pub sdk_version: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            install_dir: default_install_dir(),
            port: DEFAULT_PORT,
            peacock_version: None,
            node_version: None,
            sdk_version: None,
        }
    }
}

impl Config {
    /// Load config from disk, falling back to defaults.
    /// Respects `PEACOCK_INSTALL_DIR` env var override.
    pub fn load() -> Result<Self> {
        let path = config_file_path();
        let mut config = if path.exists() {
            let contents = std::fs::read_to_string(&path).context("Failed to read config file")?;
            toml::from_str::<Config>(&contents).context("Failed to parse config file")?
        } else {
            Config::default()
        };

        // Env var override for install dir
        if let Ok(dir) = std::env::var(INSTALL_DIR_ENV) {
            config.install_dir = PathBuf::from(dir);
        }

        Ok(config)
    }

    /// Persist config to disk, creating parent directories as needed.
    pub fn save(&self) -> Result<()> {
        let path = config_file_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create config directory")?;
        }
        let contents = toml::to_string_pretty(self).context("Failed to serialize config")?;
        std::fs::write(&path, contents).context("Failed to write config file")?;
        Ok(())
    }

    // --- Derived paths ---

    /// `{install_dir}/Peacock/`
    pub fn peacock_dir(&self) -> PathBuf {
        self.install_dir.join("Peacock")
    }

    /// `{install_dir}/Peacock/chunk0.js`
    pub fn peacock_entry(&self) -> PathBuf {
        self.peacock_dir().join("chunk0.js")
    }

    /// `{install_dir}/Peacock/userdata/`
    pub fn peacock_userdata_dir(&self) -> PathBuf {
        self.peacock_dir().join("userdata")
    }

    /// `{install_dir}/Peacock/plugins/`
    pub fn peacock_plugins_dir(&self) -> PathBuf {
        self.peacock_dir().join("plugins")
    }

    /// `{install_dir}/Peacock/.nvmrc`
    pub fn peacock_nvmrc(&self) -> PathBuf {
        self.peacock_dir().join(".nvmrc")
    }

    /// `{install_dir}/node/`
    pub fn node_dir(&self) -> PathBuf {
        self.install_dir.join("node")
    }

    /// `{install_dir}/node/bin/node`
    pub fn node_bin(&self) -> PathBuf {
        self.node_dir().join("bin").join("node")
    }

    /// Whether Peacock appears to be installed (chunk0.js exists).
    pub fn is_peacock_installed(&self) -> bool {
        self.peacock_entry().exists()
    }

    /// Whether Node.js appears to be installed.
    pub fn is_node_installed(&self) -> bool {
        self.node_bin().exists()
    }
}

pub(crate) fn config_base_dir() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".config"))
}

fn data_base_dir() -> PathBuf {
    dirs::data_dir().unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".local/share"))
}

/// Config file path inside the user's config directory.
pub fn config_file_path() -> PathBuf {
    config_base_dir()
        .join(CONFIG_DIR_NAME)
        .join(CONFIG_FILE_NAME)
}

/// Default install dir inside the user's data directory.
fn default_install_dir() -> PathBuf {
    data_base_dir().join(CONFIG_DIR_NAME)
}

/// Check if a given directory looks like an old `linux-steam-setup` install.
pub fn is_legacy_install(dir: &Path) -> bool {
    dir.join("start.sh").exists() && dir.join("Peacock").is_dir()
}

/// Candidate paths for legacy installs.
pub fn legacy_install_candidates() -> Vec<PathBuf> {
    let home = dirs::home_dir().unwrap_or_default();
    vec![
        home.join("linux-steam-setup"),
        home.join("build").join("linux-steam-setup"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_round_trip() {
        let config = Config {
            install_dir: PathBuf::from("/tmp/test-peacock"),
            port: 4000,
            peacock_version: Some("v7.1.0".into()),
            node_version: Some("v20.11.0".into()),
            sdk_version: None,
        };
        let serialized = toml::to_string_pretty(&config).unwrap();
        let deserialized: Config = toml::from_str(&serialized).unwrap();
        assert_eq!(config.install_dir, deserialized.install_dir);
        assert_eq!(config.port, deserialized.port);
        assert_eq!(config.peacock_version, deserialized.peacock_version);
        assert_eq!(config.node_version, deserialized.node_version);
        assert_eq!(config.sdk_version, deserialized.sdk_version);
    }

    #[test]
    fn default_config_paths() {
        let config = Config::default();
        assert!(config.peacock_dir().ends_with("Peacock"));
        assert!(config.node_dir().ends_with("node"));
        assert_eq!(config.port, 3000);
    }
}
