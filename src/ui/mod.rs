pub mod theme;
pub mod profiles;
pub mod queue_list;
pub mod message_list;
pub mod message_detail;
pub mod exchange_list;
pub mod popup;

use ratatui::prelude::*;
use ratatui::widgets::Paragraph;
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

/// Draw version tag on the right side of the header bar.
/// `header_content_width` is the character width of the left-side content;
/// the tag is only rendered if there's enough room with at least 2 chars gap.
pub fn draw_version_tag(frame: &mut Frame, app: &App, area: Rect, header_content_width: u16) {
    let version = format!("queuepeek v{} ", env!("CARGO_PKG_VERSION"));
    let width = version.len() as u16;
    // Only show if there's a 2-char gap between header content and version
    if header_content_width + width + 2 <= area.width {
        let x = area.x + area.width - width;
        let tag_area = Rect::new(x, area.y, width, 1);
        let style = Style::default().fg(app.theme.muted).bg(app.theme.sidebar_bg);
        frame.render_widget(Paragraph::new(Span::styled(version, style)), tag_area);
    }
}

/// Returns a pulsing green dot span to indicate live/auto-refresh is active.
/// Alternates between bright and dim every ~500ms.
pub fn live_pulse_span(app: &App) -> Span<'static> {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let bright = (ms / 500).is_multiple_of(2);
    let color = if bright { app.theme.success } else { app.theme.muted };
    Span::styled(" ●", Style::default().fg(color).bg(app.theme.sidebar_bg))
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    match app.screen {
        Screen::ProfileSelect => profiles::draw(frame, app),
        Screen::QueueList => queue_list::draw(frame, app),
        Screen::MessageList => message_list::draw(frame, app),
        Screen::MessageDetail => message_detail::draw(frame, app),
        Screen::ExchangeList => exchange_list::draw(frame, app),
    }
}
