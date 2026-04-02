use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, Focus, Popup, QueueTab, RightView};

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Fill bg
    frame.render_widget(
        Block::default().style(Style::default().bg(app.theme.bg)),
        area,
    );

    // Main layout: body + status bar
    let outer = Layout::vertical([
        Constraint::Min(1),    // body
        Constraint::Length(1), // status bar
    ])
    .split(area);

    // Body: sidebar | main panel
    let body = Layout::horizontal([
        Constraint::Length(20), // sidebar fixed width
        Constraint::Min(1),    // right panel
    ])
    .split(outer[0]);

    draw_sidebar(frame, app, body[0]);
    draw_right_panel(frame, app, body[1]);
    draw_status_bar(frame, app, outer[1]);

    if app.popup != Popup::None {
        super::popup::draw(frame, app);
    }
}

// ─── Sidebar ───────────────────────────────────────────────────────────────

fn draw_sidebar(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == Focus::Sidebar;
    let border_color = if focused {
        app.theme.accent
    } else {
        app.theme.divider
    };

    let block = Block::bordered()
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(app.theme.sidebar_bg));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Vertical layout inside sidebar
    let sections = Layout::vertical([
        Constraint::Length(2), // connection header
        Constraint::Length(1), // divider
        Constraint::Length(1), // "Navigation" header
        Constraint::Length(4), // nav items (Overview, Queues, Exchanges, Policies)
        Constraint::Length(1), // divider
        Constraint::Length(1), // "Admin" header
        Constraint::Length(2), // admin items (Virtual Hosts, Users)
        Constraint::Min(0),    // spacer
        Constraint::Length(2), // bottom info
    ])
    .split(inner);

    // 1. Connection section
    let loading_indicator = if app.loading { " ~" } else { "" };
    let cluster_display = if app.cluster_name.is_empty() {
        app.profile_name.as_str()
    } else {
        app.cluster_name.as_str()
    };

    let conn_lines = vec![
        Line::from(Span::styled(
            format!(" {} {}", app.profile_name, "\u{25be}"),
            Style::default().fg(app.theme.accent).bold(),
        )),
        Line::from(Span::styled(
            format!(" {}{}", cluster_display, loading_indicator),
            Style::default().fg(app.theme.muted),
        )),
    ];
    frame.render_widget(
        Paragraph::new(conn_lines).style(Style::default().bg(app.theme.sidebar_bg)),
        sections[0],
    );

    // Divider line
    draw_sidebar_divider(frame, app, sections[1]);

    // 2. "Navigation" header
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            " NAVIGATION",
            Style::default().fg(app.theme.muted),
        )))
        .style(Style::default().bg(app.theme.sidebar_bg)),
        sections[2],
    );

    // Navigation items: indices 0-3
    let nav_items = &App::sidebar_items()[0..4];
    let mut nav_lines: Vec<Line> = Vec::new();
    for (i, (label, _)) in nav_items.iter().enumerate() {
        let is_selected = app.sidebar_cursor == i;
        let (prefix, style) = if is_selected {
            (
                " \u{25b8} ",
                Style::default().fg(app.theme.accent).bold(),
            )
        } else {
            ("   ", Style::default().fg(app.theme.primary))
        };
        let bg = if is_selected && focused {
            app.theme.selected_bg
        } else {
            app.theme.sidebar_bg
        };
        nav_lines.push(Line::from(Span::styled(
            format!("{}{}", prefix, label),
            style.bg(bg),
        )));
    }
    frame.render_widget(
        Paragraph::new(nav_lines).style(Style::default().bg(app.theme.sidebar_bg)),
        sections[3],
    );

    // Divider line
    draw_sidebar_divider(frame, app, sections[4]);

    // 3. "Admin" header
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            " ADMIN",
            Style::default().fg(app.theme.muted),
        )))
        .style(Style::default().bg(app.theme.sidebar_bg)),
        sections[5],
    );

    // Admin items: indices 4-5
    let admin_items = &App::sidebar_items()[4..6];
    let mut admin_lines: Vec<Line> = Vec::new();
    for (i, (label, _)) in admin_items.iter().enumerate() {
        let cursor_idx = i + 4;
        let is_selected = app.sidebar_cursor == cursor_idx;
        let (prefix, style) = if is_selected {
            (
                " \u{25b8} ",
                Style::default().fg(app.theme.accent).bold(),
            )
        } else {
            ("   ", Style::default().fg(app.theme.primary))
        };
        let bg = if is_selected && focused {
            app.theme.selected_bg
        } else {
            app.theme.sidebar_bg
        };
        admin_lines.push(Line::from(Span::styled(
            format!("{}{}", prefix, label),
            style.bg(bg),
        )));
    }
    frame.render_widget(
        Paragraph::new(admin_lines).style(Style::default().bg(app.theme.sidebar_bg)),
        sections[6],
    );

    // Spacer (sections[7]) is empty, just fill bg
    frame.render_widget(
        Block::default().style(Style::default().bg(app.theme.sidebar_bg)),
        sections[7],
    );

    // 4. Bottom info: version + vhost
    let version_display = if app.rabbitmq_version.is_empty() {
        "RabbitMQ".to_string()
    } else {
        format!("RabbitMQ {}", app.rabbitmq_version)
    };
    let info_lines = vec![
        Line::from(Span::styled(
            format!(" {}", version_display),
            Style::default().fg(app.theme.muted),
        )),
        Line::from(Span::styled(
            format!(" vhost: {} \u{25be}", app.selected_vhost),
            Style::default().fg(app.theme.muted),
        )),
    ];
    frame.render_widget(
        Paragraph::new(info_lines).style(Style::default().bg(app.theme.sidebar_bg)),
        sections[8],
    );
}

fn draw_sidebar_divider(frame: &mut Frame, app: &App, area: Rect) {
    let width = area.width as usize;
    let divider_str = "\u{2500}".repeat(width);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            divider_str,
            Style::default().fg(app.theme.divider),
        )))
        .style(Style::default().bg(app.theme.sidebar_bg)),
        area,
    );
}

// ─── Right Panel ───────────────────────────────────────────────────────────

fn draw_right_panel(frame: &mut Frame, app: &mut App, area: Rect) {
    // Fill background
    frame.render_widget(
        Block::default().style(Style::default().bg(app.theme.bg)),
        area,
    );

    match app.right_view {
        RightView::Queues => draw_queues_panel(frame, app, area),
        _ => draw_placeholder(frame, app, area),
    }
}

fn draw_placeholder(frame: &mut Frame, app: &App, area: Rect) {
    let view_name = match app.right_view {
        RightView::Overview => "Overview",
        RightView::Queues => "Queues",
        RightView::Exchanges => "Exchanges",
        RightView::Policies => "Policies",
        RightView::Vhosts => "Virtual Hosts",
        RightView::Users => "Users",
    };

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            view_name,
            Style::default().fg(app.theme.white).bold(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Coming soon",
            Style::default().fg(app.theme.muted),
        )),
    ];

    frame.render_widget(
        Paragraph::new(text)
            .alignment(Alignment::Center)
            .style(Style::default().bg(app.theme.bg)),
        area,
    );
}

// ─── Queues Panel ──────────────────────────────────────────────────────────

fn draw_queues_panel(frame: &mut Frame, app: &mut App, area: Rect) {
    let sections = Layout::vertical([
        Constraint::Length(1), // R1: header
        Constraint::Length(1), // R2: tabs
        Constraint::Min(1),    // R3: content
    ])
    .split(area);

    draw_queue_header(frame, app, sections[0]);
    draw_queue_tabs(frame, app, sections[1]);
    draw_queue_content(frame, app, sections[2]);
}

fn draw_queue_header(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == Focus::RightHeader;

    let queue_name = if app.current_queue_name.is_empty() {
        "(select a queue)".to_string()
    } else {
        app.current_queue_name.clone()
    };

    let style = if focused {
        Style::default()
            .fg(app.theme.accent)
            .add_modifier(Modifier::UNDERLINED)
    } else {
        Style::default().fg(app.theme.white)
    };

    let line = Line::from(vec![
        Span::styled(" Queue: ", Style::default().fg(app.theme.muted).bg(app.theme.highlight_bg)),
        Span::styled(format!("{} \u{25be}", queue_name), style.bg(app.theme.highlight_bg)),
        Span::styled(
            format!(
                "  ({} queues, fetch: {})",
                app.filtered_indices.len(),
                app.fetch_count
            ),
            Style::default().fg(app.theme.muted).bg(app.theme.highlight_bg),
        ),
    ]);

    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(app.theme.highlight_bg)),
        area,
    );
}

fn draw_queue_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == Focus::RightTabs;

    let tabs = [
        ("Overview", QueueTab::Overview),
        ("Publish", QueueTab::Publish),
        ("Consume", QueueTab::Consume),
        ("Routing", QueueTab::Routing),
        ("Settings", QueueTab::Settings),
    ];

    let mut spans: Vec<Span> = vec![Span::styled(" ", Style::default().bg(app.theme.bg))];

    for (i, (label, variant)) in tabs.iter().enumerate() {
        let is_active = app.queue_tab == *variant;

        let style = if is_active {
            let mut s = Style::default().fg(app.theme.accent).bold().bg(app.theme.bg);
            if focused {
                s = s.add_modifier(Modifier::UNDERLINED);
            }
            s
        } else {
            Style::default().fg(app.theme.muted).bg(app.theme.bg)
        };

        spans.push(Span::styled(format!(" {} ", label), style));

        if i < tabs.len() - 1 {
            spans.push(Span::styled(
                "\u{2502}",
                Style::default().fg(app.theme.divider).bg(app.theme.bg),
            ));
        }
    }

    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(app.theme.bg)),
        area,
    );
}

fn draw_queue_content(frame: &mut Frame, app: &mut App, area: Rect) {
    match app.queue_tab {
        QueueTab::Consume => draw_consume(frame, app, area),
        _ => {
            let tab_name = match app.queue_tab {
                QueueTab::Overview => "Queue Overview",
                QueueTab::Publish => "Publish",
                QueueTab::Consume => "Consume",
                QueueTab::Routing => "Routing",
                QueueTab::Settings => "Settings",
            };

            // Show stats bar if we have a selected queue and we're on Overview
            if app.queue_tab == QueueTab::Overview {
                if let Some(q) = app.selected_queue() {
                    let label = Style::default().fg(app.theme.muted);
                    let val = Style::default().fg(app.theme.primary).bold();
                    let sep = Span::styled(" \u{2502} ", Style::default().fg(app.theme.divider));

                    let stats_line = Line::from(vec![
                        Span::styled("  \u{25c6} ", Style::default().fg(app.theme.accent)),
                        Span::styled("Msgs: ", label),
                        Span::styled(format!("{}", q.messages), val),
                        sep.clone(),
                        Span::styled("Pub: ", label),
                        Span::styled(format!("{:.1}/s", q.publish_rate), val),
                        sep.clone(),
                        Span::styled("Del: ", label),
                        Span::styled(format!("{:.1}/s", q.deliver_rate), val),
                        sep.clone(),
                        Span::styled("Ack: ", label),
                        Span::styled(format!("{:.1}/s", q.ack_rate), val),
                        sep.clone(),
                        Span::styled("Consumers: ", label),
                        Span::styled(format!("{}", q.consumers), val),
                        sep,
                        Span::styled(&q.state, Style::default().fg(app.theme.success)),
                    ]);

                    let text = vec![
                        Line::from(""),
                        stats_line,
                        Line::from(""),
                        Line::from(Span::styled(
                            "  More queue details coming soon",
                            Style::default().fg(app.theme.muted),
                        )),
                    ];

                    frame.render_widget(
                        Paragraph::new(text).style(Style::default().bg(app.theme.bg)),
                        area,
                    );
                    return;
                }
            }

            let text = vec![
                Line::from(""),
                Line::from(Span::styled(
                    tab_name,
                    Style::default().fg(app.theme.white).bold(),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Coming soon",
                    Style::default().fg(app.theme.muted),
                )),
            ];

            frame.render_widget(
                Paragraph::new(text)
                    .alignment(Alignment::Center)
                    .style(Style::default().bg(app.theme.bg)),
                area,
            );
        }
    }
}

// ─── Consume View (Messages) ───────────────────────────────────────────────

fn draw_consume(frame: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.focus == Focus::RightContent;
    let border_color = if focused {
        app.theme.accent
    } else {
        app.theme.divider
    };

    let title = if app.current_queue_name.is_empty() {
        " Consume ".to_string()
    } else {
        format!(" Consume \u{2014} {} ", app.current_queue_name)
    };

    let block = Block::bordered()
        .title(title)
        .title_style(Style::default().fg(app.theme.white).bold())
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(app.theme.bg));

    if app.messages.is_empty() {
        let empty = if app.loading {
            "  Loading..."
        } else if app.current_queue_name.is_empty() {
            "  Select a queue and press Enter to peek"
        } else {
            "  No messages in this queue"
        };

        frame.render_widget(
            Paragraph::new(Span::styled(
                empty,
                Style::default().fg(app.theme.muted),
            ))
            .block(block),
            area,
        );
        return;
    }

    let mut lines: Vec<Line> = Vec::new();

    for msg in &app.messages {
        let ts = msg
            .timestamp
            .map(format_timestamp)
            .unwrap_or_else(|| "no timestamp".into());

        // Message header: #N  timestamp  key=routing_key
        lines.push(Line::from(vec![
            Span::styled(
                format!("  #{}", msg.index),
                Style::default().fg(app.theme.accent).bold(),
            ),
            Span::styled("  ", Style::default()),
            Span::styled(ts, Style::default().fg(app.theme.muted)),
            Span::styled("  key=", Style::default().fg(app.theme.muted)),
            Span::styled(
                msg.routing_key.clone(),
                Style::default().fg(app.theme.primary),
            ),
        ]));

        // Body preview (max 3 lines)
        let formatted = format_message_body(&msg.body);
        for body_line in formatted.lines().take(3) {
            lines.push(Line::from(Span::styled(
                format!("    {}", body_line),
                Style::default().fg(app.theme.primary),
            )));
        }
        if formatted.lines().count() > 3 {
            lines.push(Line::from(Span::styled(
                "    ...",
                Style::default().fg(app.theme.muted),
            )));
        }

        // Separator
        lines.push(Line::from(Span::styled(
            format!(
                "  {}",
                "\u{2500}".repeat(area.width.saturating_sub(6) as usize)
            ),
            Style::default().fg(app.theme.divider),
        )));
    }

    let total = lines.len() as u16;
    let visible = area.height.saturating_sub(2);
    let scroll_info = if total > visible {
        format!(
            " scroll: {}/{} ",
            app.message_scroll + 1,
            total.saturating_sub(visible) + 1
        )
    } else {
        String::new()
    };

    let block = block.title_bottom(
        Line::from(Span::styled(
            scroll_info,
            Style::default().fg(app.theme.muted),
        ))
        .right_aligned(),
    );

    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .scroll((app.message_scroll, 0)),
        area,
    );
}

// ─── Status Bar ────────────────────────────────────────────────────────────

fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let (status_text, status_color) = if !app.status_message.is_empty() {
        let c = if app.status_is_error {
            app.theme.error
        } else {
            app.theme.success
        };
        (app.status_message.as_str(), c)
    } else {
        ("", app.theme.muted)
    };

    let ks = Style::default().fg(app.theme.accent).bold();
    let ds = Style::default().fg(app.theme.muted);

    let line = Line::from(vec![
        Span::styled(" ", Style::default()),
        Span::styled("tab", ks),
        Span::styled(":panel ", ds),
        Span::styled("j/k", ks),
        Span::styled(":nav ", ds),
        Span::styled("\u{23ce}", ks),
        Span::styled(":select ", ds),
        Span::styled("/", ks),
        Span::styled(":filter ", ds),
        Span::styled("r", ks),
        Span::styled(":reload ", ds),
        Span::styled("+/-", ks),
        Span::styled(":fetch ", ds),
        Span::styled("?", ks),
        Span::styled(":help ", ds),
        Span::styled("q", ks),
        Span::styled(":quit", ds),
        Span::styled("  \u{2502} ", Style::default().fg(app.theme.divider)),
        Span::styled(status_text, Style::default().fg(status_color)),
    ]);

    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(app.theme.sidebar_bg)),
        area,
    );
}

// ─── Helpers ───────────────────────────────────────────────────────────────

fn format_message_body(body: &str) -> String {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(body) {
        serde_json::to_string_pretty(&value).unwrap_or_else(|_| body.to_string())
    } else {
        body.to_string()
    }
}

fn format_timestamp(ts: i64) -> String {
    let remaining = ts % 86400;
    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;
    let seconds = remaining % 60;
    let mut y = 1970i64;
    let mut d = ts / 86400;
    loop {
        let diy = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
            366
        } else {
            365
        };
        if d < diy {
            break;
        }
        d -= diy;
        y += 1;
    }
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let mds = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut m = 0usize;
    for md in &mds {
        if d < *md as i64 {
            break;
        }
        d -= *md as i64;
        m += 1;
    }
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        y,
        m + 1,
        d + 1,
        hours,
        minutes,
        seconds
    )
}
