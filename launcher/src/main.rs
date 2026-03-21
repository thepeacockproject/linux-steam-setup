mod app;
mod core;
mod ui;

use crossterm::{
    ExecutableCommand,
    event::{self, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use std::env;
use std::io::{IsTerminal, stdout};
use std::process::Command;

use app::Screen;

fn launcher_path() -> String {
    env::var("APPIMAGE")
        .ok()
        .filter(|path| !path.is_empty())
        .or_else(|| {
            env::current_exe()
                .ok()
                .map(|path| path.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "peacock-launcher".to_string())
}

fn main() -> anyhow::Result<()> {
    // --- Terminal auto-detection for AppImage ---
    // If we're not in a terminal (e.g. launched from a file manager),
    // re-launch ourselves inside a terminal emulator.
    if !std::io::stdout().is_terminal() {
        let app_path = launcher_path();

        let terminals = [
            ("x-terminal-emulator", "-e"),
            ("alacritty", "-e"),
            ("gnome-terminal", "--"),
            ("konsole", "-e"),
            ("xfce4-terminal", "-e"),
            ("kitty", "--"),
            ("xterm", "-e"),
        ];

        for (term, flag) in terminals {
            if let Ok(mut child) = Command::new(term).arg(flag).arg(&app_path).spawn() {
                let _ = child.wait();
                return Ok(());
            }
        }

        return Ok(());
    }

    // --- Run the TUI application ---
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(run_app())
}

async fn run_app() -> anyhow::Result<()> {
    // Load config
    let config = crate::core::config::Config::load()?;

    // Build HTTP client
    let client = crate::core::download::build_client()?;

    // Create app state
    let mut app = app::App::new(config, client);

    // Fetch Peacock status asynchronously
    {
        let client = app.client.clone();
        let config = app.config.clone();
        let tx = app.msg_tx.clone();

        tokio::spawn(async move {
            let status = crate::core::peacock::check_status(&client, &config).await;
            let _ = tx.send(app::AppMessage::PeacockStatusLoaded(status));
        });
    }

    // Install panic hook to restore terminal on panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = stdout().execute(LeaveAlternateScreen);
        let _ = disable_raw_mode();
        original_hook(info);
    }));

    // Setup terminal
    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;

    // Main event loop
    loop {
        // Process async messages
        app.process_messages();

        // Draw UI
        terminal.draw(|frame| match app.screen {
            Screen::MainMenu => ui::main_menu::render(frame, &app),
            Screen::Install => ui::install::render(frame, &app),
            Screen::Service => ui::service::render(frame, &app),
            Screen::Sdk => ui::sdk::render(frame, &app),
            Screen::Settings => ui::settings::render(frame, &app),
            Screen::Migration => ui::migration::render(frame, &app),
        })?;

        // Handle input (with timeout for async progress updates)
        if event::poll(std::time::Duration::from_millis(50))? {
            if let event::Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.screen {
                        Screen::MainMenu => ui::main_menu::handle_key(&mut app, key),
                        Screen::Install => ui::install::handle_key(&mut app, key),
                        Screen::Service => ui::service::handle_key(&mut app, key),
                        Screen::Sdk => ui::sdk::handle_key(&mut app, key),
                        Screen::Settings => ui::settings::handle_key(&mut app, key),
                        Screen::Migration => ui::migration::handle_key(&mut app, key),
                    }
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}
