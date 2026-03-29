use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::slsk::DownloadStatus;
use crate::tui::app::{App, InputMode};

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    let show_input_bar = app.input_mode == InputMode::Adding;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if show_input_bar {
            vec![Constraint::Min(3), Constraint::Length(3), Constraint::Length(2)]
        } else {
            vec![Constraint::Min(3), Constraint::Length(2)]
        })
        .split(area);

    render_queue_list(frame, chunks[0], app);

    if show_input_bar {
        render_input_bar(frame, chunks[1], app);
        render_status_bar(frame, chunks[2], app);
    } else {
        render_status_bar(frame, chunks[1], app);
    }
}

fn render_queue_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let is_adding = app.input_mode == InputMode::Adding;
    let hints = if is_adding {
        " esc cancel · enter confirm"
    } else {
        " a add · d delete · r run · c config · s summary · q quit"
    };

    let block = Block::default()
        .title(format!(" soulpull —{hints} "))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if is_adding { Color::DarkGray } else { Color::Cyan }));

    let items: Vec<ListItem> = if app.queue.is_empty() {
        vec![ListItem::new(Line::from(vec![
            Span::styled(
                "  queue is empty — press ",
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled("a", Style::default().fg(Color::Cyan)),
            Span::styled(
                " to add a search, URL, or CSV path",
                Style::default().fg(Color::DarkGray),
            ),
        ]))]
    } else {
        app.queue
            .iter()
            .map(|entry| {
                let (symbol, color) = status_style(&entry.status);
                let suffix = match &entry.status {
                    DownloadStatus::Downloading { progress_pct } => {
                        format!(" [{:>3}%]", progress_pct)
                    }
                    DownloadStatus::Done { format } => format!(" [{}]", format.to_uppercase()),
                    DownloadStatus::Settled { received, .. } => {
                        format!(" [{}]", received.to_uppercase())
                    }
                    DownloadStatus::Failed { reason } => format!(" [{}]", truncate(reason, 35)),
                    _ => String::new(),
                };

                ListItem::new(Line::from(vec![
                    Span::styled(format!(" {} ", symbol), Style::default().fg(color)),
                    Span::raw(entry.label.clone()),
                    Span::styled(suffix, Style::default().fg(Color::DarkGray)),
                ]))
            })
            .collect()
    };

    let mut list_state = ListState::default();
    if !app.queue.is_empty() {
        list_state.select(Some(app.selected_index));
    }

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut list_state);
}

fn render_input_bar(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" add — search string, URL, or path to CSV ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let display = format!("{}█", app.input_buffer);
    let para = Paragraph::new(display)
        .block(block)
        .style(Style::default().fg(Color::White));

    frame.render_widget(para, area);
}

fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let counts = app.summary_counts();
    let msg = app.status_message.as_deref().unwrap_or("");

    let text = format!(
        " ✓ {}  ~ {}  ✗ {}  · {}  ⇣ {}   {}",
        counts.done, counts.settled, counts.failed, counts.queued, counts.in_progress, msg
    );

    let bar = Paragraph::new(text)
        .style(Style::default().fg(Color::White).bg(Color::DarkGray));

    frame.render_widget(bar, area);
}

fn status_style(status: &DownloadStatus) -> (&'static str, Color) {
    match status {
        DownloadStatus::Queued           => ("·", Color::DarkGray),
        DownloadStatus::Searching        => ("?", Color::Yellow),
        DownloadStatus::Downloading { .. } => ("⇣", Color::Cyan),
        DownloadStatus::Done { .. }      => ("✓", Color::Green),
        DownloadStatus::Settled { .. }   => ("~", Color::Yellow),
        DownloadStatus::Failed { .. }    => ("✗", Color::Red),
    }
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max { s } else { &s[..max] }
}
