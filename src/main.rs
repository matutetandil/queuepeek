mod app;
mod config;
mod backend;
mod ui;
mod updater;

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

    // Trigger initial update check
    app.update_checker.start_check();

    loop {
        app.process_bg_results();
        app.update_checker.poll();

        // Periodic update check
        if app.update_checker.should_check() {
            app.update_checker.start_check();
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
                last_refresh = Instant::now();
            }
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

    // Global: U to trigger update (only when update is available and no popup open)
    if code == KeyCode::Char('U') && app.popup == Popup::None && app.update_checker.update_available {
        app.set_status("Updating...", false);
        match updater::perform_update() {
            Ok(msg) => app.set_status(msg, false),
            Err(e) => app.set_status(format!("Update failed: {}", e), true),
        }
        return;
    }

    match app.screen {
        Screen::ProfileSelect => handle_profile_key(app, code, modifiers),
        Screen::QueueList     => handle_queue_list_key(app, code, modifiers),
        Screen::MessageList   => handle_message_list_key(app, code, modifiers),
        Screen::MessageDetail => handle_message_detail_key(app, code, modifiers),
    }
}

// ---------------------------------------------------------------------------
// Profile screen
// ---------------------------------------------------------------------------

fn handle_profile_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    // Popups first (theme picker, etc.)
    if app.popup != Popup::None {
        handle_popup_key(app, code, modifiers);
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

fn handle_queue_list_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    // Popups first
    if app.popup != Popup::None {
        handle_popup_key(app, code, modifiers);
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
        KeyCode::Char('P') => {
            // Publish to selected queue
            if let Some(q) = app.selected_queue() {
                let name = q.name.clone();
                app.publish_form = app::PublishForm::new_for_queue(&name);
                app.popup = Popup::PublishMessage;
            }
        }
        KeyCode::Char('x') => {
            // Purge selected queue
            if app.selected_queue().is_some() {
                app.popup = Popup::ConfirmPurge;
            }
        }
        KeyCode::Char('D') => {
            // Delete selected queue
            if app.selected_queue().is_some() {
                app.popup = Popup::ConfirmDelete;
            }
        }
        KeyCode::Char('C') => {
            // Copy messages from selected queue
            if app.selected_queue().is_some() {
                app.queue_picker_filter.clear();
                app.queue_picker_filter_active = false;
                app.popup = Popup::QueuePicker(app::QueueOperation::Copy);
                app.popup_list_state.select(Some(0));
            }
        }
        KeyCode::Char('m') => {
            // Move messages from selected queue
            if app.selected_queue().is_some() {
                app.queue_picker_filter.clear();
                app.queue_picker_filter_active = false;
                app.popup = Popup::QueuePicker(app::QueueOperation::Move);
                app.popup_list_state.select(Some(0));
            }
        }
        KeyCode::Char('G') => {
            // Show consumer groups for selected queue
            if let Some(q) = app.selected_queue() {
                let name = q.name.clone();
                app.consumer_groups.clear();
                app.consumer_groups_scroll = 0;
                app.popup = Popup::ConsumerGroups;
                app.loading = true;
                app.load_consumer_groups(&name);
            }
        }
        KeyCode::Char('i') => {
            // Show queue info/detail
            if let Some(q) = app.selected_queue() {
                let name = q.name.clone();
                app.queue_info_name = name.clone();
                app.queue_detail.clear();
                app.queue_info_scroll = 0;
                app.popup = Popup::QueueInfo;
                app.loading = true;
                app.load_queue_detail(&name);
            }
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

fn handle_message_list_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    // Popups first
    if app.popup != Popup::None {
        handle_popup_key(app, code, modifiers);
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
        KeyCode::Char('P') => {
            app.publish_form = app::PublishForm::new_for_queue(&app.current_queue_name);
            app.popup = Popup::PublishMessage;
        }
        KeyCode::Char(' ') => {
            // Toggle selection on current message
            app.toggle_message_selection();
            // Move down after selecting
            let len = app.filtered_message_indices.len();
            if len > 0 {
                let i = app.message_list_state.selected().unwrap_or(0);
                if i + 1 < len {
                    app.message_list_state.select(Some(i + 1));
                }
            }
        }
        KeyCode::Char('a') => {
            // Select/deselect all
            app.select_all_messages();
        }
        KeyCode::Char('C') => {
            // Copy selected messages to another queue
            if app.selection_count() > 0 {
                app.queue_picker_filter.clear();
                app.queue_picker_filter_active = false;
                app.popup = Popup::MessageQueuePicker(app::QueueOperation::Copy);
                app.popup_list_state.select(Some(0));
            }
        }
        KeyCode::Char('M') => {
            // Move selected messages to another queue (via delete+publish)
            if app.selection_count() > 0 {
                app.queue_picker_filter.clear();
                app.queue_picker_filter_active = false;
                app.popup = Popup::MessageQueuePicker(app::QueueOperation::Move);
                app.popup_list_state.select(Some(0));
            }
        }
        KeyCode::Char('D') => {
            // Delete selected messages
            if app.selection_count() > 0 {
                app.popup = Popup::ConfirmDeleteMessages;
            }
        }
        KeyCode::Char('e') => {
            // Export selected messages to JSON
            if app.selection_count() > 0 {
                match app.export_messages_to_json() {
                    Ok(msg) => app.set_status(msg, false),
                    Err(e) => app.set_status(e, true),
                }
            }
        }
        KeyCode::Char('R') => {
            // Re-publish selected messages
            if app.selection_count() > 0 {
                let count = app.selection_count();
                app.re_publish_selected();
                app.set_status(format!("Re-publishing {} message(s)...", count), false);
            }
        }
        KeyCode::Char('W') => {
            // Dump entire queue to JSONL file (streaming)
            app.do_dump_queue();
        }
        KeyCode::Char('I') => {
            // Import messages from JSONL/JSON file
            app.import_file_path = "./".to_string();
            app.popup = Popup::ImportFile;
        }
        KeyCode::Char('T') => {
            // Toggle auto-refresh (tail mode)
            app.message_auto_refresh = !app.message_auto_refresh;
            if app.message_auto_refresh {
                app.set_status("Auto-refresh ON (every 5s)", false);
            } else {
                app.set_status("Auto-refresh OFF", false);
            }
        }
        KeyCode::Char('r') => {
            // Manual refresh
            app.loading = true;
            app.load_messages();
            app.set_status("Refreshing messages...", false);
        }
        KeyCode::Char('L') => {
            // DLQ re-route: parse x-death and offer to re-route
            if let Some((exchange, routing_key)) = app.parse_dlq_info() {
                let count = app.selection_count().max(1);
                app.popup = Popup::ConfirmReroute { exchange, routing_key, count };
            } else {
                app.set_status("No x-death header found — not a dead-lettered message", true);
            }
        }
        KeyCode::Esc => {
            if !app.selected_messages.is_empty() {
                // First Esc clears selection
                app.selected_messages.clear();
            } else {
                app.screen = Screen::QueueList;
                app.messages.clear();
                app.message_filter.clear();
                app.message_filter_active = false;
                app.message_auto_refresh = false;
            }
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

fn handle_message_detail_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    // Popups first
    if app.popup != Popup::None {
        handle_popup_key(app, code, modifiers);
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
        KeyCode::Char('L') => {
            // DLQ re-route from detail view
            if let Some(msg) = app.messages.get(app.detail_message_idx) {
                let dlq_info = msg.headers.iter()
                    .find(|(k, _)| k == "x-death")
                    .and_then(|(_, v)| crate::app::parse_x_death_value(v));
                if let Some((exchange, routing_key)) = dlq_info {
                    app.popup = Popup::ConfirmReroute { exchange, routing_key, count: 1 };
                } else {
                    app.set_status("No x-death header found — not a dead-lettered message", true);
                }
            }
        }
        KeyCode::Char('E') => {
            // Edit & re-publish current message
            if let Some(msg) = app.messages.get(app.detail_message_idx) {
                app.publish_form = app::PublishForm {
                    routing_key: msg.routing_key.clone(),
                    content_type: if msg.content_type.is_empty() { "application/json".to_string() } else { msg.content_type.clone() },
                    body: msg.body.clone(),
                    focused_field: 2, // Focus body by default
                    error: String::new(),
                };
                app.popup = Popup::EditMessage;
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

fn handle_popup_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
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
        Popup::PublishMessage => {
            match code {
                KeyCode::Esc => {
                    app.popup = Popup::None;
                    app.publish_form.error.clear();
                }
                KeyCode::Tab | KeyCode::Down => {
                    app.publish_form.focused_field = (app.publish_form.focused_field + 1) % app::PublishForm::field_count();
                }
                KeyCode::BackTab | KeyCode::Up => {
                    let count = app::PublishForm::field_count();
                    app.publish_form.focused_field = (app.publish_form.focused_field + count - 1) % count;
                }
                KeyCode::Enter => {
                    if app.publish_form.focused_field == 2 {
                        // In body field, Enter adds newline
                        app.publish_form.newline();
                    } else if modifiers.contains(KeyModifiers::CONTROL) || app.publish_form.focused_field != 2 {
                        // Ctrl+Enter or Enter on non-body field: submit
                        if app.publish_form.body.is_empty() {
                            app.publish_form.error = "Body is required".into();
                        } else {
                            app.publish_form.error.clear();
                            app.do_publish();
                        }
                    }
                }
                KeyCode::Backspace => {
                    app.publish_form.pop_char();
                }
                KeyCode::Char(c) => {
                    app.publish_form.push_char(c);
                }
                _ => {}
            }
        }
        Popup::EditMessage => {
            match code {
                KeyCode::Esc => {
                    app.popup = Popup::None;
                    app.publish_form.error.clear();
                }
                KeyCode::Tab | KeyCode::Down => {
                    app.publish_form.focused_field = (app.publish_form.focused_field + 1) % app::PublishForm::field_count();
                }
                KeyCode::BackTab | KeyCode::Up => {
                    let count = app::PublishForm::field_count();
                    app.publish_form.focused_field = (app.publish_form.focused_field + count - 1) % count;
                }
                KeyCode::Enter => {
                    if app.publish_form.focused_field == 2 {
                        app.publish_form.newline();
                    } else if modifiers.contains(KeyModifiers::CONTROL) || app.publish_form.focused_field != 2 {
                        if app.publish_form.body.is_empty() {
                            app.publish_form.error = "Body is required".into();
                        } else {
                            app.publish_form.error.clear();
                            app.do_publish();
                        }
                    }
                }
                KeyCode::Backspace => {
                    app.publish_form.pop_char();
                }
                KeyCode::Char(c) => {
                    app.publish_form.push_char(c);
                }
                _ => {}
            }
        }
        Popup::ConfirmPurge => {
            match code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    if let Some(q) = app.selected_queue() {
                        let name = q.name.clone();
                        app.popup = Popup::None;
                        app.do_purge(&name);
                    }
                }
                _ => {
                    app.popup = Popup::None;
                }
            }
        }
        Popup::ConfirmDelete => {
            match code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    if let Some(q) = app.selected_queue() {
                        let name = q.name.clone();
                        app.popup = Popup::None;
                        app.do_delete(&name);
                    }
                }
                _ => {
                    app.popup = Popup::None;
                }
            }
        }
        Popup::QueuePicker(_) => {
            let filtered: Vec<usize> = app.queues.iter().enumerate()
                .filter(|(_, q)| {
                    app.queue_picker_filter.is_empty()
                        || q.name.to_lowercase().contains(&app.queue_picker_filter.to_lowercase())
                })
                .map(|(i, _)| i)
                .collect();
            let len = filtered.len();

            if app.queue_picker_filter_active {
                match code {
                    KeyCode::Char(c) => {
                        app.queue_picker_filter.push(c);
                        app.popup_list_state.select(Some(0));
                    }
                    KeyCode::Backspace => {
                        app.queue_picker_filter.pop();
                        app.popup_list_state.select(Some(0));
                    }
                    KeyCode::Esc => {
                        app.queue_picker_filter.clear();
                        app.queue_picker_filter_active = false;
                    }
                    KeyCode::Enter => {
                        app.queue_picker_filter_active = false;
                    }
                    _ => {}
                }
            } else {
                match code {
                    KeyCode::Esc => {
                        app.popup = Popup::None;
                        app.queue_picker_filter.clear();
                    }
                    KeyCode::Char('/') => {
                        app.queue_picker_filter_active = true;
                    }
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
                            let dest_idx = filtered[selected];
                            let dest_name = app.queues[dest_idx].name.clone();
                            let source_name = app.selected_queue()
                                .map(|q| q.name.clone())
                                .unwrap_or_default();

                            if dest_name == source_name {
                                app.set_status("Source and destination must be different", true);
                            } else if let Popup::QueuePicker(ref op) = app.popup.clone() {
                                let op = op.clone();
                                app.popup = Popup::None;
                                app.queue_picker_filter.clear();
                                app.do_copy_or_move(&source_name, &dest_name, op);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        Popup::MessageQueuePicker(_) => {
            let filtered: Vec<usize> = app.queues.iter().enumerate()
                .filter(|(_, q)| {
                    app.queue_picker_filter.is_empty()
                        || q.name.to_lowercase().contains(&app.queue_picker_filter.to_lowercase())
                })
                .map(|(i, _)| i)
                .collect();
            let len = filtered.len();

            if app.queue_picker_filter_active {
                match code {
                    KeyCode::Char(c) => {
                        app.queue_picker_filter.push(c);
                        app.popup_list_state.select(Some(0));
                    }
                    KeyCode::Backspace => {
                        app.queue_picker_filter.pop();
                        app.popup_list_state.select(Some(0));
                    }
                    KeyCode::Esc => {
                        app.queue_picker_filter.clear();
                        app.queue_picker_filter_active = false;
                    }
                    KeyCode::Enter => {
                        app.queue_picker_filter_active = false;
                    }
                    _ => {}
                }
            } else {
                match code {
                    KeyCode::Esc => {
                        app.popup = Popup::None;
                        app.queue_picker_filter.clear();
                    }
                    KeyCode::Char('/') => {
                        app.queue_picker_filter_active = true;
                    }
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
                            let dest_idx = filtered[selected];
                            let dest_name = app.queues[dest_idx].name.clone();

                            if let Popup::MessageQueuePicker(ref op) = app.popup.clone() {
                                let op = op.clone();
                                app.popup = Popup::None;
                                app.queue_picker_filter.clear();

                                match op {
                                    app::QueueOperation::Copy => {
                                        app.do_copy_selected_to(&dest_name);
                                    }
                                    app::QueueOperation::Move => {
                                        // Move = copy selected to dest + delete selected from source
                                        app.do_copy_selected_to(&dest_name);
                                        // After copy completes, user can delete from source
                                        // (full atomic move requires consume-all which is too risky for selected)
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        Popup::ConfirmDeleteMessages => {
            match code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    app.popup = Popup::None;
                    app.do_delete_selected();
                }
                _ => {
                    app.popup = Popup::None;
                }
            }
        }
        Popup::ExportMessages => {
            app.popup = Popup::None;
        }
        Popup::ConsumerGroups => {
            match code {
                KeyCode::Char('j') | KeyCode::Down => {
                    app.consumer_groups_scroll = app.consumer_groups_scroll.saturating_add(1);
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    app.consumer_groups_scroll = app.consumer_groups_scroll.saturating_sub(1);
                }
                KeyCode::PageDown => {
                    app.consumer_groups_scroll = app.consumer_groups_scroll.saturating_add(10);
                }
                KeyCode::PageUp => {
                    app.consumer_groups_scroll = app.consumer_groups_scroll.saturating_sub(10);
                }
                KeyCode::Esc | KeyCode::Char('G') => {
                    app.popup = Popup::None;
                }
                _ => {}
            }
        }
        Popup::ConfirmReroute { .. } => {
            match code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    if let Popup::ConfirmReroute { exchange, routing_key, .. } = app.popup.clone() {
                        app.popup = Popup::None;
                        app.do_reroute_messages(&exchange, &routing_key);
                    }
                }
                _ => {
                    app.popup = Popup::None;
                }
            }
        }
        Popup::QueueInfo => {
            match code {
                KeyCode::Char('j') | KeyCode::Down => {
                    app.queue_info_scroll = app.queue_info_scroll.saturating_add(1);
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    app.queue_info_scroll = app.queue_info_scroll.saturating_sub(1);
                }
                KeyCode::PageDown => {
                    app.queue_info_scroll = app.queue_info_scroll.saturating_add(10);
                }
                KeyCode::PageUp => {
                    app.queue_info_scroll = app.queue_info_scroll.saturating_sub(10);
                }
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('i') => {
                    app.popup = Popup::None;
                }
                _ => {}
            }
        }
        Popup::ImportFile => {
            match code {
                KeyCode::Char(c) => {
                    app.import_file_path.push(c);
                }
                KeyCode::Backspace => {
                    app.import_file_path.pop();
                }
                KeyCode::Enter => {
                    app.popup = Popup::None;
                    app.do_import_jsonl();
                }
                KeyCode::Esc => {
                    app.popup = Popup::None;
                    app.import_file_path.clear();
                }
                _ => {}
            }
        }
        Popup::OperationProgress => {
            if code == KeyCode::Esc {
                app.operation_cancel.store(true, std::sync::atomic::Ordering::Relaxed);
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
