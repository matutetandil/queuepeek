use crossterm::event::{KeyCode, KeyModifiers};

use crate::app::{self, App, Popup, QueueOperation};
use crate::backend::OffsetResetStrategy;
use crate::ui;
use crate::utils;

pub fn handle_popup_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
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
                    if let Some(sel) = app.popup_list_state.selected() {
                        if sel < names.len() {
                            app.theme = ui::theme::get_theme(names[sel]);
                        }
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    let i = app.popup_list_state.selected().unwrap_or(0);
                    if i > 0 { app.popup_list_state.select(Some(i - 1)); }
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
        // Unified handler for PublishMessage and EditMessage
        Popup::PublishMessage | Popup::EditMessage => {
            handle_publish_key(app, code, modifiers);
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
        // Unified handler for all three queue picker variants
        Popup::QueuePicker(_) | Popup::MessageQueuePicker(_) | Popup::CompareQueuePicker => {
            handle_queue_picker_key(app, code);
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
            handle_consumer_groups_key(app, code);
        }
        Popup::ResetOffsetPicker => {
            let strategies = ["Earliest", "Latest", "To Timestamp", "To Offset"];
            match code {
                KeyCode::Esc => {
                    app.popup = Popup::ConsumerGroups;
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    let i = app.popup_list_state.selected().unwrap_or(0);
                    if i + 1 < strategies.len() { app.popup_list_state.select(Some(i + 1)); }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    let i = app.popup_list_state.selected().unwrap_or(0);
                    if i > 0 { app.popup_list_state.select(Some(i - 1)); }
                }
                KeyCode::Enter => {
                    let selected = app.popup_list_state.selected().unwrap_or(0);
                    match selected {
                        0 => {
                            app.reset_strategy = Some(OffsetResetStrategy::Earliest);
                            app.popup = Popup::ConfirmResetOffset;
                        }
                        1 => {
                            app.reset_strategy = Some(OffsetResetStrategy::Latest);
                            app.popup = Popup::ConfirmResetOffset;
                        }
                        2 | 3 => {
                            app.reset_input.clear();
                            app.popup = Popup::ResetOffsetInput;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        Popup::ResetOffsetInput => {
            match code {
                KeyCode::Esc => {
                    app.popup = Popup::ResetOffsetPicker;
                    app.popup_list_state.select(Some(0));
                }
                KeyCode::Char(c) if c.is_ascii_digit() || c == '-' => {
                    app.reset_input.push(c);
                }
                KeyCode::Backspace => {
                    app.reset_input.pop();
                }
                KeyCode::Enter => {
                    if let Ok(val) = app.reset_input.parse::<i64>() {
                        let picker_sel = app.popup_list_state.selected().unwrap_or(2);
                        app.reset_strategy = Some(if picker_sel == 2 {
                            OffsetResetStrategy::ToTimestamp(val)
                        } else {
                            OffsetResetStrategy::ToOffset(val)
                        });
                        app.popup = Popup::ConfirmResetOffset;
                    } else {
                        app.set_status("Invalid number", true);
                    }
                }
                _ => {}
            }
        }
        Popup::ConfirmResetOffset => {
            match code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    app.popup = Popup::None;
                    app.do_reset_offsets();
                    app.set_status("Resetting offsets...", false);
                }
                _ => {
                    app.popup = Popup::ConsumerGroups;
                }
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
            handle_scrollable_popup(app, code, |a| &mut a.queue_info_scroll, &[
                KeyCode::Char('q'), KeyCode::Char('i'),
            ]);
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
        Popup::MessageDiff => {
            match code {
                KeyCode::Esc => {
                    app.popup = Popup::None;
                    app.diff_messages = None;
                }
                _ => handle_scroll_keys(code, &mut app.diff_scroll),
            }
        }
        Popup::CompareResults => {
            match code {
                KeyCode::Esc => {
                    app.popup = Popup::None;
                    app.comparison_result = None;
                }
                KeyCode::Tab => {
                    app.comparison_tab = match app.comparison_tab {
                        app::ComparisonTab::Summary => app::ComparisonTab::OnlyInA,
                        app::ComparisonTab::OnlyInA => app::ComparisonTab::OnlyInB,
                        app::ComparisonTab::OnlyInB => app::ComparisonTab::Summary,
                    };
                    app.comparison_scroll = 0;
                }
                KeyCode::BackTab => {
                    app.comparison_tab = match app.comparison_tab {
                        app::ComparisonTab::Summary => app::ComparisonTab::OnlyInB,
                        app::ComparisonTab::OnlyInA => app::ComparisonTab::Summary,
                        app::ComparisonTab::OnlyInB => app::ComparisonTab::OnlyInA,
                    };
                    app.comparison_scroll = 0;
                }
                _ => handle_scroll_keys(code, &mut app.comparison_scroll),
            }
        }
        Popup::ScheduleDelay => {
            let presets = app::SCHEDULE_PRESETS;
            match code {
                KeyCode::Esc => {
                    app.popup = Popup::PublishMessage;
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    let i = app.popup_list_state.selected().unwrap_or(0);
                    if i + 1 < presets.len() { app.popup_list_state.select(Some(i + 1)); }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    let i = app.popup_list_state.selected().unwrap_or(0);
                    if i > 0 { app.popup_list_state.select(Some(i - 1)); }
                }
                KeyCode::Enter => {
                    let selected = app.popup_list_state.selected().unwrap_or(0);
                    if selected < presets.len() {
                        let delay_secs = presets[selected].0;
                        app.schedule_message(delay_secs);
                        app.popup = Popup::None;
                        app.set_status(format!("Message scheduled ({})", presets[selected].1), false);
                    }
                }
                _ => {}
            }
        }
        Popup::ScheduledMessages => {
            let count = app.scheduled_messages.len();
            match code {
                KeyCode::Esc => {
                    app.popup = Popup::None;
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    if count > 0 {
                        let i = app.scheduled_list_state.selected().unwrap_or(0);
                        if i + 1 < count { app.scheduled_list_state.select(Some(i + 1)); }
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    let i = app.scheduled_list_state.selected().unwrap_or(0);
                    if i > 0 { app.scheduled_list_state.select(Some(i - 1)); }
                }
                KeyCode::Char('d') | KeyCode::Delete => {
                    if let Some(sel) = app.scheduled_list_state.selected() {
                        if sel < count {
                            let id = app.scheduled_messages[sel].id;
                            app.cancel_scheduled_message(id);
                            app.set_status("Scheduled message cancelled", false);
                            if app.scheduled_messages.is_empty() {
                                app.popup = Popup::None;
                            } else if sel >= app.scheduled_messages.len() {
                                app.scheduled_list_state.select(Some(app.scheduled_messages.len() - 1));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        Popup::SavedFilters => {
            handle_saved_filters_key(app, code);
        }
        Popup::SaveFilter => {
            match code {
                KeyCode::Esc => app.popup = Popup::None,
                KeyCode::Char(c) => app.save_filter_name.push(c),
                KeyCode::Backspace => { app.save_filter_name.pop(); }
                KeyCode::Enter => {
                    if !app.save_filter_name.is_empty() {
                        let queue = app.current_queue_name.clone();
                        let filter = crate::config::SavedFilter {
                            name: app.save_filter_name.clone(),
                            expression: app.message_filter.clone(),
                            advanced: app.message_filter_advanced,
                        };
                        app.config.filters.entry(queue).or_default().push(filter);
                        let _ = app.config.save(app.config_path.as_deref());
                        app.popup = Popup::None;
                        app.set_status(format!("Filter saved: {}", app.save_filter_name), false);
                    }
                }
                _ => {}
            }
        }
        Popup::TemplatePicker => {
            handle_template_picker_key(app, code);
        }
        Popup::SaveTemplate => {
            match code {
                KeyCode::Esc => app.popup = Popup::PublishMessage,
                KeyCode::Char(c) => app.save_template_name.push(c),
                KeyCode::Backspace => { app.save_template_name.pop(); }
                KeyCode::Enter => {
                    if !app.save_template_name.is_empty() {
                        let tmpl = crate::config::MessageTemplate {
                            name: app.save_template_name.clone(),
                            routing_key: app.publish_form.routing_key.clone(),
                            content_type: app.publish_form.content_type.clone(),
                            body: app.publish_form.body.clone(),
                        };
                        app.config.templates.push(tmpl);
                        let _ = app.config.save(app.config_path.as_deref());
                        app.popup = Popup::PublishMessage;
                        app.set_status(format!("Template saved: {}", app.save_template_name), false);
                    }
                }
                _ => {}
            }
        }
        Popup::ReplayConfig => {
            match code {
                KeyCode::Esc => app.popup = Popup::None,
                KeyCode::Tab | KeyCode::Down => {
                    app.replay_focused_field = (app.replay_focused_field + 1) % 3;
                }
                KeyCode::BackTab | KeyCode::Up => {
                    app.replay_focused_field = (app.replay_focused_field + 2) % 3;
                }
                KeyCode::Char(c) => {
                    match app.replay_focused_field {
                        0 => app.replay_start.push(c),
                        1 => app.replay_end.push(c),
                        2 => app.replay_dest.push(c),
                        _ => {}
                    }
                }
                KeyCode::Backspace => {
                    match app.replay_focused_field {
                        0 => { app.replay_start.pop(); }
                        1 => { app.replay_end.pop(); }
                        2 => { app.replay_dest.pop(); }
                        _ => {}
                    }
                }
                KeyCode::Enter => {
                    if app.replay_dest.is_empty() {
                        app.set_status("Destination topic is required", true);
                    } else {
                        app.popup = Popup::None;
                        app.set_status("Replaying messages...", false);
                        app.do_replay();
                    }
                }
                _ => {}
            }
        }
        Popup::TopologyView => {
            match code {
                KeyCode::Esc | KeyCode::Char('X') => {
                    app.popup = Popup::None;
                }
                _ => handle_scroll_keys(code, &mut app.topology_scroll),
            }
        }
        Popup::BenchmarkConfig => {
            match code {
                KeyCode::Esc => app.popup = Popup::None,
                KeyCode::Tab | KeyCode::Down => {
                    app.bench_focused_field = (app.bench_focused_field + 1) % 2;
                }
                KeyCode::BackTab | KeyCode::Up => {
                    app.bench_focused_field = (app.bench_focused_field + 1) % 2;
                }
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    match app.bench_focused_field {
                        0 => app.bench_count.push(c),
                        1 => app.bench_concurrency.push(c),
                        _ => {}
                    }
                }
                KeyCode::Backspace => {
                    match app.bench_focused_field {
                        0 => { app.bench_count.pop(); }
                        1 => { app.bench_concurrency.pop(); }
                        _ => {}
                    }
                }
                KeyCode::Enter => {
                    app.do_benchmark();
                }
                _ => {}
            }
        }
        Popup::BenchmarkRunning => {
            match code {
                KeyCode::Esc => {
                    app.operation_cancel.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                _ => {}
            }
            if app.bench_stats.is_some() {
                app.popup = Popup::None;
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

// ---------------------------------------------------------------------------
// Shared helpers to reduce duplication
// ---------------------------------------------------------------------------

fn handle_scroll_keys(code: KeyCode, scroll: &mut u16) {
    match code {
        KeyCode::Char('j') | KeyCode::Down => {
            *scroll = scroll.saturating_add(1);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            *scroll = scroll.saturating_sub(1);
        }
        KeyCode::PageDown => {
            *scroll = scroll.saturating_add(10);
        }
        KeyCode::PageUp => {
            *scroll = scroll.saturating_sub(10);
        }
        _ => {}
    }
}

fn handle_scrollable_popup(
    app: &mut App,
    code: KeyCode,
    scroll_field: fn(&mut App) -> &mut u16,
    extra_close_keys: &[KeyCode],
) {
    if code == KeyCode::Esc || extra_close_keys.contains(&code) {
        app.popup = Popup::None;
        return;
    }
    handle_scroll_keys(code, scroll_field(app));
}

/// Unified publish/edit popup handler (deduplicated)
fn handle_publish_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
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
        KeyCode::Char('s') if modifiers.contains(KeyModifiers::CONTROL) => {
            if app.publish_form.body.is_empty() {
                app.publish_form.error = "Body is required".into();
            } else {
                app.publish_form.error.clear();
                app.popup = Popup::ScheduleDelay;
                app.popup_list_state.select(Some(0));
            }
        }
        KeyCode::Char('t') if modifiers.contains(KeyModifiers::CONTROL) => {
            if !app.config.templates.is_empty() {
                app.template_list_state.select(Some(0));
                app.popup = Popup::TemplatePicker;
            } else {
                app.publish_form.error = "No templates saved yet".into();
            }
        }
        KeyCode::Char('w') if modifiers.contains(KeyModifiers::CONTROL) => {
            if !app.publish_form.body.is_empty() {
                app.save_template_name.clear();
                app.popup = Popup::SaveTemplate;
            }
        }
        KeyCode::Char(c) => {
            app.publish_form.push_char(c);
        }
        _ => {}
    }
}

/// Unified queue picker handler for QueuePicker, MessageQueuePicker, CompareQueuePicker
fn handle_queue_picker_key(app: &mut App, code: KeyCode) {
    let exclude_name = match &app.popup {
        Popup::CompareQueuePicker => Some(app.compare_queue_a.clone()),
        _ => None,
    };

    let filtered: Vec<usize> = app.queues.iter().enumerate()
        .filter(|(_, q)| {
            if let Some(ref excl) = exclude_name {
                if q.name == *excl { return false; }
            }
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
        return;
    }

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

                match app.popup.clone() {
                    Popup::QueuePicker(op) => {
                        let source_name = app.selected_queue()
                            .map(|q| q.name.clone())
                            .unwrap_or_default();
                        if dest_name == source_name {
                            app.set_status("Source and destination must be different", true);
                        } else {
                            app.popup = Popup::None;
                            app.queue_picker_filter.clear();
                            app.do_copy_or_move(&source_name, &dest_name, op);
                        }
                    }
                    Popup::MessageQueuePicker(op) => {
                        app.popup = Popup::None;
                        app.queue_picker_filter.clear();
                        match op {
                            QueueOperation::Copy => {
                                app.do_copy_selected_to(&dest_name);
                            }
                            QueueOperation::Move => {
                                app.do_copy_selected_to(&dest_name);
                            }
                        }
                    }
                    Popup::CompareQueuePicker => {
                        app.popup = Popup::None;
                        app.queue_picker_filter.clear();
                        app.loading = true;
                        app.set_status(format!("Comparing {} vs {}...", app.compare_queue_a, dest_name), false);
                        let qa = app.compare_queue_a.clone();
                        app.load_comparison(&qa, &dest_name);
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

fn handle_consumer_groups_key(app: &mut App, code: KeyCode) {
    let group_count = app.consumer_groups.len();
    match code {
        KeyCode::Char('j') | KeyCode::Down => {
            if group_count > 0 {
                let sel = app.consumer_groups_selected.unwrap_or(0);
                if sel + 1 < group_count {
                    app.consumer_groups_selected = Some(sel + 1);
                }
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if group_count > 0 {
                let sel = app.consumer_groups_selected.unwrap_or(0);
                if sel > 0 {
                    app.consumer_groups_selected = Some(sel - 1);
                }
            }
        }
        KeyCode::PageDown => {
            if group_count > 0 {
                let sel = app.consumer_groups_selected.unwrap_or(0);
                app.consumer_groups_selected = Some((sel + 5).min(group_count - 1));
            }
        }
        KeyCode::PageUp => {
            if group_count > 0 {
                let sel = app.consumer_groups_selected.unwrap_or(0);
                app.consumer_groups_selected = Some(sel.saturating_sub(5));
            }
        }
        KeyCode::Char('R') => {
            if let Some(ref backend) = app.backend {
                if backend.backend_type() != "kafka" {
                    app.set_status("Offset reset is only supported for Kafka", true);
                } else if let Some(sel) = app.consumer_groups_selected {
                    if sel < group_count {
                        let group = &app.consumer_groups[sel];
                        if group.state == "Stable" {
                            app.set_status("Cannot reset offsets: group is active (Stable). Stop consumers first.", true);
                        } else {
                            app.reset_group_name = group.name.clone();
                            app.popup = Popup::ResetOffsetPicker;
                            app.popup_list_state.select(Some(0));
                        }
                    }
                }
            }
        }
        KeyCode::Esc | KeyCode::Char('G') => {
            app.popup = Popup::None;
            app.consumer_groups_selected = None;
        }
        _ => {}
    }
}

fn handle_saved_filters_key(app: &mut App, code: KeyCode) {
    let queue = app.current_queue_name.clone();
    let count = app.config.filters.get(&queue).map(|f| f.len()).unwrap_or(0);
    match code {
        KeyCode::Esc => app.popup = Popup::None,
        KeyCode::Char('j') | KeyCode::Down => {
            let i = app.saved_filter_list_state.selected().unwrap_or(0);
            if i + 1 < count { app.saved_filter_list_state.select(Some(i + 1)); }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            let i = app.saved_filter_list_state.selected().unwrap_or(0);
            if i > 0 { app.saved_filter_list_state.select(Some(i - 1)); }
        }
        KeyCode::Enter => {
            if let Some(sel) = app.saved_filter_list_state.selected() {
                let filter_data = app.config.filters.get(&queue)
                    .and_then(|f| f.get(sel))
                    .map(|f| (f.expression.clone(), f.advanced, f.name.clone()));
                if let Some((expr, advanced, name)) = filter_data {
                    app.message_filter = expr;
                    app.message_filter_advanced = advanced;
                    app.update_filtered_messages();
                    if !app.filtered_message_indices.is_empty() {
                        app.message_list_state.select(Some(0));
                    }
                    app.popup = Popup::None;
                    app.set_status(format!("Filter loaded: {}", name), false);
                }
            }
        }
        KeyCode::Char('d') | KeyCode::Delete => {
            if let Some(sel) = app.saved_filter_list_state.selected() {
                if sel < count {
                    if let Some(filters) = app.config.filters.get_mut(&queue) {
                        filters.remove(sel);
                    }
                    let _ = app.config.save(app.config_path.as_deref());
                    let new_count = app.config.filters.get(&queue).map(|f| f.len()).unwrap_or(0);
                    if new_count == 0 {
                        app.popup = Popup::None;
                    } else if sel >= new_count {
                        app.saved_filter_list_state.select(Some(new_count - 1));
                    }
                }
            }
        }
        _ => {}
    }
}

fn handle_template_picker_key(app: &mut App, code: KeyCode) {
    let count = app.config.templates.len();
    match code {
        KeyCode::Esc => app.popup = Popup::PublishMessage,
        KeyCode::Char('j') | KeyCode::Down => {
            let i = app.template_list_state.selected().unwrap_or(0);
            if i + 1 < count { app.template_list_state.select(Some(i + 1)); }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            let i = app.template_list_state.selected().unwrap_or(0);
            if i > 0 { app.template_list_state.select(Some(i - 1)); }
        }
        KeyCode::Enter => {
            if let Some(sel) = app.template_list_state.selected() {
                if let Some(tmpl) = app.config.templates.get(sel) {
                    let body = utils::interpolate_template(&tmpl.body, &mut app.template_counter);
                    app.publish_form.body = body;
                    if !tmpl.routing_key.is_empty() {
                        app.publish_form.routing_key = tmpl.routing_key.clone();
                    }
                    if !tmpl.content_type.is_empty() {
                        app.publish_form.content_type = tmpl.content_type.clone();
                    }
                    app.popup = Popup::PublishMessage;
                    app.set_status(format!("Template loaded: {}", tmpl.name), false);
                }
            }
        }
        KeyCode::Char('d') | KeyCode::Delete => {
            if let Some(sel) = app.template_list_state.selected() {
                if sel < count {
                    app.config.templates.remove(sel);
                    let _ = app.config.save(app.config_path.as_deref());
                    if app.config.templates.is_empty() {
                        app.popup = Popup::PublishMessage;
                    } else if sel >= app.config.templates.len() {
                        app.template_list_state.select(Some(app.config.templates.len() - 1));
                    }
                }
            }
        }
        _ => {}
    }
}
