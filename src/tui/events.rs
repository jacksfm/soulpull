use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::app::{ActiveView, App, InputMode, SetupField};

/// Returns `true` if downloads should start.
pub fn handle(app: &mut App, event: Event) -> Result<bool> {
    let Event::Key(key) = event else { return Ok(false) };
    if key.kind != KeyEventKind::Press { return Ok(false); }

    let run = match app.active_view {
        ActiveView::Setup   => { handle_setup(app, key); false }
        ActiveView::Queue   => handle_queue(app, key),
        ActiveView::Config  => { handle_config(app, key); false }
        ActiveView::Summary => { handle_summary(app, key); false }
    };

    Ok(run)
}

// ---------------------------------------------------------------------------
// Queue view
// ---------------------------------------------------------------------------

fn handle_queue(app: &mut App, key: KeyEvent) -> bool {
    // Ctrl+C always quits
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return false;
    }

    match app.input_mode {
        InputMode::Adding => handle_queue_input(app, key),
        InputMode::Normal => handle_queue_normal(app, key),
    }
}

fn handle_queue_normal(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Up | KeyCode::Char('k')   => app.move_up(),
        KeyCode::Down | KeyCode::Char('j') => app.move_down(),

        // Enter add mode
        KeyCode::Char('a') => {
            app.input_mode = InputMode::Adding;
            app.input_buffer.clear();
            app.status_message = None;
        }

        // Delete selected entry
        KeyCode::Char('d') | KeyCode::Delete => app.delete_selected(),

        // Start downloads
        KeyCode::Char('r') | KeyCode::Enter => {
            if !app.is_running && !app.queue.is_empty() {
                app.status_message = Some("starting downloads…".into());
                return true;
            }
        }

        KeyCode::Char('c') => app.active_view = ActiveView::Config,
        KeyCode::Char('s') => app.active_view = ActiveView::Summary,
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        _ => {}
    }
    false
}

fn handle_queue_input(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Enter => {
            let input = app.input_buffer.trim().to_string();
            app.input_mode = InputMode::Normal;
            app.input_buffer.clear();
            if !input.is_empty() {
                app.add_input(input);
            }
        }
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.input_buffer.clear();
        }
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        _ => {}
    }
    false
}

// ---------------------------------------------------------------------------
// Setup view
// ---------------------------------------------------------------------------

fn handle_setup(app: &mut App, key: KeyEvent) {
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
                Err(e) => app.status_message = Some(format!("save failed: {e}")),
            }
        }
        KeyCode::Esc => app.active_view = ActiveView::Queue,
        KeyCode::Char(' ') if app.setup_field == SetupField::Password => {
            app.setup_show_password = !app.setup_show_password;
        }
        KeyCode::Backspace => { active_setup_field_mut(app).pop(); }
        KeyCode::Char(c)   => active_setup_field_mut(app).push(c),
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

// ---------------------------------------------------------------------------
// Config view
// ---------------------------------------------------------------------------

fn handle_config(app: &mut App, key: KeyEvent) {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('s') => { app.save_config_editor(); return; }
            KeyCode::Char('c') => { app.should_quit = true; return; }
            _ => {}
        }
    }
    match key.code {
        KeyCode::Esc       => { app.config_save_message = None; app.active_view = ActiveView::Queue; }
        KeyCode::Char(c)   => app.config_editor_content.push(c),
        KeyCode::Backspace => { app.config_editor_content.pop(); }
        KeyCode::Enter     => app.config_editor_content.push('\n'),
        KeyCode::Tab       => app.config_editor_content.push_str("  "),
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Summary view
// ---------------------------------------------------------------------------

fn handle_summary(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => app.active_view = ActiveView::Queue,
        _ => {}
    }
}
