mod data;
mod db;
mod dict;
mod srs;
mod ui;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};

use db::Database;
use ui::app_state::App;
use ui::events::handle_event;
use ui::render::render;

fn main() -> Result<()> {
    // Init DB
    let db = Database::open()?;
    db.seed_words()?;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(db);

    // Event loop
    loop {
        // Clear status message after 3 seconds
        if app.status_set_at.map(|t| t.elapsed().as_secs() >= 3).unwrap_or(false) {
            app.status_message.clear();
            app.status_set_at = None;
        }

        // Perform pending dictionary lookup after drawing "Searching..." first
        if app.dict_searching {
            app.dict_searching = false;
            terminal.draw(|f| render(f, &app))?; // show "Searching..." before blocking
            let searched_query = app.dict_query.clone();
            match dict::lookup(&searched_query) {
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
                    app.dict_status = format!("Error: {}", e);
                    app.dict_results.clear();
                }
            }
        }

        terminal.draw(|f| render(f, &app))?;

        if event::poll(Duration::from_millis(50))? {
            let ev = event::read()?;
            if handle_event(&mut app, ev)? {
                break;
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    println!("再见!");
    Ok(())
}
