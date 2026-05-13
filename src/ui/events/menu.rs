use anyhow::Result;
use crossterm::event::KeyCode;
use std::time::Instant;

use crate::srs::CardDirection;
use crate::ui::app_state::{App, ClearAction, Screen};
use dirs;

pub(super) fn handle_main_menu(app: &mut App, code: KeyCode) -> bool {
    match code {
        KeyCode::Up | KeyCode::Char('k') if app.menu_cursor > 0 => {
            app.menu_cursor -= 1;
        }
        KeyCode::Down | KeyCode::Char('j') if app.menu_cursor < 10 => {
            app.menu_cursor += 1;
        }
        KeyCode::Char('g') if app.last_char == Some('g') => {
            app.menu_cursor = 0;
        }
        KeyCode::Char('G') => {
            app.menu_cursor = 10;
        }
        KeyCode::Enter => match app.menu_cursor {
            0 => {
                app.screen = Screen::ModeSelect;
            }
            1 => {
                if let Err(e) = app.refresh_stats() {
                    app.status_message = format!("Failed to load stats: {e}");
                    app.status_set_at = Some(Instant::now());
                }
                app.screen = Screen::Stats;
            }
            2 => {
                if let Err(e) = app.load_decks() {
                    app.status_message = format!("Failed to load decks: {e}");
                    app.status_set_at = Some(Instant::now());
                }
                app.deck_name_input.clear();
                app.creating_new_deck = false;
                app.renaming_deck = false;
                app.screen = Screen::AddToDeck;
            }
            3 => {
                app.search_query.clear();
                app.search_results.clear();
                app.screen = Screen::SearchDeck;
            }
            4 => match app.db.suspended_words() {
                Ok(words) => {
                    app.suspended_words = words;
                    app.suspended_cursor = 0;
                    app.screen = Screen::SuspendedWords;
                }
                Err(e) => {
                    app.status_message = format!("Failed to load: {e}");
                    app.status_set_at = Some(Instant::now());
                }
            },
            5 => {
                match app.db.export_custom_words(&app.export_path) {
                    Ok(n) => {
                        app.status_message =
                            format!("Exported {n} word(s) → {}", app.export_path.display());
                    }
                    Err(e) => {
                        app.status_message = format!("Export failed: {e}");
                    }
                }
                app.status_set_at = Some(Instant::now());
            }
            6 => {
                app.import_path = app.export_path.display().to_string();
                app.screen = Screen::ImportFile;
            }
            7 => {
                app.screen = Screen::About;
            }
            8 => {
                app.screen = Screen::Confirm(ClearAction::CustomWords);
            }
            9 => {
                app.screen = Screen::Confirm(ClearAction::Progress);
            }
            10 => {
                return true;
            }
            _ => {}
        },
        KeyCode::Char('q') | KeyCode::Char('Q') => {
            return true;
        }
        _ => {}
    }
    false
}

pub(super) fn handle_suspended_words(app: &mut App, code: KeyCode) -> Result<()> {
    let n = app.suspended_words.len();
    match code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.screen = Screen::MainMenu;
        }
        KeyCode::Up | KeyCode::Char('k') if app.suspended_cursor > 0 => {
            app.suspended_cursor -= 1;
        }
        KeyCode::Down | KeyCode::Char('j') if app.suspended_cursor + 1 < n => {
            app.suspended_cursor += 1;
        }
        KeyCode::Char('g') if app.last_char == Some('g') => {
            app.suspended_cursor = 0;
        }
        KeyCode::Char('G') if n > 0 => {
            app.suspended_cursor = n - 1;
        }
        KeyCode::Enter | KeyCode::Char('u') => {
            if let Some(word) = app.suspended_words.get(app.suspended_cursor) {
                let id = word.id;
                let hanzi = word.hanzi.clone();
                match app.db.unsuspend_word(id) {
                    Ok(()) => {
                        app.suspended_words.remove(app.suspended_cursor);
                        if app.suspended_cursor > 0
                            && app.suspended_cursor >= app.suspended_words.len()
                        {
                            app.suspended_cursor -= 1;
                        }
                        app.status_message = format!("Unsuspended '{hanzi}'.");
                    }
                    Err(e) => {
                        app.status_message = format!("Error: {e}");
                    }
                }
                app.status_set_at = Some(Instant::now());
            }
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn handle_mode_select(app: &mut App, code: KeyCode) {
    let directions = [
        CardDirection::ZhToPinyin,
        CardDirection::ZhToEn,
        CardDirection::EnToZh,
        CardDirection::PinyinToZh,
    ];
    match code {
        KeyCode::Up | KeyCode::Char('k') if app.mode_cursor > 0 => {
            app.mode_cursor -= 1;
        }
        KeyCode::Down | KeyCode::Char('j') if app.mode_cursor < 3 => {
            app.mode_cursor += 1;
        }
        KeyCode::Char('g') if app.last_char == Some('g') => {
            app.mode_cursor = 0;
        }
        KeyCode::Char('G') => {
            app.mode_cursor = 3;
        }
        KeyCode::Char(' ') => {
            let dir = directions[app.mode_cursor].clone();
            if let Some(pos) = app.selected_directions.iter().position(|d| d == &dir) {
                app.selected_directions.remove(pos);
            } else {
                app.selected_directions.push(dir);
            }
            app.save_study_settings();
        }
        KeyCode::Enter if app.selected_directions.is_empty() => {
            app.status_message = "Select at least one mode (Space to toggle).".to_string();
            app.status_set_at = Some(Instant::now());
        }
        KeyCode::Enter => {
            app.refresh_content_items();
            app.content_cursor = 0;
            app.save_study_settings();
            app.screen = Screen::ContentSelect;
        }
        KeyCode::Esc => {
            app.screen = Screen::MainMenu;
        }
        _ => {}
    }
}

pub(super) fn handle_content_select(app: &mut App, code: KeyCode) -> Result<()> {
    let n = app.content_items.len();
    match code {
        KeyCode::Up | KeyCode::Char('k') if app.content_cursor > 0 => {
            app.content_cursor -= 1;
        }
        KeyCode::Down | KeyCode::Char('j') if app.content_cursor + 1 < n => {
            app.content_cursor += 1;
        }
        KeyCode::Char('g') if app.last_char == Some('g') => {
            app.content_cursor = 0;
        }
        KeyCode::Char('G') if n > 0 => {
            app.content_cursor = n - 1;
        }
        KeyCode::Char(' ') => {
            if let Some(item) = app.content_items.get_mut(app.content_cursor) {
                item.selected = !item.selected;
            }
            app.save_study_settings();
        }
        KeyCode::Char('a') => {
            for item in &mut app.content_items {
                item.selected = true;
            }
            app.save_study_settings();
        }
        KeyCode::Char('n') => {
            for item in &mut app.content_items {
                item.selected = false;
            }
            app.save_study_settings();
        }
        KeyCode::Char('r') => {
            app.refresh_content_items();
        }
        KeyCode::Char('c') => {
            app.cram_mode = !app.cram_mode;
        }
        KeyCode::Enter if app.content_items.iter().any(|i| i.selected) => {
            match app.load_review_session() {
                Ok(()) => {
                    if app.review_queue.is_empty() {
                        app.status_message = "No cards due — all caught up!".to_string();
                        app.status_set_at = Some(Instant::now());
                        app.screen = Screen::MainMenu;
                    } else {
                        app.screen = Screen::Review;
                    }
                }
                Err(e) => {
                    app.status_message = format!("Failed to load cards: {e}");
                    app.status_set_at = Some(Instant::now());
                }
            }
        }
        KeyCode::Esc => {
            app.screen = Screen::ModeSelect;
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn handle_stats(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.screen = Screen::MainMenu;
        }
        KeyCode::Char('r') => {
            if let Err(e) = app.refresh_stats() {
                app.status_message = format!("Failed to refresh stats: {e}");
                app.status_set_at = Some(Instant::now());
            }
            app.stats_scroll = 0;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.stats_scroll = app.stats_scroll.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.stats_scroll += 1;
        }
        _ => {}
    }
}

pub(super) fn handle_about(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => {
            app.screen = Screen::MainMenu;
        }
        _ => {}
    }
}

pub(super) fn handle_import_file(app: &mut App, code: KeyCode) -> Result<()> {
    match code {
        KeyCode::Esc => {
            app.screen = Screen::MainMenu;
        }
        KeyCode::Backspace => {
            app.import_path.pop();
        }
        KeyCode::Enter => {
            let raw = app.import_path.trim().to_string();
            let path = expand_tilde(&raw);
            match app.db.import_words_json(&path) {
                Ok((imported, skipped)) => {
                    app.status_message = format!("Imported {imported} word(s), skipped {skipped}.");
                    app.refresh_content_items();
                }
                Err(e) => {
                    app.status_message = format!("Import failed: {e}");
                }
            }
            app.status_set_at = Some(Instant::now());
            app.screen = Screen::MainMenu;
        }
        KeyCode::Char(c) => {
            app.import_path.push(c);
        }
        _ => {}
    }
    Ok(())
}

fn expand_tilde(s: &str) -> std::path::PathBuf {
    if let Some(rest) = s.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    std::path::PathBuf::from(s)
}

pub(super) fn handle_confirm(app: &mut App, code: KeyCode, action: ClearAction) -> Result<()> {
    match code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            let result = match action {
                ClearAction::CustomWords => app.db.clear_custom_words().map(|()| {
                    app.status_message = "Custom words cleared.".to_string();
                    app.screen = Screen::MainMenu;
                }),
                ClearAction::Progress => app.db.reset_progress().map(|()| {
                    app.status_message = "All progress reset.".to_string();
                    app.screen = Screen::MainMenu;
                }),
                ClearAction::Deck(name) => app
                    .db
                    .delete_deck(&name)
                    .and_then(|()| app.load_decks())
                    .map(|()| {
                        app.status_message = format!("Deck '{}' deleted.", name);
                        app.deck_cursor = 0;
                        app.screen = Screen::AddToDeck;
                    }),
                ClearAction::Word(word_id, hanzi) => app
                    .db
                    .delete_word(word_id)
                    .and_then(|()| app.do_search())
                    .map(|()| {
                        app.status_message = format!("Deleted '{}'.", hanzi);
                        app.screen = Screen::SearchDeck;
                    }),
            };
            if let Err(e) = result {
                app.status_message = format!("Error: {e}");
                app.screen = Screen::MainMenu;
            }
            app.status_set_at = Some(Instant::now());
        }
        _ => match action {
            ClearAction::Deck(_) => {
                app.screen = Screen::AddToDeck;
            }
            ClearAction::Word(..) => {
                app.screen = Screen::SearchDeck;
            }
            _ => {
                app.screen = Screen::MainMenu;
            }
        },
    }
    Ok(())
}
