use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, Popup};

pub fn draw(frame: &mut Frame, app: &mut App) {
    match &app.popup {
        Popup::Help => draw_help(frame, app),
        Popup::ProfileSwitch => draw_profile_switch(frame, app),
        Popup::NamespacePicker => draw_namespace_picker(frame, app),
        Popup::FetchCount => draw_fetch_count(frame, app),
        Popup::None => {}
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let v = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ]).split(area);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ]).split(v[1])[1]
}

fn draw_help(frame: &mut Frame, app: &App) {
    let popup_area = centered_rect(60, 60, frame.area());
    frame.render_widget(Clear, popup_area);

    let block = Block::bordered()
        .title(" Keyboard Shortcuts ")
        .title_style(Style::default().fg(app.theme.accent).bold())
        .border_style(Style::default().fg(app.theme.accent))
        .style(Style::default().bg(app.theme.bg));

    let shortcuts = vec![
        ("j/k ↑/↓", "Navigate lists"),
        ("enter", "Select / open detail"),
        ("/", "Filter"),
        ("r", "Reload"),
        ("v", "Switch vhost/namespace"),
        ("p", "Switch profile"),
        ("+/-", "Adjust fetch count"),
        ("c", "Copy payload (detail)"),
        ("h", "Copy headers (detail)"),
        ("P", "Toggle pretty (detail)"),
        ("esc", "Go back"),
        ("?", "Toggle help"),
        ("q", "Quit"),
    ];

    let mut lines: Vec<Line> = vec![Line::from("")];
    for (key, desc) in &shortcuts {
        lines.push(Line::from(vec![
            Span::styled(format!("  {:<14}", key), Style::default().fg(app.theme.accent).bold()),
            Span::styled(*desc, Style::default().fg(app.theme.primary)),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("  Press ? or Esc to close", Style::default().fg(app.theme.muted))));

    frame.render_widget(
        Paragraph::new(lines).block(block).style(Style::default().bg(app.theme.bg)),
        popup_area,
    );
}

fn draw_profile_switch(frame: &mut Frame, app: &mut App) {
    let popup_area = centered_rect(50, 50, frame.area());
    frame.render_widget(Clear, popup_area);

    let block = Block::bordered()
        .title(" Switch Profile ")
        .title_style(Style::default().fg(app.theme.accent).bold())
        .border_style(Style::default().fg(app.theme.accent))
        .style(Style::default().bg(app.theme.bg));

    let names = app.config.profile_names();
    let items: Vec<ListItem> = names.iter().map(|name| {
        let is_current = name == &app.profile_name;
        let display = if is_current { format!("* {}", name) } else { format!("  {}", name) };
        let style = if is_current { Style::default().fg(app.theme.accent) } else { Style::default().fg(app.theme.primary) };
        ListItem::new(Line::from(Span::styled(display, style)))
    }).collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(app.theme.selected_bg).fg(app.theme.white).add_modifier(Modifier::BOLD))
        .highlight_symbol("▸ ")
        .style(Style::default().bg(app.theme.bg));

    frame.render_stateful_widget(list, popup_area, &mut app.popup_list_state);
}

fn draw_namespace_picker(frame: &mut Frame, app: &mut App) {
    let popup_area = centered_rect(40, 50, frame.area());
    frame.render_widget(Clear, popup_area);

    let block = Block::bordered()
        .title(" Select Namespace ")
        .title_style(Style::default().fg(app.theme.accent).bold())
        .border_style(Style::default().fg(app.theme.accent))
        .style(Style::default().bg(app.theme.bg));

    let items: Vec<ListItem> = app.namespaces.iter().map(|ns| {
        let is_current = ns == &app.selected_namespace;
        let display = if is_current { format!("* {}", ns) } else { format!("  {}", ns) };
        let style = if is_current { Style::default().fg(app.theme.accent) } else { Style::default().fg(app.theme.primary) };
        ListItem::new(Line::from(Span::styled(display, style)))
    }).collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(app.theme.selected_bg).fg(app.theme.white).add_modifier(Modifier::BOLD))
        .highlight_symbol("▸ ")
        .style(Style::default().bg(app.theme.bg));

    frame.render_stateful_widget(list, popup_area, &mut app.popup_list_state);
}

fn draw_fetch_count(frame: &mut Frame, app: &mut App) {
    use crate::app::FETCH_PRESETS;

    let popup_area = centered_rect(30, 40, frame.area());
    frame.render_widget(Clear, popup_area);

    let block = Block::bordered()
        .title(" Fetch Count ")
        .title_style(Style::default().fg(app.theme.accent).bold())
        .border_style(Style::default().fg(app.theme.accent))
        .style(Style::default().bg(app.theme.bg));

    let items: Vec<ListItem> = FETCH_PRESETS.iter().map(|&count| {
        let is_current = count == app.fetch_count;
        let label = if is_current {
            format!("* {} messages", count)
        } else {
            format!("  {} messages", count)
        };
        let st = if is_current {
            Style::default().fg(app.theme.accent).bold()
        } else {
            Style::default().fg(app.theme.primary)
        };
        ListItem::new(Line::from(Span::styled(label, st)))
    }).collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default().bg(app.theme.selected_bg).fg(app.theme.white).add_modifier(Modifier::BOLD)
        )
        .highlight_symbol("▸ ");

    frame.render_stateful_widget(list, popup_area, &mut app.popup_list_state);
}
