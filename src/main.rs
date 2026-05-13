mod config;
mod data;
mod db;
mod dict;
mod srs;
mod ui;

#[cfg(unix)]
extern crate libc;

use anyhow::Result;
use crossterm::{
    event,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};

use config::Config;
use db::Database;
use ui::app_state::{App, Screen};
use ui::events::handle_event;
use ui::render::render;

const STATUS_CLEAR_SECS: u64 = 3;
const DICT_TIMEOUT_SECS: u64 = 15;

fn main() -> Result<()> {
    let cfg = Config::load();

    let db = Database::open(&cfg.db_path())?;
    db.seed_words()?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let data_dir = cfg.data_dir();
    let cfg_warnings = cfg.warnings.clone();
    let mut app = App::new(db, cfg.session_limit, data_dir);
    if !cfg_warnings.is_empty() {
        app.status_message = format!("Config: {}", cfg_warnings.join("; "));
        app.status_set_at = Some(std::time::Instant::now());
    }

    loop {
        if app.should_suspend {
            app.should_suspend = false;
            disable_raw_mode()?;
            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
            terminal.show_cursor()?;
            #[cfg(unix)]
            unsafe { libc::raise(libc::SIGTSTP); }
            enable_raw_mode()?;
            execute!(io::stdout(), EnterAlternateScreen)?;
            terminal.clear()?;
        }

        if app.status_set_at.map(|t| t.elapsed().as_secs() >= STATUS_CLEAR_SECS).unwrap_or(false) {
            app.status_message.clear();
            app.status_set_at = None;
        }

        if app.dict_rx.is_some()
            && app.dict_lookup_started.map(|s| s.elapsed().as_secs() > DICT_TIMEOUT_SECS).unwrap_or(false)
        {
            app.dict_rx = None;
            app.dict_lookup_started = None;
            app.dict_status = "Search timed out — press Enter to retry.".to_string();
        }

        let dict_done = if let Some(rx) = &app.dict_rx {
            rx.try_recv().ok()
        } else {
            None
        };
        if let Some(result) = dict_done {
            app.dict_rx = None;
            app.dict_lookup_started = None;
            let searched_query = app.dict_query.clone();
            match result {
                Ok(results) => {
                    app.dict_status = if results.is_empty() {
                        "No results found.".to_string()
                    } else {
                        format!("{} result(s) — ↑↓ to scroll, Enter to add", results.len())
                    };
                    app.dict_cursor = 0;
                    app.dict_last_query = searched_query;
                    app.dict_results = results;
                }
                Err(e) => {
                    app.dict_status = format!("Error: {e}");
                    app.dict_results.clear();
                }
            }
        }

        terminal.draw(|f| render(f, &app))?;

        if event::poll(Duration::from_millis(50))? {
            let ev = event::read()?;
            let prev_screen = app.screen.clone();
            if handle_event(&mut app, ev)? {
                break;
            }
            if matches!(app.screen, Screen::MainMenu)
                && !matches!(prev_screen, Screen::MainMenu)
            {
                app.refresh_heatmap();
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    println!("再见!");
    Ok(())
}
