pub mod install;
pub mod main_menu;
pub mod migration;
pub mod sdk;
pub mod service;
pub mod settings;

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

/// Render a centered status badge with color.
#[allow(dead_code)]
pub fn status_badge<'a>(label: &'a str, color: Color) -> Paragraph<'a> {
    Paragraph::new(label).style(Style::default().fg(color).bold())
}

/// Render a simple info line: "Label: Value". Returns an owned Line.
pub fn info_line(label: &str, value: &str, value_color: Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label}: "), Style::default().bold()),
        Span::styled(value.to_owned(), Style::default().fg(value_color)),
    ])
}

/// Standard block with a title.
pub fn titled_block(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .title(format!(" {title} "))
        .border_style(Style::default().fg(Color::Cyan))
}

/// Footer hint text.
pub fn footer_text<'a>(hints: &'a [(&'a str, &'a str)]) -> Line<'a> {
    let mut spans = Vec::new();
    for (i, (key, desc)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(
            format!("[{key}]"),
            Style::default().fg(Color::Yellow).bold(),
        ));
        spans.push(Span::raw(format!(" {desc}")));
    }
    Line::from(spans)
}

/// Format bytes into human-readable size.
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}
