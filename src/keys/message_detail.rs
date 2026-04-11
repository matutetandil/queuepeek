use crossterm::event::{KeyCode, KeyModifiers};

use crate::app::{self, App, Popup, Screen};
use crate::operations;
use crate::utils;

pub fn handle_message_detail_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    if app.popup != Popup::None {
        super::popup::handle_popup_key(app, code, modifiers);
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
        }
        KeyCode::Char('b') => {
            app.detail_decoded = !app.detail_decoded;
            if app.detail_decoded {
                app.set_status("Binary decode ON (base64/gzip)", false);
            } else {
                app.set_status("Binary decode OFF", false);
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
        KeyCode::Esc => {
            app.screen = Screen::MessageList;
        }
        _ => {}
    }
}
