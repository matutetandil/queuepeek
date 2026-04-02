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

    let total = app.messages.len();
    let position = app.detail_message_idx + 1;

    let header = Line::from(vec![
        Span::styled(
            format!("  {} ", app.profile_name),
            Style::default().fg(app.theme.muted).bg(app.theme.sidebar_bg),
        ),
        Span::styled("› ", Style::default().fg(app.theme.divider).bg(app.theme.sidebar_bg)),
        Span::styled(
            format!("{} ", app.selected_namespace),
            Style::default().fg(app.theme.muted).bg(app.theme.sidebar_bg),
        ),
        Span::styled("› ", Style::default().fg(app.theme.divider).bg(app.theme.sidebar_bg)),
        Span::styled(
            format!("{} ", app.current_queue_name),
            Style::default().fg(app.theme.muted).bg(app.theme.sidebar_bg),
        ),
        Span::styled("› ", Style::default().fg(app.theme.divider).bg(app.theme.sidebar_bg)),
        Span::styled(
            format!("Message #{} ", msg.index),
            Style::default().fg(app.theme.white).bold().bg(app.theme.sidebar_bg),
        ),
        Span::styled(
            format!("({} of {})", position, total),
            Style::default().fg(app.theme.muted).bg(app.theme.sidebar_bg),
        ),
    ]);

    let bar = Paragraph::new(header)
        .style(Style::default().bg(app.theme.sidebar_bg));
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

    // Payload block — auto-detect format and pretty-print
    let (payload_body, format_label) = if app.detail_pretty {
        pretty_format(&msg.body)
    } else {
        (msg.body.clone(), "raw")
    };

    let pretty_indicator = format!("[{}]", format_label);

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

/// Auto-detect format and pretty-print. Returns (formatted_body, format_label).
fn pretty_format(body: &str) -> (String, &'static str) {
    let trimmed = body.trim();

    // Try JSON
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
            if let Ok(pretty) = serde_json::to_string_pretty(&val) {
                return (pretty, "json");
            }
        }
    }

    // Try XML — simple indent-based formatting
    if trimmed.starts_with('<') && trimmed.contains('>') {
        let formatted = pretty_xml(trimmed);
        if formatted != trimmed {
            return (formatted, "xml");
        }
    }

    // Plain text
    (body.to_string(), "text")
}

/// Simple XML indentation formatter
fn pretty_xml(xml: &str) -> String {
    let mut result = String::new();
    let mut indent = 0usize;
    let mut in_tag = false;
    let mut tag_content = String::new();

    for ch in xml.chars() {
        match ch {
            '<' => {
                if !tag_content.trim().is_empty() {
                    result.push_str(&"  ".repeat(indent));
                    result.push_str(tag_content.trim());
                    result.push('\n');
                }
                tag_content.clear();
                in_tag = true;
                tag_content.push(ch);
            }
            '>' => {
                tag_content.push(ch);
                in_tag = false;
                let tag = tag_content.trim().to_string();

                if tag.starts_with("</") {
                    // Closing tag
                    indent = indent.saturating_sub(1);
                    result.push_str(&"  ".repeat(indent));
                    result.push_str(&tag);
                    result.push('\n');
                } else if tag.ends_with("/>") || tag.starts_with("<?") || tag.starts_with("<!") {
                    // Self-closing or processing instruction
                    result.push_str(&"  ".repeat(indent));
                    result.push_str(&tag);
                    result.push('\n');
                } else {
                    // Opening tag
                    result.push_str(&"  ".repeat(indent));
                    result.push_str(&tag);
                    result.push('\n');
                    indent += 1;
                }
                tag_content.clear();
            }
            _ => {
                tag_content.push(ch);
            }
        }
    }

    // Remaining content
    if !tag_content.trim().is_empty() {
        result.push_str(&"  ".repeat(indent));
        result.push_str(tag_content.trim());
        result.push('\n');
    }

    result
}

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let ks = Style::default().fg(app.theme.accent).bg(app.theme.sidebar_bg);
    let ds = Style::default().fg(app.theme.muted).bg(app.theme.sidebar_bg);
    let footer = Line::from(vec![
        Span::styled("  j/k", ks), Span::styled(":scroll ", ds),
        Span::styled("p", ks), Span::styled(":pretty ", ds),
        Span::styled("c", ks), Span::styled(":copy payload ", ds),
        Span::styled("h", ks), Span::styled(":copy headers ", ds),
        Span::styled("esc", ks), Span::styled(":back ", ds),
        Span::styled("q", ks), Span::styled(":quit", ds),
    ]);
    let bar = Paragraph::new(footer)
        .style(Style::default().bg(app.theme.sidebar_bg));
    frame.render_widget(bar, area);
}
