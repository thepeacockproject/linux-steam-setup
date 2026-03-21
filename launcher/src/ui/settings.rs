use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{prelude::*, widgets::Paragraph};

use super::{footer_text, titled_block};
use crate::app::{App, SettingsField};

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(3), // Install dir field
            Constraint::Length(3), // Port field
            Constraint::Length(3), // Save button
            Constraint::Length(3), // Message
            Constraint::Min(1),    // Spacer
            Constraint::Length(1), // Footer
        ])
        .split(area);

    // Title
    let title = Paragraph::new("Settings")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Cyan).bold())
        .block(titled_block(""));
    frame.render_widget(title, chunks[0]);

    // Install directory
    let dir_selected = app.settings_field == SettingsField::InstallDir;
    let dir_style = if dir_selected && app.settings_editing {
        Style::default().fg(Color::White).bg(Color::DarkGray)
    } else if dir_selected {
        Style::default().fg(Color::Cyan).bold()
    } else {
        Style::default().fg(Color::White)
    };

    let dir_prefix = if dir_selected { "▸ " } else { "  " };
    let dir_field = Paragraph::new(format!(
        "{dir_prefix}Install directory: {}",
        app.settings_install_dir
    ))
    .style(dir_style)
    .block(titled_block("Install Directory"));
    frame.render_widget(dir_field, chunks[1]);

    // Port
    let port_selected = app.settings_field == SettingsField::Port;
    let port_style = if port_selected && app.settings_editing {
        Style::default().fg(Color::White).bg(Color::DarkGray)
    } else if port_selected {
        Style::default().fg(Color::Cyan).bold()
    } else {
        Style::default().fg(Color::White)
    };

    let port_prefix = if port_selected { "▸ " } else { "  " };
    let port_field = Paragraph::new(format!("{port_prefix}Peacock port: {}", app.settings_port))
        .style(port_style)
        .block(titled_block("Port"));
    frame.render_widget(port_field, chunks[2]);

    // Save button
    let save_selected = app.settings_field == SettingsField::Save;
    let save_style = if save_selected {
        Style::default().fg(Color::Cyan).bold()
    } else {
        Style::default().fg(Color::White)
    };
    let save_prefix = if save_selected { "▸ " } else { "  " };
    let save_btn = Paragraph::new(format!("{save_prefix}[ Save Settings ]"))
        .style(save_style)
        .block(titled_block(""));
    frame.render_widget(save_btn, chunks[3]);

    // Message
    if let Some(msg) = &app.settings_message {
        let color = if msg.starts_with("Error") {
            Color::Red
        } else {
            Color::Green
        };
        frame.render_widget(
            Paragraph::new(msg.as_str()).style(Style::default().fg(color)),
            chunks[4],
        );
    }

    // Footer
    let hints = if app.settings_editing {
        vec![
            ("Type", "Edit value"),
            ("Enter", "Confirm"),
            ("Esc", "Cancel"),
        ]
    } else {
        vec![("↑↓", "Select"), ("Enter", "Edit / Save"), ("Esc", "Back")]
    };
    frame.render_widget(
        Paragraph::new(footer_text(&hints)).alignment(Alignment::Center),
        chunks[6],
    );
}

pub fn handle_key(app: &mut App, key: KeyEvent) {
    if key.kind != KeyEventKind::Press {
        return;
    }

    if app.settings_editing {
        handle_editing(app, key);
    } else {
        handle_navigation(app, key);
    }
}

fn handle_navigation(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Up => {
            app.settings_field = match app.settings_field {
                SettingsField::InstallDir => SettingsField::InstallDir,
                SettingsField::Port => SettingsField::InstallDir,
                SettingsField::Save => SettingsField::Port,
            };
        }
        KeyCode::Down => {
            app.settings_field = match app.settings_field {
                SettingsField::InstallDir => SettingsField::Port,
                SettingsField::Port => SettingsField::Save,
                SettingsField::Save => SettingsField::Save,
            };
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            if app.settings_field == SettingsField::Save {
                save_settings(app);
            } else {
                app.settings_editing = true;
            }
        }
        KeyCode::Esc => {
            app.go_back();
        }
        _ => {}
    }
}

fn handle_editing(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Enter | KeyCode::Esc => {
            app.settings_editing = false;
        }
        KeyCode::Backspace => match app.settings_field {
            SettingsField::InstallDir => {
                app.settings_install_dir.pop();
            }
            SettingsField::Port => {
                app.settings_port.pop();
            }
            SettingsField::Save => {}
        },
        KeyCode::Char(c) => match app.settings_field {
            SettingsField::InstallDir => {
                app.settings_install_dir.push(c);
            }
            SettingsField::Port => {
                if c.is_ascii_digit() {
                    app.settings_port.push(c);
                }
            }
            SettingsField::Save => {}
        },
        _ => {}
    }
}

fn save_settings(app: &mut App) {
    // Validate port
    let port: u16 = match app.settings_port.parse() {
        Ok(p) if p > 0 => p,
        _ => {
            app.settings_message = Some("Error: Invalid port number".into());
            return;
        }
    };

    let install_dir = std::path::PathBuf::from(&app.settings_install_dir);
    if app.settings_install_dir.is_empty() {
        app.settings_message = Some("Error: Install directory cannot be empty".into());
        return;
    }

    let dir_changed = app.config.install_dir != install_dir;

    app.config.install_dir = install_dir;
    app.config.port = port;

    match app.config.save() {
        Ok(()) => {
            let mut msg = "Settings saved".to_string();
            if dir_changed {
                msg.push_str(" (install directory changed — re-install recommended)");
            }
            app.settings_message = Some(msg);
        }
        Err(e) => {
            app.settings_message = Some(format!("Error: Failed to save: {e}"));
        }
    }
}
