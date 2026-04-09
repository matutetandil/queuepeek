use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, Popup, PublishForm, QueueOperation};

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

    for group in &app.consumer_groups {
        if !lines.is_empty() {
            lines.push(Line::from(""));
        }

        // Group header
        let state_style = if group.state == "Stable" {
            Style::default().fg(app.theme.success)
        } else {
            Style::default().fg(app.theme.muted)
        };

        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", group.name), name_style),
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
        Span::styled(":scroll  ", Style::default().fg(app.theme.muted)),
        Span::styled("esc", Style::default().fg(app.theme.accent).bold()),
        Span::styled(":close", Style::default().fg(app.theme.muted)),
    ]));

    let scroll = app.consumer_groups_scroll;
    let content = Paragraph::new(lines)
        .style(Style::default().bg(app.theme.bg))
        .scroll((scroll, 0));
    frame.render_widget(content, inner);
}
