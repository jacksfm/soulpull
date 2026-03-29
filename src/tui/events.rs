use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::app::{ActiveView, App, SetupField};

/// Returns `true` if the user pressed the run key and downloads should start.
pub fn handle(app: &mut App, event: Event) -> Result<bool> {
    let Event::Key(key) = event else { return Ok(false) };
    // Ignore key release and repeat — only act on press
    if key.kind != KeyEventKind::Press { return Ok(false); }

    let run = match app.active_view {
        ActiveView::Setup   => { handle_setup(app, key); false }
        ActiveView::Queue   => handle_queue(app, key),
        ActiveView::Config  => { handle_config(app, key); false }
        ActiveView::Summary => { handle_summary(app, key); false }
    };

    Ok(run)
}

fn handle_setup(app: &mut App, key: KeyEvent) {
    // Ctrl+C always quits
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }

    match key.code {
        KeyCode::Tab => {
            app.setup_field = match app.setup_field {
                SetupField::Username   => SetupField::Password,
                SetupField::Password   => SetupField::SldlPath,
                SetupField::SldlPath   => SetupField::OutputPath,
                SetupField::OutputPath => SetupField::Username,
            };
        }
        KeyCode::Enter => {
            match app.commit_setup() {
                Ok(()) => app.active_view = ActiveView::Queue,
                Err(e) => app.status_message = Some(format!("Save failed: {e}")),
            }
        }
        KeyCode::Esc => {
            // Skip setup — user can still browse queue, but downloads will fail without creds
            app.active_view = ActiveView::Queue;
        }
        KeyCode::Char(' ') if app.setup_field == SetupField::Password => {
            app.setup_show_password = !app.setup_show_password;
        }
        KeyCode::Backspace => {
            active_setup_field_mut(app).pop();
        }
        KeyCode::Char(c) => {
            active_setup_field_mut(app).push(c);
        }
        _ => {}
    }
}

fn active_setup_field_mut(app: &mut App) -> &mut String {
    match app.setup_field {
        SetupField::Username   => &mut app.setup_username,
        SetupField::Password   => &mut app.setup_password,
        SetupField::SldlPath   => &mut app.setup_sldl_path,
        SetupField::OutputPath => &mut app.setup_output_path,
    }
}

fn handle_queue(app: &mut App, key: KeyEvent) -> bool {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return false;
    }

    match key.code {
        KeyCode::Up | KeyCode::Char('k')   => app.move_up(),
        KeyCode::Down | KeyCode::Char('j') => app.move_down(),
        KeyCode::Char('c') => app.active_view = ActiveView::Config,
        KeyCode::Char('s') => app.active_view = ActiveView::Summary,
        KeyCode::Char('r') | KeyCode::Enter => {
            if !app.is_running {
                app.status_message = Some("Starting downloads…".into());
                return true;
            }
        }
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        _ => {}
    }
    false
}

fn handle_config(app: &mut App, key: KeyEvent) {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('s') => {
                app.save_config_editor();
                return;
            }
            KeyCode::Char('c') => {
                app.should_quit = true;
                return;
            }
            _ => {}
        }
    }

    match key.code {
        KeyCode::Esc       => {
            app.config_save_message = None;
            app.active_view = ActiveView::Queue;
        }
        KeyCode::Char(c)   => app.config_editor_content.push(c),
        KeyCode::Backspace => { app.config_editor_content.pop(); }
        KeyCode::Enter     => app.config_editor_content.push('\n'),
        KeyCode::Tab       => app.config_editor_content.push_str("  "),
        _ => {}
    }
}

fn handle_summary(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => app.active_view = ActiveView::Queue,
        _ => {}
    }
}
