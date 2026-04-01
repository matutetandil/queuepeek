use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, Focus, Popup};

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Fill background
    frame.render_widget(
        Block::default().style(Style::default().bg(app.theme.bg)),
        area,
    );

    let stats_h = if app.selected_queue().is_some() { 1 } else { 0 };

    let chunks = Layout::vertical([
        Constraint::Length(1),      // title bar
        Constraint::Length(1),      // toolbar (selectors)
        Constraint::Length(stats_h),// stats bar
        Constraint::Min(3),         // messages area
        Constraint::Length(1),      // status bar
    ])
    .split(area);

    draw_title_bar(frame, app, chunks[0]);
    draw_toolbar(frame, app, chunks[1]);
    if stats_h > 0 {
        draw_stats_bar(frame, app, chunks[2]);
    }
    draw_messages(frame, app, chunks[3]);
    draw_status_bar(frame, app, chunks[4]);

    if app.popup != Popup::None {
        super::popup::draw(frame, app);
    }
}

// ─── Title Bar ──────────────────────────────────────────────────────────────
// Darkest bg, app name prominent, cluster info subtle
fn draw_title_bar(frame: &mut Frame, app: &App, area: Rect) {
    let loading = if app.loading { "  ⟳ loading" } else { "" };

    let cluster_display = if app.cluster_name.is_empty() {
        app.profile_name.as_str()
    } else {
        app.cluster_name.as_str()
    };

    let line = Line::from(vec![
        Span::styled(" 🐇 ", Style::default().fg(app.theme.accent)),
        Span::styled("rabbitpeek", Style::default().fg(app.theme.white).bold()),
        Span::styled(" v0.1.0 ", Style::default().fg(app.theme.muted)),
        Span::styled("│", Style::default().fg(app.theme.divider)),
        Span::styled(format!(" {} ", cluster_display), Style::default().fg(app.theme.primary)),
        Span::styled(loading, Style::default().fg(app.theme.accent)),
    ]);

    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(app.theme.sidebar_bg)),
        area,
    );
}

// ─── Toolbar ────────────────────────────────────────────────────────────────
// Slightly lighter than title, shows interactive selectors
fn draw_toolbar(frame: &mut Frame, app: &App, area: Rect) {
    let vhost_focused = app.focus == Focus::VhostSelector;
    let queue_focused = app.focus == Focus::QueueSelector;

    // Focused selector gets accent + underline, unfocused is white
    let vhost_val_style = if vhost_focused {
        Style::default().fg(app.theme.accent).bold().add_modifier(Modifier::UNDERLINED)
    } else {
        Style::default().fg(app.theme.white)
    };

    let queue_name = if app.current_queue_name.is_empty() {
        "(select with Enter)".to_string()
    } else {
        app.current_queue_name.clone()
    };
    let queue_val_style = if queue_focused {
        Style::default().fg(app.theme.accent).bold().add_modifier(Modifier::UNDERLINED)
    } else {
        Style::default().fg(app.theme.white)
    };

    let label = Style::default().fg(app.theme.muted);
    let sep = Span::styled(" │ ", Style::default().fg(app.theme.divider));
    let arrow_style = Style::default().fg(app.theme.muted);

    let line = Line::from(vec![
        Span::styled("  Vhost: ", label),
        Span::styled(&app.selected_vhost, vhost_val_style),
        Span::styled(" ▾ ", arrow_style),
        sep.clone(),
        Span::styled("Queue: ", label),
        Span::styled(queue_name, queue_val_style),
        Span::styled(" ▾ ", arrow_style),
        sep,
        Span::styled(format!("{} queues", app.filtered_indices.len()), Style::default().fg(app.theme.muted)),
        Span::styled(format!("  fetch: {}", app.fetch_count), Style::default().fg(app.theme.muted)),
    ]);

    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(app.theme.highlight_bg)),
        area,
    );
}

// ─── Stats Bar ──────────────────────────────────────────────────────────────
// Different bg to stand out as "info strip"
fn draw_stats_bar(frame: &mut Frame, app: &App, area: Rect) {
    let q = match app.selected_queue() {
        Some(q) => q,
        None => return,
    };

    let label = Style::default().fg(app.theme.muted);
    let val = Style::default().fg(app.theme.primary).bold();
    let sep = Span::styled(" │ ", Style::default().fg(app.theme.divider));

    let line = Line::from(vec![
        Span::styled("  ◆ ", Style::default().fg(app.theme.accent)),
        Span::styled("Msgs: ", label),
        Span::styled(format!("{}", q.messages), val),
        sep.clone(),
        Span::styled("Pub: ", label),
        Span::styled(format!("{:.1}/s", q.publish_rate), val),
        sep.clone(),
        Span::styled("Del: ", label),
        Span::styled(format!("{:.1}/s", q.deliver_rate), val),
        sep.clone(),
        Span::styled("Ack: ", label),
        Span::styled(format!("{:.1}/s", q.ack_rate), val),
        sep.clone(),
        Span::styled("Consumers: ", label),
        Span::styled(format!("{}", q.consumers), val),
        sep,
        Span::styled(&q.state, Style::default().fg(app.theme.success)),
    ]);

    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(app.theme.status_bg)),
        area,
    );
}

// ─── Messages ───────────────────────────────────────────────────────────────
fn draw_messages(frame: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.focus == Focus::Messages;
    let border_color = if focused { app.theme.accent } else { app.theme.divider };

    let title = if app.current_queue_name.is_empty() {
        " Messages ".to_string()
    } else {
        format!(" Messages — {} ", app.current_queue_name)
    };

    let block = Block::bordered()
        .title(title)
        .title_style(Style::default().fg(app.theme.white).bold())
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(app.theme.bg));

    if app.messages.is_empty() {
        let empty = if app.loading { "  Loading..." }
        else if app.current_queue_name.is_empty() { "  Select a queue and press Enter to peek" }
        else { "  No messages in this queue" };

        frame.render_widget(
            Paragraph::new(Span::styled(empty, Style::default().fg(app.theme.muted))).block(block),
            area,
        );
        return;
    }

    let mut lines: Vec<Line> = Vec::new();

    for msg in &app.messages {
        let ts = msg.timestamp.map(format_timestamp).unwrap_or_else(|| "no timestamp".into());

        // Message header: #N  timestamp  key=routing_key
        lines.push(Line::from(vec![
            Span::styled(format!("  #{}", msg.index), Style::default().fg(app.theme.accent).bold()),
            Span::styled("  ", Style::default()),
            Span::styled(ts, Style::default().fg(app.theme.muted)),
            Span::styled("  key=", Style::default().fg(app.theme.muted)),
            Span::styled(msg.routing_key.clone(), Style::default().fg(app.theme.primary)),
        ]));

        // Body preview (max 3 lines)
        let formatted = format_message_body(&msg.body);
        for body_line in formatted.lines().take(3) {
            lines.push(Line::from(Span::styled(
                format!("    {}", body_line),
                Style::default().fg(app.theme.primary),
            )));
        }
        if formatted.lines().count() > 3 {
            lines.push(Line::from(Span::styled("    ...", Style::default().fg(app.theme.muted))));
        }

        // Separator
        lines.push(Line::from(Span::styled(
            format!("  {}", "─".repeat(area.width.saturating_sub(4) as usize)),
            Style::default().fg(app.theme.divider),
        )));
    }

    let total = lines.len() as u16;
    let visible = area.height.saturating_sub(2);
    let scroll_info = if total > visible {
        format!(" scroll: {}/{} ", app.message_scroll + 1, total.saturating_sub(visible) + 1)
    } else {
        String::new()
    };

    let block = block.title_bottom(Line::from(Span::styled(
        scroll_info,
        Style::default().fg(app.theme.muted),
    )).right_aligned());

    frame.render_widget(
        Paragraph::new(lines).block(block).scroll((app.message_scroll, 0)),
        area,
    );
}

// ─── Status Bar ─────────────────────────────────────────────────────────────
// Bottom bar with keybindings and status message
fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let (status_text, status_color) = if !app.status_message.is_empty() {
        let c = if app.status_is_error { app.theme.error } else { app.theme.success };
        (app.status_message.as_str(), c)
    } else {
        ("", app.theme.muted)
    };

    let ks = Style::default().fg(app.theme.accent).bold();
    let ds = Style::default().fg(app.theme.muted);

    let line = Line::from(vec![
        Span::styled(" ", Style::default()),
        Span::styled("tab", ks), Span::styled(":focus ", ds),
        Span::styled("⏎", ks), Span::styled(":open ", ds),
        Span::styled("/", ks), Span::styled(":filter ", ds),
        Span::styled("r", ks), Span::styled(":reload ", ds),
        Span::styled("+/-", ks), Span::styled(":fetch ", ds),
        Span::styled("v", ks), Span::styled(":vhost ", ds),
        Span::styled("p", ks), Span::styled(":profile ", ds),
        Span::styled("?", ks), Span::styled(":help ", ds),
        Span::styled("q", ks), Span::styled(":quit", ds),
        Span::styled("  │ ", Style::default().fg(app.theme.divider)),
        Span::styled(status_text, Style::default().fg(status_color)),
    ]);

    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(app.theme.sidebar_bg)),
        area,
    );
}

// ─── Helpers ────────────────────────────────────────────────────────────────

fn format_message_body(body: &str) -> String {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(body) {
        serde_json::to_string_pretty(&value).unwrap_or_else(|_| body.to_string())
    } else {
        body.to_string()
    }
}

fn format_timestamp(ts: i64) -> String {
    let remaining = ts % 86400;
    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;
    let seconds = remaining % 60;
    let mut y = 1970i64;
    let mut d = ts / 86400;
    loop {
        let diy = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) { 366 } else { 365 };
        if d < diy { break; }
        d -= diy;
        y += 1;
    }
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let mds = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut m = 0usize;
    for md in &mds {
        if d < *md as i64 { break; }
        d -= *md as i64;
        m += 1;
    }
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", y, m + 1, d + 1, hours, minutes, seconds)
}
