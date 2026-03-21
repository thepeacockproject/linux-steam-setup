use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    prelude::*,
    widgets::{Paragraph, Wrap},
};

use super::{footer_text, titled_block};
use crate::app::App;
use crate::core::options::OptionType;

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(8),    // Options list
            Constraint::Length(5), // Description
            Constraint::Length(1), // Message
            Constraint::Length(1), // Footer
        ])
        .split(area);

    // Title
    let title = Paragraph::new("Peacock Options")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Cyan).bold())
        .block(titled_block(""));
    frame.render_widget(title, chunks[0]);

    // Options list
    if app.options.is_empty() {
        let msg = Paragraph::new(
            "No options available. Run Peacock at least once to generate options.ini.",
        )
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray))
        .block(titled_block("Options"));
        frame.render_widget(msg, chunks[1]);
    } else {
        render_options_list(frame, app, chunks[1]);
    }

    // Description of selected option
    if let Some(opt) = app.options.get(app.options_index) {
        let type_hint = match &opt.option_type {
            OptionType::Boolean => " [boolean]".to_string(),
            OptionType::Enum(variants) => format!(" [{}]", variants.join(" | ")),
        };
        let desc_text = format!("{}{}", opt.description, type_hint);
        let desc = Paragraph::new(desc_text)
            .wrap(Wrap { trim: true })
            .block(titled_block("Description"));
        frame.render_widget(desc, chunks[2]);
    }

    // Message
    if let Some(msg) = &app.options_message {
        let color = if msg.starts_with("Error") {
            Color::Red
        } else {
            Color::Green
        };
        frame.render_widget(
            Paragraph::new(msg.as_str()).style(Style::default().fg(color)),
            chunks[3],
        );
    }

    // Footer
    let hints = vec![
        ("↑↓", "Navigate"),
        ("Enter", "Toggle"),
        ("←→", "Cycle"),
        ("s", "Save"),
        ("Esc", "Back"),
    ];
    frame.render_widget(
        Paragraph::new(footer_text(&hints)).alignment(Alignment::Center),
        chunks[4],
    );
}

fn render_options_list(frame: &mut Frame, app: &App, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();
    let mut selected_line: usize = 0;
    let mut current_category = "";

    for (i, opt) in app.options.iter().enumerate() {
        if opt.category != current_category {
            if !lines.is_empty() {
                lines.push(Line::raw(""));
            }
            current_category = opt.category;
            lines.push(Line::styled(
                format!(" ── {current_category} ──"),
                Style::default().fg(Color::Yellow).bold(),
            ));
        }

        if i == app.options_index {
            selected_line = lines.len();
        }

        let selected = i == app.options_index;
        let prefix = if selected { " ▸ " } else { "   " };
        let value_str = format_value(opt);

        let style = if selected {
            Style::default().fg(Color::Cyan).bold()
        } else {
            Style::default().fg(Color::White)
        };

        lines.push(Line::styled(
            format!("{prefix}{}: {value_str}", opt.label),
            style,
        ));
    }

    // Compute scroll to keep the selected item visible
    let viewport_height = area.height.saturating_sub(2) as usize;
    let scroll = if viewport_height == 0 || selected_line < viewport_height / 2 {
        0
    } else {
        let max_scroll = lines.len().saturating_sub(viewport_height);
        (selected_line.saturating_sub(viewport_height / 2)).min(max_scroll)
    };

    let paragraph = Paragraph::new(lines)
        .block(titled_block("Options"))
        .scroll((scroll as u16, 0));
    frame.render_widget(paragraph, area);
}

fn format_value(opt: &crate::core::options::PeacockOption) -> String {
    match &opt.option_type {
        OptionType::Boolean => {
            if opt.value == "true" {
                "✓ Enabled".into()
            } else {
                "✗ Disabled".into()
            }
        }
        OptionType::Enum(_) => {
            format!("◂ {} ▸", opt.value)
        }
    }
}

pub fn handle_key(app: &mut App, key: KeyEvent) {
    if key.kind != KeyEventKind::Press {
        return;
    }

    match key.code {
        KeyCode::Up => {
            if app.options_index > 0 {
                app.options_index -= 1;
            }
        }
        KeyCode::Down => {
            if app.options_index < app.options.len().saturating_sub(1) {
                app.options_index += 1;
            }
        }
        KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Right => {
            let idx = app.options_index;
            if let Some(opt) = app.options.get_mut(idx) {
                opt.cycle_value();
            }
        }
        KeyCode::Left => {
            let idx = app.options_index;
            if let Some(opt) = app.options.get_mut(idx) {
                opt.cycle_value_back();
            }
        }
        KeyCode::Char('s') => {
            let peacock_dir = app.config.peacock_dir();
            match crate::core::options::save_options(&peacock_dir, &app.options) {
                Ok(()) => {
                    app.options_message = Some("Options saved successfully".into());
                }
                Err(e) => {
                    app.options_message = Some(format!("Error: {e}"));
                }
            }
        }
        KeyCode::Esc => {
            app.go_back();
        }
        _ => {}
    }
}
