use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    prelude::*,
    widgets::{List, ListItem, Paragraph},
};

use super::{footer_text, format_bytes, titled_block};
use crate::app::{App, AppMessage};
use crate::core::game_detect;

/// Whether the user is selecting a game or an action for the selected game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Focus {
    GameList,
    ActionMenu,
}

fn current_focus(app: &App) -> Focus {
    // We encode the focus via sdk_action_index: usize::MAX means game list focus
    if app.sdk_action_index == usize::MAX {
        Focus::GameList
    } else {
        Focus::ActionMenu
    }
}

/// Build the list of available actions for the currently selected game install.
fn actions_for_install(app: &App) -> Vec<&'static str> {
    if app.game_installs.is_empty() {
        return Vec::new();
    }
    let install = &app.game_installs[app.sdk_game_index];
    let has_sdk = game_detect::is_sdk_installed(install);

    if has_sdk {
        vec!["Update SDK", "Remove SDK"]
    } else {
        vec!["Install SDK"]
    }
}

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    if app.game_installs.is_empty() {
        let msg = Paragraph::new("No Hitman 3 installations found.\n\nEnsure the game is installed via Steam or Heroic.\n\nPress Esc to go back.")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Yellow))
            .block(titled_block("ZHMModSDK"));
        frame.render_widget(msg, area);
        return;
    }

    let focus = current_focus(app);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(6),    // Game installs (needs more room for paths)
            Constraint::Length(5), // Actions menu
            Constraint::Length(3), // Progress / status
            Constraint::Length(1), // Footer
        ])
        .split(area);

    // Title
    let title = Paragraph::new("ZHMModSDK Management")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Cyan).bold())
        .block(titled_block(""));
    frame.render_widget(title, chunks[0]);

    // Game installs list — show launcher, path, and SDK status
    let game_items: Vec<ListItem> = app
        .game_installs
        .iter()
        .enumerate()
        .map(|(i, install)| {
            let sdk_installed = game_detect::is_sdk_installed(install);

            let status_str = if sdk_installed {
                "SDK ✓"
            } else {
                "Not installed"
            };

            let status_color = if sdk_installed {
                Color::Green
            } else {
                Color::DarkGray
            };

            let is_selected = i == app.sdk_game_index;
            let prefix = if is_selected { "▸ " } else { "  " };
            let name_style = if is_selected && focus == Focus::GameList {
                Style::default().fg(Color::Cyan).bold()
            } else if is_selected {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::White)
            };

            let path_str = install.game_dir.display().to_string();

            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(format!("{prefix}{} ", install.launcher), name_style),
                    Span::styled(format!("[{status_str}]"), Style::default().fg(status_color)),
                ]),
                Line::from(Span::styled(
                    format!("    {path_str}"),
                    Style::default().fg(Color::DarkGray),
                )),
            ])
        })
        .collect();

    let game_block_style = if focus == Focus::GameList {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let game_list = List::new(game_items).block(
        ratatui::widgets::Block::default()
            .borders(ratatui::widgets::Borders::ALL)
            .title(" Detected Installs ")
            .border_style(game_block_style),
    );
    frame.render_widget(game_list, chunks[1]);

    // Action menu for selected game
    let actions = actions_for_install(app);
    let action_items: Vec<ListItem> = actions
        .iter()
        .enumerate()
        .map(|(i, action)| {
            let is_selected = focus == Focus::ActionMenu && i == app.sdk_action_index;
            let prefix = if is_selected { "▸ " } else { "  " };
            let style = if is_selected {
                Style::default().fg(Color::Cyan).bold()
            } else if focus == Focus::ActionMenu {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            ListItem::new(Span::styled(format!("{prefix}{action}"), style))
        })
        .collect();

    let action_block_style = if focus == Focus::ActionMenu {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let action_list = List::new(action_items).block(
        ratatui::widgets::Block::default()
            .borders(ratatui::widgets::Borders::ALL)
            .title(" Actions ")
            .border_style(action_block_style),
    );
    frame.render_widget(action_list, chunks[2]);

    // Progress / status area
    let status_widget: Paragraph = if let Some(err) = &app.sdk_error {
        Paragraph::new(format!("Error: {err}")).style(Style::default().fg(Color::Red))
    } else if app.sdk_done {
        Paragraph::new(format!("✓ {}", app.sdk_step)).style(Style::default().fg(Color::Green))
    } else if app.task_running {
        let (downloaded, total) = app.sdk_progress;
        let ratio_text = if total > 0 {
            format!(
                "⏳ {} — {} / {} ({:.0}%)",
                app.sdk_step,
                format_bytes(downloaded),
                format_bytes(total),
                (downloaded as f64 / total as f64 * 100.0).min(100.0)
            )
        } else {
            format!("⏳ {}", app.sdk_step)
        };
        Paragraph::new(ratio_text).style(Style::default().fg(Color::Yellow))
    } else {
        Paragraph::new("Select a game then press Enter to choose an action")
            .style(Style::default().fg(Color::DarkGray))
    };
    frame.render_widget(status_widget.block(titled_block("Status")), chunks[3]);

    // Footer
    let hints = if app.task_running {
        vec![("", "Operation in progress...")]
    } else if app.sdk_done || app.sdk_error.is_some() {
        vec![("Esc", "Back")]
    } else if focus == Focus::ActionMenu {
        vec![
            ("↑↓", "Select action"),
            ("Enter", "Confirm"),
            ("Esc", "Back to games"),
        ]
    } else {
        vec![("↑↓", "Select game"), ("Enter", "Actions"), ("Esc", "Back")]
    };
    frame.render_widget(
        Paragraph::new(footer_text(&hints)).alignment(Alignment::Center),
        chunks[4],
    );
}

pub fn handle_key(app: &mut App, key: KeyEvent) {
    if key.kind != KeyEventKind::Press {
        return;
    }

    if app.task_running {
        return;
    }

    // Allow dismissing results with Esc from any state
    if key.code == KeyCode::Esc {
        if app.sdk_done || app.sdk_error.is_some() {
            // Reset and go back to game selection
            app.sdk_done = false;
            app.sdk_error = None;
            app.sdk_step.clear();
            app.sdk_action_index = usize::MAX; // back to game list focus
            return;
        }
        if current_focus(app) == Focus::ActionMenu {
            app.sdk_action_index = usize::MAX; // back to game list
            return;
        }
        app.go_back();
        return;
    }

    if app.game_installs.is_empty() {
        return;
    }

    let focus = current_focus(app);

    match focus {
        Focus::GameList => match key.code {
            KeyCode::Up => {
                if app.sdk_game_index > 0 {
                    app.sdk_game_index -= 1;
                }
            }
            KeyCode::Down => {
                if app.sdk_game_index < app.game_installs.len().saturating_sub(1) {
                    app.sdk_game_index += 1;
                }
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                // Enter action menu for selected game
                app.sdk_action_index = 0;
                app.sdk_done = false;
                app.sdk_error = None;
                app.sdk_step.clear();
            }
            _ => {}
        },
        Focus::ActionMenu => {
            let actions = actions_for_install(app);
            match key.code {
                KeyCode::Up => {
                    if app.sdk_action_index > 0 {
                        app.sdk_action_index -= 1;
                    }
                }
                KeyCode::Down => {
                    if app.sdk_action_index < actions.len().saturating_sub(1) {
                        app.sdk_action_index += 1;
                    }
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    if let Some(&action) = actions.get(app.sdk_action_index) {
                        execute_sdk_action(app, action);
                    }
                }
                _ => {}
            }
        }
    }
}

fn execute_sdk_action(app: &mut App, action: &str) {
    match action {
        "Install SDK" | "Update SDK" => {
            start_sdk_install(app);
        }
        "Remove SDK" => {
            let install = &app.game_installs[app.sdk_game_index];
            if let Err(e) = crate::core::sdk::remove(install) {
                app.sdk_error = Some(format!("{e}"));
            } else {
                app.sdk_step = "SDK removed successfully".into();
                app.sdk_done = true;
            }
        }
        _ => {}
    }
}

fn start_sdk_install(app: &mut App) {
    let install = app.game_installs[app.sdk_game_index].clone();

    app.task_running = true;
    app.sdk_step = "Downloading ZHMModSDK...".into();
    app.sdk_progress = (0, 0);
    app.sdk_error = None;
    app.sdk_done = false;

    let tx = app.msg_tx.clone();
    let client = app.client.clone();

    tokio::spawn(async move {
        let progress_tx = tx.clone();
        let progress_fn: Option<crate::core::download::ProgressFn> =
            Some(std::sync::Arc::new(move |downloaded, total| {
                let _ = progress_tx.send(AppMessage::Progress(downloaded, total));
            }));

        let _ = tx.send(AppMessage::StepUpdate("Downloading ZHMModSDK...".into()));

        match crate::core::sdk::install_or_update(&client, &install, progress_fn).await {
            Ok(version) => {
                let _ = tx.send(AppMessage::TaskDone(format!(
                    "ZHMModSDK {version} installed successfully"
                )));
                let _ = tx.send(AppMessage::RefreshStatus);
            }
            Err(e) => {
                let _ = tx.send(AppMessage::TaskError(format!("SDK install failed: {e}")));
            }
        }
    });
}
