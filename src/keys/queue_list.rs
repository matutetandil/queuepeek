use crossterm::event::{KeyCode, KeyModifiers};

use crate::app::{self, App, Popup, Screen};

pub fn handle_queue_list_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    if app.popup != Popup::None {
        super::popup::handle_popup_key(app, code, modifiers);
        return;
    }

    if app.queue_filter_active && app.queue_filter_focused {
        handle_queue_filter_key(app, code);
        return;
    }

    // Shift+Tab returns focus to the filter input when filter is active
    if app.queue_filter_active && code == KeyCode::BackTab {
        app.queue_filter_focused = true;
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
                    app.messages.clear();
                    app.filtered_message_indices.clear();
                    app.selected_messages.clear();
                    app.message_list_state.select(None);
                    app.loading = true;
                    app.set_status(format!("Loading messages from {}", app.current_queue_name), false);
                    app.load_messages();
                }
            }
        }
        KeyCode::Char('/') => {
            app.queue_filter_active = true;
            app.queue_filter_focused = true;
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
            if app.queue_filter_active {
                app.queue_filter.clear();
                app.queue_filter_active = false;
                app.queue_filter_focused = false;
                app.update_filtered_queues();
            } else {
                app.screen = Screen::ProfileSelect;
                app.backend = None;
                app.queues.clear();
                app.filtered_queue_indices.clear();
                app.messages.clear();
                app.current_queue_name.clear();
                app.queue_filter.clear();
                app.set_status(String::new(), false);
            }
        }
        KeyCode::Char('f') => {
            app.popup = Popup::FetchCount;
            let idx = app::FETCH_PRESETS.iter().position(|&c| c == app.fetch_count).unwrap_or(2);
            app.popup_list_state.select(Some(idx));
        }
        KeyCode::Char('+') => {
            if app.fetch_count < 500 { app.fetch_count += 10; }
            app.set_status(format!("Fetch count: {}", app.fetch_count), false);
        }
        KeyCode::Char('-') => {
            app.fetch_count = app.fetch_count.saturating_sub(10).max(1);
            app.set_status(format!("Fetch count: {}", app.fetch_count), false);
        }
        KeyCode::Char('=') => {
            if let Some(q) = app.selected_queue() {
                app.compare_queue_a = q.name.clone();
                app.queue_picker_filter.clear();
                app.queue_picker_filter_active = false;
                app.popup = Popup::CompareQueuePicker;
                app.popup_list_state.select(Some(0));
            }
        }
        KeyCode::Char('P') => {
            if let Some(q) = app.selected_queue() {
                let name = q.name.clone();
                app.publish_form = app::PublishForm::new_for_queue(&name);
                app.popup = Popup::PublishMessage;
            }
        }
        KeyCode::Char('x') => {
            if app.selected_queue().is_some() {
                app.popup = Popup::ConfirmPurge;
            }
        }
        KeyCode::Char('D') => {
            if app.selected_queue().is_some() {
                app.popup = Popup::ConfirmDelete;
            }
        }
        KeyCode::Char('C') => {
            if app.selected_queue().is_some() {
                app.queue_picker_filter.clear();
                app.queue_picker_filter_active = false;
                app.popup = Popup::QueuePicker(app::QueueOperation::Copy);
                app.popup_list_state.select(Some(0));
            }
        }
        KeyCode::Char('m') => {
            if app.selected_queue().is_some() {
                app.queue_picker_filter.clear();
                app.queue_picker_filter_active = false;
                app.popup = Popup::QueuePicker(app::QueueOperation::Move);
                app.popup_list_state.select(Some(0));
            }
        }
        KeyCode::Char('G') => {
            if let Some(q) = app.selected_queue() {
                let name = q.name.clone();
                app.consumer_groups.clear();
                app.consumer_groups_scroll = 0;
                app.consumer_groups_selected = Some(0);
                app.popup = Popup::ConsumerGroups;
                app.loading = true;
                app.load_consumer_groups(&name);
            }
        }
        KeyCode::Char('i') => {
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
        KeyCode::Char('S') => {
            if !app.scheduled_messages.is_empty() {
                app.scheduled_list_state.select(Some(0));
                app.popup = Popup::ScheduledMessages;
            } else {
                app.set_status("No scheduled messages", false);
            }
        }
        KeyCode::Char('X') => {
            app.topology_exchanges.clear();
            app.topology_bindings.clear();
            app.topology_scroll = 0;
            app.popup = Popup::TopologyView;
            app.loading = true;
            app.load_topology();
        }
        KeyCode::F(5) => {
            if app.selected_queue().is_some() {
                app.bench_focused_field = 0;
                app.popup = Popup::BenchmarkConfig;
            }
        }
        KeyCode::Char('W') => {
            // Webhook alert config
            if !app.config.webhook_alerts.is_empty() {
                app.alert_list_state.select(Some(0));
            }
            app.popup = Popup::AlertConfig;
        }
        KeyCode::Char('A') => {
            // ACL/Permission viewer
            app.permissions.clear();
            app.permissions_scroll = 0;
            app.popup = Popup::Permissions;
            app.loading = true;
            app.load_permissions();
        }
        KeyCode::Char('H') => {
            // Retained messages (MQTT only)
            if let Some(ref backend) = app.backend {
                if backend.backend_type() == "mqtt" {
                    app.retained_messages.clear();
                    app.popup = Popup::RetainedMessages;
                    app.loading = true;
                    app.load_retained_messages();
                } else {
                    app.set_status("Retained messages are only available for MQTT", true);
                }
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
        KeyCode::Tab | KeyCode::Down => {
            // Move focus to the list, keep filter active
            app.queue_filter_focused = false;
            let len = app.filtered_queue_indices.len();
            if len > 0 {
                let i = app.queue_list_state.selected().unwrap_or(0);
                if code == KeyCode::Down && i + 1 < len {
                    app.queue_list_state.select(Some(i + 1));
                }
            }
        }
        KeyCode::Up => {
            // Move focus to the list, keep filter active
            app.queue_filter_focused = false;
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
                    app.queue_filter_focused = false;
                    app.screen = Screen::MessageList;
                    app.messages.clear();
                    app.filtered_message_indices.clear();
                    app.selected_messages.clear();
                    app.message_list_state.select(None);
                    app.loading = true;
                    app.set_status(format!("Loading messages from {}", app.current_queue_name), false);
                    app.load_messages();
                }
            }
        }
        KeyCode::Esc => {
            if app.queue_filter.is_empty() {
                app.queue_filter_active = false;
                app.queue_filter_focused = false;
            } else {
                // First Esc: just unfocus to browse the filtered list
                app.queue_filter_focused = false;
            }
        }
        _ => {}
    }
}
