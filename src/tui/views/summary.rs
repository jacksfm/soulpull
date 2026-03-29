use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::slsk::DownloadStatus;
use crate::tui::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),  // summary stats
            Constraint::Min(3),     // per-item breakdown
            Constraint::Length(2),  // footer
        ])
        .split(area);

    render_stats(frame, chunks[0], app);
    render_breakdown(frame, chunks[1], app);
    render_footer(frame, chunks[2]);
}

fn render_stats(frame: &mut Frame, area: Rect, app: &App) {
    let counts = app.summary_counts();
    let total = app.queue.len();

    let text = vec![
        Line::from(Span::styled(" Summary", Style::default().add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![
            Span::styled(" ✓ Done:      ", Style::default().fg(Color::Green)),
            Span::raw(format!("{}", counts.done)),
        ]),
        Line::from(vec![
            Span::styled(" ~ Settled:   ", Style::default().fg(Color::Yellow)),
            Span::raw(format!("{}", counts.settled)),
        ]),
        Line::from(vec![
            Span::styled(" ✗ Failed:    ", Style::default().fg(Color::Red)),
            Span::raw(format!("{}", counts.failed)),
        ]),
        Line::from(vec![
            Span::styled("   Total:     ", Style::default().fg(Color::White)),
            Span::raw(format!("{}", total)),
        ]),
    ];

    let block = Block::default()
        .title(" Results ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let para = Paragraph::new(text).block(block);
    frame.render_widget(para, area);
}

fn render_breakdown(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Per-Item Breakdown ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let items: Vec<ListItem> = app
        .queue
        .iter()
        .filter(|e| !matches!(e.status, DownloadStatus::Queued | DownloadStatus::Downloading { .. } | DownloadStatus::Searching))
        .map(|entry| {
            let line = match &entry.status {
                DownloadStatus::Done { format } => Line::from(vec![
                    Span::styled(" ✓ ", Style::default().fg(Color::Green)),
                    Span::raw(format!("{} [{}]", entry.label, format.to_uppercase())),
                ]),
                DownloadStatus::Settled { wanted, received } => Line::from(vec![
                    Span::styled(" ~ ", Style::default().fg(Color::Yellow)),
                    Span::raw(format!("{} [wanted: {}, got: {}]", entry.label, wanted.to_uppercase(), received.to_uppercase())),
                ]),
                DownloadStatus::Failed { reason } => Line::from(vec![
                    Span::styled(" ✗ ", Style::default().fg(Color::Red)),
                    Span::raw(format!("{} — {}", entry.label, reason)),
                ]),
                _ => Line::from(Span::raw(&entry.label)),
            };
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

fn render_footer(frame: &mut Frame, area: Rect) {
    let para = Paragraph::new(" Press q or Esc to return to queue")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(para, area);
}
