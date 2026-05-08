use anyhow::Result;
use crossterm::event::KeyCode;
use std::time::Instant;

use crate::ui::app_state::{App, ClearAction, Screen};
use super::spawn_dict_lookup;

pub(super) fn handle_add_to_deck(app: &mut App, code: KeyCode) -> Result<()> {
    if app.creating_new_deck {
        match code {
            KeyCode::Esc => {
                app.creating_new_deck = false;
                app.deck_name_input.clear();
            }
            KeyCode::Backspace => { app.deck_name_input.pop(); }
            KeyCode::Enter => {
                let name = app.deck_name_input.trim().to_string();
                if !name.is_empty() {
                    match app.db.create_deck(&name) {
                        Ok(()) => {
                            app.deck_name_input = name;
                            app.creating_new_deck = false;
                            if let Err(e) = app.load_decks() {
                                app.status_message = format!("Error loading decks: {e}");
                                app.status_set_at = Some(Instant::now());
                            }
                            app.search_query.clear();
                            app.search_results.clear();
                            app.screen = Screen::SearchDeck;
                        }
                        Err(e) => {
                            app.status_message = e.to_string();
                            app.status_set_at = Some(Instant::now());
                        }
                    }
                }
            }
            KeyCode::Char(c) if app.deck_name_input.len() < 64 => { app.deck_name_input.push(c); }
            _ => {}
        }
    } else {
        let total = app.available_decks.len() + 1;
        let max_cursor = total.saturating_sub(1);
        match code {
            KeyCode::Esc => { app.screen = Screen::MainMenu; }
            KeyCode::Up | KeyCode::Char('k')
                if app.deck_cursor > 0 => {
                    app.deck_cursor -= 1;
                    app.load_deck_preview()?;
                }
            KeyCode::Down | KeyCode::Char('j')
                if app.deck_cursor + 1 < total => {
                    app.deck_cursor += 1;
                    app.load_deck_preview()?;
                }
            KeyCode::Char('g')
                if app.last_char == Some('g') => {
                    app.deck_cursor = 0;
                    app.load_deck_preview()?;
                }
            KeyCode::Char('G') => {
                app.deck_cursor = max_cursor;
                let _ = app.load_deck_preview();
            }
            KeyCode::Enter => {
                if app.deck_cursor == 0 {
                    app.creating_new_deck = true;
                    app.deck_name_input.clear();
                } else if let Some(name) = app.available_decks.get(app.deck_cursor - 1) {
                    app.deck_name_input = name.clone();
                    app.search_query.clear();
                    app.search_results.clear();
                    app.screen = Screen::SearchDeck;
                }
            }
            KeyCode::Char('d') | KeyCode::Delete
                if app.deck_cursor > 0 => {
                    if let Some(name) = app.available_decks.get(app.deck_cursor - 1) {
                        app.screen = Screen::Confirm(ClearAction::Deck(name.clone()));
                    }
                }
            _ => {}
        }
    }
    Ok(())
}

pub(super) fn handle_search_deck(app: &mut App, code: KeyCode) -> Result<()> {
    let max_cursor = app.search_results.len();
    match code {
        KeyCode::Esc => { app.screen = Screen::MainMenu; }
        KeyCode::Up
            if app.search_cursor > 0 => { app.search_cursor -= 1; }
        KeyCode::Down
            if app.search_cursor < max_cursor => { app.search_cursor += 1; }
        KeyCode::Enter => {
            if app.search_cursor < app.search_results.len() {
                if let Err(e) = app.add_selected_to_deck() {
                    app.status_message = e.to_string();
                    app.status_set_at = Some(Instant::now());
                }
            } else {
                app.dict_query = app.search_query.clone();
                app.dict_results.clear();
                app.dict_cursor = 0;
                app.dict_status = if app.dict_query.is_empty() {
                    "Type a word and press Enter to search online.".to_string()
                } else {
                    "Press Enter to search online.".to_string()
                };
                app.dict_rx = None;
                app.screen = Screen::AddCustomWord;
            }
        }
        KeyCode::Char('/') => {
            app.dict_query = app.search_query.clone();
            app.dict_results.clear();
            app.dict_cursor = 0;
            if app.dict_query.is_empty() {
                app.dict_status = "Type a word and press Enter to search online.".to_string();
                app.dict_rx = None;
            } else {
                app.dict_status = "Searching...".to_string();
                spawn_dict_lookup(app);
            }
            app.screen = Screen::AddCustomWord;
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            if app.search_query.is_empty() {
                app.search_results.clear();
                app.search_cursor = 0;
            } else {
                app.do_search()?;
            }
        }
        // Uppercase actions — caught before the char catch-all
        KeyCode::Char('D') | KeyCode::Delete
            if app.search_cursor < app.search_results.len() => {
                if let Some(word) = app.search_results.get(app.search_cursor) {
                    app.screen = Screen::Confirm(ClearAction::Word(word.id, word.hanzi.clone()));
                }
            }
        KeyCode::Char('E') => {
            if let Some(word) = app.search_results.get(app.search_cursor) {
                if word.level == 0 {
                    app.edit_word_id = word.id;
                    app.edit_bufs = [word.hanzi.clone(), word.pinyin.clone(), word.english.clone()];
                    app.edit_field = 0;
                    app.screen = Screen::EditWord;
                } else {
                    app.status_message = "Only custom words can be edited.".to_string();
                    app.status_set_at = Some(Instant::now());
                }
            }
        }
        KeyCode::Char('S') => {
            if let Some(word) = app.search_results.get(app.search_cursor) {
                let id = word.id;
                let hanzi = word.hanzi.clone();
                let currently_suspended = word.suspended;
                let result = if currently_suspended {
                    app.db.unsuspend_word(id)
                } else {
                    app.db.suspend_word(id)
                };
                match result {
                    Ok(()) => {
                        app.status_message = if currently_suspended {
                            format!("Unsuspended '{}'.", hanzi)
                        } else {
                            format!("Suspended '{}'.", hanzi)
                        };
                        app.do_search().ok();
                    }
                    Err(e) => { app.status_message = e.to_string(); }
                }
                app.status_set_at = Some(Instant::now());
            }
        }
        KeyCode::Char('R')
            if !app.deck_name_input.is_empty() => {
                if let Some(word) = app.search_results.get(app.search_cursor) {
                    let id = word.id;
                    let hanzi = word.hanzi.clone();
                    let deck = app.deck_name_input.clone();
                    match app.db.remove_word_from_deck(id, &deck) {
                        Ok(()) => {
                            app.status_message = format!("Removed '{}' from '{}'.", hanzi, deck);
                            app.do_search().ok();
                        }
                        Err(e) => { app.status_message = e.to_string(); }
                    }
                    app.status_set_at = Some(Instant::now());
                }
            }
        // All other chars are search input
        KeyCode::Char(c) => {
            app.search_query.push(c);
            app.search_cursor = 0;
            app.do_search()?;
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn handle_add_custom_word(app: &mut App, code: KeyCode) -> Result<()> {
    if app.dict_filter_active {
        match code {
            KeyCode::Esc => {
                app.dict_filter_active = false;
                app.dict_filter.clear();
                app.dict_cursor = 0;
            }
            KeyCode::Backspace => {
                app.dict_filter.pop();
                app.dict_cursor = 0;
            }
            KeyCode::Up
                if app.dict_cursor > 0 => { app.dict_cursor -= 1; }
            KeyCode::Down => {
                let visible = crate::dict::filter_results(&app.dict_results, &app.dict_filter).len();
                if app.dict_cursor + 1 < visible { app.dict_cursor += 1; }
            }
            KeyCode::Enter => {
                let filtered = crate::dict::filter_results(&app.dict_results, &app.dict_filter);
                if let Some(&entry_ref) = filtered.get(app.dict_cursor) {
                    let entry = entry_ref.clone();
                    let deck = if app.deck_name_input.is_empty() { None } else { Some(app.deck_name_input.as_str()) };
                    match app.db.add_custom_word(&entry.hanzi, &entry.pinyin, &entry.english, 0, deck) {
                        Ok(()) => {
                            app.status_message = format!("Added '{}'{}.", entry.hanzi,
                                deck.map(|d| format!(" to '{}'", d)).unwrap_or_default());
                            app.dict_results.clear();
                            app.dict_filter.clear();
                            app.dict_filter_active = false;
                            app.dict_status.clear();
                            app.dict_query.clear();
                            app.dict_last_query.clear();
                            app.screen = Screen::SearchDeck;
                        }
                        Err(e) => { app.status_message = e.to_string(); }
                    }
                    app.status_set_at = Some(Instant::now());
                }
            }
            KeyCode::Char(c) => {
                app.dict_filter.push(c);
                app.dict_cursor = 0;
            }
            _ => {}
        }
        return Ok(());
    }

    match code {
        KeyCode::Esc => {
            app.dict_results.clear();
            app.dict_filter.clear();
            app.dict_filter_active = false;
            app.dict_status.clear();
            app.screen = Screen::SearchDeck;
        }
        KeyCode::Backspace => {
            app.dict_query.pop();
            if !app.dict_results.is_empty() {
                app.dict_status = "Query changed — press Enter to search again.".to_string();
            } else {
                app.dict_status = if app.dict_query.is_empty() {
                    "Type a word and press Enter to search online.".to_string()
                } else {
                    "Press Enter to search online.".to_string()
                };
            }
        }
        KeyCode::Char('/') if !app.dict_results.is_empty() => {
            app.dict_filter_active = true;
            app.dict_filter.clear();
            app.dict_cursor = 0;
        }
        KeyCode::Up
            if app.dict_cursor > 0 => { app.dict_cursor -= 1; }
        KeyCode::Down
            if app.dict_cursor + 1 < app.dict_results.len() => { app.dict_cursor += 1; }
        KeyCode::Enter => {
            let query_changed = app.dict_query != app.dict_last_query;
            if !app.dict_results.is_empty() && !query_changed {
                if let Some(entry) = app.dict_results.get(app.dict_cursor).cloned() {
                    let deck = if app.deck_name_input.is_empty() { None } else { Some(app.deck_name_input.as_str()) };
                    match app.db.add_custom_word(&entry.hanzi, &entry.pinyin, &entry.english, 0, deck) {
                        Ok(()) => {
                            app.status_message = format!("Added '{}'{}.", entry.hanzi,
                                deck.map(|d| format!(" to '{}'", d)).unwrap_or_default());
                            app.dict_results.clear();
                            app.dict_status.clear();
                            app.dict_query.clear();
                            app.dict_last_query.clear();
                            app.screen = Screen::SearchDeck;
                        }
                        Err(e) => { app.status_message = e.to_string(); }
                    }
                    app.status_set_at = Some(Instant::now());
                }
            } else if !app.dict_query.is_empty() {
                app.dict_status = "Searching...".to_string();
                spawn_dict_lookup(app);
            }
        }
        KeyCode::Char(c) => {
            app.dict_query.push(c);
            if !app.dict_results.is_empty() {
                app.dict_status = "Query changed — press Enter to search again.".to_string();
            } else {
                app.dict_status = "Press Enter to search online.".to_string();
            }
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn handle_edit_word(app: &mut App, code: KeyCode) -> Result<()> {
    match code {
        KeyCode::Esc => { app.screen = Screen::SearchDeck; }
        KeyCode::Tab | KeyCode::Down => {
            app.edit_field = (app.edit_field + 1) % 3;
        }
        KeyCode::BackTab | KeyCode::Up => {
            app.edit_field = (app.edit_field + 2) % 3;
        }
        KeyCode::Backspace => { app.edit_bufs[app.edit_field].pop(); }
        KeyCode::Enter => {
            let hanzi   = app.edit_bufs[0].trim().to_string();
            let pinyin  = app.edit_bufs[1].trim().to_string();
            let english = app.edit_bufs[2].trim().to_string();
            if hanzi.is_empty() || pinyin.is_empty() || english.is_empty() {
                app.status_message = "All fields are required.".to_string();
                app.status_set_at = Some(Instant::now());
            } else {
                match app.db.update_word(app.edit_word_id, &hanzi, &pinyin, &english) {
                    Ok(()) => {
                        app.status_message = format!("Updated '{}'.", hanzi);
                        app.do_search().ok();
                        app.screen = Screen::SearchDeck;
                    }
                    Err(e) => {
                        app.status_message = e.to_string();
                    }
                }
                app.status_set_at = Some(Instant::now());
            }
        }
        KeyCode::Char(c) => { app.edit_bufs[app.edit_field].push(c); }
        _ => {}
    }
    Ok(())
}
