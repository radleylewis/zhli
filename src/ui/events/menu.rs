use anyhow::Result;
use crossterm::event::KeyCode;
use std::time::Instant;

use crate::ui::app_state::{App, ClearAction, ContentItem, ContentKind, Screen};
use crate::srs::CardDirection;

pub(super) fn handle_main_menu(app: &mut App, code: KeyCode) -> bool {
    match code {
        KeyCode::Up | KeyCode::Char('k')
            if app.menu_cursor > 0 => { app.menu_cursor -= 1; }
        KeyCode::Down | KeyCode::Char('j')
            if app.menu_cursor < 7 => { app.menu_cursor += 1; }
        KeyCode::Char('g')
            if app.last_char == Some('g') => { app.menu_cursor = 0; }
        KeyCode::Char('G') => { app.menu_cursor = 7; }
        KeyCode::Enter => {
            match app.menu_cursor {
                0 => { app.screen = Screen::ModeSelect; }
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
                    app.screen = Screen::AddToDeck;
                }
                3 => {
                    app.search_query.clear();
                    app.search_results.clear();
                    app.screen = Screen::SearchDeck;
                }
                4 => { app.screen = Screen::About; }
                5 => { app.screen = Screen::Confirm(ClearAction::CustomWords); }
                6 => { app.screen = Screen::Confirm(ClearAction::Progress); }
                7 => { return true; }
                _ => {}
            }
        }
        KeyCode::Char('q') | KeyCode::Char('Q') => { return true; }
        _ => {}
    }
    false
}

pub(super) fn handle_mode_select(app: &mut App, code: KeyCode) {
    let directions = [
        CardDirection::ZhToPinyin,
        CardDirection::ZhToEn,
        CardDirection::EnToZh,
        CardDirection::PinyinToZh,
    ];
    match code {
        KeyCode::Up | KeyCode::Char('k')
            if app.mode_cursor > 0 => { app.mode_cursor -= 1; }
        KeyCode::Down | KeyCode::Char('j')
            if app.mode_cursor < 3 => { app.mode_cursor += 1; }
        KeyCode::Char('g')
            if app.last_char == Some('g') => { app.mode_cursor = 0; }
        KeyCode::Char('G') => { app.mode_cursor = 3; }
        KeyCode::Enter => {
            app.selected_directions = vec![directions[app.mode_cursor].clone()];
            let decks = app.db.list_decks().unwrap_or_default();
            let old_hsk: Vec<(u8, bool)> = app.content_items.iter()
                .filter_map(|it| if let ContentKind::Hsk(l) = it.kind { Some((l, it.selected)) } else { None })
                .collect();
            let old_decks: Vec<(String, bool)> = app.content_items.iter()
                .filter_map(|it| if let ContentKind::Deck(d) = &it.kind { Some((d.clone(), it.selected)) } else { None })
                .collect();
            let saved_deck_sel: Vec<String> = app.db.load_setting("selected_decks").ok().flatten()
                .map(|v| v.split('\x1F').filter(|s| !s.is_empty()).map(|s| s.to_string()).collect())
                .unwrap_or_default();
            app.content_items = (1u8..=6)
                .map(|l| {
                    let sel = old_hsk.iter().find(|(level, _)| *level == l).map(|(_, s)| *s).unwrap_or(true);
                    ContentItem { kind: ContentKind::Hsk(l), selected: sel }
                })
                .chain(decks.into_iter().map(|d| {
                    let sel = old_decks.iter().find(|(name, _)| name == &d)
                        .map(|(_, s)| *s)
                        .unwrap_or_else(|| saved_deck_sel.contains(&d));
                    ContentItem { kind: ContentKind::Deck(d), selected: sel }
                }))
                .collect();
            app.content_cursor = 0;
            app.save_study_settings();
            app.screen = Screen::ContentSelect;
        }
        KeyCode::Esc => { app.screen = Screen::MainMenu; }
        _ => {}
    }
}

pub(super) fn handle_content_select(app: &mut App, code: KeyCode) -> Result<()> {
    let n = app.content_items.len();
    match code {
        KeyCode::Up | KeyCode::Char('k')
            if app.content_cursor > 0 => { app.content_cursor -= 1; }
        KeyCode::Down | KeyCode::Char('j')
            if app.content_cursor + 1 < n => { app.content_cursor += 1; }
        KeyCode::Char('g')
            if app.last_char == Some('g') => { app.content_cursor = 0; }
        KeyCode::Char('G')
            if n > 0 => { app.content_cursor = n - 1; }
        KeyCode::Char(' ') => {
            if let Some(item) = app.content_items.get_mut(app.content_cursor) {
                item.selected = !item.selected;
            }
            app.save_study_settings();
        }
        KeyCode::Char('a') => {
            for item in &mut app.content_items { item.selected = true; }
            app.save_study_settings();
        }
        KeyCode::Char('n') => {
            for item in &mut app.content_items { item.selected = false; }
            app.save_study_settings();
        }
        KeyCode::Enter
            if app.content_items.iter().any(|i| i.selected) => {
                match app.load_review_session() {
                    Ok(()) => {
                        if app.review_queue.is_empty() {
                            app.status_message = "No cards due — all caught up!".to_string();
                            app.status_set_at = Some(Instant::now());
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
        KeyCode::Esc => { app.screen = Screen::ModeSelect; }
        _ => {}
    }
    Ok(())
}

pub(super) fn handle_stats(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => { app.screen = Screen::MainMenu; }
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
        KeyCode::Down | KeyCode::Char('j') => { app.stats_scroll += 1; }
        _ => {}
    }
}

pub(super) fn handle_about(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => { app.screen = Screen::MainMenu; }
        _ => {}
    }
}

pub(super) fn handle_confirm(app: &mut App, code: KeyCode, action: ClearAction) -> Result<()> {
    match code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            let result = match action {
                ClearAction::CustomWords => {
                    app.db.clear_custom_words().map(|()| {
                        app.status_message = "Custom words cleared.".to_string();
                        app.screen = Screen::MainMenu;
                    })
                }
                ClearAction::Progress => {
                    app.db.reset_progress().map(|()| {
                        app.status_message = "All progress reset.".to_string();
                        app.screen = Screen::MainMenu;
                    })
                }
                ClearAction::Deck(name) => {
                    app.db.delete_deck(&name)
                        .and_then(|()| app.load_decks())
                        .map(|()| {
                            app.status_message = format!("Deck '{}' deleted.", name);
                            app.deck_cursor = 0;
                            app.screen = Screen::AddToDeck;
                        })
                }
                ClearAction::Word(word_id, hanzi) => {
                    app.db.delete_word(word_id)
                        .and_then(|()| app.do_search())
                        .map(|()| {
                            app.status_message = format!("Deleted '{}'.", hanzi);
                            app.screen = Screen::SearchDeck;
                        })
                }
            };
            if let Err(e) = result {
                app.status_message = format!("Error: {e}");
                app.screen = Screen::MainMenu;
            }
            app.status_set_at = Some(Instant::now());
        }
        _ => {
            match action {
                ClearAction::Deck(_)  => { app.screen = Screen::AddToDeck; }
                ClearAction::Word(..) => { app.screen = Screen::SearchDeck; }
                _ => { app.screen = Screen::MainMenu; }
            }
        }
    }
    Ok(())
}
