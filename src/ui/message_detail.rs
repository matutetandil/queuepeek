use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    frame.render_widget(Block::default().style(Style::default().bg(app.theme.bg)), area);

    let chunks = Layout::vertical([
        Constraint::Length(1), // header bar
        Constraint::Min(3),   // content (headers + payload)
        Constraint::Length(1), // footer
    ])
    .split(area);

    draw_header(frame, app, chunks[0]);
    draw_content(frame, app, chunks[1]);
    draw_footer(frame, app, chunks[2]);
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let msg = match app.messages.get(app.detail_message_idx) {
        Some(m) => m,
        None => {
            let bar = Paragraph::new("  Message not found")
                .style(Style::default().fg(app.theme.primary).bg(app.theme.sidebar_bg));
            frame.render_widget(bar, area);
            return;
        }
    };

    let text = format!(
        "  Message #{}  key={}  exchange={}",
        msg.index,
        msg.routing_key,
        if msg.exchange.is_empty() {
            "(default)"
        } else {
            &msg.exchange
        }
    );

    let bar = Paragraph::new(text)
        .style(Style::default().fg(app.theme.primary).bg(app.theme.sidebar_bg));
    frame.render_widget(bar, area);
}

fn draw_content(frame: &mut Frame, app: &App, area: Rect) {
    let msg = match app.messages.get(app.detail_message_idx) {
        Some(m) => m,
        None => {
            let empty = Paragraph::new("No message selected")
                .style(Style::default().fg(app.theme.muted).bg(app.theme.bg));
            frame.render_widget(empty, area);
            return;
        }
    };

    // Build header lines
    let mut header_lines: Vec<Line> = Vec::new();

    let kv_pairs: Vec<(&str, String)> = vec![
        ("routing_key", msg.routing_key.clone()),
        ("exchange", if msg.exchange.is_empty() { "(default)".to_string() } else { msg.exchange.clone() }),
        ("redelivered", msg.redelivered.to_string()),
        (
            "timestamp",
            msg.timestamp
                .map(|ts| format_timestamp(ts))
                .unwrap_or_else(|| "N/A".to_string()),
        ),
        (
            "content_type",
            if msg.content_type.is_empty() {
                "N/A".to_string()
            } else {
                msg.content_type.clone()
            },
        ),
    ];

    for (key, value) in &kv_pairs {
        header_lines.push(Line::from(vec![
            Span::styled(
                format!("  {}: ", key),
                Style::default().fg(app.theme.accent).bg(app.theme.bg),
            ),
            Span::styled(
                value.clone(),
                Style::default().fg(app.theme.primary).bg(app.theme.bg),
            ),
        ]));
    }

    let header_height = (header_lines.len() + 2) as u16; // +2 for border

    let content_chunks = Layout::vertical([
        Constraint::Length(header_height),
        Constraint::Min(3),
    ])
    .split(area);

    // Headers block
    let headers_block = Block::bordered()
        .title(" Headers ")
        .title_style(Style::default().fg(app.theme.accent))
        .border_style(Style::default().fg(app.theme.divider))
        .style(Style::default().bg(app.theme.bg));

    let headers_paragraph = Paragraph::new(header_lines).block(headers_block);
    frame.render_widget(headers_paragraph, content_chunks[0]);

    // Payload block
    let payload_body = if app.detail_pretty {
        match serde_json::from_str::<serde_json::Value>(&msg.body) {
            Ok(val) => serde_json::to_string_pretty(&val).unwrap_or_else(|_| msg.body.clone()),
            Err(_) => msg.body.clone(),
        }
    } else {
        msg.body.clone()
    };

    let pretty_indicator = if app.detail_pretty { "[pretty]" } else { "[raw]" };

    let payload_block = Block::bordered()
        .title(format!(" Payload {} ", pretty_indicator))
        .title_style(Style::default().fg(app.theme.accent))
        .border_style(Style::default().fg(app.theme.divider))
        .style(Style::default().bg(app.theme.bg));

    let payload_paragraph = Paragraph::new(payload_body)
        .style(Style::default().fg(app.theme.primary).bg(app.theme.bg))
        .block(payload_block)
        .wrap(Wrap { trim: false })
        .scroll((app.detail_scroll, 0));

    frame.render_widget(payload_paragraph, content_chunks[1]);
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
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC", y, m + 1, d + 1, hours, minutes, seconds)
}

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let keys = "  j/k:scroll  p:pretty  c:copy payload  h:copy headers  esc:back  q:quit";
    let bar = Paragraph::new(keys)
        .style(Style::default().fg(app.theme.muted).bg(app.theme.sidebar_bg));
    frame.render_widget(bar, area);
}
