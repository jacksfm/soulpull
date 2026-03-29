use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use super::app::{ActiveView, App};

/// Returns `true` if the user pressed the run key and a download should start.
pub fn handle(app: &mut App, event: Event) -> Result<bool> {
    let Event::Key(key) = event else { return Ok(false) };

    let run = match app.active_view {
        ActiveView::Queue => handle_queue(app, key),
        ActiveView::Config => { handle_config(app, key); false }
        ActiveView::Summary => { handle_summary(app, key); false }
    };

    Ok(run)
}

/// Returns true when the user requests a run.
fn handle_queue(app: &mut App, key: KeyEvent) -> bool {
    // Ctrl+C always quits
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return false;
    }

    match key.code {
        // Navigation
        KeyCode::Up | KeyCode::Char('k') => app.move_up(),
        KeyCode::Down | KeyCode::Char('j') => app.move_down(),

        // View switching
        KeyCode::Char('c') => app.active_view = ActiveView::Config,
        KeyCode::Char('s') => app.active_view = ActiveView::Summary,

        // Start downloads
        KeyCode::Char('r') | KeyCode::Enter => {
            if !app.is_running {
                app.status_message = Some("Starting downloads…".into());
                return true;
            }
        }

        // Quit
        KeyCode::Char('q') | KeyCode::Esc => {
            app.should_quit = true;
        }

        _ => {}
    }

    false
}

fn handle_config(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => app.active_view = ActiveView::Queue,
        KeyCode::Char(c) => app.config_editor_content.push(c),
        KeyCode::Backspace => { app.config_editor_content.pop(); }
        KeyCode::Enter => app.config_editor_content.push('\n'),
        _ => {}
    }
}

fn handle_summary(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => app.active_view = ActiveView::Queue,
        _ => {}
    }
}
