use anyhow::{Context, Result};
use regex::Regex;
use std::fmt;
use std::path::{Path, PathBuf};

/// Which game launcher a Hitman 3 install belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Launcher {
    Steam,
    Heroic,
}

impl fmt::Display for Launcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Launcher::Steam => write!(f, "Steam"),
            Launcher::Heroic => write!(f, "Heroic"),
        }
    }
}

/// A detected Hitman 3 installation.
#[derive(Debug, Clone)]
pub struct GameInstall {
    /// Path to the Hitman 3 game directory (contains Launcher.exe, Retail/, etc.)
    pub game_dir: PathBuf,
    /// Which launcher this install belongs to
    pub launcher: Launcher,
    /// Path to the Wine prefix for this install (contains user.reg, etc.)
    pub wine_prefix: Option<PathBuf>,
}

const HITMAN_STEAM_APPID: &str = "1659040";
const HITMAN_DIRNAME: &str = "HITMAN 3";

/// Detect all Hitman 3 installations (Steam + Heroic), deduplicated by real path.
pub fn detect_all() -> Vec<GameInstall> {
    let mut installs = Vec::new();
    installs.extend(detect_steam());
    installs.extend(detect_heroic());

    // Deduplicate by canonical path to handle symlinks pointing to the same install
    let mut seen = std::collections::HashSet::new();
    installs.retain(|install| {
        let canonical = install
            .game_dir
            .canonicalize()
            .unwrap_or_else(|_| install.game_dir.clone());
        seen.insert(canonical)
    });

    installs
}

// --- Steam detection ---

fn detect_steam() -> Vec<GameInstall> {
    let mut results = Vec::new();

    for steam_dir in steam_root_candidates() {
        if !steam_dir.is_dir() {
            continue;
        }

        let vdf_path = steam_dir.join("steamapps").join("libraryfolders.vdf");
        if !vdf_path.exists() {
            continue;
        }

        let Ok(library_paths) = parse_library_folders(&vdf_path) else {
            continue;
        };

        for lib_path in library_paths {
            let game_dir = lib_path
                .join("steamapps")
                .join("common")
                .join(HITMAN_DIRNAME);

            if game_dir.is_dir() {
                let wine_prefix = lib_path
                    .join("steamapps")
                    .join("compatdata")
                    .join(HITMAN_STEAM_APPID)
                    .join("pfx");

                results.push(GameInstall {
                    game_dir,
                    launcher: Launcher::Steam,
                    wine_prefix: if wine_prefix.is_dir() {
                        Some(wine_prefix)
                    } else {
                        None
                    },
                });
            }
        }
    }

    results
}

fn steam_root_candidates() -> Vec<PathBuf> {
    let home = dirs::home_dir().unwrap_or_default();
    vec![
        home.join(".local/share/Steam"),
        home.join(".steam/steam"),
        home.join(".steam/root"),
        // Flatpak Steam
        home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"),
        // Snap Steam
        home.join("snap/steam/common/.local/share/Steam"),
    ]
}

/// Parse `libraryfolders.vdf` to extract library paths.
fn parse_library_folders(vdf_path: &Path) -> Result<Vec<PathBuf>> {
    let contents = std::fs::read_to_string(vdf_path).context("Failed to read VDF file")?;
    let re = Regex::new(r#""path"\s+"([^"]+)""#)?;

    Ok(re
        .captures_iter(&contents)
        .filter_map(|cap| cap.get(1).map(|m| PathBuf::from(m.as_str())))
        .collect())
}

// --- Heroic detection ---

fn detect_heroic() -> Vec<GameInstall> {
    let mut results = Vec::new();

    for heroic_config_dir in heroic_config_candidates() {
        if !heroic_config_dir.is_dir() {
            continue;
        }

        // Try Legendary library cache (Epic store)
        let legendary_lib = heroic_config_dir.join("store_cache/legendary_library.json");
        if legendary_lib.exists() {
            if let Some(install) = parse_legendary_library(&legendary_lib, &heroic_config_dir) {
                results.push(install);
            }
        }

        // Try GamesConfig directory for per-game info
        let games_config_dir = heroic_config_dir.join("GamesConfig");
        if games_config_dir.is_dir() {
            results.extend(parse_heroic_games_config(&games_config_dir));
        }
    }

    results
}

fn heroic_config_candidates() -> Vec<PathBuf> {
    let home = dirs::home_dir().unwrap_or_default();
    vec![
        home.join(".config/heroic"),
        // Flatpak Heroic
        home.join(".var/app/com.heroicgameslauncher.hgl/config/heroic"),
    ]
}

fn parse_legendary_library(lib_path: &Path, heroic_config_dir: &Path) -> Option<GameInstall> {
    let contents = std::fs::read_to_string(lib_path).ok()?;
    let data: serde_json::Value = serde_json::from_str(&contents).ok()?;
    let library = data.get("library")?.as_array()?;

    for game in library {
        let title = game.get("title")?.as_str()?;
        // Hitman 3 / World of Assassination
        if !title.contains("HITMAN") && !title.contains("World of Assassination") {
            continue;
        }

        let install_info = game.get("install")?;
        let install_path = install_info.get("install_path")?.as_str()?;
        let game_dir = PathBuf::from(install_path);

        if !game_dir.is_dir() {
            continue;
        }

        let app_name = game.get("app_name")?.as_str()?;

        // Try to find Wine prefix from per-game config
        let wine_prefix = find_heroic_wine_prefix(heroic_config_dir, app_name);

        return Some(GameInstall {
            game_dir,
            launcher: Launcher::Heroic,
            wine_prefix,
        });
    }

    None
}

fn find_heroic_wine_prefix(heroic_config_dir: &Path, app_name: &str) -> Option<PathBuf> {
    let config_path = heroic_config_dir
        .join("GamesConfig")
        .join(format!("{app_name}.json"));

    let contents = std::fs::read_to_string(config_path).ok()?;
    let data: serde_json::Value = serde_json::from_str(&contents).ok()?;

    // Heroic stores winePrefix or wine_prefix
    let prefix = data
        .get(app_name)
        .or_else(|| data.get("default"))
        .and_then(|cfg| {
            cfg.get("winePrefix")
                .or_else(|| cfg.get("wine_prefix"))
                .and_then(|v| v.as_str())
                .map(PathBuf::from)
        })?;

    if prefix.is_dir() { Some(prefix) } else { None }
}

fn parse_heroic_games_config(games_config_dir: &Path) -> Vec<GameInstall> {
    let results = Vec::new();

    let Ok(entries) = std::fs::read_dir(games_config_dir) else {
        return results;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.extension().is_some_and(|e| e == "json") {
            continue;
        }

        let Ok(contents) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(data) = serde_json::from_str::<serde_json::Value>(&contents) else {
            continue;
        };

        // Look for a winePrefix that suggests this is a Hitman config
        // We need to cross-reference with installed games, but as a fallback
        // check if the json contains hitman-related paths
        let json_str = contents.to_lowercase();
        if !json_str.contains("hitman") {
            continue;
        }

        // Extract wine prefix from the config
        for (_key, value) in data.as_object().into_iter().flatten() {
            if let Some(prefix_str) = value
                .get("winePrefix")
                .or_else(|| value.get("wine_prefix"))
                .and_then(|v| v.as_str())
            {
                let prefix = PathBuf::from(prefix_str);
                // Try to find the actual game directory from this config
                // Heroic configs don't always have the install path directly
                if prefix.is_dir() {
                    // We'd need the actual game path — skip if we can't determine it
                    // This is a best-effort fallback; the legendary_library.json path is preferred
                    continue;
                }
            }
        }
    }

    results
}

/// Check if ZHMModSDK is installed in a game's Retail directory.
pub fn is_sdk_installed(game_install: &GameInstall) -> bool {
    game_install
        .game_dir
        .join("Retail")
        .join("dinput8.dll")
        .exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_vdf_paths() {
        let vdf_content = r#"
"libraryfolders"
{
    "0"
    {
        "path"      "/home/user/.local/share/Steam"
        "label"     ""
    }
    "1"
    {
        "path"      "/mnt/games/SteamLibrary"
        "label"     ""
    }
}
"#;
        let tmp = std::env::temp_dir().join("test_vdf.vdf");
        std::fs::write(&tmp, vdf_content).unwrap();
        let paths = parse_library_folders(&tmp).unwrap();
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0], PathBuf::from("/home/user/.local/share/Steam"));
        assert_eq!(paths[1], PathBuf::from("/mnt/games/SteamLibrary"));
        let _ = std::fs::remove_file(&tmp);
    }
}
