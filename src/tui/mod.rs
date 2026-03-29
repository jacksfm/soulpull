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
use std::{io, time::Duration};

use crate::config::Config;
use crate::sources::Track;
use crate::slsk::runner::{run_all, RunnerTask};
use app::{App, ActiveView};

pub async fn run(config: Config, tracks: Vec<Track>) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, config, tracks).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    config: Config,
    tracks: Vec<Track>,
) -> Result<()> {
    let mut app = App::new(config, tracks);

    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            match app.active_view {
                ActiveView::Queue => views::queue::render(frame, area, &mut app),
                ActiveView::Config => views::config::render(frame, area, &mut app),
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

/// Build RunnerTasks from the current queue and spawn run_all on a tokio task.
fn dispatch_downloads(app: &mut App) {
    let aggregation = app.config.defaults.aggregation;
    let tasks: Vec<RunnerTask> = app
        .queue
        .iter()
        .zip(app.tracks.iter())
        .map(|(entry, track)| {
            use crate::config::Aggregation;
            use crate::slsk::runner::search_term;

            let album_mode = matches!(aggregation, Aggregation::Album | Aggregation::Artist);
            let term = match aggregation {
                Aggregation::Song => search_term(&track.artist, &track.title),
                Aggregation::Album | Aggregation::Artist => {
                    let album = track.album.as_deref().unwrap_or(&track.title);
                    search_term(&track.artist, album)
                }
            };

            RunnerTask {
                id: entry.id,
                search_term: term,
                album_mode,
                output_path: None, // uses config default
            }
        })
        .collect();

    let config = app.config.clone();
    let tx = app.event_tx.clone();

    tokio::spawn(async move {
        run_all(tasks, config, tx).await;
    });
}
