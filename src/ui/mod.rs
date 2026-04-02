pub mod theme;
pub mod profiles;
pub mod queue_list;
pub mod message_list;
pub mod message_detail;
pub mod popup;

use ratatui::Frame;
use crate::app::{App, Screen};

pub fn draw(frame: &mut Frame, app: &mut App) {
    match app.screen {
        Screen::ProfileSelect => profiles::draw(frame, app),
        Screen::QueueList => queue_list::draw(frame, app),
        Screen::MessageList => message_list::draw(frame, app),
        Screen::MessageDetail => message_detail::draw(frame, app),
    }
}
