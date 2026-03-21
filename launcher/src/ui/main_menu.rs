use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    prelude::*,
    widgets::{List, ListItem, Paragraph},
};

use super::{footer_text, info_line, titled_block};
use crate::app::{App, MenuAction, Screen};
use crate::core::service::ServiceState;

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(9), // Status panel
            Constraint::Min(6),    // Menu
            Constraint::Length(1), // Footer
        ])
        .split(area);

    // Title
    let title = Paragraph::new("Peacock Linux Launcher")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Cyan).bold())
        .block(titled_block(""));
    frame.render_widget(title, chunks[0]);

    // Status panel
    render_status(frame, app, chunks[1]);

    // Menu
    render_menu(frame, app, chunks[2]);

    // Footer
    let footer = footer_text(&[("↑↓", "Navigate"), ("Enter", "Select"), ("Esc", "Quit")]);
    frame.render_widget(
        Paragraph::new(footer).alignment(Alignment::Center),
        chunks[3],
    );
}

fn render_status(frame: &mut Frame, app: &App, area: Rect) {
    let mut lines = Vec::new();

    // Launcher version
    if let Some(status) = &app.launcher_status {
        let (value, color) = if status.update_available() {
            let latest = status
                .latest_version
                .as_deref()
                .unwrap_or("unknown")
                .trim_start_matches('v');
            (
                format!(
                    "v{} → v{latest} available — please download the latest release!",
                    status.current_version
                ),
                Color::Yellow,
            )
        } else {
            (
                format!("v{} (up to date)", status.current_version),
                Color::Green,
            )
        };
        lines.push(info_line("Launcher", &value, color));
    } else {
        lines.push(info_line(
            "Launcher",
            &format!(
                "v{} (checking for updates…)",
                crate::core::launcher::LAUNCHER_VERSION
            ),
            Color::DarkGray,
        ));
    }

    // Peacock version
    if let Some(status) = &app.peacock_status {
        let (value, color) = match (&status.installed_version, &status.latest_version) {
            (Some(installed), Some(latest)) if installed == latest => {
                (format!("{installed} (up to date)"), Color::Green)
            }
            (Some(installed), Some(latest)) => {
                (format!("{installed} → {latest} available"), Color::Yellow)
            }
            (Some(installed), None) => (installed.clone(), Color::Green),
            (None, Some(latest)) => (format!("Not installed (latest: {latest})"), Color::Red),
            (None, None) => ("Not installed".into(), Color::Red),
        };
        lines.push(info_line("Peacock", &value, color));
    } else {
        lines.push(info_line("Peacock", "Checking...", Color::DarkGray));
    }

    // Node version
    if let Some(status) = &app.node_status {
        let (value, color) = match (&status.installed_version, &status.required_version) {
            (Some(installed), Some(required)) if installed.trim() == required.trim() => {
                (format!("{installed} (matches required)"), Color::Green)
            }
            (Some(installed), Some(required)) => {
                (format!("{installed} (need {required})"), Color::Yellow)
            }
            (Some(installed), None) => (installed.clone(), Color::Green),
            (None, Some(required)) => (format!("Not installed (need {required})"), Color::Red),
            (None, None) => ("Not installed".into(), Color::Red),
        };
        lines.push(info_line("Node.js", &value, color));
    } else {
        lines.push(info_line("Node.js", "Checking...", Color::DarkGray));
    }

    // Service status
    if let Some(status) = &app.service_status {
        let (value, color): (String, Color) = match &status.state {
            ServiceState::Active => ("Active (running)".into(), Color::Green),
            ServiceState::Inactive => ("Inactive (stopped)".into(), Color::Yellow),
            ServiceState::Failed => ("Failed".into(), Color::Red),
            ServiceState::NotInstalled => ("Not installed".into(), Color::DarkGray),
        };
        let enabled_str = if status.state != ServiceState::NotInstalled {
            if status.enabled {
                " [enabled on boot]"
            } else {
                " [manual start]"
            }
        } else {
            ""
        };
        lines.push(info_line(
            "Service",
            &format!("{value}{enabled_str}"),
            color,
        ));
    }

    // Game installs
    if app.game_installs.is_empty() {
        lines.push(info_line(
            "Games",
            "No Hitman 3 installs found",
            Color::DarkGray,
        ));
    } else {
        let game_list: Vec<String> = app
            .game_installs
            .iter()
            .map(|g| {
                let sdk = if crate::core::game_detect::is_sdk_installed(g) {
                    " [SDK ✓]"
                } else {
                    ""
                };
                format!("{}{sdk}", g.launcher)
            })
            .collect();
        lines.push(info_line("Games", &game_list.join(", "), Color::White));
    }

    let block = titled_block("Status");
    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn render_menu(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .menu_items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let style = if !item.enabled {
                Style::default().fg(Color::DarkGray)
            } else if i == app.menu_index {
                Style::default().fg(Color::Cyan).bold()
            } else {
                Style::default().fg(Color::White)
            };

            let prefix = if i == app.menu_index { "▸ " } else { "  " };
            ListItem::new(format!("{prefix}{}", item.label)).style(style)
        })
        .collect();

    let list = List::new(items).block(titled_block("Actions"));
    frame.render_widget(list, area);
}

pub fn handle_key(app: &mut App, key: KeyEvent) {
    if key.kind != KeyEventKind::Press {
        return;
    }

    match key.code {
        KeyCode::Up => {
            if app.menu_index > 0 {
                app.menu_index -= 1;
                // Skip disabled items
                while app.menu_index > 0 && !app.menu_items[app.menu_index].enabled {
                    app.menu_index -= 1;
                }
            }
        }
        KeyCode::Down => {
            if app.menu_index < app.menu_items.len().saturating_sub(1) {
                app.menu_index += 1;
                // Skip disabled items
                while app.menu_index < app.menu_items.len().saturating_sub(1)
                    && !app.menu_items[app.menu_index].enabled
                {
                    app.menu_index += 1;
                }
            }
        }
        KeyCode::Enter => {
            if let Some(item) = app.menu_items.get(app.menu_index) {
                if !item.enabled {
                    return;
                }
                match &item.action {
                    MenuAction::Install => app.go_to(Screen::Install),
                    MenuAction::Service => app.go_to(Screen::Service),
                    MenuAction::Sdk => app.go_to(Screen::Sdk),
                    MenuAction::Settings => app.go_to(Screen::Settings),
                    MenuAction::Options => app.go_to(Screen::Options),
                    MenuAction::Migration => app.go_to(Screen::Migration),
                    MenuAction::DownloadLauncher => {
                        crate::core::launcher::open_download_page();
                    }
                    MenuAction::Quit => app.should_quit = true,
                }
            }
        }
        KeyCode::Esc => {
            app.should_quit = true;
        }
        _ => {}
    }
}
