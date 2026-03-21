use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;

/// Type of value an option accepts.
#[derive(Debug, Clone)]
pub enum OptionType {
    Boolean,
    Enum(&'static [&'static str]),
}

/// A single Peacock option with metadata and current value.
#[derive(Debug, Clone)]
pub struct PeacockOption {
    pub key: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub category: &'static str,
    pub option_type: OptionType,
    pub value: String,
}

impl PeacockOption {
    /// Cycle to the next value (toggle for booleans, next variant for enums).
    pub fn cycle_value(&mut self) {
        match &self.option_type {
            OptionType::Boolean => {
                self.value = if self.value == "true" {
                    "false".into()
                } else {
                    "true".into()
                };
            }
            OptionType::Enum(variants) => {
                let idx = variants.iter().position(|v| *v == self.value).unwrap_or(0);
                self.value = variants[(idx + 1) % variants.len()].to_string();
            }
        }
    }

    /// Cycle to the previous value (same as toggle for booleans).
    pub fn cycle_value_back(&mut self) {
        match &self.option_type {
            OptionType::Boolean => self.cycle_value(),
            OptionType::Enum(variants) => {
                let idx = variants.iter().position(|v| *v == self.value).unwrap_or(0);
                let prev = if idx == 0 { variants.len() - 1 } else { idx - 1 };
                self.value = variants[prev].to_string();
            }
        }
    }
}

/// Define all known Peacock options with defaults and metadata.
fn define_options() -> Vec<PeacockOption> {
    vec![
        // ── Gameplay ──
        PeacockOption {
            key: "gameplayUnlockAllShortcuts",
            label: "Unlock All Shortcuts",
            description: "When enabled, all shortcuts will always be unlocked.",
            category: "Gameplay",
            option_type: OptionType::Boolean,
            value: "false".into(),
        },
        PeacockOption {
            key: "gameplayUnlockAllFreelancerMasteries",
            label: "Unlock All Freelancer Masteries",
            description: "When enabled, all Freelancer unlocks will always be available.",
            category: "Gameplay",
            option_type: OptionType::Boolean,
            value: "false".into(),
        },
        PeacockOption {
            key: "mapDiscoveryState",
            label: "Map Discovery State",
            description: "REVEALED resets all locations to discovered, CLOUDED resets to undiscovered, KEEP preserves current state. Applied every time you connect.",
            category: "Gameplay",
            option_type: OptionType::Enum(&["KEEP", "REVEALED", "CLOUDED"]),
            value: "KEEP".into(),
        },
        PeacockOption {
            key: "enableMasteryProgression",
            label: "Enable Mastery Progression",
            description: "When disabled, mastery progression is off and all unlockables are awarded immediately.",
            category: "Gameplay",
            option_type: OptionType::Boolean,
            value: "true".into(),
        },
        PeacockOption {
            key: "enableIsolatedUnlockables",
            label: "Enable Isolated Unlockables",
            description: "Unlock items with no associated unlocking approaches. Requires mastery progression to be enabled.",
            category: "Gameplay",
            option_type: OptionType::Boolean,
            value: "true".into(),
        },
        PeacockOption {
            key: "elusivesAreShown",
            label: "Show Elusive Targets in Instinct",
            description: "Show elusive targets in instinct like normal targets on normal missions.",
            category: "Gameplay",
            option_type: OptionType::Boolean,
            value: "false".into(),
        },
        PeacockOption {
            key: "enableContractsModeSaving",
            label: "Enable Contracts Mode Saving",
            description: "Enable saving in Contracts Mode for both user-created and featured contracts.",
            category: "Gameplay",
            option_type: OptionType::Boolean,
            value: "false".into(),
        },
        PeacockOption {
            key: "legacyNoticedKillScoring",
            label: "Legacy Noticed Kill Scoring",
            description: "In HITMAN 2016, noticed kill scoring: 'vanilla' for official behavior, 'sane' for previous Peacock behavior.",
            category: "Gameplay",
            option_type: OptionType::Enum(&["vanilla", "sane"]),
            value: "vanilla".into(),
        },
        // ── Services ──
        PeacockOption {
            key: "legacyElusivesEnableSaving",
            label: "Legacy Elusives Enable Saving",
            description: "When enabled, elusive targets in HITMAN 2016 share normal mission saving rules, but ET challenges won't be completable.",
            category: "Services",
            option_type: OptionType::Boolean,
            value: "false".into(),
        },
        PeacockOption {
            key: "getDefaultSuits",
            label: "Get Default Suits",
            description: "Add all default starting suits to your inventory. Note: enabling both this and mastery progression may lock some suits behind challenges.",
            category: "Services",
            option_type: OptionType::Boolean,
            value: "false".into(),
        },
        PeacockOption {
            key: "jokes",
            label: "Startup Jokes",
            description: "The Peacock server will tell you a joke on startup.",
            category: "Services",
            option_type: OptionType::Boolean,
            value: "false".into(),
        },
        PeacockOption {
            key: "leaderboards",
            label: "Leaderboards",
            description: "Allow your times to be submitted to the in-game leaderboards.",
            category: "Services",
            option_type: OptionType::Boolean,
            value: "true".into(),
        },
        PeacockOption {
            key: "updateChecking",
            label: "Update Checking",
            description: "Allow Peacock to check for updates on startup.",
            category: "Services",
            option_type: OptionType::Boolean,
            value: "true".into(),
        },
        PeacockOption {
            key: "loadoutSaving",
            label: "Loadout Saving Mode",
            description: "PROFILES uses loadout profiles, LEGACY uses per-user saving.",
            category: "Services",
            option_type: OptionType::Enum(&["PROFILES", "LEGACY"]),
            value: "PROFILES".into(),
        },
        PeacockOption {
            key: "legacyContractDownloader",
            label: "Legacy Contract Downloader",
            description: "When enabled, use official servers for H3 contract downloads (platform-specific). When disabled, use HITMAPS servers.",
            category: "Services",
            option_type: OptionType::Boolean,
            value: "false".into(),
        },
        PeacockOption {
            key: "imageLoading",
            label: "Image Loading Mode",
            description: "SAVEASREQUESTED fetches and caches images, ONLINE fetches without saving, OFFLINE loads from local folder only.",
            category: "Services",
            option_type: OptionType::Enum(&["SAVEASREQUESTED", "ONLINE", "OFFLINE"]),
            value: "SAVEASREQUESTED".into(),
        },
        // ── Splitter ──
        PeacockOption {
            key: "liveSplit",
            label: "LiveSplit Support",
            description: "Toggle LiveSplit support on or off.",
            category: "Splitter",
            option_type: OptionType::Boolean,
            value: "false".into(),
        },
        PeacockOption {
            key: "autoSplitterCampaign",
            label: "AutoSplitter Campaign",
            description: "Which main campaign to use for the AutoSplitter: 1, 2, 3, or trilogy.",
            category: "Splitter",
            option_type: OptionType::Enum(&["trilogy", "1", "2", "3"]),
            value: "trilogy".into(),
        },
        PeacockOption {
            key: "autoSplitterRacetimegg",
            label: "AutoSplitter racetime.gg",
            description: "Enable special AutoSplitter mode for racetime.gg realtime races.",
            category: "Splitter",
            option_type: OptionType::Boolean,
            value: "false".into(),
        },
        PeacockOption {
            key: "autoSplitterForceSilentAssassin",
            label: "AutoSplitter Force SA",
            description: "When enabled, only Silent Assassin completions count as valid for the AutoSplitter.",
            category: "Splitter",
            option_type: OptionType::Boolean,
            value: "true".into(),
        },
        // ── Discord ──
        PeacockOption {
            key: "discordRp",
            label: "Discord Rich Presence",
            description: "Toggle Discord rich presence on or off.",
            category: "Discord",
            option_type: OptionType::Boolean,
            value: "false".into(),
        },
        PeacockOption {
            key: "discordRpAppTime",
            label: "Discord RP Show App Time",
            description: "When enabled, shows total Peacock usage time. When disabled, shows time in current level.",
            category: "Discord",
            option_type: OptionType::Boolean,
            value: "false".into(),
        },
        // ── Modding ──
        PeacockOption {
            key: "overrideFrameworkChecks",
            label: "Override Framework Checks",
            description: "Forcibly disable installed mod checks.",
            category: "Modding",
            option_type: OptionType::Boolean,
            value: "false".into(),
        },
        // ── Experimental ──
        PeacockOption {
            key: "experimentalHMR",
            label: "Experimental Hot Reload",
            description: "Toggle hot reloading of contracts.",
            category: "Experimental",
            option_type: OptionType::Boolean,
            value: "false".into(),
        },
    ]
}

/// Parse an INI file into a key→value map, skipping comments and section headers.
fn parse_ini(content: &str) -> HashMap<&str, String> {
    let mut map = HashMap::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('[') {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once('=') {
            map.insert(key.trim(), value.trim().to_string());
        }
    }
    map
}

/// Load options from the options.ini file, falling back to defaults for missing keys.
pub fn load_options(peacock_dir: &Path) -> Vec<PeacockOption> {
    let mut options = define_options();
    let ini_path = peacock_dir.join("options.ini");

    if let Ok(content) = std::fs::read_to_string(&ini_path) {
        let values = parse_ini(&content);
        for opt in &mut options {
            if let Some(val) = values.get(opt.key) {
                opt.value = val.clone();
            }
        }
    }

    options
}

/// Save options back to the options.ini file.
///
/// If the file exists, updates values in-place preserving all comments and
/// unknown options. If it doesn't exist, generates a minimal new file.
pub fn save_options(peacock_dir: &Path, options: &[PeacockOption]) -> Result<()> {
    let ini_path = peacock_dir.join("options.ini");
    let value_map: HashMap<&str, &str> =
        options.iter().map(|o| (o.key, o.value.as_str())).collect();

    if ini_path.exists() {
        // Update existing file, preserving structure and comments
        let content =
            std::fs::read_to_string(&ini_path).context("Failed to read options.ini")?;

        let mut output = String::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty()
                && !trimmed.starts_with(';')
                && !trimmed.starts_with('[')
                && let Some((key, _)) = trimmed.split_once('=')
            {
                let key = key.trim();
                if let Some(new_value) = value_map.get(key) {
                    output.push_str(&format!("{key}={new_value}"));
                    output.push('\n');
                    continue;
                }
            }
            output.push_str(line);
            output.push('\n');
        }

        std::fs::write(&ini_path, output).context("Failed to write options.ini")?;
    } else {
        // Generate a minimal new file
        let mut output = String::from("; Peacock options\n[peacock]\n");
        let mut current_category = "";

        for opt in options {
            if opt.category != current_category {
                current_category = opt.category;
                output.push('\n');
                output.push_str(&format!("; ── {} ──\n", opt.category));
            }
            output.push_str(&format!("{}={}\n", opt.key, opt.value));
        }

        std::fs::write(&ini_path, output).context("Failed to write options.ini")?;
    }

    Ok(())
}
