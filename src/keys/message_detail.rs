use crossterm::event::{KeyCode, KeyModifiers};

use crate::app::{self, App, Popup, Screen};
use crate::operations;
use crate::utils;

pub fn handle_message_detail_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    if app.popup != Popup::None {
        super::popup::handle_popup_key(app, code, modifiers);
        return;
    }

    // Search input mode
    if app.detail_search_active {
        match code {
            KeyCode::Esc => {
                app.detail_search_active = false;
            }
            KeyCode::Enter => {
                app.detail_search_active = false;
                if !app.detail_search_query.is_empty() {
                    if app.detail_search_matches.is_empty() {
                        app.set_status("Pattern not found", true);
                    } else {
                        let line = app.detail_search_matches[app.detail_search_current];
                        app.detail_scroll = line;
                        app.set_status(
                            format!("{}/{} matches", app.detail_search_current + 1, app.detail_search_matches.len()),
                            false,
                        );
                    }
                }
            }
            KeyCode::Char(c) => {
                app.detail_search_query.push(c);
                update_search_matches(app);
            }
            KeyCode::Backspace => {
                app.detail_search_query.pop();
                update_search_matches(app);
            }
            _ => {}
        }
        return;
    }

    match code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('?') => {
            app.popup = if app.popup == Popup::Help { Popup::None } else { Popup::Help };
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.detail_scroll = app.detail_scroll.saturating_add(1);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.detail_scroll = app.detail_scroll.saturating_sub(1);
        }
        KeyCode::PageDown => {
            app.detail_scroll = app.detail_scroll.saturating_add(10);
        }
        KeyCode::PageUp => {
            app.detail_scroll = app.detail_scroll.saturating_sub(10);
        }
        KeyCode::Char('p') => {
            app.detail_pretty = !app.detail_pretty;
            // Re-run search if there's an active query
            if !app.detail_search_query.is_empty() {
                update_search_matches(app);
            }
        }
        KeyCode::Char('b') => {
            app.detail_decoded = !app.detail_decoded;
            if app.detail_decoded {
                app.set_status("Binary decode ON (base64/gzip)", false);
            } else {
                app.set_status("Binary decode OFF", false);
            }
            if !app.detail_search_query.is_empty() {
                update_search_matches(app);
            }
        }
        KeyCode::Char('c') => {
            if let Some(msg) = app.messages.get(app.detail_message_idx) {
                let text = msg.body.clone();
                match utils::copy_to_clipboard(&text) {
                    Ok(()) => app.set_status("Payload copied to clipboard", false),
                    Err(e) => app.set_status(e, true),
                }
            }
        }
        KeyCode::Char('h') => {
            if let Some(msg) = app.messages.get(app.detail_message_idx) {
                let mut header_text = format!("routing_key: {}\n", msg.routing_key);
                header_text += &format!("exchange: {}\n", msg.exchange);
                header_text += &format!("redelivered: {}\n", msg.redelivered);
                header_text += &format!("content_type: {}\n", msg.content_type);
                for (k, v) in &msg.headers {
                    header_text += &format!("{}: {}\n", k, v);
                }
                match utils::copy_to_clipboard(&header_text) {
                    Ok(()) => app.set_status("Headers copied to clipboard", false),
                    Err(e) => app.set_status(e, true),
                }
            }
        }
        KeyCode::Char('L') => {
            if let Some(msg) = app.messages.get(app.detail_message_idx) {
                let dlq_info = msg.headers.iter()
                    .find(|(k, _)| k == "x-death")
                    .and_then(|(_, v)| operations::parse_x_death_value(v));
                if let Some((exchange, routing_key)) = dlq_info {
                    app.popup = Popup::ConfirmReroute { exchange, routing_key, count: 1 };
                } else {
                    app.set_status("No x-death header found — not a dead-lettered message", true);
                }
            }
        }
        KeyCode::Char('s') => {
            if app.schema_client.is_some() {
                app.schema_decode_enabled = !app.schema_decode_enabled;
                app.schema_decoded_cache.clear();
                if app.schema_decode_enabled {
                    app.set_status("Schema decode ON", false);
                } else {
                    app.set_status("Schema decode OFF", false);
                }
                if !app.detail_search_query.is_empty() {
                    update_search_matches(app);
                }
            } else {
                app.set_status("No Schema Registry configured for this profile", true);
            }
        }
        KeyCode::Char('E') => {
            if let Some(msg) = app.messages.get(app.detail_message_idx) {
                app.publish_form = app::PublishForm {
                    routing_key: msg.routing_key.clone(),
                    content_type: if msg.content_type.is_empty() { "application/json".to_string() } else { msg.content_type.clone() },
                    body: msg.body.clone(),
                    focused_field: 2,
                    error: String::new(),
                };
                app.popup = Popup::EditMessage;
            }
        }
        KeyCode::Char('/') => {
            app.detail_search_active = true;
            app.detail_search_query.clear();
            app.detail_search_matches.clear();
            app.detail_search_current = 0;
        }
        KeyCode::Char('n') => {
            // Next match
            if !app.detail_search_matches.is_empty() {
                app.detail_search_current = (app.detail_search_current + 1) % app.detail_search_matches.len();
                let line = app.detail_search_matches[app.detail_search_current];
                app.detail_scroll = line;
                app.set_status(
                    format!("{}/{} matches", app.detail_search_current + 1, app.detail_search_matches.len()),
                    false,
                );
            } else if !app.detail_search_query.is_empty() {
                app.set_status("Pattern not found", true);
            }
        }
        KeyCode::Char('N') => {
            // Previous match
            if !app.detail_search_matches.is_empty() {
                if app.detail_search_current == 0 {
                    app.detail_search_current = app.detail_search_matches.len() - 1;
                } else {
                    app.detail_search_current -= 1;
                }
                let line = app.detail_search_matches[app.detail_search_current];
                app.detail_scroll = line;
                app.set_status(
                    format!("{}/{} matches", app.detail_search_current + 1, app.detail_search_matches.len()),
                    false,
                );
            } else if !app.detail_search_query.is_empty() {
                app.set_status("Pattern not found", true);
            }
        }
        KeyCode::Esc => {
            if !app.detail_search_query.is_empty() {
                // First Esc clears the search
                app.detail_search_query.clear();
                app.detail_search_matches.clear();
                app.detail_search_current = 0;
            } else {
                app.screen = Screen::MessageList;
            }
        }
        _ => {}
    }
}

/// Rebuild the list of matching line numbers from the current payload text.
fn update_search_matches(app: &mut App) {
    app.detail_search_matches.clear();
    app.detail_search_current = 0;

    if app.detail_search_query.is_empty() {
        return;
    }

    let body = match app.messages.get(app.detail_message_idx) {
        Some(msg) => msg.body.clone(),
        None => return,
    };

    // Apply same transformations as the UI does to get the actual displayed text
    let text = resolve_displayed_body(app, &body);
    let query_lower = app.detail_search_query.to_lowercase();

    for (i, line) in text.lines().enumerate() {
        if line.to_lowercase().contains(&query_lower) {
            app.detail_search_matches.push(i as u16);
        }
    }

    // Jump to first match
    if !app.detail_search_matches.is_empty() {
        let line = app.detail_search_matches[0];
        app.detail_scroll = line;
    }
}

/// Reproduce the same body text transformations as the UI draw code,
/// so search matches correspond to displayed lines.
fn resolve_displayed_body(app: &App, raw_body: &str) -> String {
    // Schema decode
    let msg_idx = app.detail_message_idx;
    let schema_body = if app.schema_decode_enabled && app.schema_client.is_some() {
        app.schema_decoded_cache.get(&msg_idx)
            .and_then(|r| r.as_ref().ok())
            .map(|d| d.decoded_body.clone())
    } else {
        None
    };

    let decoded = if let Some(body) = schema_body {
        body
    } else if app.detail_decoded {
        decode_payload_text(raw_body)
    } else {
        raw_body.to_string()
    };

    if app.detail_pretty {
        pretty_format_text(&decoded)
    } else {
        decoded
    }
}

/// Minimal decode logic mirroring ui::message_detail::decode_payload
fn decode_payload_text(body: &str) -> String {
    use base64::Engine;
    use std::io::Read;
    let trimmed = body.trim();
    if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(trimmed) {
        if bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b {
            let mut decoder = flate2::read::GzDecoder::new(&bytes[..]);
            let mut s = String::new();
            if decoder.read_to_string(&mut s).is_ok() { return s; }
        }
        if let Ok(text) = String::from_utf8(bytes) { return text; }
    }
    if let Ok(bytes) = base64::engine::general_purpose::URL_SAFE.decode(trimmed) {
        if let Ok(text) = String::from_utf8(bytes) { return text; }
    }
    body.to_string()
}

/// Minimal pretty-format mirroring ui::message_detail::pretty_format
fn pretty_format_text(body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
            if let Ok(pretty) = serde_json::to_string_pretty(&val) {
                return pretty;
            }
        }
    }
    body.to_string()
}
