use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, Popup};

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Fill bg
    frame.render_widget(
        Block::default().style(Style::default().bg(app.theme.bg)),
        area,
    );

    let chunks = Layout::vertical([
        Constraint::Length(1), // header bar
        Constraint::Length(1), // filter bar
        Constraint::Min(3),    // queue list
        Constraint::Length(1), // footer/status
    ])
    .split(area);

    draw_header(frame, app, chunks[0]);
    draw_filter(frame, app, chunks[1]);
    draw_list(frame, app, chunks[2]);
    draw_footer(frame, app, chunks[3]);

    // Popups on top
    if app.popup != Popup::None {
        super::popup::draw(frame, app);
    }
}

// ─── Header Bar ───────────────────────────────────────────────────────────

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let loading = if app.loading { " ⟳" } else { "" };
    let count = app.filtered_queue_indices.len();
    let sep = Span::styled(" › ", Style::default().fg(app.theme.divider).bg(app.theme.sidebar_bg));
    let muted = Style::default().fg(app.theme.muted).bg(app.theme.sidebar_bg);

    let line = Line::from(vec![
        Span::styled(format!("  {} ", app.profile_name), muted),
        sep,
        Span::styled(
            format!("{} ", app.selected_namespace),
            Style::default().fg(app.theme.white).bold().bg(app.theme.sidebar_bg),
        ),
        Span::styled(format!("({} queues){}", count, loading), muted),
    ]);

    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(app.theme.sidebar_bg)),
        area,
    );
}

// ─── Filter Bar ───────────────────────────────────────────────────────────

fn draw_filter(frame: &mut Frame, app: &App, area: Rect) {
    let line = if app.queue_filter_active || !app.queue_filter.is_empty() {
        let cursor = if app.queue_filter_focused { "▎" } else { "" };
        let slash_style = if app.queue_filter_focused {
            Style::default().fg(app.theme.accent).bold()
        } else {
            Style::default().fg(app.theme.muted)
        };
        Line::from(vec![
            Span::styled(" / ", slash_style),
            Span::styled(
                format!("{}{}", app.queue_filter, cursor),
                Style::default().fg(app.theme.primary),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                " / type to filter...",
                Style::default().fg(app.theme.muted),
            ),
        ])
    };

    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(app.theme.highlight_bg)),
        area,
    );
}

// ─── Queue List ───────────────────────────────────────────────────────────

fn draw_list(frame: &mut Frame, app: &mut App, area: Rect) {
    let border_color = app.theme.accent;

    let title = if app.loading {
        " Queues (loading...) ".to_string()
    } else {
        " Queues ".to_string()
    };

    let block = Block::bordered()
        .title(title)
        .title_style(Style::default().fg(app.theme.white).bold())
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(app.theme.bg));

    if app.filtered_queue_indices.is_empty() {
        let empty_text = if app.loading {
            "  Loading queues..."
        } else if !app.queue_filter.is_empty() {
            "  No queues match the filter"
        } else {
            "  No queues found"
        };

        frame.render_widget(
            Paragraph::new(Span::styled(
                empty_text,
                Style::default().fg(app.theme.muted),
            ))
            .block(block),
            area,
        );
        return;
    }

    let inner_width = area.width.saturating_sub(2) as usize; // account for borders

    let items: Vec<ListItem> = app
        .filtered_queue_indices
        .iter()
        .map(|&idx| {
            let q = &app.queues[idx];

            let msg_color = if q.messages > 1000 {
                app.theme.error
            } else if q.messages == 0 {
                app.theme.success
            } else {
                app.theme.accent
            };

            let msg_text = format!("({})", q.messages);
            let rate_text = if q.publish_rate > 0.0 || q.deliver_rate > 0.0 {
                format!(" ↑{:.0}/s ↓{:.0}/s", q.publish_rate, q.deliver_rate)
            } else {
                String::new()
            };
            let consumers_text = format!("{}c", q.consumers);

            // Sparkline from rate history
            let sparkline_width = 8;
            let sparkline_str = app.rate_history.get(&q.name)
                .map(|h| h.sparkline_str(sparkline_width))
                .unwrap_or_else(|| " ".repeat(sparkline_width));
            let has_activity = sparkline_str.trim().len() > 0;

            // Right side stats
            let right = format!("  {}{}  {}  {}  {}", msg_text, rate_text, consumers_text, q.state, sparkline_str);
            let right_len = right.len();

            // Left side: queue name, truncated if needed
            let max_name_len = inner_width.saturating_sub(right_len + 2);
            let name = if q.name.len() > max_name_len {
                format!("{}…", &q.name[..max_name_len.saturating_sub(1)])
            } else {
                q.name.clone()
            };

            let padding = inner_width.saturating_sub(name.len() + right_len + 2);

            let line = Line::from(vec![
                Span::styled(
                    format!("  {}", name),
                    Style::default().fg(app.theme.primary),
                ),
                Span::styled(
                    " ".repeat(padding),
                    Style::default(),
                ),
                Span::styled(
                    msg_text,
                    Style::default().fg(msg_color).bold(),
                ),
                Span::styled(
                    rate_text,
                    Style::default().fg(app.theme.muted),
                ),
                Span::styled(
                    format!("  {}", consumers_text),
                    Style::default().fg(app.theme.primary),
                ),
                Span::styled(
                    format!("  {}", q.state),
                    Style::default().fg(if q.state == "running" { app.theme.success } else { app.theme.muted }),
                ),
                Span::styled(
                    format!(" {}", sparkline_str),
                    Style::default().fg(if has_activity { app.theme.accent } else { app.theme.muted }),
                ),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(app.theme.selected_bg)
                .fg(app.theme.white)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▸ ");

    frame.render_stateful_widget(list, area, &mut app.queue_list_state);
}

// ─── Footer Bar ───────────────────────────────────────────────────────────

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let ks = Style::default().fg(app.theme.accent).bold();
    let ds = Style::default().fg(app.theme.muted);

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

    let bt = app.current_backend_type();
    let mut spans = vec![
        Span::styled(" ", Style::default()),
        Span::styled("j/k", ks),
        Span::styled(":nav ", ds),
        Span::styled("⏎", ks),
        Span::styled(":open ", ds),
        Span::styled("P", ks),
        Span::styled(":publish ", ds),
        Span::styled("C", ks),
        Span::styled(":copy ", ds),
        Span::styled("m", ks),
        Span::styled(":move ", ds),
        Span::styled("x", ks),
        Span::styled(":purge ", ds),
        Span::styled("D", ks),
        Span::styled(":del ", ds),
        Span::styled("i", ks),
        Span::styled(":info ", ds),
        Span::styled("=", ks),
        Span::styled(":compare ", ds),
    ];
    if bt == "kafka" {
        spans.extend([Span::styled("G", ks), Span::styled(":groups ", ds)]);
    }
    if bt == "rabbitmq" {
        spans.extend([Span::styled("X", ks), Span::styled(":topology ", ds)]);
    }
    if bt == "mqtt" {
        spans.extend([Span::styled("H", ks), Span::styled(":retained ", ds)]);
    }
    spans.extend([
        Span::styled("A", ks), Span::styled(":perms ", ds),
        Span::styled("?", ks), Span::styled(":help", ds),
    ]);
    if !app.scheduled_messages.is_empty() {
        spans.push(Span::styled(
            format!(" ⏱{} ", app.scheduled_messages.len()),
            Style::default().fg(app.theme.success).bold(),
        ));
        spans.push(Span::styled("S", ks));
        spans.push(Span::styled(":view ", ds));
    }
    spans.extend(super::update_hint_spans(app));
    spans.push(Span::styled("  │ ", Style::default().fg(app.theme.divider)));
    spans.push(Span::styled(status_text, Style::default().fg(status_color)));
    let line = Line::from(spans);

    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(app.theme.sidebar_bg)),
        area,
    );
}
