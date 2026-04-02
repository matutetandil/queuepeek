mod app;
mod config;
mod rabbit;
mod ui;

use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyModifiers, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::prelude::*;

use app::{App, Focus, Popup, ProfileMode, QueueTab, RightView, Screen};
use config::Config;

fn main() -> io::Result<()> {
    // Parse args
    let args: Vec<String> = std::env::args().collect();
    let config_path = args.iter().position(|a| a == "-c" || a == "--config")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str());

    let config = Config::load(config_path);
    let mut app = App::new(config, config_path.map(String::from));

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Main loop
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        eprintln!("Error: {}", err);
    }

    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> io::Result<()> {
    loop {
        // Process background results
        app.process_bg_results();

        // Draw
        terminal.draw(|frame| ui::draw(frame, app))?;

        // Handle events with timeout for non-blocking bg result processing
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                handle_key(app, key.code, key.modifiers);
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn handle_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    // Ctrl+C always quits
    if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
        app.should_quit = true;
        return;
    }

    match app.screen {
        Screen::ProfileSelect => handle_profile_key(app, code, modifiers),
        Screen::Main => handle_main_key(app, code, modifiers),
    }
}

fn handle_profile_key(app: &mut App, code: KeyCode, _modifiers: KeyModifiers) {
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
                // Connect to profile
                let name = names[selected].clone();
                app.connect_profile(&name);
            } else if selected == names.len() {
                // Add new profile
                app.profile_form.clear();
                app.profile_mode = ProfileMode::Add;
            } else if selected == names.len() + 1 {
                // Cycle theme
                let theme_names = ui::theme::theme_names();
                let current = app.config.theme.as_deref().unwrap_or("slack");
                let idx = theme_names.iter().position(|&n| n == current).unwrap_or(0);
                let next = theme_names[(idx + 1) % theme_names.len()];
                app.config.theme = Some(next.to_string());
                app.theme = ui::theme::get_theme(next);
                let _ = app.config.save(app.config_path.as_deref());
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
            // Same as selecting theme item
            let theme_names = ui::theme::theme_names();
            let current = app.config.theme.as_deref().unwrap_or("slack");
            let idx = theme_names.iter().position(|&n| n == current).unwrap_or(0);
            let next = theme_names[(idx + 1) % theme_names.len()];
            app.config.theme = Some(next.to_string());
            app.theme = ui::theme::get_theme(next);
            let _ = app.config.save(app.config_path.as_deref());
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
            match app.profile_form.to_profile() {
                Ok(profile) => {
                    let name = app.profile_form.name.clone();
                    // If editing and name changed, delete old
                    if let ProfileMode::Edit(ref old_name) = app.profile_mode {
                        if old_name != &name {
                            app.config.delete_profile(old_name);
                        }
                    }
                    app.config.add_profile(name.clone(), profile);
                    let _ = app.config.save(app.config_path.as_deref());
                    app.profile_mode = ProfileMode::Select;
                    // Connect immediately
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
                // Adjust selection
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

fn handle_main_key(app: &mut App, code: KeyCode, _modifiers: KeyModifiers) {
    // Popup handling first
    if app.popup != Popup::None {
        handle_popup_key(app, code);
        return;
    }

    // Global keys
    match code {
        KeyCode::Char('q') => { app.should_quit = true; return; }
        KeyCode::Char('?') => {
            app.popup = if app.popup == Popup::Help { Popup::None } else { Popup::Help };
            return;
        }
        KeyCode::Char('p') => {
            app.popup = Popup::ProfileSwitch;
            app.popup_list_state.select(Some(0));
            return;
        }
        KeyCode::Char('v') => {
            if !app.vhosts.is_empty() {
                app.popup = Popup::VhostPicker;
                let idx = app.vhosts.iter().position(|v| v == &app.selected_vhost).unwrap_or(0);
                app.popup_list_state.select(Some(idx));
            }
            return;
        }
        KeyCode::Char('r') => {
            if !app.current_queue_name.is_empty() {
                app.loading = true;
                app.load_messages();
            }
            return;
        }
        KeyCode::Char('R') => {
            app.loading = true;
            app.load_queues();
            return;
        }
        KeyCode::Char('+') | KeyCode::Char('=') => {
            if app.fetch_count < 500 { app.fetch_count += 10; }
            app.set_status(format!("Fetch count: {}", app.fetch_count), false);
            return;
        }
        KeyCode::Char('-') => {
            app.fetch_count = app.fetch_count.saturating_sub(10).max(1);
            app.set_status(format!("Fetch count: {}", app.fetch_count), false);
            return;
        }
        KeyCode::Tab => {
            app.focus = match app.focus {
                Focus::Sidebar => Focus::RightHeader,
                Focus::RightHeader => Focus::RightTabs,
                Focus::RightTabs => Focus::RightContent,
                Focus::RightContent => Focus::Sidebar,
            };
            return;
        }
        KeyCode::BackTab => {
            app.focus = match app.focus {
                Focus::Sidebar => Focus::RightContent,
                Focus::RightHeader => Focus::Sidebar,
                Focus::RightTabs => Focus::RightHeader,
                Focus::RightContent => Focus::RightTabs,
            };
            return;
        }
        _ => {}
    }

    // Focus-specific keys
    match app.focus {
        Focus::Sidebar => match code {
            KeyCode::Char('j') | KeyCode::Down => {
                if app.sidebar_cursor + 1 < App::sidebar_item_count() {
                    app.sidebar_cursor += 1;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if app.sidebar_cursor > 0 {
                    app.sidebar_cursor -= 1;
                }
            }
            KeyCode::Enter => {
                app.right_view = App::right_view_for_sidebar(app.sidebar_cursor);
                if app.right_view == RightView::Queues {
                    app.focus = Focus::RightContent;
                }
            }
            _ => {}
        },
        Focus::RightHeader => match code {
            KeyCode::Enter => {
                if app.right_view == RightView::Queues {
                    // Open queue picker popup
                    app.popup = Popup::QueuePicker;
                    app.picker_filter.clear();
                    app.picker_filter_active = false;
                    app.update_filtered_queues();
                    let idx = app.filtered_indices.iter()
                        .position(|&i| app.queues[i].name == app.current_queue_name)
                        .unwrap_or(0);
                    app.popup_list_state.select(Some(idx));
                }
            }
            KeyCode::Char('/') => {
                if app.right_view == RightView::Queues {
                    app.popup = Popup::QueuePicker;
                    app.picker_filter.clear();
                    app.picker_filter_active = true;
                    app.update_filtered_queues();
                    app.popup_list_state.select(Some(0));
                }
            }
            _ => {}
        },
        Focus::RightTabs => {
            if app.right_view == RightView::Queues {
                match code {
                    KeyCode::Char('h') | KeyCode::Left => {
                        app.queue_tab = match app.queue_tab {
                            QueueTab::Overview => QueueTab::Settings,
                            QueueTab::Publish => QueueTab::Overview,
                            QueueTab::Consume => QueueTab::Publish,
                            QueueTab::Routing => QueueTab::Consume,
                            QueueTab::Settings => QueueTab::Routing,
                        };
                    }
                    KeyCode::Char('l') | KeyCode::Right => {
                        app.queue_tab = match app.queue_tab {
                            QueueTab::Overview => QueueTab::Publish,
                            QueueTab::Publish => QueueTab::Consume,
                            QueueTab::Consume => QueueTab::Routing,
                            QueueTab::Routing => QueueTab::Settings,
                            QueueTab::Settings => QueueTab::Overview,
                        };
                    }
                    KeyCode::Char('1') => app.queue_tab = QueueTab::Overview,
                    KeyCode::Char('2') => app.queue_tab = QueueTab::Publish,
                    KeyCode::Char('3') => app.queue_tab = QueueTab::Consume,
                    KeyCode::Char('4') => app.queue_tab = QueueTab::Routing,
                    KeyCode::Char('5') => app.queue_tab = QueueTab::Settings,
                    _ => {}
                }
            }
        },
        Focus::RightContent => match code {
            KeyCode::Char('j') | KeyCode::Down => {
                app.message_scroll = app.message_scroll.saturating_add(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                app.message_scroll = app.message_scroll.saturating_sub(1);
            }
            KeyCode::PageDown => {
                app.message_scroll = app.message_scroll.saturating_add(10);
            }
            KeyCode::PageUp => {
                app.message_scroll = app.message_scroll.saturating_sub(10);
            }
            KeyCode::Char('/') => {
                app.popup = Popup::QueuePicker;
                app.picker_filter.clear();
                app.picker_filter_active = true;
                app.update_filtered_queues();
                app.popup_list_state.select(Some(0));
            }
            _ => {}
        },
    }
}

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
        Popup::VhostPicker => {
            let len = app.vhosts.len();
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
                        let vhost = app.vhosts[selected].clone();
                        app.popup = Popup::None;
                        if vhost != app.selected_vhost {
                            app.selected_vhost = vhost;
                            app.queues.clear();
                            app.filtered_indices.clear();
                            app.messages.clear();
                            app.queue_filter.clear();
                            app.current_queue_name.clear();
                            app.queue_list_state.select(None);
                            app.loading = true;
                            app.set_status(format!("Switching to vhost: {}", app.selected_vhost), false);
                            app.load_queues();
                        }
                    }
                }
                _ => {}
            }
        }
        Popup::QueuePicker => {
            if app.picker_filter_active {
                // Filter mode in queue picker
                match code {
                    KeyCode::Esc => {
                        if app.picker_filter.is_empty() {
                            app.picker_filter_active = false;
                            app.popup = Popup::None;
                        } else {
                            app.picker_filter.clear();
                            app.picker_filter_active = false;
                            app.update_filtered_queues();
                            if !app.filtered_indices.is_empty() { app.popup_list_state.select(Some(0)); }
                        }
                    }
                    KeyCode::Enter => {
                        // Select current item
                        let selected = app.popup_list_state.selected().unwrap_or(0);
                        if selected < app.filtered_indices.len() {
                            let idx = app.filtered_indices[selected];
                            app.current_queue_name = app.queues[idx].name.clone();
                            app.popup = Popup::None;
                            app.picker_filter_active = false;
                            app.loading = true;
                            app.set_status(format!("Loading {}", app.current_queue_name), false);
                            app.load_messages();
                            app.focus = Focus::RightContent;
                        }
                    }
                    KeyCode::Down => {
                        let i = app.popup_list_state.selected().unwrap_or(0);
                        if i + 1 < app.filtered_indices.len() { app.popup_list_state.select(Some(i + 1)); }
                    }
                    KeyCode::Up => {
                        let i = app.popup_list_state.selected().unwrap_or(0);
                        if i > 0 { app.popup_list_state.select(Some(i - 1)); }
                    }
                    KeyCode::Backspace => {
                        app.picker_filter.pop();
                        app.update_filtered_queues();
                        if !app.filtered_indices.is_empty() { app.popup_list_state.select(Some(0)); }
                    }
                    KeyCode::Char(c) => {
                        app.picker_filter.push(c);
                        app.update_filtered_queues();
                        if !app.filtered_indices.is_empty() { app.popup_list_state.select(Some(0)); }
                    }
                    _ => {}
                }
            } else {
                // Normal navigation in queue picker
                match code {
                    KeyCode::Esc => app.popup = Popup::None,
                    KeyCode::Char('/') => {
                        app.picker_filter_active = true;
                        app.picker_filter.clear();
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        let i = app.popup_list_state.selected().unwrap_or(0);
                        if i + 1 < app.filtered_indices.len() { app.popup_list_state.select(Some(i + 1)); }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        let i = app.popup_list_state.selected().unwrap_or(0);
                        if i > 0 { app.popup_list_state.select(Some(i - 1)); }
                    }
                    KeyCode::Enter => {
                        let selected = app.popup_list_state.selected().unwrap_or(0);
                        if selected < app.filtered_indices.len() {
                            let idx = app.filtered_indices[selected];
                            app.current_queue_name = app.queues[idx].name.clone();
                            app.popup = Popup::None;
                            app.loading = true;
                            app.set_status(format!("Loading {}", app.current_queue_name), false);
                            app.load_messages();
                            app.focus = Focus::RightContent;
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}
