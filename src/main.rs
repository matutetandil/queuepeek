mod app;
mod config;
mod backend;
mod ui;
mod updater;
mod filters;
mod comparison;
mod operations;
mod utils;
mod keys;
mod schema;

use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::prelude::*;

use app::{App, Popup, Screen};
use config::Config;

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let config_path = args.iter().position(|a| a == "-c" || a == "--config")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str());

    let config = Config::load(config_path);
    let mut app = App::new(config, config_path.map(String::from));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        eprintln!("Error: {}", err);
    }

    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> io::Result<()> {
    let mut last_refresh = Instant::now();
    let mut last_marquee = Instant::now();

    // Trigger initial update check
    app.update_checker.start_check();

    loop {
        app.process_bg_results();
        app.check_scheduled_messages();
        app.check_alerts();
        app.update_checker.poll();

        // Periodic update check
        if app.update_checker.should_check() {
            app.update_checker.start_check();
        }

        // Marquee tick every 2 seconds
        if last_marquee.elapsed() >= Duration::from_secs(2) {
            app.marquee_tick = app.marquee_tick.wrapping_add(1);
            last_marquee = Instant::now();
        }

        // Auto-refresh every 5 seconds
        if !app.loading && last_refresh.elapsed() >= Duration::from_secs(5) {
            if app.screen == Screen::QueueList {
                app.load_queues();
                if app.popup == Popup::QueueInfo && !app.queue_info_name.is_empty() {
                    app.load_queue_detail(&app.queue_info_name.clone());
                }
                last_refresh = Instant::now();
            } else if app.screen == Screen::MessageList && app.message_auto_refresh {
                app.load_messages();
                app.load_queues(); // refresh queue stats for the activity bar
                last_refresh = Instant::now();
            }
        }

        terminal.draw(|frame| ui::draw(frame, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press { continue; }

                // Reset refresh timer on manual refresh
                if key.code == crossterm::event::KeyCode::Char('r') || key.code == crossterm::event::KeyCode::Char('R') {
                    last_refresh = Instant::now();
                }

                keys::handle_key(app, key.code, key.modifiers);
            }
        }

        if app.should_quit { return Ok(()); }
    }
}
