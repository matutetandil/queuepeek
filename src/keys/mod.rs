mod profile;
mod queue_list;
mod message_list;
mod message_detail;
mod exchange_list;
pub mod popup;

use crossterm::event::{KeyCode, KeyModifiers};

use crate::app::{App, Popup, Screen};

pub fn handle_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    // Ctrl+C always quits
    if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
        app.should_quit = true;
        return;
    }

    // Global: U to trigger update (only when update is available and no popup open)
    if code == KeyCode::Char('U') && app.popup == Popup::None && app.update_checker.update_available {
        let version = app.update_checker.latest_version.clone().unwrap_or_else(|| "new".into());
        app.popup = Popup::ConfirmUpdate;
        app.set_status(format!("v{} available", version), false);
        return;
    }

    match app.screen {
        Screen::ProfileSelect => profile::handle_profile_key(app, code, modifiers),
        Screen::QueueList     => queue_list::handle_queue_list_key(app, code, modifiers),
        Screen::MessageList   => message_list::handle_message_list_key(app, code, modifiers),
        Screen::MessageDetail => message_detail::handle_message_detail_key(app, code, modifiers),
        Screen::ExchangeList  => exchange_list::handle_exchange_list_key(app, code, modifiers),
    }
}
