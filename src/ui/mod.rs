pub mod theme;
pub mod profiles;
pub mod main_view;
pub mod popup;

use ratatui::Frame;
use crate::app::{App, Screen};

pub fn draw(frame: &mut Frame, app: &mut App) {
    match app.screen {
        Screen::ProfileSelect => profiles::draw(frame, app),
        Screen::Main => main_view::draw(frame, app),
    }
}
