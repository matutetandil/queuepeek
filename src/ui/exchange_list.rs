use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, Popup};
use crate::keys::popup::{topology_flat_list, TopologyFlatItem};

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
        Constraint::Min(3),    // exchange list
        Constraint::Length(2), // footer/status
    ])
    .split(area);

    let hw = draw_header(frame, app, chunks[0]);
    super::draw_version_tag(frame, app, chunks[0], hw);
    draw_filter(frame, app, chunks[1]);
    draw_list(frame, app, chunks[2]);
    draw_footer(frame, app, chunks[3]);

    // Popups on top
    if app.popup != Popup::None {
        super::popup::draw(frame, app);
    }
}

// --- Header Bar ---

fn draw_header(frame: &mut Frame, app: &App, area: Rect) -> u16 {
    let loading = if app.loading { " ⟳" } else { "" };
    let count = app.filtered_exchange_indices.len();
    let total = app.topology_exchanges.len();
    let sep = Span::styled(" › ", Style::default().fg(app.theme.divider).bg(app.theme.sidebar_bg));
    let muted = Style::default().fg(app.theme.muted).bg(app.theme.sidebar_bg);

    let mut spans = vec![
        Span::styled(format!("  {} ", app.profile_name), muted),
        sep.clone(),
        Span::styled(
            format!("{} ", app.selected_namespace),
            Style::default().fg(app.theme.white).bold().bg(app.theme.sidebar_bg),
        ),
        sep,
        Span::styled(
            if count == total {
                format!("Exchanges ({}){}", count, loading)
            } else {
                format!("Exchanges ({} of {}){}", count, total, loading)
            },
            muted,
        ),
    ];
    if !app.topology_exchanges.is_empty() {
        spans.push(super::live_pulse_span(app));
    }
    let content_width: u16 = spans.iter().map(|s| s.content.len() as u16).sum();
    let line = Line::from(spans);

    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(app.theme.sidebar_bg)),
        area,
    );
    content_width
}

// --- Filter Bar ---

fn draw_filter(frame: &mut Frame, app: &App, area: Rect) {
    let line = if app.exchange_filter_active || !app.exchange_filter.is_empty() {
        let cursor = if app.exchange_filter_focused { "▎" } else { "" };
        let slash_style = if app.exchange_filter_focused {
            Style::default().fg(app.theme.accent).bold()
        } else {
            Style::default().fg(app.theme.muted)
        };
        Line::from(vec![
            Span::styled(" / ", slash_style),
            Span::styled(
                format!("{}{}", app.exchange_filter, cursor),
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

// --- Exchange List ---

fn draw_list(frame: &mut Frame, app: &mut App, area: Rect) {
    let border_color = app.theme.accent;

    let title = if app.loading {
        " Exchanges (loading...) ".to_string()
    } else {
        " Exchanges ".to_string()
    };

    let block = Block::bordered()
        .title(title)
        .title_style(Style::default().fg(app.theme.white).bold())
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(app.theme.bg));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let flat = topology_flat_list(app);

    if flat.is_empty() {
        let msg = if app.loading { "Loading..." } else { "No exchanges found" };
        frame.render_widget(
            Paragraph::new(format!("  {}", msg))
                .style(Style::default().fg(app.theme.muted).bg(app.theme.bg)),
            inner,
        );
        return;
    }

    let visible_height = inner.height as usize;

    // Scroll: keep selected visible, cursor moves freely within view
    let scroll_offset = if app.topology_selected >= app.topology_scroll as usize + visible_height {
        app.topology_selected - visible_height + 1
    } else if app.topology_selected < app.topology_scroll as usize {
        app.topology_selected
    } else {
        app.topology_scroll as usize
    };
    // Persist scroll position for next frame
    app.topology_scroll = scroll_offset as u16;

    let mut lines: Vec<Line> = Vec::new();
    for (idx, item) in flat.iter().enumerate().skip(scroll_offset).take(visible_height) {
        let selected = idx == app.topology_selected;
        let sel_marker = if selected { "▸ " } else { "  " };

        match item {
            TopologyFlatItem::Exchange(ref name) => {
                let expanded = app.topology_expanded.contains(name);
                let has_bindings = app.topology_bindings.iter().any(|b| &b.source == name);
                let arrow = if !has_bindings { " " } else if expanded { "▾" } else { "▸" };
                let ex_info = app.topology_exchanges.iter().find(|e| &e.name == name);
                let type_str = ex_info.map(|e| e.exchange_type.as_str()).unwrap_or("");
                let durable = ex_info.map(|e| e.durable).unwrap_or(false);
                let base_style = if selected {
                    Style::default().fg(app.theme.bg).bg(app.theme.accent)
                } else {
                    Style::default().bg(app.theme.bg)
                };
                lines.push(Line::from(vec![
                    Span::styled(sel_marker, base_style),
                    Span::styled(format!("{} ", arrow), base_style.fg(if selected { app.theme.bg } else { app.theme.muted })),
                    Span::styled(format!("{} ", name), if selected { base_style } else { base_style.fg(app.theme.accent).bold() }),
                    Span::styled(format!("[{}]", type_str), base_style.fg(if selected { app.theme.bg } else { app.theme.muted })),
                    if durable {
                        Span::styled(" durable", base_style.fg(if selected { app.theme.bg } else { app.theme.success }))
                    } else {
                        Span::styled(" transient", base_style.fg(if selected { app.theme.bg } else { app.theme.muted }))
                    },
                ]));
            }
            TopologyFlatItem::Binding(ref b) => {
                let is_last = {
                    let next = flat.get(idx + 1);
                    !matches!(next, Some(TopologyFlatItem::Binding(ref nb)) if nb.source == b.source)
                };
                let pfx = if is_last { "\u{2514}\u{2500}\u{2500} " } else { "\u{251c}\u{2500}\u{2500} " };
                let rk = if b.routing_key.is_empty() { "*".to_string() } else { b.routing_key.clone() };
                let base_style = if selected {
                    Style::default().fg(app.theme.bg).bg(app.theme.accent)
                } else {
                    Style::default().bg(app.theme.bg)
                };
                lines.push(Line::from(vec![
                    Span::styled(sel_marker, base_style),
                    Span::styled(format!("    {}", pfx), base_style.fg(if selected { app.theme.bg } else { app.theme.muted })),
                    Span::styled(rk, base_style.fg(if selected { app.theme.bg } else { app.theme.primary })),
                    Span::styled(format!(" \u{2192} {}", b.destination), base_style.fg(if selected { app.theme.bg } else { app.theme.white })),
                ]));
            }
        }
    }

    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(app.theme.bg)),
        inner,
    );
}

// --- Footer ---

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let ks = Style::default().fg(app.theme.accent).bold();
    let ds = Style::default().fg(app.theme.muted);

    let (status_text, status_color) = if !app.status_message.is_empty() {
        let c = if app.status_is_error { app.theme.error } else { app.theme.success };
        (app.status_message.as_str(), c)
    } else {
        ("", app.theme.muted)
    };

    // Line 1: keyboard shortcuts
    let shortcut_spans = vec![
        Span::styled(" ", Style::default()),
        Span::styled("j/k", ks), Span::styled(":nav ", ds),
        Span::styled("⏎", ks), Span::styled(":expand ", ds),
        Span::styled("b", ks), Span::styled(":bind ", ds),
        Span::styled("d", ks), Span::styled(":unbind ", ds),
        Span::styled("/", ks), Span::styled(":filter ", ds),
        Span::styled("i", ks), Span::styled(":info ", ds),
        Span::styled("?", ks), Span::styled(":help ", ds),
        Span::styled("Esc", ks), Span::styled(":back", ds),
    ];
    let line1 = Line::from(shortcut_spans);

    // Line 2: status
    let mut status_spans: Vec<Span> = vec![Span::styled(" ", Style::default())];
    status_spans.extend(super::update_hint_spans(app));
    status_spans.push(Span::styled("  │ ", Style::default().fg(app.theme.divider)));
    status_spans.push(Span::styled(status_text, Style::default().fg(status_color)));
    let line2 = Line::from(status_spans);

    let text = ratatui::text::Text::from(vec![line1, line2]);
    frame.render_widget(
        Paragraph::new(text).style(Style::default().bg(app.theme.sidebar_bg)),
        area,
    );
}
