use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, Focus, Popup};

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    frame.render_widget(
        Block::default().style(Style::default().bg(app.theme.bg)),
        area,
    );

    // All horizontal panels:
    // 1. Header (1 line): app name + version + cluster
    // 2. Selectors (1 line): Vhost: xxx ▾  │  Queue: xxx ▾
    // 3. Stats (1 line): msgs, rates, consumers (or 0 if no queue)
    // 4. Messages (rest): scrollable message list
    // 5. Footer (1 line): keybindings + status
    let stats_h = if app.selected_queue().is_some() { 1 } else { 0 };

    let chunks = Layout::vertical([
        Constraint::Length(1),      // header
        Constraint::Length(1),      // selectors
        Constraint::Length(stats_h),// stats
        Constraint::Min(3),         // messages
        Constraint::Length(1),      // footer
    ])
    .split(area);

    draw_header(frame, app, chunks[0]);
    draw_selectors(frame, app, chunks[1]);
    if stats_h > 0 {
        draw_stats(frame, app, chunks[2]);
    }
    draw_messages(frame, app, chunks[3]);
    draw_footer(frame, app, chunks[4]);

    if app.popup != Popup::None {
        super::popup::draw(frame, app);
    }
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let loading = if app.loading { " ⟳" } else { "" };
    let sep = Span::styled(" │ ", Style::default().fg(app.theme.divider));

    let header = Line::from(vec![
        Span::styled(" rabbitpeek ", Style::default().fg(app.theme.accent).bold()),
        Span::styled("v0.1.0", Style::default().fg(app.theme.muted)),
        sep.clone(),
        Span::styled(
            if app.cluster_name.is_empty() { &app.profile_name } else { &app.cluster_name },
            Style::default().fg(app.theme.primary),
        ),
        Span::styled(loading, Style::default().fg(app.theme.accent)),
    ]);

    frame.render_widget(
        Paragraph::new(header).style(Style::default().bg(app.theme.sidebar_bg)),
        area,
    );
}

fn draw_selectors(frame: &mut Frame, app: &App, area: Rect) {
    let vhost_focused = app.focus == Focus::VhostSelector;
    let queue_focused = app.focus == Focus::QueueSelector;

    let vhost_label_style = if vhost_focused {
        Style::default().fg(app.theme.accent).bold()
    } else {
        Style::default().fg(app.theme.primary)
    };

    let queue_name = if app.current_queue_name.is_empty() {
        "(none)".to_string()
    } else {
        app.current_queue_name.clone()
    };
    let queue_label_style = if queue_focused {
        Style::default().fg(app.theme.accent).bold()
    } else {
        Style::default().fg(app.theme.primary)
    };

    let sep = Span::styled("  │  ", Style::default().fg(app.theme.divider));
    let arrow = Span::styled(" ▾", Style::default().fg(app.theme.muted));

    let line = Line::from(vec![
        Span::styled(" Vhost: ", Style::default().fg(app.theme.muted)),
        Span::styled(&app.selected_vhost, vhost_label_style),
        arrow.clone(),
        sep,
        Span::styled("Queue: ", Style::default().fg(app.theme.muted)),
        Span::styled(queue_name, queue_label_style),
        arrow,
        Span::styled(
            format!("  ({} queues)", app.filtered_indices.len()),
            Style::default().fg(app.theme.muted),
        ),
    ]);

    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(app.theme.bg)),
        area,
    );
}

fn draw_stats(frame: &mut Frame, app: &App, area: Rect) {
    let q = match app.selected_queue() {
        Some(q) => q,
        None => return,
    };

    let stats = Line::from(vec![
        Span::styled(" Msgs: ", Style::default().fg(app.theme.muted)),
        Span::styled(format!("{}", q.messages), Style::default().fg(app.theme.primary).bold()),
        Span::styled("  Pub: ", Style::default().fg(app.theme.muted)),
        Span::styled(format!("{:.1}/s", q.publish_rate), Style::default().fg(app.theme.primary)),
        Span::styled("  Del: ", Style::default().fg(app.theme.muted)),
        Span::styled(format!("{:.1}/s", q.deliver_rate), Style::default().fg(app.theme.primary)),
        Span::styled("  Ack: ", Style::default().fg(app.theme.muted)),
        Span::styled(format!("{:.1}/s", q.ack_rate), Style::default().fg(app.theme.primary)),
        Span::styled("  Consumers: ", Style::default().fg(app.theme.muted)),
        Span::styled(format!("{}", q.consumers), Style::default().fg(app.theme.primary)),
        Span::styled("  State: ", Style::default().fg(app.theme.muted)),
        Span::styled(&q.state, Style::default().fg(app.theme.success)),
    ]);

    frame.render_widget(
        Paragraph::new(stats).style(Style::default().bg(app.theme.status_bg)),
        area,
    );
}

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
        .title_style(Style::default().fg(app.theme.primary).bold())
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(app.theme.bg));

    if app.messages.is_empty() {
        let empty = if app.loading { "Loading..." }
        else if app.current_queue_name.is_empty() { "Select a queue and press Enter to peek" }
        else { "No messages in this queue" };

        frame.render_widget(
            Paragraph::new(Span::styled(empty, Style::default().fg(app.theme.muted))).block(block),
            area,
        );
        return;
    }

    let mut lines: Vec<Line> = Vec::new();

    for msg in &app.messages {
        let ts = msg.timestamp.map(format_timestamp).unwrap_or_else(|| "no timestamp".into());

        lines.push(Line::from(vec![
            Span::styled(format!("#{}", msg.index), Style::default().fg(app.theme.accent).bold()),
            Span::styled("  ", Style::default()),
            Span::styled(ts, Style::default().fg(app.theme.muted)),
            Span::styled("  key=", Style::default().fg(app.theme.muted)),
            Span::styled(msg.routing_key.clone(), Style::default().fg(app.theme.primary)),
        ]));

        let formatted = format_message_body(&msg.body);
        for body_line in formatted.lines().take(3) {
            lines.push(Line::from(Span::styled(
                format!("  {}", body_line),
                Style::default().fg(app.theme.primary),
            )));
        }
        if formatted.lines().count() > 3 {
            lines.push(Line::from(Span::styled("  ...", Style::default().fg(app.theme.muted))));
        }
        lines.push(Line::from(""));
    }

    let total = lines.len() as u16;
    let visible = area.height.saturating_sub(2);
    let scroll_info = if total > visible {
        format!(" [{}/{}] ", app.message_scroll + 1, total.saturating_sub(visible) + 1)
    } else {
        String::new()
    };

    let block = block.title_bottom(Line::from(scroll_info).right_aligned());

    frame.render_widget(
        Paragraph::new(lines).block(block).scroll((app.message_scroll, 0)).style(Style::default().bg(app.theme.bg)),
        area,
    );
}

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let (status_text, status_color) = if !app.status_message.is_empty() {
        let c = if app.status_is_error { app.theme.error } else { app.theme.success };
        (app.status_message.as_str(), c)
    } else {
        ("", app.theme.muted)
    };

    let ks = Style::default().fg(app.theme.accent).bold();
    let ds = Style::default().fg(app.theme.muted);

    let footer = Line::from(vec![
        Span::raw(" "),
        Span::styled("tab", ks), Span::styled(":focus ", ds),
        Span::styled("enter", ks), Span::styled(":open ", ds),
        Span::styled("/", ks), Span::styled(":filter ", ds),
        Span::styled("r", ks), Span::styled(":reload ", ds),
        Span::styled("+/-", ks), Span::styled(":fetch ", ds),
        Span::styled("p", ks), Span::styled(":profile ", ds),
        Span::styled("?", ks), Span::styled(":help ", ds),
        Span::styled("q", ks), Span::styled(":quit", ds),
        Span::styled("  │ ", Style::default().fg(app.theme.divider)),
        Span::styled(status_text, Style::default().fg(status_color)),
    ]);

    frame.render_widget(
        Paragraph::new(footer).style(Style::default().bg(app.theme.status_bg)),
        area,
    );
}

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
