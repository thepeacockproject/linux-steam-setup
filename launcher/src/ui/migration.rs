use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    prelude::*,
    widgets::{Paragraph, Wrap},
};

use super::{footer_text, info_line, titled_block};
use crate::app::{App, AppMessage, MigrationMode};
use crate::core::migration;

// ---------------------------------------------------------------------------
// Source-selection helpers
// ---------------------------------------------------------------------------

/// Build the list of source options visible in SelectSource mode.
fn source_options(app: &App) -> Vec<String> {
    let mut opts = Vec::new();
    if let Some(legacy) = &app.legacy_install {
        opts.push(format!("Auto-detected: {}", legacy.path.display()));
    }
    opts.push("Browse for folder…".into());
    opts
}

/// Number of source options.
fn source_count(app: &App) -> usize {
    if app.legacy_install.is_some() { 2 } else { 1 }
}

/// Whether the currently highlighted source option is "browse".
fn is_browse_selected(app: &App) -> bool {
    app.migration_source_index == source_count(app) - 1
}

// ---------------------------------------------------------------------------
// Render
// ---------------------------------------------------------------------------

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    match app.migration_mode {
        MigrationMode::SelectSource => render_select_source(frame, area, app),
        MigrationMode::PickingFolder => render_picking_folder(frame, area, app),
        MigrationMode::Ready => render_ready(frame, area, app),
    }
}

fn render_select_source(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(4),    // Options + error
            Constraint::Length(1), // Footer
        ])
        .split(area);

    let title = Paragraph::new("Select Migration Source")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Cyan).bold())
        .block(titled_block(""));
    frame.render_widget(title, chunks[0]);

    let opts = source_options(app);
    let mut lines: Vec<Line> = opts
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let selected = i == app.migration_source_index;
            let marker = if selected { "▸ " } else { "  " };
            let style = if selected {
                Style::default().fg(Color::Cyan).bold()
            } else {
                Style::default().fg(Color::White)
            };
            Line::styled(format!("{marker}{label}"), style)
        })
        .collect();

    // Show error from a previous failed pick, if any
    if let Some(err) = &app.migration_error {
        lines.push(Line::raw(""));
        lines.push(Line::styled(
            format!("⚠ {err}"),
            Style::default().fg(Color::Red),
        ));
    }

    let options = Paragraph::new(lines).block(titled_block("Choose Source"));
    frame.render_widget(options, chunks[1]);

    let hints = vec![("↑↓", "Navigate"), ("Enter", "Select"), ("Esc", "Back")];
    frame.render_widget(
        Paragraph::new(footer_text(&hints)).alignment(Alignment::Center),
        chunks[2],
    );
}

fn render_picking_folder(frame: &mut Frame, area: Rect, _app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(3),    // Message
            Constraint::Length(1), // Footer
        ])
        .split(area);

    let title = Paragraph::new("Select Migration Source")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Cyan).bold())
        .block(titled_block(""));
    frame.render_widget(title, chunks[0]);

    let msg = Paragraph::new(
        "Waiting for folder selection…\n\nPlease choose a folder in the file dialog.",
    )
    .alignment(Alignment::Center)
    .style(Style::default().fg(Color::Yellow))
    .block(titled_block(""));
    frame.render_widget(msg, chunks[1]);

    frame.render_widget(
        Paragraph::new(footer_text(&[("", "File dialog open…")])).alignment(Alignment::Center),
        chunks[2],
    );
}

fn render_ready(frame: &mut Frame, area: Rect, app: &App) {
    let Some(legacy) = &app.legacy_install else {
        let msg = Paragraph::new("No migration source selected.")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray))
            .block(titled_block("Migration"));
        frame.render_widget(msg, area);
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(8), // Detection info
            Constraint::Min(4),    // Status / action
            Constraint::Length(1), // Footer
        ])
        .split(area);

    let title = Paragraph::new("Migrate from Legacy Setup")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Cyan).bold())
        .block(titled_block(""));
    frame.render_widget(title, chunks[0]);

    // Detection info
    let path_str = legacy.path.display().to_string();
    let version_str = legacy.peacock_version.as_deref().unwrap_or("Unknown");

    let lines = vec![
        info_line("Source path", &path_str, Color::White),
        info_line(
            "Peacock",
            if legacy.has_peacock {
                "Found"
            } else {
                "Not found"
            },
            if legacy.has_peacock {
                Color::Green
            } else {
                Color::Red
            },
        ),
        info_line(
            "Node.js",
            if legacy.has_node {
                "Found"
            } else {
                "Not found"
            },
            if legacy.has_node {
                Color::Green
            } else {
                Color::Red
            },
        ),
        info_line("Peacock version", version_str, Color::White),
        info_line(
            "Userdata",
            if legacy.has_userdata {
                "Will be preserved"
            } else {
                "None"
            },
            if legacy.has_userdata {
                Color::Green
            } else {
                Color::DarkGray
            },
        ),
        info_line(
            "New location",
            &app.config.install_dir.display().to_string(),
            Color::Cyan,
        ),
        info_line(
            "Patching",
            "Switch to ZHMModSDK + OnlineTools",
            Color::Yellow,
        ),
    ];
    let info_panel = Paragraph::new(lines).block(titled_block("Migration Source"));
    frame.render_widget(info_panel, chunks[1]);

    // Status / action area
    let status_content = if app.migration_done {
        render_post_migration(app)
    } else if let Some(err) = &app.migration_error {
        Paragraph::new(format!("Error: {err}")).style(Style::default().fg(Color::Red))
    } else if app.task_running {
        Paragraph::new("⏳ Migrating files…").style(Style::default().fg(Color::Yellow))
    } else if app.migration_confirmed {
        Paragraph::new("Starting migration…").style(Style::default().fg(Color::Yellow))
    } else {
        Paragraph::new(
            "This will copy Peacock and Node.js into the launcher-managed location, replace any existing peacock.service with the new launcher service, and remove old PeacockPatcher.exe/WineLaunch.bat files it finds.\n\n\
             After migration, use Manage ZHMModSDK and OnlineTools in-game instead of the old batch-file patcher flow.\n\n\
             Press Enter to start migration.",
        )
        .style(Style::default().fg(Color::DarkGray))
        .wrap(Wrap { trim: false })
    };
    frame.render_widget(status_content.block(titled_block("Migration")), chunks[2]);

    // Footer
    let hints = if app.task_running {
        vec![("", "Migration in progress…")]
    } else if app.migration_done {
        vec![("↑↓", "Navigate"), ("Enter", "Select"), ("Esc", "Back")]
    } else {
        vec![("Enter", "Start migration"), ("Esc", "Back")]
    };
    frame.render_widget(
        Paragraph::new(footer_text(&hints)).alignment(Alignment::Center),
        chunks[3],
    );
}

fn render_post_migration(app: &App) -> Paragraph<'static> {
    let options = ["Keep old directory", "Delete old directory"];
    let mut lines = vec![
        Line::styled(
            "✓ Migration completed successfully!",
            Style::default().fg(Color::Green),
        ),
        Line::raw(""),
    ];

    if let Some(result) = &app.migration_result {
        if result.legacy_service_replaced {
            lines.push(Line::raw(
                "Replaced your legacy systemd service with the launcher-managed one.",
            ));
        } else if result.service_replaced {
            lines.push(Line::raw(
                "Reinstalled your existing Peacock service for the new launcher layout.",
            ));
        } else if result.service_migrated {
            lines.push(Line::raw(
                "Installed a new launcher-managed Peacock service.",
            ));
        }

        if result.service_enabled_restored {
            lines.push(Line::raw("Restored its boot-time enabled state."));
        }

        if result.service_restarted {
            lines.push(Line::raw(
                "Restarted the service because it was already running before migration.",
            ));
        }

        if result.legacy_files_cleaned > 0 {
            lines.push(Line::raw(format!(
                "Removed {} old patcher file(s) from detected game folders.",
                result.legacy_files_cleaned
            )));
        }

        if let Some(error) = &result.service_error {
            lines.push(Line::styled(
                format!("Service warning: {error}"),
                Style::default().fg(Color::Yellow),
            ));
        }

        lines.push(Line::raw(""));
    }

    lines.push(Line::styled(
        "Next steps:",
        Style::default().fg(Color::Cyan).bold(),
    ));
    lines.push(Line::raw(
        "1. Open Manage ZHMModSDK and install it into HITMAN 3.",
    ));
    lines.push(Line::raw(
        "2. Launch the game and use OnlineTools instead of PeacockPatcher or WineLaunch.bat.",
    ));
    lines.push(Line::raw("3. In OnlineTools, load the old patcher settings and confirm the address is localhost:3000."));
    lines.push(Line::raw(""));

    for (i, label) in options.iter().enumerate() {
        let sel = i == app.migration_step;
        let marker = if sel { "▸ " } else { "  " };
        let style = if sel {
            Style::default().fg(Color::Cyan).bold()
        } else {
            Style::default().fg(Color::White)
        };
        lines.push(Line::styled(format!("{marker}{label}"), style));
    }
    Paragraph::new(lines).wrap(Wrap { trim: false })
}

// ---------------------------------------------------------------------------
// Key handling
// ---------------------------------------------------------------------------

pub fn handle_key(app: &mut App, key: KeyEvent) {
    if key.kind != KeyEventKind::Press {
        return;
    }
    if app.task_running {
        return;
    }

    match app.migration_mode {
        MigrationMode::SelectSource => handle_select_source(app, key),
        MigrationMode::PickingFolder => { /* ignore input while dialog is open */ }
        MigrationMode::Ready => handle_ready(app, key),
    }
}

fn handle_select_source(app: &mut App, key: KeyEvent) {
    let count = source_count(app);
    match key.code {
        KeyCode::Up => {
            if app.migration_source_index > 0 {
                app.migration_source_index -= 1;
            }
        }
        KeyCode::Down => {
            if app.migration_source_index + 1 < count {
                app.migration_source_index += 1;
            }
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            if is_browse_selected(app) {
                // Open native folder picker in a background task
                app.migration_mode = MigrationMode::PickingFolder;
                app.migration_error = None;
                let tx = app.msg_tx.clone();
                tokio::task::spawn_blocking(move || {
                    let picked = rfd::FileDialog::new()
                        .set_title("Select Peacock folder to migrate")
                        .pick_folder();
                    let _ = tx.send(AppMessage::FolderPicked(picked));
                });
            } else {
                // Auto-detected legacy install already in app.legacy_install
                app.migration_mode = MigrationMode::Ready;
            }
        }
        KeyCode::Esc => {
            app.go_back();
        }
        _ => {}
    }
}

fn handle_ready(app: &mut App, key: KeyEvent) {
    if app.migration_done {
        match key.code {
            KeyCode::Up => {
                if app.migration_step > 0 {
                    app.migration_step -= 1;
                }
            }
            KeyCode::Down => {
                if app.migration_step < 1 {
                    app.migration_step += 1;
                }
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if app.migration_step == 0 {
                    app.go_back();
                } else if let Some(legacy) = &app.legacy_install {
                    let legacy_clone = legacy.clone();
                    if let Err(e) = migration::remove_legacy_dir(&legacy_clone) {
                        app.migration_error = Some(format!("Failed to remove old directory: {e}"));
                        app.migration_done = false;
                    } else {
                        app.legacy_install = None;
                        app.go_back();
                    }
                }
            }
            KeyCode::Esc => {
                app.go_back();
            }
            _ => {}
        }
        return;
    }

    match key.code {
        KeyCode::Enter if !app.migration_confirmed => {
            start_migration(app);
        }
        KeyCode::Esc => {
            app.migration_mode = MigrationMode::SelectSource;
            app.migration_error = None;
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn start_migration(app: &mut App) {
    let Some(legacy) = &app.legacy_install else {
        return;
    };

    app.task_running = true;
    app.migration_confirmed = true;

    let legacy = legacy.clone();
    let mut config = app.config.clone();
    let tx = app.msg_tx.clone();

    tokio::spawn(async move {
        let result = tokio::task::spawn_blocking(move || {
            let migration_result = migration::migrate(&legacy, &mut config)?;
            Ok::<_, anyhow::Error>((migration_result, config))
        })
        .await;

        match result {
            Ok(Ok((migration_result, config))) => {
                let _ = tx.send(AppMessage::MigrationFinished(migration_result, config));
                let _ = tx.send(AppMessage::RefreshStatus);
            }
            Ok(Err(e)) => {
                let _ = tx.send(AppMessage::TaskError(format!("Migration failed: {e}")));
            }
            Err(e) => {
                let _ = tx.send(AppMessage::TaskError(format!(
                    "Migration task panicked: {e}"
                )));
            }
        }
    });
}
