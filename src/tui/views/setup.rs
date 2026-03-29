use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::tui::app::{App, SetupField};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // title
            Constraint::Length(3),  // username
            Constraint::Length(3),  // password
            Constraint::Length(3),  // sldl path
            Constraint::Length(3),  // output path
            Constraint::Length(3),  // footer
            Constraint::Min(0),
        ])
        .split(area);

    let title = Paragraph::new(Line::from(vec![
        Span::styled(" soulpull — first run setup ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(" Tab to switch fields · Enter to save · Esc to skip", Style::default().fg(Color::DarkGray)),
    ]))
    .block(Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(Color::DarkGray)));
    frame.render_widget(title, chunks[0]);

    render_field(frame, chunks[1], "Soulseek Username", &app.setup_username, app.setup_field == SetupField::Username, false);

    let pw_display = if app.setup_show_password {
        app.setup_password.clone()
    } else {
        "•".repeat(app.setup_password.len())
    };
    render_field(frame, chunks[2], "Soulseek Password  (Space to toggle visibility)", &pw_display, app.setup_field == SetupField::Password, false);

    render_field(frame, chunks[3], "sldl path  (\"sldl\" if on PATH, otherwise full path to sldl.exe)", &app.setup_sldl_path, app.setup_field == SetupField::SldlPath, false);
    render_field(frame, chunks[4], "Download folder", &app.setup_output_path, app.setup_field == SetupField::OutputPath, false);

    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" Tab ", Style::default().fg(Color::Yellow)),
        Span::raw("next field  "),
        Span::styled(" Enter ", Style::default().fg(Color::Yellow)),
        Span::raw("save & continue  "),
        Span::styled(" Esc ", Style::default().fg(Color::Yellow)),
        Span::raw("skip (credentials required to download)"),
    ]))
    .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, chunks[5]);
}

fn render_field(frame: &mut Frame, area: Rect, label: &str, value: &str, active: bool, _secret: bool) {
    let border_color = if active { Color::Cyan } else { Color::DarkGray };
    let block = Block::default()
        .title(format!(" {label} "))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let display = if active {
        format!("{value}█") // fake cursor
    } else {
        value.to_string()
    };

    let para = Paragraph::new(display)
        .block(block)
        .style(Style::default().fg(if active { Color::White } else { Color::Gray }));
    frame.render_widget(para, area);
}
