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
            Constraint::Length(2), // save status / footer
        ])
        .split(area);

    render_editor(frame, chunks[0], app);
    render_footer(frame, chunks[1], app);
}

fn render_editor(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(format!(
            " Config — {} — Ctrl+S to save · Esc to return ",
            app.config_path.display()
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let para = Paragraph::new(app.config_editor_content.as_str())
        .block(block)
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(Color::White));

    frame.render_widget(para, area);
}

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let msg = app.config_save_message.as_deref().unwrap_or(
        " Ctrl+S save · Esc back to queue"
    );
    let color = if app.config_save_message.as_deref().map_or(false, |m| m.starts_with("Save") || m.starts_with("Saved")) {
        Color::Green
    } else if app.config_save_message.as_deref().map_or(false, |m| m.contains("error") || m.contains("failed")) {
        Color::Red
    } else {
        Color::DarkGray
    };
    let para = Paragraph::new(msg).style(Style::default().fg(color));
    frame.render_widget(para, area);
}
