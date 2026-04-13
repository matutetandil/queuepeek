use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::App;
use crate::ui::theme::Theme;

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

fn draw_content(frame: &mut Frame, app: &mut App, area: Rect) {
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

    // Try schema registry decode first
    let msg_idx = app.detail_message_idx;
    let schema_result = if app.schema_decode_enabled && app.schema_client.is_some() {
        app.decode_message_schema(msg_idx);
        app.schema_decoded_cache.get(&msg_idx)
            .and_then(|r| r.as_ref().ok())
            .map(|d| (d.decoded_body.clone(), format!("{}#{}",d.schema_type, d.schema_id)))
    } else {
        None
    };

    let (decoded_body, decode_label) = if let Some((body, label)) = schema_result {
        (body, label)
    } else {
        // Fallback to binary decode or raw
        if app.detail_decoded {
            let (b, l) = decode_payload(&app.messages.get(msg_idx).map(|m| m.body.as_str()).unwrap_or(""));
            (b, l.to_string())
        } else {
            (app.messages.get(msg_idx).map(|m| m.body.clone()).unwrap_or_default(), String::new())
        }
    };

    // Payload block — auto-detect format and pretty-print with syntax highlighting
    let (payload_text, format_label) = if app.detail_pretty {
        let (formatted, label) = pretty_format(&decoded_body);
        let full_label = if decode_label.is_empty() {
            label.to_string()
        } else {
            format!("{}+{}", label, &decode_label)
        };
        let text = match label {
            "json" => highlight_json(&formatted, app.theme),
            "xml" => highlight_xml(&formatted, app.theme),
            _ => Text::styled(formatted, Style::default().fg(app.theme.primary)),
        };
        (text, full_label)
    } else {
        let label = if decode_label.is_empty() { "raw".to_string() } else { format!("raw+{}", &decode_label) };
        (Text::styled(decoded_body, Style::default().fg(app.theme.primary)), label)
    };

    let pretty_indicator = format!("[{}]", format_label);

    let payload_block = Block::bordered()
        .title(format!(" Payload {} ", pretty_indicator))
        .title_style(Style::default().fg(app.theme.accent))
        .border_style(Style::default().fg(app.theme.divider))
        .style(Style::default().bg(app.theme.bg));

    let payload_paragraph = Paragraph::new(payload_text)
        .style(Style::default().bg(app.theme.bg))
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

/// Syntax-highlight JSON text into colored spans
fn highlight_json(text: &str, theme: &Theme) -> Text<'static> {
    let key_style = Style::default().fg(theme.accent);
    let string_style = Style::default().fg(theme.success);
    let number_style = Style::default().fg(theme.primary).bold();
    let punct_style = Style::default().fg(theme.muted);

    let mut lines: Vec<Line<'static>> = Vec::new();

    for line_str in text.lines() {
        let mut spans: Vec<Span<'static>> = Vec::new();
        let chars: Vec<char> = line_str.chars().collect();
        let len = chars.len();
        let mut i = 0;

        while i < len {
            let ch = chars[i];
            match ch {
                '"' => {
                    // Collect the full string (including quotes)
                    let mut s = String::new();
                    s.push('"');
                    i += 1;
                    while i < len {
                        let c = chars[i];
                        s.push(c);
                        if c == '\\' && i + 1 < len {
                            i += 1;
                            s.push(chars[i]);
                        } else if c == '"' {
                            break;
                        }
                        i += 1;
                    }
                    i += 1;

                    // Check if followed by ':' (it's a key)
                    let mut j = i;
                    while j < len && chars[j] == ' ' { j += 1; }
                    let is_key = j < len && chars[j] == ':';

                    if is_key {
                        spans.push(Span::styled(s, key_style));
                    } else {
                        spans.push(Span::styled(s, string_style));
                    }
                }
                '{' | '}' | '[' | ']' | ':' | ',' => {
                    spans.push(Span::styled(ch.to_string(), punct_style));
                    i += 1;
                }
                ' ' | '\t' => {
                    let mut ws = String::new();
                    while i < len && (chars[i] == ' ' || chars[i] == '\t') {
                        ws.push(chars[i]);
                        i += 1;
                    }
                    spans.push(Span::raw(ws));
                }
                _ => {
                    // Numbers, bools, null
                    let mut token = String::new();
                    while i < len && !matches!(chars[i], ',' | '}' | ']' | ' ' | '\t' | '\n') {
                        token.push(chars[i]);
                        i += 1;
                    }
                    spans.push(Span::styled(token, number_style));
                }
            }
        }

        lines.push(Line::from(spans));
    }

    Text::from(lines)
}

/// Syntax-highlight XML text into colored spans
fn highlight_xml(text: &str, theme: &Theme) -> Text<'static> {
    let tag_style = Style::default().fg(theme.accent);
    let attr_style = Style::default().fg(theme.muted);
    let text_style = Style::default().fg(theme.primary);
    let punct_style = Style::default().fg(theme.accent).bold();

    let mut lines: Vec<Line<'static>> = Vec::new();

    for line_str in text.lines() {
        let mut spans: Vec<Span<'static>> = Vec::new();
        let chars: Vec<char> = line_str.chars().collect();
        let len = chars.len();
        let mut i = 0;

        while i < len {
            if chars[i] == '<' {
                // Collect entire tag
                let mut tag = String::new();
                while i < len {
                    tag.push(chars[i]);
                    if chars[i] == '>' { i += 1; break; }
                    i += 1;
                }

                // Split tag into name and attributes
                let inner = tag.trim_start_matches('<').trim_end_matches('>').trim_end_matches('/');
                let parts: Vec<&str> = inner.splitn(2, char::is_whitespace).collect();

                spans.push(Span::styled("<".to_string(), punct_style));
                if let Some(name) = parts.first() {
                    spans.push(Span::styled(name.to_string(), tag_style));
                }
                if parts.len() > 1 {
                    spans.push(Span::styled(format!(" {}", parts[1]), attr_style));
                }
                if tag.ends_with("/>") {
                    spans.push(Span::styled("/>".to_string(), punct_style));
                } else {
                    spans.push(Span::styled(">".to_string(), punct_style));
                }
            } else {
                // Text content
                let mut content = String::new();
                while i < len && chars[i] != '<' {
                    content.push(chars[i]);
                    i += 1;
                }
                if !content.is_empty() {
                    spans.push(Span::styled(content, text_style));
                }
            }
        }

        lines.push(Line::from(spans));
    }

    Text::from(lines)
}

/// Attempt to decode a base64-encoded and/or gzip-compressed payload.
/// Returns (decoded_string, decode_label).
fn decode_payload(body: &str) -> (String, &'static str) {
    use base64::Engine;
    use std::io::Read;

    let trimmed = body.trim();

    // Try base64 decode first
    if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(trimmed) {
        // Check if decoded bytes are gzip
        if bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b {
            let mut decoder = flate2::read::GzDecoder::new(&bytes[..]);
            let mut decompressed = String::new();
            if decoder.read_to_string(&mut decompressed).is_ok() {
                return (decompressed, "b64+gz");
            }
        }
        // Not gzip, try as UTF-8
        if let Ok(text) = String::from_utf8(bytes) {
            return (text, "b64");
        }
    }

    // Also try URL-safe base64
    if let Ok(bytes) = base64::engine::general_purpose::URL_SAFE.decode(trimmed) {
        if let Ok(text) = String::from_utf8(bytes) {
            return (text, "b64url");
        }
    }

    // Try raw gzip (body as bytes)
    let raw_bytes = trimmed.as_bytes();
    if raw_bytes.len() >= 2 && raw_bytes[0] == 0x1f && raw_bytes[1] == 0x8b {
        let mut decoder = flate2::read::GzDecoder::new(raw_bytes);
        let mut decompressed = String::new();
        if decoder.read_to_string(&mut decompressed).is_ok() {
            return (decompressed, "gzip");
        }
    }

    // No decode possible
    (body.to_string(), "")
}

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let ks = Style::default().fg(app.theme.accent).bg(app.theme.sidebar_bg);
    let ds = Style::default().fg(app.theme.muted).bg(app.theme.sidebar_bg);
    let bt = app.current_backend_type();
    let mut spans = vec![
        Span::styled("  j/k", ks), Span::styled(":scroll ", ds),
        Span::styled("p", ks), Span::styled(":pretty ", ds),
        Span::styled("b", ks), Span::styled(":decode ", ds),
    ];
    if app.schema_client.is_some() {
        spans.extend([Span::styled("s", ks), Span::styled(":schema ", ds)]);
    }
    spans.extend([
        Span::styled("c", ks), Span::styled(":copy payload ", ds),
        Span::styled("h", ks), Span::styled(":copy headers ", ds),
        Span::styled("E", ks), Span::styled(":edit ", ds),
    ]);
    if bt == "rabbitmq" {
        spans.extend([Span::styled("L", ks), Span::styled(":reroute ", ds)]);
    }
    spans.extend([
        Span::styled("esc", ks), Span::styled(":back ", ds),
        Span::styled("q", ks), Span::styled(":quit", ds),
    ]);
    spans.extend(super::update_hint_spans(app));
    let footer = Line::from(spans);
    let bar = Paragraph::new(footer)
        .style(Style::default().bg(app.theme.sidebar_bg));
    frame.render_widget(bar, area);
}
