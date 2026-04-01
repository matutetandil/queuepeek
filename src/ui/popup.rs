use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, Popup};

pub fn draw(frame: &mut Frame, app: &mut App) {
    match &app.popup {
        Popup::Help => draw_help(frame, app),
        Popup::ProfileSwitch => draw_profile_switch(frame, app),
        Popup::VhostPicker => draw_vhost_picker(frame, app),
        Popup::QueuePicker => draw_queue_picker(frame, app),
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
        ("tab/S-tab", "Cycle focus forward/back"),
        ("enter", "Open picker / peek messages"),
        ("/", "Filter queues (from queue selector)"),
        ("j/k ↑/↓", "Navigate lists / scroll messages"),
        ("r", "Reload messages"),
        ("R", "Reload queues"),
        ("v", "Quick switch vhost"),
        ("p", "Switch profile"),
        ("+/-", "Adjust fetch count"),
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

fn draw_vhost_picker(frame: &mut Frame, app: &mut App) {
    let popup_area = centered_rect(40, 50, frame.area());
    frame.render_widget(Clear, popup_area);

    let block = Block::bordered()
        .title(" Select Vhost ")
        .title_style(Style::default().fg(app.theme.accent).bold())
        .border_style(Style::default().fg(app.theme.accent))
        .style(Style::default().bg(app.theme.bg));

    let items: Vec<ListItem> = app.vhosts.iter().map(|v| {
        let is_current = v == &app.selected_vhost;
        let display = if is_current { format!("* {}", v) } else { format!("  {}", v) };
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

fn draw_queue_picker(frame: &mut Frame, app: &mut App) {
    let popup_area = centered_rect(60, 70, frame.area());
    frame.render_widget(Clear, popup_area);

    // Title with filter
    let title = if app.picker_filter_active {
        format!(" Select Queue [/{}▎] ", app.picker_filter)
    } else if !app.picker_filter.is_empty() {
        format!(" Select Queue [/{}] ", app.picker_filter)
    } else {
        " Select Queue [/ to filter] ".to_string()
    };

    let block = Block::bordered()
        .title(title)
        .title_style(Style::default().fg(app.theme.accent).bold())
        .border_style(Style::default().fg(app.theme.accent))
        .style(Style::default().bg(app.theme.bg));

    let items: Vec<ListItem> = app.filtered_indices.iter().map(|&idx| {
        let queue = &app.queues[idx];
        let is_current = queue.name == app.current_queue_name;

        let count_style = if queue.messages > 1000 {
            Style::default().fg(app.theme.error).bold()
        } else if queue.messages == 0 {
            Style::default().fg(app.theme.success)
        } else {
            Style::default().fg(app.theme.accent)
        };

        let name_style = if is_current {
            Style::default().fg(app.theme.accent).bold()
        } else {
            Style::default().fg(app.theme.primary)
        };

        ListItem::new(Line::from(vec![
            Span::styled(if is_current { "* " } else { "  " }, name_style),
            Span::styled(&queue.name, name_style),
            Span::styled(" ", Style::default()),
            Span::styled(format!("({})", queue.messages), count_style),
            Span::styled(
                format!("  {} consumers", queue.consumers),
                Style::default().fg(app.theme.muted),
            ),
        ]))
    }).collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(app.theme.selected_bg).fg(app.theme.white).add_modifier(Modifier::BOLD))
        .highlight_symbol("▸ ")
        .style(Style::default().bg(app.theme.bg));

    frame.render_stateful_widget(list, popup_area, &mut app.popup_list_state);
}
