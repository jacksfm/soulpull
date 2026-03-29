use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::tui::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),    // editor
            Constraint::Length(2), // footer
        ])
        .split(area);

    render_editor(frame, chunks[0], app);
    render_footer(frame, chunks[1]);
}

fn render_editor(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Config Editor (TOML) — Esc to return, Enter for newline ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let para = Paragraph::new(app.config_editor_content.as_str())
        .block(block)
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(Color::White));

    frame.render_widget(para, area);
}

fn render_footer(frame: &mut Frame, area: Rect) {
    let para = Paragraph::new(
        " Note: config edits are not yet persisted. Edit config.toml directly for now.",
    )
    .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(para, area);
}
