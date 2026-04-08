mod app;
mod config;
mod backend;
mod ui;

use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyModifiers, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::prelude::*;

use app::{App, Popup, ProfileMode, Screen};
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

    loop {
        app.process_bg_results();

        // Auto-refresh queues every 5 seconds when on QueueList screen
        if app.screen == Screen::QueueList && !app.loading && last_refresh.elapsed() >= Duration::from_secs(5) {
            app.load_queues();
            last_refresh = Instant::now();
        }

        terminal.draw(|frame| ui::draw(frame, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press { continue; }

                // Reset refresh timer on manual refresh
                if key.code == KeyCode::Char('r') || key.code == KeyCode::Char('R') {
                    last_refresh = Instant::now();
                }

                handle_key(app, key.code, key.modifiers);
            }
        }

        if app.should_quit { return Ok(()); }
    }
}

// ---------------------------------------------------------------------------
// Top-level key dispatch
// ---------------------------------------------------------------------------

fn handle_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    // Ctrl+C always quits
    if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
        app.should_quit = true;
        return;
    }

    match app.screen {
        Screen::ProfileSelect => handle_profile_key(app, code, modifiers),
        Screen::QueueList     => handle_queue_list_key(app, code),
        Screen::MessageList   => handle_message_list_key(app, code),
        Screen::MessageDetail => handle_message_detail_key(app, code),
    }
}

// ---------------------------------------------------------------------------
// Profile screen
// ---------------------------------------------------------------------------

fn handle_profile_key(app: &mut App, code: KeyCode, _modifiers: KeyModifiers) {
    // Popups first (theme picker, etc.)
    if app.popup != Popup::None {
        handle_popup_key(app, code);
        return;
    }

    match app.profile_mode {
        ProfileMode::Select => handle_profile_select_key(app, code),
        ProfileMode::Add | ProfileMode::Edit(_) => handle_profile_form_key(app, code),
        ProfileMode::ConfirmDelete => handle_profile_delete_key(app, code),
    }
}

fn handle_profile_select_key(app: &mut App, code: KeyCode) {
    let names = app.config.profile_names();
    // Total items: profiles + "Add new" + "Theme"
    let total = names.len() + 2;

    match code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('j') | KeyCode::Down => {
            let i = app.profile_list_state.selected().unwrap_or(0);
            if i + 1 < total {
                app.profile_list_state.select(Some(i + 1));
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            let i = app.profile_list_state.selected().unwrap_or(0);
            if i > 0 {
                app.profile_list_state.select(Some(i - 1));
            }
        }
        KeyCode::Enter => {
            let selected = app.profile_list_state.selected().unwrap_or(0);
            if selected < names.len() {
                let name = names[selected].clone();
                app.connect_profile(&name);
            } else if selected == names.len() {
                app.profile_form.clear();
                app.profile_mode = ProfileMode::Add;
            } else if selected == names.len() + 1 {
                // Open theme picker
                app.popup = Popup::ThemePicker;
                let current = app.config.theme.as_deref().unwrap_or("slack");
                let idx = ui::theme::theme_names().iter().position(|&n| n == current).unwrap_or(0);
                app.popup_list_state.select(Some(idx));
            }
        }
        KeyCode::Char('a') => {
            app.profile_form.clear();
            app.profile_mode = ProfileMode::Add;
        }
        KeyCode::Char('e') => {
            let selected = app.profile_list_state.selected().unwrap_or(0);
            if selected < names.len() {
                let name = &names[selected];
                if let Some(profile) = app.config.profiles.get(name) {
                    app.profile_form = app::ProfileForm::from_profile(name, profile);
                    app.profile_mode = ProfileMode::Edit(name.clone());
                }
            }
        }
        KeyCode::Char('d') => {
            let selected = app.profile_list_state.selected().unwrap_or(0);
            if selected < names.len() {
                app.profile_mode = ProfileMode::ConfirmDelete;
            }
        }
        KeyCode::Char('t') => {
            app.popup = Popup::ThemePicker;
            let current = app.config.theme.as_deref().unwrap_or("slack");
            let idx = ui::theme::theme_names().iter().position(|&n| n == current).unwrap_or(0);
            app.popup_list_state.select(Some(idx));
        }
        _ => {}
    }
}

fn handle_profile_form_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => {
            app.profile_mode = ProfileMode::Select;
            app.profile_form.error.clear();
        }
        KeyCode::Tab | KeyCode::Down => {
            let next = (app.profile_form.focused_field + 1) % app::ProfileForm::field_count();
            app.profile_form.focused_field = next;
        }
        KeyCode::BackTab | KeyCode::Up => {
            let count = app::ProfileForm::field_count();
            let prev = (app.profile_form.focused_field + count - 1) % count;
            app.profile_form.focused_field = prev;
        }
        KeyCode::Backspace => {
            app.profile_form.pop_char();
        }
        KeyCode::Char(c) => {
            app.profile_form.push_char(c);
        }
        KeyCode::Enter => {
            // Open backend type picker popup when on Type field
            if app.profile_form.focused_field == 0 {
                app.popup = Popup::BackendTypePicker;
                let idx = app::BACKEND_TYPES.iter()
                    .position(|&t| t == app.profile_form.profile_type)
                    .unwrap_or(0);
                app.popup_list_state.select(Some(idx));
                return;
            }
            // Also toggle TLS on Enter
            if app.profile_form.focused_field == 7 {
                app.profile_form.tls = !app.profile_form.tls;
                return;
            }
            match app.profile_form.to_profile() {
                Ok(profile) => {
                    let name = app.profile_form.name.clone();
                    if let ProfileMode::Edit(ref old_name) = app.profile_mode {
                        if old_name != &name {
                            app.config.delete_profile(old_name);
                        }
                    }
                    app.config.add_profile(name.clone(), profile);
                    let _ = app.config.save(app.config_path.as_deref());
                    app.profile_mode = ProfileMode::Select;
                    app.connect_profile(&name);
                }
                Err(e) => {
                    app.profile_form.error = e;
                }
            }
        }
        _ => {}
    }
}

fn handle_profile_delete_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            let names = app.config.profile_names();
            let selected = app.profile_list_state.selected().unwrap_or(0);
            if selected < names.len() {
                let name = names[selected].clone();
                app.config.delete_profile(&name);
                let _ = app.config.save(app.config_path.as_deref());
                let new_len = app.config.profile_names().len() + 2;
                if selected >= new_len {
                    app.profile_list_state.select(Some(new_len.saturating_sub(1)));
                }
            }
            app.profile_mode = ProfileMode::Select;
        }
        _ => {
            app.profile_mode = ProfileMode::Select;
        }
    }
}

// ---------------------------------------------------------------------------
// Queue list screen
// ---------------------------------------------------------------------------

fn handle_queue_list_key(app: &mut App, code: KeyCode) {
    // Popups first
    if app.popup != Popup::None {
        handle_popup_key(app, code);
        return;
    }

    // Queue filter mode
    if app.queue_filter_active {
        handle_queue_filter_key(app, code);
        return;
    }

    match code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('?') => {
            app.popup = if app.popup == Popup::Help { Popup::None } else { Popup::Help };
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let len = app.filtered_queue_indices.len();
            if len > 0 {
                let i = app.queue_list_state.selected().unwrap_or(0);
                if i + 1 < len {
                    app.queue_list_state.select(Some(i + 1));
                }
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            let i = app.queue_list_state.selected().unwrap_or(0);
            if i > 0 {
                app.queue_list_state.select(Some(i - 1));
            }
        }
        KeyCode::Enter => {
            if let Some(selected) = app.queue_list_state.selected() {
                if selected < app.filtered_queue_indices.len() {
                    let idx = app.filtered_queue_indices[selected];
                    app.current_queue_name = app.queues[idx].name.clone();
                    app.screen = Screen::MessageList;
                    app.message_list_state.select(Some(0));
                    app.loading = true;
                    app.set_status(format!("Loading messages from {}", app.current_queue_name), false);
                    app.load_messages();
                }
            }
        }
        KeyCode::Char('/') => {
            app.queue_filter_active = true;
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            app.loading = true;
            app.load_queues();
        }
        KeyCode::Char('v') => {
            if !app.namespaces.is_empty() {
                app.popup = Popup::NamespacePicker;
                let idx = app.namespaces.iter().position(|v| v == &app.selected_namespace).unwrap_or(0);
                app.popup_list_state.select(Some(idx));
            }
        }
        KeyCode::Char('p') => {
            app.popup = Popup::ProfileSwitch;
            app.popup_list_state.select(Some(0));
        }
        KeyCode::Esc => {
            // Go back to profile select (disconnect)
            app.screen = Screen::ProfileSelect;
            app.backend = None;
            app.queues.clear();
            app.filtered_queue_indices.clear();
            app.messages.clear();
            app.current_queue_name.clear();
            app.queue_filter.clear();
            app.set_status(String::new(), false);
        }
        KeyCode::Char('f') => {
            app.popup = Popup::FetchCount;
            let idx = app::FETCH_PRESETS.iter().position(|&c| c == app.fetch_count).unwrap_or(2);
            app.popup_list_state.select(Some(idx));
        }
        KeyCode::Char('+') | KeyCode::Char('=') => {
            if app.fetch_count < 500 { app.fetch_count += 10; }
            app.set_status(format!("Fetch count: {}", app.fetch_count), false);
        }
        KeyCode::Char('-') => {
            app.fetch_count = app.fetch_count.saturating_sub(10).max(1);
            app.set_status(format!("Fetch count: {}", app.fetch_count), false);
        }
        _ => {}
    }
}

fn handle_queue_filter_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char(c) => {
            app.queue_filter.push(c);
            app.update_filtered_queues();
            if !app.filtered_queue_indices.is_empty() {
                app.queue_list_state.select(Some(0));
            }
        }
        KeyCode::Backspace => {
            app.queue_filter.pop();
            app.update_filtered_queues();
            if !app.filtered_queue_indices.is_empty() {
                app.queue_list_state.select(Some(0));
            }
        }
        KeyCode::Down => {
            let len = app.filtered_queue_indices.len();
            if len > 0 {
                let i = app.queue_list_state.selected().unwrap_or(0);
                if i + 1 < len {
                    app.queue_list_state.select(Some(i + 1));
                }
            }
        }
        KeyCode::Up => {
            let i = app.queue_list_state.selected().unwrap_or(0);
            if i > 0 {
                app.queue_list_state.select(Some(i - 1));
            }
        }
        KeyCode::Enter => {
            if let Some(selected) = app.queue_list_state.selected() {
                if selected < app.filtered_queue_indices.len() {
                    let idx = app.filtered_queue_indices[selected];
                    app.current_queue_name = app.queues[idx].name.clone();
                    app.queue_filter_active = false;
                    app.screen = Screen::MessageList;
                    app.message_list_state.select(Some(0));
                    app.loading = true;
                    app.set_status(format!("Loading messages from {}", app.current_queue_name), false);
                    app.load_messages();
                }
            }
        }
        KeyCode::Esc => {
            app.queue_filter.clear();
            app.queue_filter_active = false;
            app.update_filtered_queues();
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Message list screen
// ---------------------------------------------------------------------------

fn handle_message_list_key(app: &mut App, code: KeyCode) {
    // Popups first
    if app.popup != Popup::None {
        handle_popup_key(app, code);
        return;
    }

    // Message filter mode
    if app.message_filter_active {
        handle_message_filter_key(app, code);
        return;
    }

    match code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('?') => {
            app.popup = if app.popup == Popup::Help { Popup::None } else { Popup::Help };
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let len = app.messages.len();
            if len > 0 {
                let i = app.message_list_state.selected().unwrap_or(0);
                if i + 1 < len {
                    app.message_list_state.select(Some(i + 1));
                }
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            let i = app.message_list_state.selected().unwrap_or(0);
            if i > 0 {
                app.message_list_state.select(Some(i - 1));
            }
        }
        KeyCode::Enter => {
            if let Some(selected) = app.message_list_state.selected() {
                if selected < app.filtered_message_indices.len() {
                    app.detail_message_idx = app.filtered_message_indices[selected];
                    app.detail_scroll = 0;
                    app.screen = Screen::MessageDetail;
                }
            }
        }
        KeyCode::Char('/') => {
            app.message_filter_active = true;
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            if !app.current_queue_name.is_empty() {
                app.loading = true;
                app.load_messages();
            }
        }
        KeyCode::Char('f') => {
            app.popup = Popup::FetchCount;
            let idx = app::FETCH_PRESETS.iter().position(|&c| c == app.fetch_count).unwrap_or(2);
            app.popup_list_state.select(Some(idx));
        }
        KeyCode::Char('+') | KeyCode::Char('=') => {
            if app.fetch_count < 500 { app.fetch_count += 10; }
            app.set_status(format!("Fetch count: {}", app.fetch_count), false);
        }
        KeyCode::Char('-') => {
            app.fetch_count = app.fetch_count.saturating_sub(10).max(1);
            app.set_status(format!("Fetch count: {}", app.fetch_count), false);
        }
        KeyCode::Esc => {
            app.screen = Screen::QueueList;
            app.messages.clear();
            app.message_filter.clear();
            app.message_filter_active = false;
        }
        _ => {}
    }
}

fn handle_message_filter_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char(c) => {
            app.message_filter.push(c);
            app.update_filtered_messages();
            if !app.filtered_message_indices.is_empty() {
                app.message_list_state.select(Some(0));
            } else {
                app.message_list_state.select(None);
            }
        }
        KeyCode::Backspace => {
            app.message_filter.pop();
            app.update_filtered_messages();
            if !app.filtered_message_indices.is_empty() {
                app.message_list_state.select(Some(0));
            } else {
                app.message_list_state.select(None);
            }
        }
        KeyCode::Down => {
            let len = app.filtered_message_indices.len();
            if len > 0 {
                let i = app.message_list_state.selected().unwrap_or(0);
                if i + 1 < len {
                    app.message_list_state.select(Some(i + 1));
                }
            }
        }
        KeyCode::Up => {
            let i = app.message_list_state.selected().unwrap_or(0);
            if i > 0 {
                app.message_list_state.select(Some(i - 1));
            }
        }
        KeyCode::Enter => {
            if let Some(selected) = app.message_list_state.selected() {
                if selected < app.filtered_message_indices.len() {
                    app.detail_message_idx = app.filtered_message_indices[selected];
                    app.detail_scroll = 0;
                    app.message_filter_active = false;
                    app.screen = Screen::MessageDetail;
                }
            }
        }
        KeyCode::Esc => {
            app.message_filter.clear();
            app.update_filtered_messages();
            app.message_filter_active = false;
            if !app.filtered_message_indices.is_empty() {
                app.message_list_state.select(Some(0));
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Message detail screen
// ---------------------------------------------------------------------------

fn handle_message_detail_key(app: &mut App, code: KeyCode) {
    // Popups first
    if app.popup != Popup::None {
        handle_popup_key(app, code);
        return;
    }

    match code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('?') => {
            app.popup = if app.popup == Popup::Help { Popup::None } else { Popup::Help };
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.detail_scroll = app.detail_scroll.saturating_add(1);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.detail_scroll = app.detail_scroll.saturating_sub(1);
        }
        KeyCode::PageDown => {
            app.detail_scroll = app.detail_scroll.saturating_add(10);
        }
        KeyCode::PageUp => {
            app.detail_scroll = app.detail_scroll.saturating_sub(10);
        }
        KeyCode::Char('p') => {
            app.detail_pretty = !app.detail_pretty;
        }
        KeyCode::Char('c') => {
            if let Some(msg) = app.messages.get(app.detail_message_idx) {
                let text = msg.body.clone();
                match copy_to_clipboard(&text) {
                    Ok(()) => app.set_status("Payload copied to clipboard", false),
                    Err(e) => app.set_status(e, true),
                }
            }
        }
        KeyCode::Char('h') => {
            if let Some(msg) = app.messages.get(app.detail_message_idx) {
                let mut header_text = format!("routing_key: {}\n", msg.routing_key);
                header_text += &format!("exchange: {}\n", msg.exchange);
                header_text += &format!("redelivered: {}\n", msg.redelivered);
                header_text += &format!("content_type: {}\n", msg.content_type);
                for (k, v) in &msg.headers {
                    header_text += &format!("{}: {}\n", k, v);
                }
                match copy_to_clipboard(&header_text) {
                    Ok(()) => app.set_status("Headers copied to clipboard", false),
                    Err(e) => app.set_status(e, true),
                }
            }
        }
        KeyCode::Esc => {
            app.screen = Screen::MessageList;
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Clipboard helper
// ---------------------------------------------------------------------------

fn copy_to_clipboard(text: &str) -> Result<(), String> {
    arboard::Clipboard::new()
        .and_then(|mut cb| cb.set_text(text.to_string()))
        .map_err(|e| format!("Clipboard: {}", e))
}

// ---------------------------------------------------------------------------
// Popup key handler (shared across screens)
// ---------------------------------------------------------------------------

fn handle_popup_key(app: &mut App, code: KeyCode) {
    match app.popup {
        Popup::Help => {
            match code {
                KeyCode::Esc | KeyCode::Char('?') | KeyCode::Enter => {
                    app.popup = Popup::None;
                }
                _ => {}
            }
        }
        Popup::ProfileSwitch => {
            let names = app.config.profile_names();
            match code {
                KeyCode::Esc => app.popup = Popup::None,
                KeyCode::Char('j') | KeyCode::Down => {
                    let i = app.popup_list_state.selected().unwrap_or(0);
                    if i + 1 < names.len() {
                        app.popup_list_state.select(Some(i + 1));
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    let i = app.popup_list_state.selected().unwrap_or(0);
                    if i > 0 {
                        app.popup_list_state.select(Some(i - 1));
                    }
                }
                KeyCode::Enter => {
                    let selected = app.popup_list_state.selected().unwrap_or(0);
                    if selected < names.len() {
                        let name = names[selected].clone();
                        app.popup = Popup::None;
                        app.connect_profile(&name);
                    }
                }
                _ => {}
            }
        }
        Popup::NamespacePicker => {
            let len = app.namespaces.len();
            match code {
                KeyCode::Esc => app.popup = Popup::None,
                KeyCode::Char('j') | KeyCode::Down => {
                    let i = app.popup_list_state.selected().unwrap_or(0);
                    if i + 1 < len { app.popup_list_state.select(Some(i + 1)); }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    let i = app.popup_list_state.selected().unwrap_or(0);
                    if i > 0 { app.popup_list_state.select(Some(i - 1)); }
                }
                KeyCode::Enter => {
                    let selected = app.popup_list_state.selected().unwrap_or(0);
                    if selected < len {
                        let vhost = app.namespaces[selected].clone();
                        app.popup = Popup::None;
                        if vhost != app.selected_namespace {
                            app.selected_namespace = vhost;
                            app.queues.clear();
                            app.filtered_queue_indices.clear();
                            app.messages.clear();
                            app.queue_filter.clear();
                            app.current_queue_name.clear();
                            app.queue_list_state.select(None);
                            app.loading = true;
                            app.set_status(format!("Switching to vhost: {}", app.selected_namespace), false);
                            app.load_queues();
                        }
                    }
                }
                _ => {}
            }
        }
        Popup::ThemePicker => {
            let names = ui::theme::theme_names();
            match code {
                KeyCode::Esc => app.popup = Popup::None,
                KeyCode::Char('j') | KeyCode::Down => {
                    let i = app.popup_list_state.selected().unwrap_or(0);
                    if i + 1 < names.len() { app.popup_list_state.select(Some(i + 1)); }
                    // Live preview
                    if let Some(sel) = app.popup_list_state.selected() {
                        if sel < names.len() {
                            app.theme = ui::theme::get_theme(names[sel]);
                        }
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    let i = app.popup_list_state.selected().unwrap_or(0);
                    if i > 0 { app.popup_list_state.select(Some(i - 1)); }
                    // Live preview
                    if let Some(sel) = app.popup_list_state.selected() {
                        if sel < names.len() {
                            app.theme = ui::theme::get_theme(names[sel]);
                        }
                    }
                }
                KeyCode::Enter => {
                    if let Some(sel) = app.popup_list_state.selected() {
                        if sel < names.len() {
                            let name = names[sel];
                            app.theme = ui::theme::get_theme(name);
                            app.config.theme = Some(name.to_string());
                            let _ = app.config.save(app.config_path.as_deref());
                            app.popup = Popup::None;
                        }
                    }
                }
                _ => {}
            }
        }
        Popup::FetchCount => {
            let presets = app::FETCH_PRESETS;
            match code {
                KeyCode::Esc => app.popup = Popup::None,
                KeyCode::Char('j') | KeyCode::Down => {
                    let i = app.popup_list_state.selected().unwrap_or(0);
                    if i + 1 < presets.len() { app.popup_list_state.select(Some(i + 1)); }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    let i = app.popup_list_state.selected().unwrap_or(0);
                    if i > 0 { app.popup_list_state.select(Some(i - 1)); }
                }
                KeyCode::Enter => {
                    let selected = app.popup_list_state.selected().unwrap_or(2);
                    if selected < presets.len() {
                        app.fetch_count = presets[selected];
                        app.popup = Popup::None;
                        app.set_status(format!("Fetch count: {}", app.fetch_count), false);
                    }
                }
                _ => {}
            }
        }
        Popup::BackendTypePicker => {
            let types = app::BACKEND_TYPES;
            match code {
                KeyCode::Esc => app.popup = Popup::None,
                KeyCode::Char('j') | KeyCode::Down => {
                    let i = app.popup_list_state.selected().unwrap_or(0);
                    if i + 1 < types.len() { app.popup_list_state.select(Some(i + 1)); }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    let i = app.popup_list_state.selected().unwrap_or(0);
                    if i > 0 { app.popup_list_state.select(Some(i - 1)); }
                }
                KeyCode::Enter => {
                    let selected = app.popup_list_state.selected().unwrap_or(0);
                    if selected < types.len() {
                        app.profile_form.set_backend_type(types[selected]);
                        app.popup = Popup::None;
                    }
                }
                _ => {}
            }
        }
        Popup::None => {}
    }
}
