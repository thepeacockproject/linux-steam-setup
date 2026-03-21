use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    prelude::*,
    widgets::{List, ListItem, Paragraph, Wrap},
};

use super::{footer_text, info_line, titled_block};
use crate::app::App;
use crate::core::service::{self, ServiceState};

const LOG_LINES: usize = 200;

struct ServiceAction {
    label: &'static str,
    enabled: bool,
}

fn get_actions(state: &ServiceState, enabled: bool) -> Vec<ServiceAction> {
    match state {
        ServiceState::NotInstalled => vec![ServiceAction {
            label: "Install Service",
            enabled: true,
        }],
        ServiceState::Active => vec![
            ServiceAction {
                label: "View Logs",
                enabled: true,
            },
            ServiceAction {
                label: "Stop Service",
                enabled: true,
            },
            ServiceAction {
                label: if enabled {
                    "Disable on Boot"
                } else {
                    "Enable on Boot"
                },
                enabled: true,
            },
            ServiceAction {
                label: "Remove Service",
                enabled: true,
            },
        ],
        ServiceState::Inactive | ServiceState::Failed => vec![
            ServiceAction {
                label: "View Logs",
                enabled: true,
            },
            ServiceAction {
                label: "Start Service",
                enabled: true,
            },
            ServiceAction {
                label: if enabled {
                    "Disable on Boot"
                } else {
                    "Enable on Boot"
                },
                enabled: true,
            },
            ServiceAction {
                label: "Remove Service",
                enabled: true,
            },
        ],
    }
}

// ---------------------------------------------------------------------------
// Render
// ---------------------------------------------------------------------------

pub fn render(frame: &mut Frame, app: &App) {
    if app.service_showing_log {
        render_log(frame, app);
    } else {
        render_main(frame, app);
    }
}

fn render_main(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(5), // Status
            Constraint::Min(5),    // Actions
            Constraint::Length(1), // Footer
        ])
        .split(area);

    // Title
    let title = Paragraph::new("Service Management")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Cyan).bold())
        .block(titled_block(""));
    frame.render_widget(title, chunks[0]);

    // Status
    let status = app
        .service_status
        .as_ref()
        .cloned()
        .unwrap_or(service::ServiceStatus {
            state: ServiceState::NotInstalled,
            enabled: false,
        });

    let (state_str, state_color) = match &status.state {
        ServiceState::Active => ("Active (running)", Color::Green),
        ServiceState::Inactive => ("Inactive (stopped)", Color::Yellow),
        ServiceState::Failed => ("Failed", Color::Red),
        ServiceState::NotInstalled => ("Not installed", Color::DarkGray),
    };

    let mut lines = vec![info_line("State", state_str, state_color)];
    if status.state != ServiceState::NotInstalled {
        let boot_str = if status.enabled {
            "Enabled"
        } else {
            "Disabled"
        };
        let boot_color = if status.enabled {
            Color::Green
        } else {
            Color::DarkGray
        };
        lines.push(info_line("Start on boot", boot_str, boot_color));
    }

    let status_panel = Paragraph::new(lines).block(titled_block("Current Status"));
    frame.render_widget(status_panel, chunks[1]);

    // Actions
    let actions = get_actions(&status.state, status.enabled);
    let items: Vec<ListItem> = actions
        .iter()
        .enumerate()
        .map(|(i, action)| {
            let style = if !action.enabled {
                Style::default().fg(Color::DarkGray)
            } else if i == app.service_menu_index {
                Style::default().fg(Color::Cyan).bold()
            } else {
                Style::default().fg(Color::White)
            };
            let prefix = if i == app.service_menu_index {
                "▸ "
            } else {
                "  "
            };
            ListItem::new(format!("{prefix}{}", action.label)).style(style)
        })
        .collect();

    let list = List::new(items).block(titled_block("Actions"));
    frame.render_widget(list, chunks[2]);

    // Footer
    let footer = footer_text(&[("↑↓", "Navigate"), ("Enter", "Execute"), ("Esc", "Back")]);
    frame.render_widget(
        Paragraph::new(footer).alignment(Alignment::Center),
        chunks[3],
    );
}

fn render_log(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(3),    // Log content
            Constraint::Length(1), // Footer
        ])
        .split(area);

    let total = app.service_log_lines.len();
    let title_text = format!("Service Logs ({total} lines)");
    let title = Paragraph::new(title_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Cyan).bold())
        .block(titled_block(""));
    frame.render_widget(title, chunks[0]);

    // Build log text with scroll offset
    let log_area_height = chunks[1].height.saturating_sub(2) as usize; // border takes 2 lines
    let lines: Vec<Line> = app
        .service_log_lines
        .iter()
        .skip(app.service_log_scroll)
        .take(log_area_height)
        .map(|l| Line::raw(l.as_str()))
        .collect();

    let log_panel = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(Color::White))
        .block(titled_block("journalctl --user -u peacock"));
    frame.render_widget(log_panel, chunks[1]);

    let footer = footer_text(&[
        ("↑↓", "Scroll"),
        ("Home/End", "Top/Bottom"),
        ("r", "Refresh"),
        ("Esc", "Back"),
    ]);
    frame.render_widget(
        Paragraph::new(footer).alignment(Alignment::Center),
        chunks[2],
    );
}

// ---------------------------------------------------------------------------
// Key handling
// ---------------------------------------------------------------------------

pub fn handle_key(app: &mut App, key: KeyEvent) {
    if key.kind != KeyEventKind::Press {
        return;
    }

    if app.service_showing_log {
        handle_log_key(app, key);
    } else {
        handle_main_key(app, key);
    }
}

fn handle_main_key(app: &mut App, key: KeyEvent) {
    let status = app
        .service_status
        .clone()
        .unwrap_or(service::ServiceStatus {
            state: ServiceState::NotInstalled,
            enabled: false,
        });

    let actions = get_actions(&status.state, status.enabled);
    let max_index = actions.len().saturating_sub(1);

    match key.code {
        KeyCode::Up => {
            if app.service_menu_index > 0 {
                app.service_menu_index -= 1;
            }
        }
        KeyCode::Down => {
            if app.service_menu_index < max_index {
                app.service_menu_index += 1;
            }
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            if let Some(action) = actions.get(app.service_menu_index) {
                if !action.enabled {
                    return;
                }
                if action.label == "View Logs" {
                    app.service_log_lines = service::journal(LOG_LINES);
                    // Auto-scroll to bottom
                    app.service_log_scroll = app.service_log_lines.len().saturating_sub(1);
                    app.service_showing_log = true;
                    return;
                }
                let result = execute_action(action.label, &status, &app.config);
                if let Err(e) = result {
                    eprintln!("Service action error: {e}");
                }
                // Refresh status after action
                app.service_status = Some(service::status());
                app.service_menu_index = 0;
            }
        }
        KeyCode::Esc => {
            app.go_back();
        }
        _ => {}
    }
}

fn handle_log_key(app: &mut App, key: KeyEvent) {
    let max_scroll = app.service_log_lines.len().saturating_sub(1);

    match key.code {
        KeyCode::Up => {
            app.service_log_scroll = app.service_log_scroll.saturating_sub(1);
        }
        KeyCode::Down => {
            if app.service_log_scroll < max_scroll {
                app.service_log_scroll += 1;
            }
        }
        KeyCode::PageUp => {
            app.service_log_scroll = app.service_log_scroll.saturating_sub(20);
        }
        KeyCode::PageDown => {
            app.service_log_scroll = (app.service_log_scroll + 20).min(max_scroll);
        }
        KeyCode::Home => {
            app.service_log_scroll = 0;
        }
        KeyCode::End => {
            app.service_log_scroll = max_scroll;
        }
        KeyCode::Char('r') => {
            // Refresh log
            app.service_log_lines = service::journal(LOG_LINES);
            app.service_log_scroll = app.service_log_lines.len().saturating_sub(1);
        }
        KeyCode::Esc => {
            app.service_showing_log = false;
        }
        _ => {}
    }
}

fn execute_action(
    label: &str,
    _status: &service::ServiceStatus,
    config: &crate::core::config::Config,
) -> anyhow::Result<()> {
    match label {
        "Install Service" => service::install(config),
        "Remove Service" => service::remove(),
        "Start Service" => service::start(),
        "Stop Service" => service::stop(),
        "Enable on Boot" => service::enable(),
        "Disable on Boot" => service::disable(),
        _ => Ok(()),
    }
}
