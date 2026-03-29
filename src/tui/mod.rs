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
use crate::sources::Track;
use crate::slsk::runner::{run_all, run_raw, RunnerTask, search_term};
use app::{App, ActiveView};

pub async fn run(
    config: Config,
    config_path: PathBuf,
    tracks: Vec<Track>,
    raw_input: Option<String>,
) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, config, config_path, tracks, raw_input).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    result
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    config: Config,
    config_path: PathBuf,
    tracks: Vec<Track>,
    raw_input: Option<String>,
) -> Result<()> {
    let mut app = App::new(config, config_path, tracks, raw_input);

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
    let config = app.config.clone();
    let tx = app.event_tx.clone();

    // If there's a raw input (URL, search string, CSV path), hand it directly to sldl
    if let Some(ref input) = app.raw_input {
        let input = input.clone();
        tokio::spawn(async move {
            if let Err(e) = run_raw(&input, &config, &tx).await {
                let _ = tx.send(crate::slsk::DownloadEvent::StatusChanged {
                    item_id: 0,
                    status: crate::slsk::DownloadStatus::Failed { reason: e.to_string() },
                }).await;
            }
        });
        return;
    }

    // CSV-sourced tracks: one sldl process per entry
    let aggregation = app.config.defaults.aggregation;
    let tasks: Vec<RunnerTask> = app.queue.iter().zip(app.tracks.iter())
        .map(|(entry, track)| {
            use crate::config::Aggregation;
            let album_mode = matches!(aggregation, Aggregation::Album | Aggregation::Artist);
            let term = match aggregation {
                Aggregation::Song => search_term(&track.artist, &track.title),
                Aggregation::Album | Aggregation::Artist => {
                    let album = track.album.as_deref().unwrap_or(&track.title);
                    search_term(&track.artist, album)
                }
            };
            RunnerTask { id: entry.id, input: term, album_mode, output_path: None }
        })
        .collect();

    let config = app.config.clone();
    let tx = app.event_tx.clone();
    tokio::spawn(async move {
        run_all(tasks, config, tx).await;
    });
}
