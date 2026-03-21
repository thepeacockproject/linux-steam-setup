use reqwest::Client;
use std::path::PathBuf;
use tokio::sync::mpsc;

use crate::core::config::Config;
use crate::core::game_detect::GameInstall;
use crate::core::launcher::LauncherStatus;
use crate::core::migration::{LegacyInstall, MigrationResult};
use crate::core::node::NodeStatus;
use crate::core::options::PeacockOption;
use crate::core::peacock::PeacockStatus;
use crate::core::service::ServiceStatus;

/// Which screen the app is currently showing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Screen {
    MainMenu,
    Install,
    Service,
    Sdk,
    Settings,
    Options,
    Migration,
}

/// Messages sent from background tasks to the UI.
#[derive(Debug)]
pub enum AppMessage {
    /// Download progress update: (downloaded, total)
    Progress(u64, u64),
    /// A step description changed
    StepUpdate(String),
    /// Task completed successfully with a message
    TaskDone(String),
    /// Task failed with an error
    TaskError(String),
    /// Refresh status data (e.g., after install completes)
    RefreshStatus,
    /// Updated config from a background task (e.g., install wrote new versions)
    ConfigUpdated(Config),
    /// Peacock status fetched asynchronously
    PeacockStatusLoaded(PeacockStatus),
    /// Launcher update check completed asynchronously
    LauncherStatusLoaded(LauncherStatus),
    /// Folder picked from the native file dialog (None if cancelled)
    FolderPicked(Option<PathBuf>),
    /// Migration completed and produced a detailed result.
    MigrationFinished(MigrationResult, Config),
}

/// Which field is being edited in Settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsField {
    InstallDir,
    Port,
    Save,
}

/// Migration screen phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationMode {
    /// Choosing between auto-detected path and manual entry.
    SelectSource,
    /// Waiting for the native folder picker to return.
    PickingFolder,
    /// Source chosen, ready to start migration.
    Ready,
}

/// Application state.
pub struct App {
    pub screen: Screen,
    pub config: Config,
    pub client: Client,
    pub should_quit: bool,

    // Status data (cached, refreshed on demand)
    pub peacock_status: Option<PeacockStatus>,
    pub launcher_status: Option<LauncherStatus>,
    pub node_status: Option<NodeStatus>,
    pub service_status: Option<ServiceStatus>,
    pub game_installs: Vec<GameInstall>,
    pub legacy_install: Option<LegacyInstall>,

    // Main menu state
    pub menu_index: usize,
    pub menu_items: Vec<MenuItem>,

    // Install screen state
    pub install_step: String,
    pub install_progress: (u64, u64),
    pub install_done: bool,
    pub install_error: Option<String>,

    // Service screen state
    pub service_menu_index: usize,
    pub service_showing_log: bool,
    pub service_log_lines: Vec<String>,
    pub service_log_scroll: usize,

    // SDK screen state
    pub sdk_game_index: usize,
    pub sdk_action_index: usize,
    pub sdk_step: String,
    pub sdk_progress: (u64, u64),
    pub sdk_done: bool,
    pub sdk_error: Option<String>,

    // Settings screen state
    pub settings_field: SettingsField,
    pub settings_install_dir: String,
    pub settings_port: String,
    pub settings_editing: bool,
    pub settings_message: Option<String>,

    // Options screen state
    pub options: Vec<PeacockOption>,
    pub options_index: usize,
    pub options_message: Option<String>,

    // Migration screen state
    pub migration_mode: MigrationMode,
    pub migration_source_index: usize,
    pub migration_step: usize,
    pub migration_confirmed: bool,
    pub migration_done: bool,
    pub migration_error: Option<String>,
    pub migration_result: Option<MigrationResult>,
    pub migration_remove_old: bool,

    // Async message channel
    pub msg_tx: mpsc::UnboundedSender<AppMessage>,
    pub msg_rx: mpsc::UnboundedReceiver<AppMessage>,

    // Flag to indicate a background task is running
    pub task_running: bool,
}

#[derive(Debug, Clone)]
pub struct MenuItem {
    pub label: String,
    pub action: MenuAction,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuAction {
    Install,
    Service,
    Sdk,
    Settings,
    Options,
    Migration,
    DownloadLauncher,
    Quit,
}

impl App {
    pub fn new(config: Config, client: Client) -> Self {
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();

        let legacy_install = crate::core::migration::detect_legacy();

        let mut app = Self {
            screen: Screen::MainMenu,
            config,
            client,
            should_quit: false,

            peacock_status: None,
            launcher_status: None,
            node_status: None,
            service_status: None,
            game_installs: Vec::new(),
            legacy_install,

            menu_index: 0,
            menu_items: Vec::new(),

            install_step: String::new(),
            install_progress: (0, 0),
            install_done: false,
            install_error: None,

            service_menu_index: 0,
            service_showing_log: false,
            service_log_lines: Vec::new(),
            service_log_scroll: 0,

            sdk_game_index: 0,
            sdk_action_index: usize::MAX,
            sdk_step: String::new(),
            sdk_progress: (0, 0),
            sdk_done: false,
            sdk_error: None,

            settings_field: SettingsField::InstallDir,
            settings_install_dir: String::new(),
            settings_port: String::new(),
            settings_editing: false,
            settings_message: None,

            options: Vec::new(),
            options_index: 0,
            options_message: None,

            migration_mode: MigrationMode::SelectSource,
            migration_source_index: 0,
            migration_step: 0,
            migration_confirmed: false,
            migration_done: false,
            migration_error: None,
            migration_result: None,
            migration_remove_old: false,

            msg_tx,
            msg_rx,
            task_running: false,
        };

        app.refresh_status_sync();
        app.rebuild_menu();
        app
    }

    /// Refresh all cached status data (synchronous parts).
    pub fn refresh_status_sync(&mut self) {
        self.node_status = Some(crate::core::node::check_status(&self.config));
        self.service_status = Some(crate::core::service::status());
        self.game_installs = crate::core::game_detect::detect_all();
        self.legacy_install = crate::core::migration::detect_legacy();
    }

    /// Rebuild the main menu items based on current state.
    pub fn rebuild_menu(&mut self) {
        let mut items = vec![
            MenuItem {
                label: "Install / Update Peacock & Node".into(),
                action: MenuAction::Install,
                enabled: true,
            },
            MenuItem {
                label: "Manage Service".into(),
                action: MenuAction::Service,
                enabled: true,
            },
            MenuItem {
                label: "Manage ZHMModSDK".into(),
                action: MenuAction::Sdk,
                enabled: !self.game_installs.is_empty(),
            },
            MenuItem {
                label: "Settings".into(),
                action: MenuAction::Settings,
                enabled: true,
            },
            MenuItem {
                label: "Peacock Options".into(),
                action: MenuAction::Options,
                enabled: self.config.is_peacock_installed(),
            },
        ];

        if self.legacy_install.is_some() {
            items.push(MenuItem {
                label: "Migrate from old setup".into(),
                action: MenuAction::Migration,
                enabled: true,
            });
        } else {
            items.push(MenuItem {
                label: "Migrate from folder".into(),
                action: MenuAction::Migration,
                enabled: true,
            });
        }

        let download_label = match &self.launcher_status {
            Some(s) if s.update_available() => {
                "⬇ Download latest launcher (update available!)".into()
            }
            _ => "Download latest launcher".into(),
        };
        items.push(MenuItem {
            label: download_label,
            action: MenuAction::DownloadLauncher,
            enabled: true,
        });

        items.push(MenuItem {
            label: "Quit".into(),
            action: MenuAction::Quit,
            enabled: true,
        });

        self.menu_items = items;
    }

    /// Process any pending async messages.
    pub fn process_messages(&mut self) {
        while let Ok(msg) = self.msg_rx.try_recv() {
            match msg {
                AppMessage::Progress(downloaded, total) => match self.screen {
                    Screen::Install => self.install_progress = (downloaded, total),
                    Screen::Sdk => self.sdk_progress = (downloaded, total),
                    _ => {}
                },
                AppMessage::StepUpdate(step) => match self.screen {
                    Screen::Install => self.install_step = step,
                    Screen::Sdk => self.sdk_step = step,
                    _ => {}
                },
                AppMessage::TaskDone(msg) => {
                    self.task_running = false;
                    match self.screen {
                        Screen::Install => {
                            self.install_done = true;
                            self.install_step = msg;
                        }
                        Screen::Sdk => {
                            self.sdk_done = true;
                            self.sdk_step = msg;
                        }
                        Screen::Migration => {
                            self.migration_done = true;
                        }
                        _ => {}
                    }
                }
                AppMessage::TaskError(err) => {
                    self.task_running = false;
                    match self.screen {
                        Screen::Install => {
                            self.install_error = Some(err);
                        }
                        Screen::Sdk => {
                            self.sdk_error = Some(err);
                        }
                        Screen::Migration => {
                            self.migration_error = Some(err);
                        }
                        _ => {}
                    }
                }
                AppMessage::RefreshStatus => {
                    self.refresh_status_sync();
                    self.rebuild_menu();
                    // Spawn async re-check of Peacock status (requires network)
                    let client = self.client.clone();
                    let config = self.config.clone();
                    let tx = self.msg_tx.clone();
                    tokio::spawn(async move {
                        let status = crate::core::peacock::check_status(&client, &config).await;
                        let _ = tx.send(AppMessage::PeacockStatusLoaded(status));
                    });
                }
                AppMessage::ConfigUpdated(config) => {
                    self.config = config;
                }
                AppMessage::PeacockStatusLoaded(status) => {
                    self.peacock_status = Some(status);
                }
                AppMessage::LauncherStatusLoaded(status) => {
                    self.launcher_status = Some(status);
                    self.rebuild_menu();
                }
                AppMessage::FolderPicked(path) => {
                    if self.screen == Screen::Migration {
                        if let Some(path) = path {
                            if let Some(legacy) = crate::core::migration::inspect_path(&path) {
                                self.legacy_install = Some(legacy);
                                self.migration_mode = MigrationMode::Ready;
                            } else {
                                self.migration_error = Some(
                                    "No Peacock/ sub-directory found at the selected path.".into(),
                                );
                                self.migration_mode = MigrationMode::SelectSource;
                            }
                        } else {
                            // User cancelled the dialog
                            self.migration_mode = MigrationMode::SelectSource;
                        }
                    }
                }
                AppMessage::MigrationFinished(result, config) => {
                    self.task_running = false;
                    self.config = config;
                    self.migration_done = true;
                    self.migration_result = Some(result);
                }
            }
        }
    }

    /// Navigate to a screen and reset its state.
    pub fn go_to(&mut self, screen: Screen) {
        match &screen {
            Screen::Install => {
                self.install_step = String::new();
                self.install_progress = (0, 0);
                self.install_done = false;
                self.install_error = None;
            }
            Screen::Service => {
                self.service_menu_index = 0;
                self.service_showing_log = false;
                self.service_log_lines.clear();
                self.service_log_scroll = 0;
            }
            Screen::Sdk => {
                self.sdk_game_index = 0;
                self.sdk_action_index = usize::MAX;
                self.sdk_step = String::new();
                self.sdk_progress = (0, 0);
                self.sdk_done = false;
                self.sdk_error = None;
            }
            Screen::Settings => {
                self.settings_install_dir = self.config.install_dir.display().to_string();
                self.settings_port = self.config.port.to_string();
                self.settings_editing = false;
                self.settings_message = None;
                self.settings_field = SettingsField::InstallDir;
            }
            Screen::Options => {
                self.options = crate::core::options::load_options(&self.config.peacock_dir());
                self.options_index = 0;
                self.options_message = None;
            }
            Screen::Migration => {
                self.migration_mode = MigrationMode::SelectSource;
                self.migration_source_index = 0;
                self.migration_step = 0;
                self.migration_confirmed = false;
                self.migration_done = false;
                self.migration_error = None;
                self.migration_result = None;
                self.migration_remove_old = false;
            }
            Screen::MainMenu => {
                self.refresh_status_sync();
                self.rebuild_menu();
            }
        }
        self.screen = screen;
    }

    /// Go back to the main menu (unless a task is running).
    pub fn go_back(&mut self) {
        if self.task_running {
            return;
        }
        self.go_to(Screen::MainMenu);
    }
}
