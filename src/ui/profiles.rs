use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::{App, ProfileForm, ProfileMode};

const LOGO: &str = r#"
 ██████╗ ██╗   ██╗███████╗██╗   ██╗███████╗██████╗ ███████╗███████╗██╗  ██╗
██╔═══██╗██║   ██║██╔════╝██║   ██║██╔════╝██╔══██╗██╔════╝██╔════╝██║ ██╔╝
██║   ██║██║   ██║█████╗  ██║   ██║█████╗  ██████╔╝█████╗  █████╗  █████╔╝
██║▄▄ ██║██║   ██║██╔══╝  ██║   ██║██╔══╝  ██╔═══╝ ██╔══╝  ██╔══╝  ██╔═██╗
╚██████╔╝╚██████╔╝███████╗╚██████╔╝███████╗██║     ███████╗███████╗██║  ██╗
 ╚══▀▀═╝  ╚═════╝ ╚══════╝ ╚═════╝ ╚══════╝╚═╝     ╚══════╝╚══════╝╚═╝  ╚═╝"#;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let theme = app.theme;
    let full_area = frame.area();

    // Center a box on screen
    let box_width = 82u16.min(full_area.width.saturating_sub(4));
    let box_height = 30u16.min(full_area.height.saturating_sub(2));
    let x = (full_area.width.saturating_sub(box_width)) / 2;
    let y = (full_area.height.saturating_sub(box_height)) / 2;
    let area = Rect::new(x, y, box_width, box_height);

    // Clear the full screen with background color
    frame.render_widget(Clear, full_area);
    let bg_fill = Block::default().style(Style::default().bg(theme.bg));
    frame.render_widget(bg_fill, full_area);

    // Render outer block with border
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.divider).bg(theme.bg))
        .style(Style::default().bg(theme.bg));
    frame.render_widget(outer_block, area);

    let inner = Rect::new(area.x + 2, area.y + 1, area.width.saturating_sub(4), area.height.saturating_sub(2));

    // Render logo
    let logo_lines: Vec<Line> = LOGO
        .lines()
        .map(|l| Line::from(Span::styled(l, Style::default().fg(theme.accent).bg(theme.bg))))
        .collect();
    let logo_height = logo_lines.len() as u16;
    let logo_area = Rect::new(inner.x, inner.y, inner.width, logo_height);
    let logo_widget = Paragraph::new(logo_lines)
        .alignment(ratatui::layout::Alignment::Center)
        .style(Style::default().bg(theme.bg));
    frame.render_widget(logo_widget, logo_area);

    // Version tag below logo
    let version_text = format!("v{}", env!("CARGO_PKG_VERSION"));
    let version_area = Rect::new(inner.x, inner.y + logo_height, inner.width, 1);
    frame.render_widget(
        Paragraph::new(Span::styled(version_text, Style::default().fg(theme.muted).bg(theme.bg)))
            .alignment(ratatui::layout::Alignment::Center),
        version_area,
    );

    // Content area below logo + version
    let content_y = inner.y + logo_height + 1;
    let content_height = inner.height.saturating_sub(logo_height + 3); // +3 for gap and 2-line footer
    let content_area = Rect::new(inner.x, content_y, inner.width, content_height);

    match &app.profile_mode {
        ProfileMode::Select | ProfileMode::ConfirmDelete => {
            draw_profile_list(frame, app, content_area);
        }
        ProfileMode::Add | ProfileMode::Edit(_) => {
            draw_profile_form(frame, app, content_area);
        }
    }

    // Footer (2 lines)
    let footer_y = inner.y + inner.height.saturating_sub(2);
    let footer_area = Rect::new(inner.x, footer_y, inner.width, 2);
    let ks = Style::default().fg(theme.accent).add_modifier(Modifier::BOLD).bg(theme.bg);
    let ds = Style::default().fg(theme.muted).bg(theme.bg);
    let shortcut_line = match &app.profile_mode {
        ProfileMode::Select => {
            Line::from(vec![
                Span::styled("j/k", ks), Span::styled(":nav ", ds),
                Span::styled("⏎", ks), Span::styled(":connect ", ds),
                Span::styled("a", ks), Span::styled(":add ", ds),
                Span::styled("e", ks), Span::styled(":edit ", ds),
                Span::styled("d", ks), Span::styled(":del ", ds),
                Span::styled("t", ks), Span::styled(":theme ", ds),
                Span::styled("?", ks), Span::styled(":help ", ds),
                Span::styled("q", ks), Span::styled(":quit", ds),
            ])
        }
        ProfileMode::ConfirmDelete => Line::from(vec![
            Span::styled("Delete? ", ds),
            Span::styled("y", ks), Span::styled("/", ds), Span::styled("n", ks),
        ]),
        ProfileMode::Add | ProfileMode::Edit(_) => {
            let enter_hint = match app.profile_form.focused_field {
                0 => ":select type ",
                7 => ":toggle ",
                _ => ":save ",
            };
            Line::from(vec![
                Span::styled("tab", ks), Span::styled(":next ", ds),
                Span::styled("⏎", ks), Span::styled(enter_hint, ds),
                Span::styled("esc", ks), Span::styled(":cancel", ds),
            ])
        }
    };

    // Line 2: update hints + status
    let mut status_spans: Vec<Span> = Vec::new();
    status_spans.extend(super::update_hint_spans(app));
    if !app.status_message.is_empty() {
        let status_color = if app.status_is_error { theme.error } else { theme.success };
        if !status_spans.is_empty() {
            status_spans.push(Span::styled("  │ ", Style::default().fg(theme.divider).bg(theme.bg)));
        }
        status_spans.push(Span::styled(
            app.status_message.as_str(),
            Style::default().fg(status_color).bg(theme.bg),
        ));
    }
    let status_line = Line::from(status_spans);

    let footer_text = ratatui::text::Text::from(vec![shortcut_line, status_line]);
    let footer = Paragraph::new(footer_text).style(Style::default().bg(theme.bg));
    frame.render_widget(footer, footer_area);

    // Popups on top
    if app.popup != crate::app::Popup::None {
        super::popup::draw(frame, app);
    }
}

fn draw_profile_list(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = app.theme;
    let names = app.config.profile_names();

    if names.is_empty() {
        let empty_msg = Paragraph::new(Line::from(Span::styled(
            "No profiles configured. Press 'a' to add one.",
            Style::default().fg(theme.muted).bg(theme.bg),
        )))
        .style(Style::default().bg(theme.bg));
        frame.render_widget(empty_msg, area);
        return;
    }

    let items: Vec<ListItem> = names
        .iter()
        .map(|name| {
            let description = if let Some(profile) = app.config.profiles.get(name) {
                format!("{}:{}", profile.host, profile.port)
            } else {
                String::new()
            };
            let line = Line::from(vec![
                Span::styled(
                    format!("  {}  ", name),
                    Style::default().fg(theme.primary).bg(theme.bg),
                ),
                Span::styled(
                    description,
                    Style::default().fg(theme.muted).bg(theme.bg),
                ),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .fg(theme.white)
                .bg(theme.selected_bg)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(theme.bg));

    frame.render_stateful_widget(list, area, &mut app.profile_list_state);
}

fn draw_profile_form(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = app.theme;
    let form = &app.profile_form;

    let title = match &app.profile_mode {
        ProfileMode::Add => "Add Profile",
        ProfileMode::Edit(_) => "Edit Profile",
        _ => "",
    };

    let title_line = Line::from(Span::styled(
        title,
        Style::default().fg(theme.accent).bg(theme.bg).add_modifier(Modifier::BOLD),
    ));
    let title_area = Rect::new(area.x, area.y, area.width, 1);
    frame.render_widget(
        Paragraph::new(title_line).style(Style::default().bg(theme.bg)),
        title_area,
    );

    let fields_y = area.y + 2;
    let field_count = ProfileForm::field_count();

    for i in 0..field_count {
        let y = fields_y + (i as u16) * 2;
        if y + 1 >= area.y + area.height {
            break;
        }

        let label = ProfileForm::field_label(i);
        let value = form.field_value(i);
        let is_focused = form.focused_field == i;

        // Label
        let label_style = if is_focused {
            Style::default().fg(theme.accent).bg(theme.bg)
        } else {
            Style::default().fg(theme.muted).bg(theme.bg)
        };
        let label_area = Rect::new(area.x, y, area.width, 1);
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!("{}:", label),
                label_style,
            )))
            .style(Style::default().bg(theme.bg)),
            label_area,
        );

        // Value
        let display_value = if i == 5 && !value.is_empty() {
            // Mask password
            "*".repeat(value.len())
        } else {
            value.clone()
        };

        let value_text = if is_focused {
            format!("{}_", display_value)
        } else {
            display_value
        };

        let value_style = if is_focused {
            Style::default().fg(theme.white).bg(theme.bg)
        } else {
            Style::default().fg(theme.primary).bg(theme.bg)
        };

        let value_area = Rect::new(area.x + 2, y + 1, area.width.saturating_sub(2), 1);
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(value_text, value_style)))
                .style(Style::default().bg(theme.bg)),
            value_area,
        );
    }

    // Form error
    if !form.error.is_empty() {
        let err_y = fields_y + (field_count as u16) * 2;
        if err_y < area.y + area.height {
            let err_area = Rect::new(area.x, err_y, area.width, 1);
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    &form.error,
                    Style::default().fg(theme.error).bg(theme.bg),
                )))
                .style(Style::default().bg(theme.bg)),
                err_area,
            );
        }
    }
}
