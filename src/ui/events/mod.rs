mod deck;
mod menu;
mod review;

use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Instant;
use std::sync::mpsc;

use crate::ui::app_state::{App, Screen};
use crate::srs::ReviewGrade;

pub(super) fn apply_grade(app: &mut App, grade: ReviewGrade) {
    if let Err(e) = app.apply_grade(grade) {
        app.status_message = format!("Error saving grade: {e}");
        app.status_set_at = Some(Instant::now());
    }
}

pub(super) fn spawn_dict_lookup(app: &mut App) {
    let (tx, rx) = mpsc::channel();
    app.dict_rx = Some(rx);
    app.dict_lookup_started = Some(Instant::now());
    let query = app.dict_query.clone();
    std::thread::spawn(move || {
        let _ = tx.send(crate::dict::lookup(&query));
    });
}

/// Returns true if the app should quit.
pub fn handle_event(app: &mut App, event: Event) -> Result<bool> {
    let Event::Key(KeyEvent { code, modifiers, .. }) = event else { return Ok(false); };

    if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
        return Ok(true);
    }

    if code == KeyCode::Char('z') && modifiers.contains(KeyModifiers::CONTROL) {
        app.should_suspend = true;
        return Ok(false);
    }

    if app.cmd_buffer.is_some() {
        return handle_command_mode(app, code);
    }

    let in_text_input = matches!(
        app.screen,
        Screen::SearchDeck | Screen::AddCustomWord | Screen::EditWord
    ) || (matches!(app.screen, Screen::AddToDeck) && (app.creating_new_deck || app.renaming_deck));

    if !in_text_input && code == KeyCode::Char(':') {
        app.cmd_buffer = Some(String::new());
        app.last_char = None;
        return Ok(false);
    }

    let prev_screen = app.screen.clone();
    let quit = match app.screen.clone() {
        Screen::MainMenu        => menu::handle_main_menu(app, code),
        Screen::ModeSelect      => { menu::handle_mode_select(app, code); false }
        Screen::ContentSelect   => { menu::handle_content_select(app, code)?; false }
        Screen::Review          => { review::handle_review(app, code)?; false }
        Screen::Stats           => { menu::handle_stats(app, code); false }
        Screen::AddToDeck       => { deck::handle_add_to_deck(app, code)?; false }
        Screen::AddCustomWord   => { deck::handle_add_custom_word(app, code)?; false }
        Screen::SearchDeck      => { deck::handle_search_deck(app, code)?; false }
        Screen::EditWord        => { deck::handle_edit_word(app, code)?; false }
        Screen::About           => { menu::handle_about(app, code); false }
        Screen::SuspendedWords  => { menu::handle_suspended_words(app, code)?; false }
        Screen::Confirm(action) => { menu::handle_confirm(app, code, action)?; false }
    };

    if app.screen != prev_screen {
        app.last_char = None;
    } else {
        app.last_char = if in_text_input {
            None
        } else if let KeyCode::Char(c) = code {
            Some(c)
        } else {
            None
        };
    }

    Ok(quit)
}

fn handle_command_mode(app: &mut App, code: KeyCode) -> Result<bool> {
    match code {
        KeyCode::Esc => { app.cmd_buffer = None; }
        KeyCode::Backspace => {
            if let Some(buf) = &mut app.cmd_buffer {
                if buf.is_empty() {
                    app.cmd_buffer = None;
                } else {
                    buf.pop();
                }
            }
        }
        KeyCode::Enter => {
            let cmd = app.cmd_buffer.take().unwrap_or_default();
            let cmd = cmd.trim();
            match cmd {
                "q" => {
                    match app.screen {
                        Screen::MainMenu      => return Ok(true),
                        Screen::AddCustomWord => { app.screen = Screen::SearchDeck; }
                        Screen::ContentSelect => { app.screen = Screen::ModeSelect; }
                        Screen::EditWord      => { app.screen = Screen::SearchDeck; }
                        _ => { app.screen = Screen::MainMenu; }
                    }
                }
                "q!" => return Ok(true),
                _ => {
                    app.status_message = format!("Unknown command: :{cmd}");
                    app.status_set_at = Some(Instant::now());
                }
            }
        }
        KeyCode::Char(c) => {
            if let Some(buf) = &mut app.cmd_buffer {
                buf.push(c);
            }
        }
        _ => {}
    }
    Ok(false)
}

