use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::slsk::DownloadStatus;
use crate::tui::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),    // queue list
            Constraint::Length(3), // status bar
        ])
        .split(area);

    render_queue_list(frame, chunks[0], app);
    render_status_bar(frame, chunks[1], app);
}

fn render_queue_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default()
        .title(" soulpull — Queue [j/k navigate | r run | c config | s summary | q quit] ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let items: Vec<ListItem> = app
        .queue
        .iter()
        .map(|entry| {
            let (status_symbol, status_color) = status_style(&entry.status);
            let progress_str = match &entry.status {
                DownloadStatus::Downloading { progress_pct } => {
                    format!(" [{:>3}%]", progress_pct)
                }
                DownloadStatus::Done { format } => format!(" [{}]", format.to_uppercase()),
                DownloadStatus::Settled { received, .. } => format!(" [{}]", received.to_uppercase()),
                DownloadStatus::Failed { reason } => format!(" [{}]", truncate(reason, 30)),
                _ => String::new(),
            };

            let line = Line::from(vec![
                Span::styled(
                    format!(" {} ", status_symbol),
                    Style::default().fg(status_color),
                ),
                Span::raw(entry.label.clone()),
                Span::styled(
                    progress_str,
                    Style::default().fg(Color::DarkGray),
                ),
            ]);
            ListItem::new(line)
        })
        .collect();

    let mut list_state = ListState::default();
    if !app.queue.is_empty() {
        list_state.select(Some(app.selected_index));
    }

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut list_state);
}

fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let counts = app.summary_counts();
    let msg = app.status_message.as_deref().unwrap_or("");

    let text = format!(
        " ✓ {} done  ~ {} settled  ✗ {} failed  · {} queued  ⇣ {} active   {}",
        counts.done, counts.settled, counts.failed, counts.queued, counts.in_progress, msg
    );

    let bar = Paragraph::new(text)
        .style(Style::default().fg(Color::White).bg(Color::DarkGray))
        .block(Block::default());

    frame.render_widget(bar, area);
}

fn status_style(status: &DownloadStatus) -> (&'static str, Color) {
    match status {
        DownloadStatus::Queued => ("·", Color::DarkGray),
        DownloadStatus::Searching => ("?", Color::Yellow),
        DownloadStatus::Downloading { .. } => ("⇣", Color::Cyan),
        DownloadStatus::Done { .. } => ("✓", Color::Green),
        DownloadStatus::Settled { .. } => ("~", Color::Yellow),
        DownloadStatus::Failed { .. } => ("✗", Color::Red),
    }
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        &s[..max]
    }
}
