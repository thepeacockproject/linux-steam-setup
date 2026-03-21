use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    prelude::*,
    widgets::{Gauge, Paragraph},
};

use super::{footer_text, format_bytes, info_line, titled_block};
use crate::app::{App, AppMessage};

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(5), // Version info
            Constraint::Length(3), // Progress bar
            Constraint::Min(3),    // Status/log
            Constraint::Length(1), // Footer
        ])
        .split(area);

    // Title
    let title = Paragraph::new("Install / Update Peacock & Node")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Cyan).bold())
        .block(titled_block(""));
    frame.render_widget(title, chunks[0]);

    // Version info
    let mut lines = Vec::new();
    if let Some(status) = &app.peacock_status {
        let installed = status
            .installed_version
            .as_deref()
            .unwrap_or("Not installed");
        let latest = status.latest_version.as_deref().unwrap_or("Unknown");
        lines.push(info_line("Peacock installed", installed, Color::White));
        lines.push(info_line("Peacock latest", latest, Color::Green));
    }
    if let Some(status) = &app.node_status {
        let installed = status
            .installed_version
            .as_deref()
            .unwrap_or("Not installed");
        let required = status
            .required_version
            .as_deref()
            .unwrap_or("Install Peacock first");
        lines.push(info_line("Node.js installed", installed, Color::White));
        lines.push(info_line("Node.js required", required, Color::Green));
    }
    let version_info = Paragraph::new(lines).block(titled_block("Versions"));
    frame.render_widget(version_info, chunks[1]);

    // Progress bar
    let (downloaded, total) = app.install_progress;
    let progress_ratio = if total > 0 {
        (downloaded as f64 / total as f64).min(1.0)
    } else {
        0.0
    };

    let progress_label = if total > 0 {
        format!(
            "{} / {} ({:.0}%)",
            format_bytes(downloaded),
            format_bytes(total),
            progress_ratio * 100.0
        )
    } else if app.task_running {
        "Downloading...".into()
    } else {
        String::new()
    };

    let gauge = Gauge::default()
        .block(titled_block("Progress"))
        .gauge_style(Style::default().fg(Color::Cyan).bg(Color::DarkGray))
        .ratio(progress_ratio)
        .label(progress_label);
    frame.render_widget(gauge, chunks[2]);

    // Status / log area
    let status_text = if let Some(err) = &app.install_error {
        Paragraph::new(format!("Error: {err}")).style(Style::default().fg(Color::Red))
    } else if app.install_done {
        Paragraph::new(format!("✓ {}", app.install_step)).style(Style::default().fg(Color::Green))
    } else if app.task_running {
        Paragraph::new(format!("⏳ {}", app.install_step)).style(Style::default().fg(Color::Yellow))
    } else {
        Paragraph::new("Press Enter to start installation")
            .style(Style::default().fg(Color::DarkGray))
    };
    frame.render_widget(status_text.block(titled_block("Status")), chunks[3]);

    // Footer
    let hints = if app.task_running {
        vec![("", "Installation in progress...")]
    } else if app.install_done || app.install_error.is_some() {
        vec![("Esc", "Back to menu")]
    } else {
        vec![("Enter", "Start install"), ("Esc", "Back")]
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

    match key.code {
        KeyCode::Enter if !app.task_running && !app.install_done => {
            start_install(app);
        }
        KeyCode::Esc if !app.task_running => {
            app.go_back();
        }
        _ => {}
    }
}

fn start_install(app: &mut App) {
    app.task_running = true;
    app.install_step = "Preparing...".into();
    app.install_progress = (0, 0);
    app.install_error = None;
    app.install_done = false;

    let tx = app.msg_tx.clone();
    let client = app.client.clone();
    let mut config = app.config.clone();

    tokio::spawn(async move {
        // Step 1: Install/Update Peacock
        let _ = tx.send(AppMessage::StepUpdate("Downloading Peacock...".into()));

        let progress_tx = tx.clone();
        let progress_fn: Option<super::super::core::download::ProgressFn> =
            Some(std::sync::Arc::new(move |downloaded, total| {
                let _ = progress_tx.send(AppMessage::Progress(downloaded, total));
            }));

        match crate::core::peacock::install_or_update(&client, &mut config, progress_fn).await {
            Ok(version) => {
                let _ = tx.send(AppMessage::ConfigUpdated(config.clone()));
                let _ = tx.send(AppMessage::StepUpdate(format!(
                    "Peacock {version} installed. Downloading Node.js..."
                )));
            }
            Err(e) => {
                let _ = tx.send(AppMessage::TaskError(format!(
                    "Peacock install failed: {e}"
                )));
                return;
            }
        }

        // Reset progress for Node download
        let _ = tx.send(AppMessage::Progress(0, 0));

        // Step 2: Install/Update Node.js
        let progress_tx = tx.clone();
        let progress_fn: Option<crate::core::download::ProgressFn> =
            Some(std::sync::Arc::new(move |downloaded, total| {
                let _ = progress_tx.send(AppMessage::Progress(downloaded, total));
            }));

        match crate::core::node::install_or_update(&client, &mut config, progress_fn).await {
            Ok(version) => {
                let msg = format!(
                    "Installation complete! Peacock {} + Node.js {version}",
                    config.peacock_version.as_deref().unwrap_or("?")
                );
                // Send updated config back so app picks up new version strings
                let _ = tx.send(AppMessage::ConfigUpdated(config.clone()));
                let _ = tx.send(AppMessage::TaskDone(msg));
                let _ = tx.send(AppMessage::RefreshStatus);
            }
            Err(e) => {
                let _ = tx.send(AppMessage::TaskError(format!(
                    "Node.js install failed: {e}"
                )));
            }
        }
    });
}
