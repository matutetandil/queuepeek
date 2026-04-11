mod profile;
mod queue_list;
mod message_list;
mod message_detail;
mod popup;

use crossterm::event::{KeyCode, KeyModifiers};

use crate::app::{App, Popup, Screen};
use crate::updater;

pub fn handle_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    // Ctrl+C always quits
    if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
        app.should_quit = true;
        return;
    }

    // Global: U to trigger update (only when update is available and no popup open)
    if code == KeyCode::Char('U') && app.popup == Popup::None && app.update_checker.update_available {
        app.set_status("Updating...", false);
        match updater::perform_update() {
            Ok(msg) => app.set_status(msg, false),
            Err(e) => app.set_status(format!("Update failed: {}", e), true),
        }
        return;
    }

    match app.screen {
        Screen::ProfileSelect => profile::handle_profile_key(app, code, modifiers),
        Screen::QueueList     => queue_list::handle_queue_list_key(app, code, modifiers),
        Screen::MessageList   => message_list::handle_message_list_key(app, code, modifiers),
        Screen::MessageDetail => message_detail::handle_message_detail_key(app, code, modifiers),
    }
}
