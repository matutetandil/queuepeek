pub mod theme;
pub mod profiles;
pub mod queue_list;
pub mod message_list;
pub mod message_detail;
pub mod popup;

use ratatui::Frame;
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use crate::app::{App, Screen};

pub fn update_hint_spans(app: &App) -> Vec<Span<'static>> {
    if !app.update_checker.update_available {
        return Vec::new();
    }
    let version = app.update_checker.latest_version.as_deref().unwrap_or("new");
    let ks = Style::default().fg(app.theme.success).add_modifier(Modifier::BOLD);
    let ds = Style::default().fg(app.theme.muted);
    vec![
        Span::styled(" │ ", ds),
        Span::styled(format!("v{} available ", version), ks),
        Span::styled("U", Style::default().fg(app.theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled(":update", ds),
    ]
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    match app.screen {
        Screen::ProfileSelect => profiles::draw(frame, app),
        Screen::QueueList => queue_list::draw(frame, app),
        Screen::MessageList => message_list::draw(frame, app),
        Screen::MessageDetail => message_detail::draw(frame, app),
    }
}
