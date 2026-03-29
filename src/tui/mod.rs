pub mod app;
pub mod events;
pub mod views;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, path::PathBuf, time::Duration};

use crate::config::Config;
use crate::slsk::runner::{run_all, RunnerTask};
use app::{App, ActiveView};

pub async fn run(config: Config, config_path: PathBuf, initial_inputs: Vec<String>) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, config, config_path, initial_inputs).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    result
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    config: Config,
    config_path: PathBuf,
    initial_inputs: Vec<String>,
) -> Result<()> {
    let mut app = App::new(config, config_path);

    // Pre-load any inputs passed on the command line
    for input in initial_inputs {
        app.add_input(input);
    }

    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            match app.active_view {
                ActiveView::Setup   => views::setup::render(frame, area, &app),
                ActiveView::Queue   => views::queue::render(frame, area, &mut app),
                ActiveView::Config  => views::config::render(frame, area, &mut app),
                ActiveView::Summary => views::summary::render(frame, area, &mut app),
            }
        })?;

        if event::poll(Duration::from_millis(50))? {
            let ev = event::read()?;
            let run_requested = events::handle(&mut app, ev)?;

            if run_requested && !app.is_running {
                app.is_running = true;
                dispatch_downloads(&mut app);
            }
        }

        app.drain_download_events();

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

fn dispatch_downloads(app: &mut App) {
    let tasks: Vec<RunnerTask> = app.queue.iter()
        .filter(|e| matches!(e.status, crate::slsk::DownloadStatus::Queued))
        .map(|entry| RunnerTask {
            id: entry.id,
            input: entry.input.clone(),
            album_mode: entry.album_mode,
            output_path: None,
        })
        .collect();

    if tasks.is_empty() {
        app.status_message = Some("nothing queued".into());
        app.is_running = false;
        return;
    }

    let config = app.config.clone();
    let tx = app.event_tx.clone();
    tokio::spawn(async move {
        run_all(tasks, config, tx).await;
    });
}
