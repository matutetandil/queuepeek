use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, Popup, QueueOperation};
use crate::backend::OffsetResetStrategy;

pub fn draw(frame: &mut Frame, app: &mut App) {
    match &app.popup {
        Popup::Help => draw_help(frame, app),
        Popup::ProfileSwitch => draw_profile_switch(frame, app),
        Popup::NamespacePicker => draw_namespace_picker(frame, app),
        Popup::FetchCount => draw_fetch_count(frame, app),
        Popup::ThemePicker => draw_theme_picker(frame, app),
        Popup::BackendTypePicker => draw_backend_type_picker(frame, app),
        Popup::PublishMessage => draw_publish(frame, app, " Publish Message "),
        Popup::EditMessage => draw_publish(frame, app, " Edit & Re-publish "),
        Popup::ConfirmPurge => draw_confirm(frame, app, "Purge Queue", "Purge all messages from this queue?"),
        Popup::ConfirmDelete => draw_confirm(frame, app, "Delete Queue", "Delete this queue permanently?"),
        Popup::QueuePicker(_) => draw_queue_picker(frame, app),
        Popup::MessageQueuePicker(_) => draw_queue_picker(frame, app),
        Popup::OperationProgress => draw_operation_progress(frame, app),
        Popup::ConfirmDeleteMessages => {
            let count = app.selection_count();
            draw_confirm(frame, app, "Delete Messages",
                &format!("Delete {} selected message(s)?\n\nThis consumes all messages and re-publishes\nthe ones not selected. This is destructive.", count));
        }
        Popup::ExportMessages => {}
        Popup::ImportFile => draw_import_file(frame, app),
        Popup::QueueInfo => draw_queue_info(frame, app),
        Popup::ConsumerGroups => draw_consumer_groups(frame, app),
        Popup::ResetOffsetPicker => draw_reset_offset_picker(frame, app),
        Popup::ResetOffsetInput => draw_reset_offset_input(frame, app),
        Popup::ConfirmResetOffset => draw_confirm_reset_offset(frame, app),
        Popup::ScheduleDelay => draw_schedule_delay(frame, app),
        Popup::ScheduledMessages => draw_scheduled_messages(frame, app),
        Popup::CompareQueuePicker => draw_compare_queue_picker(frame, app),
        Popup::CompareResults => draw_compare_results(frame, app),
        Popup::MessageDiff => draw_message_diff(frame, app),
        Popup::SavedFilters => draw_saved_filters(frame, app),
        Popup::SaveFilter => draw_save_filter(frame, app),
        Popup::TemplatePicker => draw_template_picker(frame, app),
        Popup::SaveTemplate => draw_save_template(frame, app),
        Popup::ReplayConfig => draw_replay_config(frame, app),
        Popup::TopologyView => draw_topology(frame, app),
        Popup::BenchmarkConfig => draw_benchmark_config(frame, app),
        Popup::BenchmarkRunning => draw_benchmark_running(frame, app),
        Popup::ConfirmReroute { ref exchange, ref routing_key, count } => {
            let msg = format!(
                "Re-route {} message(s) to:\n\n  Exchange:    {}\n  Routing Key: {}\n\nThis will publish to the original exchange.\nThe messages remain in the current queue.",
                count,
                if exchange.is_empty() { "(default)" } else { exchange },
                if routing_key.is_empty() { "(empty)" } else { routing_key },
            );
            draw_confirm(frame, app, "DLQ Re-route", &msg);
        }
        Popup::None => {}
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let v = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ]).split(area);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ]).split(v[1])[1]
}

fn draw_help(frame: &mut Frame, app: &App) {
    let popup_area = centered_rect(60, 70, frame.area());
    frame.render_widget(Clear, popup_area);

    let block = Block::bordered()
        .title(" Keyboard Shortcuts ")
        .title_style(Style::default().fg(app.theme.accent).bold())
        .border_style(Style::default().fg(app.theme.accent))
        .style(Style::default().bg(app.theme.bg));

    let shortcuts = vec![
        ("j/k ↑/↓", "Navigate lists"),
        ("enter", "Select / open detail"),
        ("/", "Filter"),
        ("r", "Reload"),
        ("v", "Switch vhost/namespace"),
        ("p", "Switch profile"),
        ("+/-", "Adjust fetch count"),
        ("c", "Copy payload (detail)"),
        ("h", "Copy headers (detail)"),
        ("P", "Publish message"),
        ("x", "Purge queue"),
        ("D", "Delete queue"),
        ("i", "Queue info (stats, config)"),
        ("G", "Consumer groups (Kafka)"),
        ("R", "Reset group offsets (Kafka)"),
        ("=", "Compare two queues"),
        ("d", "Diff two selected messages"),
        ("b", "Toggle base64/gzip decode"),
        ("B", "Load saved filter"),
        ("Ctrl+B", "Save current filter"),
        ("Ctrl+T", "Load message template"),
        ("Ctrl+W", "Save as template"),
        ("X", "Topology view (exchanges)"),
        ("Y", "Replay messages (Kafka)"),
        ("F5", "Benchmark / load test"),
        ("C", "Copy messages to queue"),
        ("m", "Move messages to queue"),
        ("spc", "Select message (list)"),
        ("a", "Select/deselect all"),
        ("e", "Export selected to JSON"),
        ("R", "Re-publish selected"),
        ("W", "Dump queue to JSONL"),
        ("I", "Import from JSONL/JSON"),
        ("L", "DLQ re-route (x-death)"),
        ("T", "Toggle tail / auto-refresh"),
        ("r", "Refresh messages"),
        ("Ctrl+S", "Schedule message (publish)"),
        ("S", "View scheduled messages"),
        ("E", "Edit & re-publish (detail)"),
        ("esc", "Go back"),
        ("?", "Toggle help"),
        ("q", "Quit"),
    ];

    let mut lines: Vec<Line> = vec![Line::from("")];
    for (key, desc) in &shortcuts {
        lines.push(Line::from(vec![
            Span::styled(format!("  {:<14}", key), Style::default().fg(app.theme.accent).bold()),
            Span::styled(*desc, Style::default().fg(app.theme.primary)),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("  Press ? or Esc to close", Style::default().fg(app.theme.muted))));

    frame.render_widget(
        Paragraph::new(lines).block(block).style(Style::default().bg(app.theme.bg)),
        popup_area,
    );
}

fn draw_profile_switch(frame: &mut Frame, app: &mut App) {
    let popup_area = centered_rect(50, 50, frame.area());
    frame.render_widget(Clear, popup_area);

    let block = Block::bordered()
        .title(" Switch Profile ")
        .title_style(Style::default().fg(app.theme.accent).bold())
        .border_style(Style::default().fg(app.theme.accent))
        .style(Style::default().bg(app.theme.bg));

    let names = app.config.profile_names();
    let items: Vec<ListItem> = names.iter().map(|name| {
        let is_current = name == &app.profile_name;
        let display = if is_current { format!("* {}", name) } else { format!("  {}", name) };
        let style = if is_current { Style::default().fg(app.theme.accent) } else { Style::default().fg(app.theme.primary) };
        ListItem::new(Line::from(Span::styled(display, style)))
    }).collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(app.theme.selected_bg).fg(app.theme.white).add_modifier(Modifier::BOLD))
        .highlight_symbol("▸ ")
        .style(Style::default().bg(app.theme.bg));

    frame.render_stateful_widget(list, popup_area, &mut app.popup_list_state);
}

fn draw_namespace_picker(frame: &mut Frame, app: &mut App) {
    let popup_area = centered_rect(40, 50, frame.area());
    frame.render_widget(Clear, popup_area);

    let block = Block::bordered()
        .title(" Select Namespace ")
        .title_style(Style::default().fg(app.theme.accent).bold())
        .border_style(Style::default().fg(app.theme.accent))
        .style(Style::default().bg(app.theme.bg));

    let items: Vec<ListItem> = app.namespaces.iter().map(|ns| {
        let is_current = ns == &app.selected_namespace;
        let display = if is_current { format!("* {}", ns) } else { format!("  {}", ns) };
        let style = if is_current { Style::default().fg(app.theme.accent) } else { Style::default().fg(app.theme.primary) };
        ListItem::new(Line::from(Span::styled(display, style)))
    }).collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(app.theme.selected_bg).fg(app.theme.white).add_modifier(Modifier::BOLD))
        .highlight_symbol("▸ ")
        .style(Style::default().bg(app.theme.bg));

    frame.render_stateful_widget(list, popup_area, &mut app.popup_list_state);
}

fn draw_fetch_count(frame: &mut Frame, app: &mut App) {
    use crate::app::FETCH_PRESETS;

    let popup_area = centered_rect(30, 40, frame.area());
    frame.render_widget(Clear, popup_area);

    let block = Block::bordered()
        .title(" Fetch Count ")
        .title_style(Style::default().fg(app.theme.accent).bold())
        .border_style(Style::default().fg(app.theme.accent))
        .style(Style::default().bg(app.theme.bg));

    let items: Vec<ListItem> = FETCH_PRESETS.iter().map(|&count| {
        let is_current = count == app.fetch_count;
        let label = if is_current {
            format!("* {} messages", count)
        } else {
            format!("  {} messages", count)
        };
        let st = if is_current {
            Style::default().fg(app.theme.accent).bold()
        } else {
            Style::default().fg(app.theme.primary)
        };
        ListItem::new(Line::from(Span::styled(label, st)))
    }).collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default().bg(app.theme.selected_bg).fg(app.theme.white).add_modifier(Modifier::BOLD)
        )
        .highlight_symbol("▸ ");

    frame.render_stateful_widget(list, popup_area, &mut app.popup_list_state);
}

fn draw_theme_picker(frame: &mut Frame, app: &mut App) {
    use crate::ui::theme::{theme_names, THEMES};

    let popup_area = centered_rect(35, 40, frame.area());
    frame.render_widget(Clear, popup_area);

    let block = Block::bordered()
        .title(" Select Theme ")
        .title_style(Style::default().fg(app.theme.accent).bold())
        .border_style(Style::default().fg(app.theme.accent))
        .style(Style::default().bg(app.theme.bg));

    let names = theme_names();
    let items: Vec<ListItem> = names.iter().enumerate().map(|(i, &name)| {
        let t = &THEMES[i];
        let is_current = name == app.theme.name;
        let swatch = Line::from(vec![
            Span::styled(if is_current { "* " } else { "  " },
                Style::default().fg(if is_current { app.theme.accent } else { app.theme.primary })),
            Span::styled(format!("{:<14}", name),
                Style::default().fg(if is_current { app.theme.accent } else { app.theme.primary }).bold()),
            Span::styled("■", Style::default().fg(t.accent)),
            Span::styled("■", Style::default().fg(t.success)),
            Span::styled("■", Style::default().fg(t.error)),
            Span::styled("■", Style::default().fg(t.selected_bg)),
        ]);
        ListItem::new(swatch)
    }).collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default().bg(app.theme.selected_bg).fg(app.theme.white).add_modifier(Modifier::BOLD)
        )
        .highlight_symbol("▸ ");

    frame.render_stateful_widget(list, popup_area, &mut app.popup_list_state);
}

fn draw_backend_type_picker(frame: &mut Frame, app: &mut App) {
    use crate::app::BACKEND_TYPES;

    let popup_area = centered_rect(40, 30, frame.area());
    frame.render_widget(Clear, popup_area);

    let block = Block::bordered()
        .title(" Backend Type ")
        .title_style(Style::default().fg(app.theme.accent).bold())
        .border_style(Style::default().fg(app.theme.accent))
        .style(Style::default().bg(app.theme.bg));

    let descriptions = ["RabbitMQ — AMQP broker", "Kafka — Event streaming", "MQTT — IoT messaging"];

    let items: Vec<ListItem> = BACKEND_TYPES.iter().enumerate().map(|(i, &name)| {
        let is_current = name == app.profile_form.profile_type;
        let desc = descriptions.get(i).unwrap_or(&"");
        let label = if is_current {
            format!("* {:<10} {}", name, desc)
        } else {
            format!("  {:<10} {}", name, desc)
        };
        let st = if is_current {
            Style::default().fg(app.theme.accent).bold()
        } else {
            Style::default().fg(app.theme.primary)
        };
        ListItem::new(Line::from(Span::styled(label, st)))
    }).collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default().bg(app.theme.selected_bg).fg(app.theme.white).add_modifier(Modifier::BOLD)
        )
        .highlight_symbol("▸ ");

    frame.render_stateful_widget(list, popup_area, &mut app.popup_list_state);
}

fn draw_publish(frame: &mut Frame, app: &mut App, title: &str) {
    let popup_area = centered_rect(60, 70, frame.area());
    frame.render_widget(Clear, popup_area);

    let block = Block::bordered()
        .title(title)
        .title_style(Style::default().fg(app.theme.accent).bold())
        .border_style(Style::default().fg(app.theme.accent))
        .style(Style::default().bg(app.theme.bg));
    frame.render_widget(block, popup_area);

    let inner = Rect::new(popup_area.x + 2, popup_area.y + 1, popup_area.width.saturating_sub(4), popup_area.height.saturating_sub(2));
    let form = &app.publish_form;

    // Routing key field
    let fields = [
        (0, "Routing Key", &form.routing_key),
        (1, "Content Type", &form.content_type),
    ];

    let mut y = inner.y;
    for (idx, label, value) in &fields {
        let is_focused = form.focused_field == *idx;
        let label_style = if is_focused { Style::default().fg(app.theme.accent) } else { Style::default().fg(app.theme.muted) };
        let value_style = if is_focused { Style::default().fg(app.theme.white) } else { Style::default().fg(app.theme.primary) };

        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(format!("{}:", label), label_style)))
                .style(Style::default().bg(app.theme.bg)),
            Rect::new(inner.x, y, inner.width, 1),
        );
        let display = if is_focused { format!("{}_", value) } else { value.to_string() };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(display, value_style)))
                .style(Style::default().bg(app.theme.bg)),
            Rect::new(inner.x + 2, y + 1, inner.width.saturating_sub(2), 1),
        );
        y += 2;
    }

    // Body field (multi-line, takes remaining space)
    let is_body_focused = form.focused_field == 2;
    let body_label_style = if is_body_focused { Style::default().fg(app.theme.accent) } else { Style::default().fg(app.theme.muted) };
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled("Body:", body_label_style)))
            .style(Style::default().bg(app.theme.bg)),
        Rect::new(inner.x, y, inner.width, 1),
    );
    y += 1;

    let body_height = inner.height.saturating_sub(y - inner.y + 3); // leave room for footer
    let body_area = Rect::new(inner.x + 1, y, inner.width.saturating_sub(1), body_height);
    let body_style = if is_body_focused { Style::default().fg(app.theme.white) } else { Style::default().fg(app.theme.primary) };
    let body_text = if is_body_focused { format!("{}_", form.body) } else { form.body.clone() };
    let body_block = Block::default()
        .borders(Borders::ALL)
        .border_style(if is_body_focused { Style::default().fg(app.theme.accent) } else { Style::default().fg(app.theme.divider) })
        .style(Style::default().bg(app.theme.bg));
    frame.render_widget(
        Paragraph::new(body_text).style(body_style.bg(app.theme.bg)).block(body_block),
        body_area,
    );

    // Error
    if !form.error.is_empty() {
        let err_y = inner.y + inner.height.saturating_sub(2);
        frame.render_widget(
            Paragraph::new(Span::styled(&form.error, Style::default().fg(app.theme.error)))
                .style(Style::default().bg(app.theme.bg)),
            Rect::new(inner.x, err_y, inner.width, 1),
        );
    }

    // Footer
    let footer_y = inner.y + inner.height.saturating_sub(1);
    let ks = Style::default().fg(app.theme.accent).bold();
    let ds = Style::default().fg(app.theme.muted);
    let hint = if form.focused_field == 2 { "⏎:newline  ctrl+⏎:send" } else { "⏎:send" };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("tab", ks), Span::styled(":next ", ds),
            Span::styled(hint, Style::default().fg(app.theme.accent)),
            Span::styled("  ", ds),
            Span::styled("ctrl+s", ks), Span::styled(":schedule ", ds),
            Span::styled("ctrl+t", ks), Span::styled(":template ", ds),
            Span::styled("ctrl+w", ks), Span::styled(":save tpl ", ds),
            Span::styled("esc", ks), Span::styled(":cancel", ds),
        ])).style(Style::default().bg(app.theme.bg)),
        Rect::new(inner.x, footer_y, inner.width, 1),
    );
}

fn draw_confirm(frame: &mut Frame, app: &App, title: &str, message: &str) {
    let popup_area = centered_rect(40, 20, frame.area());
    frame.render_widget(Clear, popup_area);

    let queue_name = app.selected_queue().map(|q| q.name.as_str()).unwrap_or("?");
    let full_msg = format!("{}\n\nQueue: {}", message, queue_name);

    let block = Block::bordered()
        .title(format!(" {} ", title))
        .title_style(Style::default().fg(app.theme.error).bold())
        .border_style(Style::default().fg(app.theme.error))
        .style(Style::default().bg(app.theme.bg));

    let ks = Style::default().fg(app.theme.accent).bold();
    let ds = Style::default().fg(app.theme.muted);

    let mut lines: Vec<Line> = vec![Line::from("")];
    for l in full_msg.lines() {
        lines.push(Line::from(Span::styled(format!("  {}", l), Style::default().fg(app.theme.primary))));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  ", ds),
        Span::styled("y", ks), Span::styled(":confirm  ", ds),
        Span::styled("esc", ks), Span::styled(":cancel", ds),
    ]));

    frame.render_widget(
        Paragraph::new(lines).block(block).style(Style::default().bg(app.theme.bg)),
        popup_area,
    );
}

fn draw_queue_picker(frame: &mut Frame, app: &mut App) {
    let popup_area = centered_rect(50, 60, frame.area());
    frame.render_widget(Clear, popup_area);

    let op_label = match &app.popup {
        Popup::QueuePicker(QueueOperation::Copy) => "Copy",
        Popup::QueuePicker(QueueOperation::Move) => "Move",
        _ => "Select",
    };

    let source = app.selected_queue().map(|q| q.name.clone()).unwrap_or_default();
    let title = format!(" {} from '{}' to: ", op_label, source);

    let block = Block::bordered()
        .title(title)
        .title_style(Style::default().fg(app.theme.accent).bold())
        .border_style(Style::default().fg(app.theme.accent))
        .style(Style::default().bg(app.theme.bg));

    let filter = app.queue_picker_filter.to_lowercase();
    let filtered: Vec<&str> = app.queues.iter()
        .map(|q| q.name.as_str())
        .filter(|name| filter.is_empty() || name.to_lowercase().contains(&filter))
        .collect();

    let items: Vec<ListItem> = filtered.iter().map(|&name| {
        let is_source = name == source;
        let st = if is_source {
            Style::default().fg(app.theme.muted)
        } else {
            Style::default().fg(app.theme.primary)
        };
        let label = if is_source { format!("  {} (source)", name) } else { format!("  {}", name) };
        ListItem::new(Line::from(Span::styled(label, st)))
    }).collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(app.theme.selected_bg).fg(app.theme.white).add_modifier(Modifier::BOLD))
        .highlight_symbol("▸ ")
        .style(Style::default().bg(app.theme.bg));

    frame.render_stateful_widget(list, popup_area, &mut app.popup_list_state);

    // Filter bar at bottom
    if app.queue_picker_filter_active || !app.queue_picker_filter.is_empty() {
        let filter_y = popup_area.y + popup_area.height.saturating_sub(2);
        let filter_text = if app.queue_picker_filter_active {
            format!("/{}_", app.queue_picker_filter)
        } else {
            format!("/{}", app.queue_picker_filter)
        };
        frame.render_widget(
            Paragraph::new(Span::styled(filter_text, Style::default().fg(app.theme.accent)))
                .style(Style::default().bg(app.theme.bg)),
            Rect::new(popup_area.x + 2, filter_y, popup_area.width.saturating_sub(4), 1),
        );
    }
}

fn draw_import_file(frame: &mut Frame, app: &App) {
    let popup_area = centered_rect(60, 25, frame.area());
    frame.render_widget(Clear, popup_area);

    let block = Block::bordered()
        .title(" Import Messages ")
        .title_style(Style::default().fg(app.theme.accent).bold())
        .border_style(Style::default().fg(app.theme.accent))
        .style(Style::default().bg(app.theme.bg));

    let inner = Rect::new(popup_area.x + 2, popup_area.y + 1, popup_area.width.saturating_sub(4), popup_area.height.saturating_sub(2));

    frame.render_widget(block, popup_area);

    let label_style = Style::default().fg(app.theme.accent);
    let value_style = Style::default().fg(app.theme.white);
    let hint_style = Style::default().fg(app.theme.muted);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled("  File path (JSONL or JSON):", label_style)),
        Line::from(""),
        Line::from(Span::styled(format!("  {}█", app.import_file_path), value_style)),
        Line::from(""),
        Line::from(Span::styled("  Supports .jsonl (dump) and .json (export) formats", hint_style)),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ⏎", Style::default().fg(app.theme.accent).bold()),
            Span::styled(":import  ", hint_style),
            Span::styled("esc", Style::default().fg(app.theme.accent).bold()),
            Span::styled(":cancel", hint_style),
        ]),
    ];

    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(app.theme.bg)),
        inner,
    );
}

fn draw_operation_progress(frame: &mut Frame, app: &App) {
    let popup_area = centered_rect(40, 20, frame.area());
    frame.render_widget(Clear, popup_area);

    let (completed, total) = app.operation_progress;

    let block = Block::bordered()
        .title(" Operation in Progress ")
        .title_style(Style::default().fg(app.theme.accent).bold())
        .border_style(Style::default().fg(app.theme.accent))
        .style(Style::default().bg(app.theme.bg));

    let pct = if total > 0 { (completed as f64 / total as f64 * 100.0) as u16 } else { 0 };
    let bar_width = popup_area.width.saturating_sub(8) as usize;
    let filled = (bar_width * pct as usize) / 100;
    let bar: String = format!("[{}{}]", "█".repeat(filled), "░".repeat(bar_width - filled));

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}/{} messages", completed, total),
            Style::default().fg(app.theme.primary),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", bar),
            Style::default().fg(app.theme.accent),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  esc:cancel",
            Style::default().fg(app.theme.muted),
        )),
    ];

    frame.render_widget(
        Paragraph::new(lines).block(block).style(Style::default().bg(app.theme.bg)),
        popup_area,
    );
}

fn draw_queue_info(frame: &mut Frame, app: &App) {
    let popup_area = centered_rect(70, 75, frame.area());
    frame.render_widget(Clear, popup_area);

    let title = format!(" Queue Info: {} ", app.queue_info_name);
    let block = Block::bordered()
        .title(title.as_str())
        .title_style(Style::default().fg(app.theme.accent).bold())
        .border_style(Style::default().fg(app.theme.accent))
        .style(Style::default().bg(app.theme.bg));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    if app.queue_detail.is_empty() {
        let loading = Paragraph::new(Line::from(Span::styled(
            "  Loading...",
            Style::default().fg(app.theme.muted),
        ))).style(Style::default().bg(app.theme.bg));
        frame.render_widget(loading, inner);
        return;
    }

    // Build all lines from sections
    let mut lines: Vec<Line> = Vec::new();
    let key_style = Style::default().fg(app.theme.muted);
    let value_style = Style::default().fg(app.theme.primary);
    let section_style = Style::default().fg(app.theme.accent).bold();
    let bar_filled_style = Style::default().fg(app.theme.accent);
    let bar_empty_style = Style::default().fg(app.theme.divider);

    // Find max rate across all sections for scaling bars
    let max_rate = app.queue_detail.iter()
        .flat_map(|s| s.entries.iter())
        .filter_map(|e| e.rate_value)
        .fold(0.0f64, f64::max);

    for section in &app.queue_detail {
        if !lines.is_empty() {
            lines.push(Line::from(""));
        }

        // Section header with decorative line
        let header_text = format!(" {} ", section.title);
        let remaining = inner.width.saturating_sub(header_text.len() as u16 + 4) as usize;
        let separator = "─".repeat(remaining);
        lines.push(Line::from(vec![
            Span::styled("  ── ", Style::default().fg(app.theme.divider)),
            Span::styled(header_text, section_style),
            Span::styled(separator, Style::default().fg(app.theme.divider)),
        ]));

        for entry in &section.entries {
            let mut spans = vec![
                Span::styled(format!("  {:<18} ", entry.key), key_style),
                Span::styled(&entry.value, value_style),
            ];

            // Rate bar
            if let Some(rate) = entry.rate_value {
                if max_rate > 0.0 {
                    let bar_width: usize = 12;
                    let filled = ((rate / max_rate) * bar_width as f64).round() as usize;
                    let empty = bar_width.saturating_sub(filled);
                    spans.push(Span::styled("  ", Style::default()));
                    spans.push(Span::styled("█".repeat(filled), bar_filled_style));
                    spans.push(Span::styled("░".repeat(empty), bar_empty_style));
                }
            }

            lines.push(Line::from(spans));
        }
    }

    // Footer
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  j/k", Style::default().fg(app.theme.accent).bold()),
        Span::styled(":scroll  ", Style::default().fg(app.theme.muted)),
        Span::styled("esc", Style::default().fg(app.theme.accent).bold()),
        Span::styled(":close", Style::default().fg(app.theme.muted)),
    ]));

    // Apply scroll
    let scroll = app.queue_info_scroll;
    let content = Paragraph::new(lines)
        .style(Style::default().bg(app.theme.bg))
        .scroll((scroll, 0));
    frame.render_widget(content, inner);
}

fn draw_consumer_groups(frame: &mut Frame, app: &App) {
    let popup_area = centered_rect(70, 75, frame.area());
    frame.render_widget(Clear, popup_area);

    let block = Block::bordered()
        .title(" Consumer Groups ")
        .title_style(Style::default().fg(app.theme.accent).bold())
        .border_style(Style::default().fg(app.theme.accent))
        .style(Style::default().bg(app.theme.bg));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    if app.consumer_groups.is_empty() && app.loading {
        let loading = Paragraph::new(Line::from(Span::styled(
            "  Loading...",
            Style::default().fg(app.theme.muted),
        ))).style(Style::default().bg(app.theme.bg));
        frame.render_widget(loading, inner);
        return;
    }

    if app.consumer_groups.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            "  No consumer groups found for this topic",
            Style::default().fg(app.theme.muted),
        ))).style(Style::default().bg(app.theme.bg));
        frame.render_widget(empty, inner);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    let name_style = Style::default().fg(app.theme.accent).bold();
    let key_style = Style::default().fg(app.theme.muted);
    let value_style = Style::default().fg(app.theme.primary);
    let lag_warn_style = Style::default().fg(app.theme.error).bold();
    let lag_ok_style = Style::default().fg(app.theme.success);

    let selected = app.consumer_groups_selected;

    for (gi, group) in app.consumer_groups.iter().enumerate() {
        if !lines.is_empty() {
            lines.push(Line::from(""));
        }

        let is_selected = selected == Some(gi);

        // Group header
        let state_style = if group.state == "Stable" {
            Style::default().fg(app.theme.success)
        } else {
            Style::default().fg(app.theme.muted)
        };

        let indicator = if is_selected { "▸ " } else { "  " };
        let group_name_style = if is_selected {
            Style::default().fg(app.theme.accent).bold().bg(app.theme.selected_bg)
        } else {
            name_style
        };

        lines.push(Line::from(vec![
            Span::styled(indicator, group_name_style),
            Span::styled(format!("{} ", group.name), group_name_style),
            Span::styled(format!("({})", group.state), state_style),
        ]));

        lines.push(Line::from(vec![
            Span::styled("    Members: ", key_style),
            Span::styled(group.members.to_string(), value_style),
            Span::styled("  Total lag: ", key_style),
            Span::styled(
                group.total_lag.to_string(),
                if group.total_lag > 0 { lag_warn_style } else { lag_ok_style },
            ),
        ]));

        // Per-partition details
        if !group.partitions.is_empty() {
            lines.push(Line::from(Span::styled(
                "    Partition   Offset        High WM       Lag",
                key_style,
            )));

            for p in &group.partitions {
                let lag_style = if p.lag > 0 { lag_warn_style } else { lag_ok_style };
                lines.push(Line::from(vec![
                    Span::styled(format!("    P{:<10}", p.partition), value_style),
                    Span::styled(format!("{:<14}", p.current_offset), value_style),
                    Span::styled(format!("{:<14}", p.high_watermark), value_style),
                    Span::styled(format!("{}", p.lag), lag_style),
                ]));
            }
        }
    }

    // Footer
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  j/k", Style::default().fg(app.theme.accent).bold()),
        Span::styled(":navigate  ", Style::default().fg(app.theme.muted)),
        Span::styled("R", Style::default().fg(app.theme.accent).bold()),
        Span::styled(":reset offsets  ", Style::default().fg(app.theme.muted)),
        Span::styled("esc", Style::default().fg(app.theme.accent).bold()),
        Span::styled(":close", Style::default().fg(app.theme.muted)),
    ]));

    // Auto-scroll to keep selected group visible
    let scroll = if let Some(sel) = selected {
        // Estimate line position of selected group (roughly 4+ lines per group)
        let estimated_line = app.consumer_groups.iter().take(sel)
            .map(|g| 3 + g.partitions.len() + 1) // header + members + header_row + partitions + blank
            .sum::<usize>();
        let visible_height = inner.height as usize;
        if estimated_line >= visible_height {
            (estimated_line - visible_height / 2) as u16
        } else {
            0
        }
    } else {
        0
    };

    let content = Paragraph::new(lines)
        .style(Style::default().bg(app.theme.bg))
        .scroll((scroll, 0));
    frame.render_widget(content, inner);
}

fn draw_reset_offset_picker(frame: &mut Frame, app: &mut App) {
    let popup_area = centered_rect(40, 35, frame.area());
    frame.render_widget(Clear, popup_area);

    let title = format!(" Reset Offsets: {} ", app.reset_group_name);
    let block = Block::bordered()
        .title(title)
        .title_style(Style::default().fg(app.theme.accent).bold())
        .border_style(Style::default().fg(app.theme.accent))
        .style(Style::default().bg(app.theme.bg));

    let strategies = ["Earliest (beginning)", "Latest (end)", "To Timestamp (unix ms)", "To Offset (specific)"];
    let items: Vec<ListItem> = strategies.iter().map(|&s| {
        ListItem::new(Line::from(Span::styled(
            format!("  {}", s),
            Style::default().fg(app.theme.primary),
        )))
    }).collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(app.theme.selected_bg).fg(app.theme.white).add_modifier(Modifier::BOLD))
        .highlight_symbol("▸ ")
        .style(Style::default().bg(app.theme.bg));

    frame.render_stateful_widget(list, popup_area, &mut app.popup_list_state);
}

fn draw_reset_offset_input(frame: &mut Frame, app: &App) {
    let popup_area = centered_rect(45, 20, frame.area());
    frame.render_widget(Clear, popup_area);

    let picker_sel = app.popup_list_state.selected().unwrap_or(2);
    let label = if picker_sel == 2 { "Timestamp (unix millis)" } else { "Offset" };
    let title = format!(" Enter {} ", label);

    let block = Block::bordered()
        .title(title)
        .title_style(Style::default().fg(app.theme.accent).bold())
        .border_style(Style::default().fg(app.theme.accent))
        .style(Style::default().bg(app.theme.bg));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Value: ", Style::default().fg(app.theme.muted)),
            Span::styled(&app.reset_input, Style::default().fg(app.theme.primary).bold()),
            Span::styled("█", Style::default().fg(app.theme.accent)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  enter", Style::default().fg(app.theme.accent).bold()),
            Span::styled(":confirm  ", Style::default().fg(app.theme.muted)),
            Span::styled("esc", Style::default().fg(app.theme.accent).bold()),
            Span::styled(":back", Style::default().fg(app.theme.muted)),
        ]),
    ];

    let content = Paragraph::new(lines).style(Style::default().bg(app.theme.bg));
    frame.render_widget(content, inner);
}

fn draw_confirm_reset_offset(frame: &mut Frame, app: &App) {
    let strategy_desc = match &app.reset_strategy {
        Some(OffsetResetStrategy::Earliest) => "earliest (beginning)".to_string(),
        Some(OffsetResetStrategy::Latest) => "latest (end)".to_string(),
        Some(OffsetResetStrategy::ToTimestamp(ts)) => format!("timestamp {}", ts),
        Some(OffsetResetStrategy::ToOffset(o)) => format!("offset {}", o),
        None => "unknown".to_string(),
    };
    let msg = format!(
        "Reset offsets for group '{}'?\n\nStrategy: {}\n\nThis will change committed offsets.\nThe group must be inactive (no consumers).",
        app.reset_group_name, strategy_desc,
    );
    draw_confirm(frame, app, "Reset Offsets", &msg);
}

fn draw_schedule_delay(frame: &mut Frame, app: &mut App) {
    use crate::app::SCHEDULE_PRESETS;

    let popup_area = centered_rect(35, 40, frame.area());
    frame.render_widget(Clear, popup_area);

    let block = Block::bordered()
        .title(" Schedule Delay ")
        .title_style(Style::default().fg(app.theme.accent).bold())
        .border_style(Style::default().fg(app.theme.accent))
        .style(Style::default().bg(app.theme.bg));

    let items: Vec<ListItem> = SCHEDULE_PRESETS.iter().map(|(_, label)| {
        ListItem::new(Line::from(Span::styled(
            format!("  {}", label),
            Style::default().fg(app.theme.primary),
        )))
    }).collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(app.theme.selected_bg).fg(app.theme.white).add_modifier(Modifier::BOLD))
        .highlight_symbol("▸ ")
        .style(Style::default().bg(app.theme.bg));

    frame.render_stateful_widget(list, popup_area, &mut app.popup_list_state);
}

fn draw_scheduled_messages(frame: &mut Frame, app: &mut App) {
    let popup_area = centered_rect(60, 60, frame.area());
    frame.render_widget(Clear, popup_area);

    let count = app.scheduled_messages.len();
    let title = format!(" Scheduled Messages ({}) ", count);
    let block = Block::bordered()
        .title(title)
        .title_style(Style::default().fg(app.theme.accent).bold())
        .border_style(Style::default().fg(app.theme.accent))
        .style(Style::default().bg(app.theme.bg));

    if app.scheduled_messages.is_empty() {
        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);
        let empty = Paragraph::new(Line::from(Span::styled(
            "  No scheduled messages",
            Style::default().fg(app.theme.muted),
        ))).style(Style::default().bg(app.theme.bg));
        frame.render_widget(empty, inner);
        return;
    }

    let now = std::time::Instant::now();
    let items: Vec<ListItem> = app.scheduled_messages.iter().map(|msg| {
        let remaining = if msg.publish_at > now {
            let secs = (msg.publish_at - now).as_secs();
            if secs >= 3600 {
                format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
            } else if secs >= 60 {
                format!("{}m {}s", secs / 60, secs % 60)
            } else {
                format!("{}s", secs)
            }
        } else {
            "publishing...".to_string()
        };

        let body_preview: String = msg.body.chars().take(40).collect();
        let body_preview = body_preview.replace('\n', " ");

        ListItem::new(Line::from(vec![
            Span::styled(format!("  {} ", remaining), Style::default().fg(app.theme.success).bold()),
            Span::styled(format!("→ {} ", msg.queue), Style::default().fg(app.theme.accent)),
            Span::styled(body_preview, Style::default().fg(app.theme.muted)),
        ]))
    }).collect();

    // Footer inside the block
    let footer_line = ListItem::new(Line::from(vec![
        Span::styled("  d", Style::default().fg(app.theme.accent).bold()),
        Span::styled(":cancel  ", Style::default().fg(app.theme.muted)),
        Span::styled("esc", Style::default().fg(app.theme.accent).bold()),
        Span::styled(":close", Style::default().fg(app.theme.muted)),
    ]));

    let mut all_items = items;
    all_items.push(ListItem::new(Line::from("")));
    all_items.push(footer_line);

    let list = List::new(all_items)
        .block(block)
        .highlight_style(Style::default().bg(app.theme.selected_bg).fg(app.theme.white).add_modifier(Modifier::BOLD))
        .highlight_symbol("▸ ")
        .style(Style::default().bg(app.theme.bg));

    frame.render_stateful_widget(list, popup_area, &mut app.scheduled_list_state);
}

fn draw_compare_queue_picker(frame: &mut Frame, app: &mut App) {
    let popup_area = centered_rect(50, 60, frame.area());
    frame.render_widget(Clear, popup_area);

    let title = format!(" Compare: {} vs ... ", app.compare_queue_a);
    let block = Block::bordered()
        .title(title)
        .title_style(Style::default().fg(app.theme.accent).bold())
        .border_style(Style::default().fg(app.theme.accent))
        .style(Style::default().bg(app.theme.bg));

    let filtered: Vec<&crate::backend::QueueInfo> = app.queues.iter()
        .filter(|q| {
            q.name != app.compare_queue_a && (
                app.queue_picker_filter.is_empty()
                || q.name.to_lowercase().contains(&app.queue_picker_filter.to_lowercase())
            )
        })
        .collect();

    let mut items: Vec<ListItem> = Vec::new();

    // Filter bar
    if app.queue_picker_filter_active || !app.queue_picker_filter.is_empty() {
        items.push(ListItem::new(Line::from(vec![
            Span::styled("  /", Style::default().fg(app.theme.accent)),
            Span::styled(&app.queue_picker_filter, Style::default().fg(app.theme.primary)),
            if app.queue_picker_filter_active {
                Span::styled("█", Style::default().fg(app.theme.accent))
            } else {
                Span::styled("", Style::default())
            },
        ])));
        items.push(ListItem::new(Line::from("")));
    }

    for q in &filtered {
        items.push(ListItem::new(Line::from(vec![
            Span::styled(format!("  {} ", q.name), Style::default().fg(app.theme.primary)),
            Span::styled(format!("({} msgs)", q.messages), Style::default().fg(app.theme.muted)),
        ])));
    }

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(app.theme.selected_bg).fg(app.theme.white).add_modifier(Modifier::BOLD))
        .highlight_symbol("▸ ")
        .style(Style::default().bg(app.theme.bg));

    frame.render_stateful_widget(list, popup_area, &mut app.popup_list_state);
}

fn draw_compare_results(frame: &mut Frame, app: &App) {
    let popup_area = centered_rect(80, 80, frame.area());
    frame.render_widget(Clear, popup_area);

    let result = match &app.comparison_result {
        Some(r) => r,
        None => return,
    };

    let title = format!(" Compare: {} vs {} ", result.queue_a, result.queue_b);
    let block = Block::bordered()
        .title(title)
        .title_style(Style::default().fg(app.theme.accent).bold())
        .border_style(Style::default().fg(app.theme.accent))
        .style(Style::default().bg(app.theme.bg));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let ks = Style::default().fg(app.theme.accent).bold();
    let ds = Style::default().fg(app.theme.muted);
    let active_tab = Style::default().fg(app.theme.accent).bold().bg(app.theme.selected_bg);
    let inactive_tab = Style::default().fg(app.theme.muted);

    // Tab bar
    let tab_style = |tab: &crate::app::ComparisonTab| {
        if *tab == app.comparison_tab { active_tab } else { inactive_tab }
    };

    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(vec![
        Span::styled(" [Summary]", tab_style(&crate::app::ComparisonTab::Summary)),
        Span::styled(" ", ds),
        Span::styled(
            format!("[Only in A ({})]", result.only_in_a.len()),
            tab_style(&crate::app::ComparisonTab::OnlyInA),
        ),
        Span::styled(" ", ds),
        Span::styled(
            format!("[Only in B ({})]", result.only_in_b.len()),
            tab_style(&crate::app::ComparisonTab::OnlyInB),
        ),
    ]));

    lines.push(Line::from(""));

    match app.comparison_tab {
        crate::app::ComparisonTab::Summary => {
            lines.push(Line::from(vec![
                Span::styled("  Queue A: ", ds),
                Span::styled(&result.queue_a, Style::default().fg(app.theme.primary).bold()),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Queue B: ", ds),
                Span::styled(&result.queue_b, Style::default().fg(app.theme.primary).bold()),
            ]));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("  In both:    ", ds),
                Span::styled(
                    result.in_both.to_string(),
                    Style::default().fg(app.theme.success).bold(),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Only in A:  ", ds),
                Span::styled(
                    result.only_in_a.len().to_string(),
                    if result.only_in_a.is_empty() {
                        Style::default().fg(app.theme.muted)
                    } else {
                        Style::default().fg(app.theme.error).bold()
                    },
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Only in B:  ", ds),
                Span::styled(
                    result.only_in_b.len().to_string(),
                    if result.only_in_b.is_empty() {
                        Style::default().fg(app.theme.muted)
                    } else {
                        Style::default().fg(app.theme.error).bold()
                    },
                ),
            ]));

            if result.only_in_a.is_empty() && result.only_in_b.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  Queues are identical (by message body)",
                    Style::default().fg(app.theme.success).bold(),
                )));
            }
        }
        crate::app::ComparisonTab::OnlyInA => {
            if result.only_in_a.is_empty() {
                lines.push(Line::from(Span::styled(
                    "  No unique messages in this queue",
                    ds,
                )));
            } else {
                for (i, msg) in result.only_in_a.iter().enumerate() {
                    let body_preview: String = msg.body.chars().take(80).collect();
                    let body_preview = body_preview.replace('\n', " ");
                    lines.push(Line::from(vec![
                        Span::styled(format!("  {}. ", i + 1), Style::default().fg(app.theme.muted)),
                        Span::styled(&msg.routing_key, Style::default().fg(app.theme.accent)),
                        Span::styled(" ", ds),
                        Span::styled(body_preview, Style::default().fg(app.theme.primary)),
                    ]));
                }
            }
        }
        crate::app::ComparisonTab::OnlyInB => {
            if result.only_in_b.is_empty() {
                lines.push(Line::from(Span::styled(
                    "  No unique messages in this queue",
                    ds,
                )));
            } else {
                for (i, msg) in result.only_in_b.iter().enumerate() {
                    let body_preview: String = msg.body.chars().take(80).collect();
                    let body_preview = body_preview.replace('\n', " ");
                    lines.push(Line::from(vec![
                        Span::styled(format!("  {}. ", i + 1), Style::default().fg(app.theme.muted)),
                        Span::styled(&msg.routing_key, Style::default().fg(app.theme.accent)),
                        Span::styled(" ", ds),
                        Span::styled(body_preview, Style::default().fg(app.theme.primary)),
                    ]));
                }
            }
        }
    }

    // Footer
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  tab", ks),
        Span::styled(":switch tab  ", ds),
        Span::styled("j/k", ks),
        Span::styled(":scroll  ", ds),
        Span::styled("esc", ks),
        Span::styled(":close", ds),
    ]));

    let content = Paragraph::new(lines)
        .style(Style::default().bg(app.theme.bg))
        .scroll((app.comparison_scroll, 0));
    frame.render_widget(content, inner);
}

fn draw_message_diff(frame: &mut Frame, app: &App) {
    use similar::{ChangeTag, TextDiff};

    let popup_area = centered_rect(90, 85, frame.area());
    frame.render_widget(Clear, popup_area);

    let (msg_a, msg_b) = match &app.diff_messages {
        Some((a, b)) => (a, b),
        None => return,
    };

    let block = Block::bordered()
        .title(format!(" Diff: #{} vs #{} ", msg_a.index, msg_b.index))
        .title_style(Style::default().fg(app.theme.accent).bold())
        .border_style(Style::default().fg(app.theme.accent))
        .style(Style::default().bg(app.theme.bg));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let ks = Style::default().fg(app.theme.accent).bold();
    let ds = Style::default().fg(app.theme.muted);

    let mut lines: Vec<Line> = Vec::new();

    // Header comparison
    let header_keys = ["routing_key", "exchange", "content_type"];
    for key in &header_keys {
        let val_a = match *key {
            "routing_key" => &msg_a.routing_key,
            "exchange" => &msg_a.exchange,
            "content_type" => &msg_a.content_type,
            _ => "",
        };
        let val_b = match *key {
            "routing_key" => &msg_b.routing_key,
            "exchange" => &msg_b.exchange,
            "content_type" => &msg_b.content_type,
            _ => "",
        };
        if val_a != val_b {
            lines.push(Line::from(vec![
                Span::styled(format!("  {}: ", key), ds),
                Span::styled(val_a, Style::default().fg(app.theme.error)),
                Span::styled(" → ", ds),
                Span::styled(val_b, Style::default().fg(app.theme.success)),
            ]));
        }
    }

    if !lines.is_empty() {
        lines.push(Line::from(""));
    }

    // Body diff using similar crate
    let body_a = pretty_format_for_diff(&msg_a.body);
    let body_b = pretty_format_for_diff(&msg_b.body);

    let diff = TextDiff::from_lines(&body_a, &body_b);

    // Use half-width columns side by side
    let half_width = (inner.width as usize).saturating_sub(4) / 2;

    lines.push(Line::from(vec![
        Span::styled(format!("  {:<width$}", format!("Message #{}", msg_a.index), width = half_width), ks),
        Span::styled("│ ", ds),
        Span::styled(format!("Message #{}", msg_b.index), ks),
    ]));
    lines.push(Line::from(vec![
        Span::styled(format!("  {}", "─".repeat(half_width)), ds),
        Span::styled("┼", ds),
        Span::styled("─".repeat(half_width), ds),
    ]));

    for change in diff.iter_all_changes() {
        let line_text: String = change.value().trim_end().chars().take(half_width).collect();
        match change.tag() {
            ChangeTag::Equal => {
                lines.push(Line::from(vec![
                    Span::styled(format!("  {:<width$}", line_text, width = half_width), Style::default().fg(app.theme.primary)),
                    Span::styled("│ ", ds),
                    Span::styled(format!("{:<width$}", line_text, width = half_width), Style::default().fg(app.theme.primary)),
                ]));
            }
            ChangeTag::Delete => {
                lines.push(Line::from(vec![
                    Span::styled(format!("- {:<width$}", line_text, width = half_width - 2), Style::default().fg(app.theme.error)),
                    Span::styled("│", ds),
                    Span::styled(" ", Style::default()),
                ]));
            }
            ChangeTag::Insert => {
                lines.push(Line::from(vec![
                    Span::styled(format!("  {:<width$}", "", width = half_width), Style::default()),
                    Span::styled("│ ", ds),
                    Span::styled(format!("+ {}", line_text), Style::default().fg(app.theme.success)),
                ]));
            }
        }
    }

    // Footer
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  j/k", ks),
        Span::styled(":scroll  ", ds),
        Span::styled("esc", ks),
        Span::styled(":close", ds),
    ]));

    let content = Paragraph::new(lines)
        .style(Style::default().bg(app.theme.bg))
        .scroll((app.diff_scroll, 0));
    frame.render_widget(content, inner);
}

/// Pretty-format body for diffing (attempt JSON/XML formatting)
fn pretty_format_for_diff(body: &str) -> String {
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(body) {
        serde_json::to_string_pretty(&val).unwrap_or_else(|_| body.to_string())
    } else {
        body.to_string()
    }
}

fn draw_saved_filters(frame: &mut Frame, app: &mut App) {
    let popup_area = centered_rect(50, 50, frame.area());
    frame.render_widget(Clear, popup_area);

    let queue = &app.current_queue_name;
    let block = Block::bordered()
        .title(format!(" Saved Filters: {} ", queue))
        .title_style(Style::default().fg(app.theme.accent).bold())
        .border_style(Style::default().fg(app.theme.accent))
        .style(Style::default().bg(app.theme.bg));

    let filters = app.config.filters.get(queue).cloned().unwrap_or_default();
    let items: Vec<ListItem> = filters.iter().map(|f| {
        let mode = if f.advanced { "adv" } else { "simple" };
        ListItem::new(Line::from(vec![
            Span::styled(format!("  {} ", f.name), Style::default().fg(app.theme.primary).bold()),
            Span::styled(format!("[{}] ", mode), Style::default().fg(app.theme.muted)),
            Span::styled(&f.expression, Style::default().fg(app.theme.accent)),
        ]))
    }).collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(app.theme.selected_bg).fg(app.theme.white).add_modifier(Modifier::BOLD))
        .highlight_symbol("▸ ")
        .style(Style::default().bg(app.theme.bg));

    frame.render_stateful_widget(list, popup_area, &mut app.saved_filter_list_state);
}

fn draw_save_filter(frame: &mut Frame, app: &App) {
    let popup_area = centered_rect(45, 20, frame.area());
    frame.render_widget(Clear, popup_area);

    let block = Block::bordered()
        .title(" Save Filter ")
        .title_style(Style::default().fg(app.theme.accent).bold())
        .border_style(Style::default().fg(app.theme.accent))
        .style(Style::default().bg(app.theme.bg));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Name: ", Style::default().fg(app.theme.muted)),
            Span::styled(&app.save_filter_name, Style::default().fg(app.theme.primary).bold()),
            Span::styled("█", Style::default().fg(app.theme.accent)),
        ]),
        Line::from(vec![
            Span::styled("  Expr: ", Style::default().fg(app.theme.muted)),
            Span::styled(&app.message_filter, Style::default().fg(app.theme.accent)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  enter", Style::default().fg(app.theme.accent).bold()),
            Span::styled(":save  ", Style::default().fg(app.theme.muted)),
            Span::styled("esc", Style::default().fg(app.theme.accent).bold()),
            Span::styled(":cancel", Style::default().fg(app.theme.muted)),
        ]),
    ];

    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(app.theme.bg)),
        inner,
    );
}

fn draw_template_picker(frame: &mut Frame, app: &mut App) {
    let popup_area = centered_rect(50, 50, frame.area());
    frame.render_widget(Clear, popup_area);

    let block = Block::bordered()
        .title(" Message Templates ")
        .title_style(Style::default().fg(app.theme.accent).bold())
        .border_style(Style::default().fg(app.theme.accent))
        .style(Style::default().bg(app.theme.bg));

    let items: Vec<ListItem> = app.config.templates.iter().map(|t| {
        let body_preview: String = t.body.chars().take(40).collect();
        let body_preview = body_preview.replace('\n', " ");
        ListItem::new(Line::from(vec![
            Span::styled(format!("  {} ", t.name), Style::default().fg(app.theme.primary).bold()),
            Span::styled(body_preview, Style::default().fg(app.theme.muted)),
        ]))
    }).collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(app.theme.selected_bg).fg(app.theme.white).add_modifier(Modifier::BOLD))
        .highlight_symbol("▸ ")
        .style(Style::default().bg(app.theme.bg));

    frame.render_stateful_widget(list, popup_area, &mut app.template_list_state);
}

fn draw_save_template(frame: &mut Frame, app: &App) {
    let popup_area = centered_rect(45, 20, frame.area());
    frame.render_widget(Clear, popup_area);

    let block = Block::bordered()
        .title(" Save Template ")
        .title_style(Style::default().fg(app.theme.accent).bold())
        .border_style(Style::default().fg(app.theme.accent))
        .style(Style::default().bg(app.theme.bg));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Name: ", Style::default().fg(app.theme.muted)),
            Span::styled(&app.save_template_name, Style::default().fg(app.theme.primary).bold()),
            Span::styled("█", Style::default().fg(app.theme.accent)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  enter", Style::default().fg(app.theme.accent).bold()),
            Span::styled(":save  ", Style::default().fg(app.theme.muted)),
            Span::styled("esc", Style::default().fg(app.theme.accent).bold()),
            Span::styled(":cancel", Style::default().fg(app.theme.muted)),
        ]),
    ];

    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(app.theme.bg)),
        inner,
    );
}

fn draw_replay_config(frame: &mut Frame, app: &App) {
    let popup_area = centered_rect(50, 35, frame.area());
    frame.render_widget(Clear, popup_area);
    let block = Block::bordered()
        .title(" Replay Messages (Kafka) ")
        .title_style(Style::default().fg(app.theme.accent).bold())
        .border_style(Style::default().fg(app.theme.accent))
        .style(Style::default().bg(app.theme.bg));
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let ks = Style::default().fg(app.theme.accent).bold();
    let ds = Style::default().fg(app.theme.muted);
    let fields = [("Start Offset", &app.replay_start), ("End Offset", &app.replay_end), ("Dest Topic", &app.replay_dest)];
    let mut lines: Vec<Line> = vec![Line::from("")];
    for (i, (label, value)) in fields.iter().enumerate() {
        let focused = i == app.replay_focused_field;
        let ls = if focused { ks } else { ds };
        let vs = if focused { Style::default().fg(app.theme.white) } else { Style::default().fg(app.theme.primary) };
        let c = if focused { "█" } else { "" };
        lines.push(Line::from(vec![Span::styled(format!("  {}: ", label), ls), Span::styled(format!("{}{}", value, c), vs)]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled("  tab", ks), Span::styled(":next  ", ds), Span::styled("enter", ks), Span::styled(":replay  ", ds), Span::styled("esc", ks), Span::styled(":cancel", ds)]));
    frame.render_widget(Paragraph::new(lines).style(Style::default().bg(app.theme.bg)), inner);
}

fn draw_topology(frame: &mut Frame, app: &App) {
    let popup_area = centered_rect(70, 75, frame.area());
    frame.render_widget(Clear, popup_area);
    let block = Block::bordered()
        .title(" Topology ")
        .title_style(Style::default().fg(app.theme.accent).bold())
        .border_style(Style::default().fg(app.theme.accent))
        .style(Style::default().bg(app.theme.bg));
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    if app.topology_exchanges.is_empty() && app.loading {
        frame.render_widget(Paragraph::new("  Loading...").style(Style::default().fg(app.theme.muted).bg(app.theme.bg)), inner);
        return;
    }
    let ds = Style::default().fg(app.theme.muted);
    let ks = Style::default().fg(app.theme.accent).bold();
    let mut lines: Vec<Line> = Vec::new();
    for exchange in &app.topology_exchanges {
        if exchange.name.is_empty() { continue; }
        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", exchange.name), ks),
            Span::styled(format!("[{}]", exchange.exchange_type), ds),
            if exchange.durable { Span::styled(" durable", Style::default().fg(app.theme.success)) } else { Span::styled(" transient", ds) },
        ]));
        let bindings: Vec<&crate::backend::BindingInfo> = app.topology_bindings.iter().filter(|b| b.source == exchange.name).collect();
        for (i, b) in bindings.iter().enumerate() {
            let pfx = if i == bindings.len() - 1 { "  └── " } else { "  ├── " };
            let rk = if b.routing_key.is_empty() { "*".to_string() } else { b.routing_key.clone() };
            lines.push(Line::from(vec![Span::styled(pfx, ds), Span::styled(rk, Style::default().fg(app.theme.primary)), Span::styled(format!(" → {}", b.destination), Style::default().fg(app.theme.white))]));
        }
        if bindings.is_empty() { lines.push(Line::from(Span::styled("  └── (no bindings)", ds))); }
        lines.push(Line::from(""));
    }
    if lines.is_empty() { lines.push(Line::from(Span::styled("  No exchanges found", ds))); }
    lines.push(Line::from(vec![Span::styled("  j/k", ks), Span::styled(":scroll  ", ds), Span::styled("esc", ks), Span::styled(":close", ds)]));
    frame.render_widget(Paragraph::new(lines).style(Style::default().bg(app.theme.bg)).scroll((app.topology_scroll, 0)), inner);
}

fn draw_benchmark_config(frame: &mut Frame, app: &App) {
    let popup_area = centered_rect(45, 30, frame.area());
    frame.render_widget(Clear, popup_area);
    let block = Block::bordered()
        .title(" Benchmark Config ")
        .title_style(Style::default().fg(app.theme.accent).bold())
        .border_style(Style::default().fg(app.theme.accent))
        .style(Style::default().bg(app.theme.bg));
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);
    let ks = Style::default().fg(app.theme.accent).bold();
    let ds = Style::default().fg(app.theme.muted);
    let fields = [("Message Count", &app.bench_count), ("Concurrency", &app.bench_concurrency)];
    let mut lines: Vec<Line> = vec![Line::from("")];
    for (i, (label, value)) in fields.iter().enumerate() {
        let focused = i == app.bench_focused_field;
        let ls = if focused { ks } else { ds };
        let vs = if focused { Style::default().fg(app.theme.white) } else { Style::default().fg(app.theme.primary) };
        let c = if focused { "█" } else { "" };
        lines.push(Line::from(vec![Span::styled(format!("  {}: ", label), ls), Span::styled(format!("{}{}", value, c), vs)]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("  Uses publish form body as template", ds)));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled("  tab", ks), Span::styled(":next  ", ds), Span::styled("enter", ks), Span::styled(":start  ", ds), Span::styled("esc", ks), Span::styled(":cancel", ds)]));
    frame.render_widget(Paragraph::new(lines).style(Style::default().bg(app.theme.bg)), inner);
}

fn draw_benchmark_running(frame: &mut Frame, app: &App) {
    let popup_area = centered_rect(50, 20, frame.area());
    frame.render_widget(Clear, popup_area);
    let block = Block::bordered()
        .title(" Benchmark Running ")
        .title_style(Style::default().fg(app.theme.accent).bold())
        .border_style(Style::default().fg(app.theme.accent))
        .style(Style::default().bg(app.theme.bg));
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let (completed, total) = app.bench_progress;
    let pct = if total > 0 { (completed as f64 / total as f64 * 100.0) as u16 } else { 0 };
    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(app.theme.accent).bg(app.theme.highlight_bg))
        .percent(pct)
        .label(format!("{}/{} ({:.0}%)", completed, total, pct as f64));
    let gauge_area = Rect::new(inner.x + 2, inner.y + 1, inner.width.saturating_sub(4), 1);
    frame.render_widget(gauge, gauge_area);

    if let Some(ref stats) = app.bench_stats {
        let mps = if stats.elapsed_ms > 0 { stats.total as f64 / (stats.elapsed_ms as f64 / 1000.0) } else { 0.0 };
        let line1 = format!("  Done: {:.0} msg/s, {} threads, {} errors", mps, stats.concurrency, stats.errors);
        let line2 = format!("  Latency: avg {}ms, p50 {}ms, p95 {}ms, p99 {}ms",
            stats.avg_latency_ms, stats.p50_latency_ms, stats.p95_latency_ms, stats.p99_latency_ms);
        let s = Style::default().fg(app.theme.primary);
        let area1 = Rect::new(inner.x, inner.y + 3, inner.width, 1);
        let area2 = Rect::new(inner.x, inner.y + 4, inner.width, 1);
        frame.render_widget(Paragraph::new(Span::styled(line1, s)).style(Style::default().bg(app.theme.bg)), area1);
        frame.render_widget(Paragraph::new(Span::styled(line2, s)).style(Style::default().bg(app.theme.bg)), area2);
    } else {
        let status_line = "  Publishing... (esc to cancel)".to_string();
        let status_area = Rect::new(inner.x, inner.y + 3, inner.width, 1);
        frame.render_widget(Paragraph::new(Span::styled(status_line, Style::default().fg(app.theme.primary))).style(Style::default().bg(app.theme.bg)), status_area);
    }
}
