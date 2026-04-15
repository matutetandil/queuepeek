use crossterm::event::{KeyCode, KeyModifiers};

use crate::app::{self, App, Popup, Screen};

pub fn handle_message_list_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    if app.popup != Popup::None {
        super::popup::handle_popup_key(app, code, modifiers);
        return;
    }

    if app.message_filter_active && app.message_filter_focused {
        handle_message_filter_key(app, code);
        return;
    }

    // Shift+Tab returns focus to the filter input when filter is active
    if app.message_filter_active && code == KeyCode::BackTab {
        app.message_filter_focused = true;
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
                    app.detail_search_active = false;
                    app.detail_search_query.clear();
                    app.detail_search_matches.clear();
                    app.detail_search_current = 0;
                    app.screen = Screen::MessageDetail;
                }
            }
        }
        KeyCode::Char('/') => {
            app.message_filter_active = true;
            app.message_filter_focused = true;
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
            app.toggle_message_selection();
            let len = app.filtered_message_indices.len();
            if len > 0 {
                let i = app.message_list_state.selected().unwrap_or(0);
                if i + 1 < len {
                    app.message_list_state.select(Some(i + 1));
                }
            }
        }
        KeyCode::Char('a') => {
            app.select_all_messages();
        }
        KeyCode::Char('C') => {
            if app.selection_count() > 0 {
                app.queue_picker_filter.clear();
                app.queue_picker_filter_active = false;
                app.popup = Popup::MessageQueuePicker(app::QueueOperation::Copy);
                app.popup_list_state.select(Some(0));
            }
        }
        KeyCode::Char('M') => {
            if app.selection_count() > 0 {
                app.queue_picker_filter.clear();
                app.queue_picker_filter_active = false;
                app.popup = Popup::MessageQueuePicker(app::QueueOperation::Move);
                app.popup_list_state.select(Some(0));
            }
        }
        KeyCode::Char('d') => {
            if app.selected_messages.len() == 2 {
                let indices: Vec<usize> = app.selected_messages.iter().cloned().collect();
                if let (Some(a), Some(b)) = (app.messages.get(indices[0]), app.messages.get(indices[1])) {
                    app.diff_messages = Some((a.clone(), b.clone()));
                    app.diff_scroll = 0;
                    app.popup = Popup::MessageDiff;
                }
            } else if !app.selected_messages.is_empty() {
                app.set_status("Select exactly 2 messages to diff", true);
            }
        }
        KeyCode::Char('D') => {
            if app.selection_count() > 0 {
                app.popup = Popup::ConfirmDeleteMessages;
            }
        }
        KeyCode::Char('e') => {
            if app.selection_count() > 0 {
                app.open_file_picker(crate::app::FilePickerMode::Export);
            }
        }
        KeyCode::Char('W') => {
            app.do_dump_queue();
        }
        KeyCode::Char('I') => {
            app.open_file_picker(crate::app::FilePickerMode::Import);
        }
        KeyCode::Char('T') => {
            app.message_auto_refresh = !app.message_auto_refresh;
            if app.message_auto_refresh {
                app.set_status("Auto-refresh ON (every 5s)", false);
            } else {
                app.set_status("Auto-refresh OFF", false);
            }
        }
        KeyCode::Char('S') => {
            if !app.scheduled_messages.is_empty() {
                app.scheduled_list_state.select(Some(0));
                app.popup = Popup::ScheduledMessages;
            } else {
                app.set_status("No scheduled messages", false);
            }
        }
        KeyCode::Char('Y') => {
            if let Some(ref backend) = app.backend {
                if backend.backend_type() == "kafka" {
                    app.replay_start.clear();
                    app.replay_end.clear();
                    app.replay_dest.clear();
                    app.replay_focused_field = 0;
                    app.popup = Popup::ReplayConfig;
                } else {
                    app.set_status("Replay is only supported for Kafka", true);
                }
            }
        }
        KeyCode::Char('B') => {
            let queue = app.current_queue_name.clone();
            let filters = app.config.filters.get(&queue);
            if let Some(f) = filters {
                if !f.is_empty() {
                    app.saved_filter_list_state.select(Some(0));
                    app.popup = Popup::SavedFilters;
                } else {
                    app.set_status("No saved filters for this queue", false);
                }
            } else {
                app.set_status("No saved filters for this queue", false);
            }
        }
        KeyCode::Char('b') if modifiers.contains(KeyModifiers::CONTROL) => {
            if !app.message_filter.is_empty() {
                app.save_filter_name.clear();
                app.popup = Popup::SaveFilter;
            } else {
                app.set_status("No filter to save", true);
            }
        }
        KeyCode::Char('L') => {
            if let Some((exchange, routing_key)) = app.parse_dlq_info() {
                let count = app.selection_count().max(1);
                app.popup = Popup::ConfirmReroute { exchange, routing_key, count };
            } else {
                app.set_status("No x-death header found — not a dead-lettered message", true);
            }
        }
        KeyCode::Esc => {
            if app.message_filter_active {
                app.message_filter.clear();
                app.message_filter_active = false;
                app.message_filter_focused = false;
                app.update_filtered_messages();
                if !app.filtered_message_indices.is_empty() {
                    app.message_list_state.select(Some(0));
                }
            } else if !app.selected_messages.is_empty() {
                app.selected_messages.clear();
            } else {
                app.screen = Screen::QueueList;
                app.messages.clear();
                app.message_filter.clear();
                app.message_filter_active = false;
                app.message_filter_focused = false;
                app.message_auto_refresh = false;
            }
        }
        _ => {}
    }
}

fn handle_message_filter_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Tab => {
            app.message_filter_advanced = !app.message_filter_advanced;
            app.update_filtered_messages();
            if !app.filtered_message_indices.is_empty() {
                app.message_list_state.select(Some(0));
            }
        }
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
            // Move focus to the list, keep filter active
            app.message_filter_focused = false;
            let len = app.filtered_message_indices.len();
            if len > 0 {
                let i = app.message_list_state.selected().unwrap_or(0);
                if i + 1 < len {
                    app.message_list_state.select(Some(i + 1));
                }
            }
        }
        KeyCode::Up => {
            // Move focus to the list, keep filter active
            app.message_filter_focused = false;
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
                    app.message_filter_focused = false;
                    app.screen = Screen::MessageDetail;
                }
            }
        }
        KeyCode::Esc => {
            if app.message_filter.is_empty() {
                app.message_filter_active = false;
                app.message_filter_focused = false;
            } else {
                // First Esc: just unfocus to browse the filtered list
                app.message_filter_focused = false;
            }
        }
        _ => {}
    }
}
