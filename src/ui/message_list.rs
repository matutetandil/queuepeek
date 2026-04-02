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
        Constraint::Length(1), // header
        Constraint::Min(3),    // message list
        Constraint::Length(1), // footer
    ])
    .split(area);

    draw_header(frame, app, chunks[0]);
    draw_list(frame, app, chunks[1]);
    draw_footer(frame, app, chunks[2]);

    if app.popup != Popup::None {
        super::popup::draw(frame, app);
    }
}

// ─── Header Bar ───────────────────────────────────────────────────────────

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let sep = Span::styled(" › ", Style::default().fg(app.theme.divider).bg(app.theme.sidebar_bg));
    let muted = Style::default().fg(app.theme.muted).bg(app.theme.sidebar_bg);

    let line = Line::from(vec![
        Span::styled(format!("  {} ", app.profile_name), muted),
        sep.clone(),
        Span::styled(format!("{} ", app.selected_namespace), muted),
        sep,
        Span::styled(
            format!("{} ", app.current_queue_name),
            Style::default().fg(app.theme.white).bold().bg(app.theme.sidebar_bg),
        ),
        Span::styled(
            format!("({} msgs, fetch: {})", app.messages.len(), app.fetch_count),
            muted,
        ),
    ]);

    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(app.theme.sidebar_bg)),
        area,
    );
}

// ─── Message List ─────────────────────────────────────────────────────────

fn draw_list(frame: &mut Frame, app: &mut App, area: Rect) {
    let border_color = app.theme.accent;

    let title = if app.message_filter_active || !app.message_filter.is_empty() {
        format!(" Messages (filter: {}) ", app.message_filter)
    } else {
        " Messages ".to_string()
    };

    let block = Block::bordered()
        .title(title)
        .title_style(Style::default().fg(app.theme.white).bold())
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(app.theme.bg));

    let filtered_msg_indices = &app.filtered_message_indices;

    if filtered_msg_indices.is_empty() {
        let empty_text = if app.loading {
            "  Loading messages..."
        } else if !app.message_filter.is_empty() {
            "  No messages match the filter"
        } else if app.current_queue_name.is_empty() {
            "  Select a queue first"
        } else {
            "  No messages in this queue"
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

    let max_body_width = area.width.saturating_sub(6) as usize;

    let items: Vec<ListItem> = filtered_msg_indices
        .iter()
        .map(|&idx| {
            let msg = &app.messages[idx];

            let ts = msg
                .timestamp
                .map(format_timestamp)
                .unwrap_or_else(|| "no timestamp".into());

            // Line 1: #N  timestamp  key=routing_key  exchange=exchange
            let line1 = Line::from(vec![
                Span::styled(
                    format!("  #{}", msg.index),
                    Style::default().fg(app.theme.accent).bold(),
                ),
                Span::styled("  ", Style::default()),
                Span::styled(
                    ts,
                    Style::default().fg(app.theme.muted),
                ),
                Span::styled("  key=", Style::default().fg(app.theme.muted)),
                Span::styled(
                    msg.routing_key.clone(),
                    Style::default().fg(app.theme.primary),
                ),
                Span::styled("  exchange=", Style::default().fg(app.theme.muted)),
                Span::styled(
                    msg.exchange.clone(),
                    Style::default().fg(app.theme.muted),
                ),
            ]);

            // Line 2: body preview, truncated to 1 line
            let body_preview = if msg.body.len() > max_body_width {
                format!("{}…", &msg.body[..max_body_width.saturating_sub(1)])
            } else {
                // Take only the first line of body if multiline
                msg.body
                    .lines()
                    .next()
                    .unwrap_or("")
                    .chars()
                    .take(max_body_width)
                    .collect::<String>()
            };

            let line2 = Line::from(Span::styled(
                format!("    {}", body_preview),
                Style::default().fg(app.theme.primary),
            ));

            ListItem::new(vec![line1, line2])
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

    frame.render_stateful_widget(list, area, &mut app.message_list_state);
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

    let line = Line::from(vec![
        Span::styled(" ", Style::default()),
        Span::styled("j/k", ks),
        Span::styled(":nav ", ds),
        Span::styled("⏎", ks),
        Span::styled(":detail ", ds),
        Span::styled("/", ks),
        Span::styled(":filter ", ds),
        Span::styled("r", ks),
        Span::styled(":reload ", ds),
        Span::styled("+/-", ks),
        Span::styled(":fetch ", ds),
        Span::styled("esc", ks),
        Span::styled(":back ", ds),
        Span::styled("q", ks),
        Span::styled(":quit", ds),
        Span::styled("  │ ", Style::default().fg(app.theme.divider)),
        Span::styled(status_text, Style::default().fg(status_color)),
    ]);

    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(app.theme.sidebar_bg)),
        area,
    );
}

// ─── Helpers ──────────────────────────────────────────────────────────────

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
