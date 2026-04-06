use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Instant;
use std::sync::mpsc;

use crate::ui::app_state::{App, ClearAction, ContentItem, ContentKind, ReviewPhase, Screen};
use crate::srs::{CardDirection, ReviewGrade};

fn spawn_dict_lookup(app: &mut App) {
    let (tx, rx) = mpsc::channel();
    app.dict_rx = Some(rx);
    let query = app.dict_query.clone();
    std::thread::spawn(move || {
        let _ = tx.send(crate::dict::lookup(&query));
    });
}

/// Returns true if the app should quit
pub fn handle_event(app: &mut App, event: Event) -> Result<bool> {
    let Event::Key(KeyEvent { code, modifiers, .. }) = event else { return Ok(false); };

    // Global quit
    if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
        return Ok(true);
    }

    // If command buffer is active, route all input there
    if app.cmd_buffer.is_some() {
        return handle_command_mode(app, code);
    }

    // Enter vim command mode on ':' — but not when the user is typing in a text field
    let in_text_input = matches!(app.screen, Screen::SearchDeck | Screen::AddCustomWord)
        || (matches!(app.screen, Screen::AddToDeck) && app.creating_new_deck);
    if !in_text_input && code == KeyCode::Char(':') {
        app.cmd_buffer = Some(String::new());
        app.last_char = None;
        return Ok(false);
    }

    let prev_screen = app.screen.clone();
    let quit = match app.screen.clone() {
        Screen::MainMenu        => handle_main_menu(app, code),
        Screen::ModeSelect      => { handle_mode_select(app, code); false }
        Screen::ContentSelect   => { handle_content_select(app, code)?; false }
        Screen::Review          => { handle_review(app, code)?; false }
        Screen::Stats           => { handle_stats(app, code); false }
        Screen::AddToDeck       => { handle_add_to_deck(app, code)?; false }
        Screen::AddCustomWord   => { handle_add_custom_word(app, code)?; false }
        Screen::SearchDeck      => { handle_search_deck(app, code)?; false }
        Screen::About           => { handle_about(app, code); false }
        Screen::Confirm(action) => { handle_confirm(app, code, action)?; false }
    };

    // Clear multi-key state on screen transitions so stale 'g' doesn't bleed through
    if app.screen != prev_screen {
        app.last_char = None;
    } else {
        // Track last character for multi-key sequences (gg, etc.)
        // Clear when in text-input screens so stale state doesn't bleed through
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

/// Handle vim command-line mode (`:q`, `:q!`, etc.)
fn handle_command_mode(app: &mut App, code: KeyCode) -> Result<bool> {
    match code {
        KeyCode::Esc => {
            app.cmd_buffer = None;
        }
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
            match cmd.trim() {
                "q" => {
                    match app.screen {
                        Screen::MainMenu => return Ok(true),
                        Screen::AddCustomWord => { app.screen = Screen::SearchDeck; }
                        Screen::ContentSelect => { app.screen = Screen::ModeSelect; }
                        _ => { app.screen = Screen::MainMenu; }
                    }
                }
                "q!" => return Ok(true),
                _ => {
                    // Unknown command — show brief feedback
                    app.status_message = format!("Unknown command: :{}", cmd.trim());
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

/// Returns true if the app should quit
fn handle_main_menu(app: &mut App, code: KeyCode) -> bool {
    match code {
        KeyCode::Up | KeyCode::Char('k') => {
            if app.menu_cursor > 0 { app.menu_cursor -= 1; }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.menu_cursor < 7 { app.menu_cursor += 1; }
        }
        KeyCode::Char('g') => {
            if app.last_char == Some('g') {
                app.menu_cursor = 0;
            }
        }
        KeyCode::Char('G') => {
            app.menu_cursor = 7;
        }
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
                7 => { return true; } // Quit
                _ => {}
            }
        }
        KeyCode::Char('q') | KeyCode::Char('Q') => { return true; }
        _ => {}
    }
    false
}

fn handle_mode_select(app: &mut App, code: KeyCode) {
    let directions = [
        CardDirection::ZhToPinyin,
        CardDirection::ZhToEn,
        CardDirection::EnToZh,
        CardDirection::PinyinToZh,
    ];
    match code {
        KeyCode::Up | KeyCode::Char('k') => {
            if app.mode_cursor > 0 { app.mode_cursor -= 1; }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.mode_cursor < 3 { app.mode_cursor += 1; }
        }
        KeyCode::Char('g') => {
            if app.last_char == Some('g') {
                app.mode_cursor = 0;
            }
        }
        KeyCode::Char('G') => {
            app.mode_cursor = 3;
        }
        KeyCode::Enter => {
            app.selected_directions = vec![directions[app.mode_cursor].clone()];
            let decks = app.db.list_decks().unwrap_or_default();
            // Snapshot old selection state so toggled items survive re-entering ModeSelect
            let old_hsk: Vec<(u8, bool)> = app.content_items.iter()
                .filter_map(|it| if let ContentKind::Hsk(l) = it.kind { Some((l, it.selected)) } else { None })
                .collect();
            let old_decks: Vec<(String, bool)> = app.content_items.iter()
                .filter_map(|it| if let ContentKind::Deck(d) = &it.kind { Some((d.clone(), it.selected)) } else { None })
                .collect();
            app.content_items = (1u8..=6)
                .map(|l| {
                    let sel = old_hsk.iter().find(|(level, _)| *level == l).map(|(_, s)| *s).unwrap_or(true);
                    ContentItem { kind: ContentKind::Hsk(l), selected: sel }
                })
                .chain(decks.into_iter().map(|d| {
                    let sel = old_decks.iter().find(|(name, _)| name == &d).map(|(_, s)| *s).unwrap_or(false);
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

fn handle_review(app: &mut App, code: KeyCode) -> Result<()> {
    // Session complete screen — go back to menu on any sensible key
    if app.review_queue.is_empty() || app.current_card_idx >= app.review_queue.len() {
        match code {
            KeyCode::Esc | KeyCode::Enter
            | KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.screen = Screen::MainMenu;
            }
            _ => {}
        }
        return Ok(());
    }

    match app.review_phase {
        ReviewPhase::Prompt => {
            match code {
                KeyCode::Enter => {
                    app.submit_answer()?;
                }
                KeyCode::Backspace => {
                    app.input_buffer.pop();
                }
                KeyCode::Esc => {
                    // Skip card
                    app.feedback = "Skipped".to_string();
                    app.last_score = 0.0;
                    app.grade_cursor = 1;
                    app.review_phase = ReviewPhase::ShowAnswer;
                }
                KeyCode::Char(c) => {
                    app.input_buffer.push(c);
                }
                _ => {}
            }
        }
        ReviewPhase::ShowAnswer => {
            match code {
                // Navigate grade options with arrows / h / l
                KeyCode::Left | KeyCode::Char('h') => {
                    if app.grade_cursor > 0 { app.grade_cursor -= 1; }
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    if app.grade_cursor < 5 { app.grade_cursor += 1; }
                }
                // Number shortcuts still work
                KeyCode::Char('0') => { app.apply_grade(ReviewGrade::Blackout)?; }
                KeyCode::Char('1') => { app.apply_grade(ReviewGrade::Wrong)?; }
                KeyCode::Char('2') => { app.apply_grade(ReviewGrade::Hard)?; }
                KeyCode::Char('3') => { app.apply_grade(ReviewGrade::Okay)?; }
                KeyCode::Char('4') => { app.apply_grade(ReviewGrade::Good)?; }
                KeyCode::Char('5') => { app.apply_grade(ReviewGrade::Perfect)?; }
                // Enter applies the highlighted grade
                KeyCode::Enter => {
                    let grade = match app.grade_cursor {
                        0 => ReviewGrade::Blackout,
                        1 => ReviewGrade::Wrong,
                        2 => ReviewGrade::Hard,
                        3 => ReviewGrade::Okay,
                        4 => ReviewGrade::Good,
                        _ => ReviewGrade::Perfect,
                    };
                    app.apply_grade(grade)?;
                }
                // Yank hanzi to clipboard
                // The Clipboard object is kept alive in app.clipboard so content persists on X11
                KeyCode::Char('y') => {
                    if let Some(card) = app.review_queue.get(app.current_card_idx) {
                        let hanzi = card.hanzi.clone();
                        let ok = if let Some(ref mut cb) = app.clipboard {
                            cb.set_text(hanzi.clone()).is_ok()
                        } else {
                            match arboard::Clipboard::new() {
                                Ok(mut cb) => {
                                    let ok = cb.set_text(hanzi.clone()).is_ok();
                                    app.clipboard = Some(cb);
                                    ok
                                }
                                Err(_) => false,
                            }
                        };
                        app.status_message = if ok {
                            format!("Copied \"{}\" to clipboard", hanzi)
                        } else {
                            "Failed to access clipboard".to_string()
                        };
                        app.status_set_at = Some(Instant::now());
                    }
                }
                // Suspend word — removes it from future review queues
                KeyCode::Char('s') => {
                    if let Some(card) = app.review_queue.get(app.current_card_idx) {
                        let word_id = card.word_id;
                        let hanzi   = card.hanzi.clone();
                        match app.db.suspend_word(word_id) {
                            Ok(()) => {
                                app.status_message = format!("Suspended '{}'", hanzi);
                                app.advance_card();
                            }
                            Err(e) => {
                                app.status_message = format!("Error suspending: {}", e);
                            }
                        }
                        app.status_set_at = Some(Instant::now());
                    }
                }
                // Open in MDBG online dictionary
                KeyCode::Char('i') => {
                    if let Some(card) = app.review_queue.get(app.current_card_idx) {
                        let encoded: String = card.hanzi.bytes().map(|b| match b {
                            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9'
                            | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
                            b => format!("%{b:02X}"),
                        }).collect();
                        let url = format!(
                            "https://www.mdbg.net/chinese/dictionary?page=worddict&wdrst=0&wdqb={encoded}"
                        );
                        match open::that(&url) {
                            Ok(()) => {
                                app.status_message = format!("Opening \"{}\" in browser…", card.hanzi);
                            }
                            Err(_) => {
                                app.status_message = "Failed to open browser".to_string();
                            }
                        }
                        app.status_set_at = Some(Instant::now());
                    }
                }
                KeyCode::Esc => {
                    app.screen = Screen::MainMenu;
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn handle_stats(app: &mut App, code: KeyCode) {
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

fn handle_add_to_deck(app: &mut App, code: KeyCode) -> Result<()> {
    if app.creating_new_deck {
        // Typing a new deck name
        match code {
            KeyCode::Esc => {
                app.creating_new_deck = false;
                app.deck_name_input.clear();
            }
            KeyCode::Backspace => { app.deck_name_input.pop(); }
            KeyCode::Enter => {
                if !app.deck_name_input.trim().is_empty() {
                    app.db.create_deck(app.deck_name_input.trim())?;
                    app.deck_name_input = app.deck_name_input.trim().to_string();
                    app.creating_new_deck = false;
                    app.load_decks()?;
                    app.search_query.clear();
                    app.search_results.clear();
                    app.screen = Screen::SearchDeck;
                }
            }
            KeyCode::Char(c) => { if app.deck_name_input.len() < 64 { app.deck_name_input.push(c); } }
            _ => {}
        }
    } else {
        // Browsing existing decks
        let total = app.available_decks.len() + 1; // +1 for "New Deck"
        let max_cursor = total.saturating_sub(1);
        match code {
            KeyCode::Esc => { app.screen = Screen::MainMenu; }
            KeyCode::Up | KeyCode::Char('k') => {
                if app.deck_cursor > 0 {
                    app.deck_cursor -= 1;
                    app.load_deck_preview()?;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if app.deck_cursor + 1 < total {
                    app.deck_cursor += 1;
                    app.load_deck_preview()?;
                }
            }
            KeyCode::Char('g') => {
                if app.last_char == Some('g') {
                    app.deck_cursor = 0;
                    app.load_deck_preview()?;
                }
            }
            KeyCode::Char('G') => {
                app.deck_cursor = max_cursor;
                let _ = app.load_deck_preview();
            }
            KeyCode::Enter => {
                if app.deck_cursor == 0 {
                    // "New Deck" selected — switch to input mode
                    app.creating_new_deck = true;
                    app.deck_name_input.clear();
                } else if let Some(name) = app.available_decks.get(app.deck_cursor - 1) {
                    // Existing deck selected — go straight to search
                    app.deck_name_input = name.clone();
                    app.search_query.clear();
                    app.search_results.clear();
                    app.screen = Screen::SearchDeck;
                }
            }
            KeyCode::Char('d') | KeyCode::Delete => {
                // Delete the highlighted deck (not available on "New Deck" row)
                if app.deck_cursor > 0 {
                    if let Some(name) = app.available_decks.get(app.deck_cursor - 1) {
                        app.screen = Screen::Confirm(ClearAction::Deck(name.clone()));
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn handle_search_deck(app: &mut App, code: KeyCode) -> Result<()> {
    // cursor can reach search_results.len() — the "＋ Add custom word" item
    let max_cursor = app.search_results.len();
    match code {
        KeyCode::Esc => { app.screen = Screen::MainMenu; }
        KeyCode::Up => {
            if app.search_cursor > 0 { app.search_cursor -= 1; }
        }
        KeyCode::Down => {
            if app.search_cursor < max_cursor { app.search_cursor += 1; }
        }
        KeyCode::Enter => {
            if app.search_cursor < app.search_results.len() {
                app.add_selected_to_deck()?;
            } else {
                // "＋ Add custom word" selected — pre-fill query from search box
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
        // '/' jumps straight to online dictionary search, firing immediately if query is set
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
                // Clear results so the ＋ Add item stays visible at position 0
                app.search_results.clear();
                app.search_cursor = 0;
            } else {
                app.do_search()?;
            }
        }
        KeyCode::Char('D') | KeyCode::Delete => {
            if app.search_cursor < app.search_results.len() {
                if let Some(word) = app.search_results.get(app.search_cursor) {
                    app.screen = Screen::Confirm(ClearAction::Word(word.id, word.hanzi.clone()));
                }
            }
        }
        // j/k/d and other chars are valid search input — do NOT intercept them
        KeyCode::Char(c) => {
            app.search_query.push(c);
            app.search_cursor = 0;
            app.do_search()?;
        }
        _ => {}
    }
    Ok(())
}

fn handle_add_custom_word(app: &mut App, code: KeyCode) -> Result<()> {
    // ── Filter mode (activated with '/') ─────────────────────────────────────
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
            KeyCode::Up => {
                if app.dict_cursor > 0 { app.dict_cursor -= 1; }
            }
            KeyCode::Down => {
                let visible = crate::dict::filter_results(&app.dict_results, &app.dict_filter).len();
                if app.dict_cursor + 1 < visible { app.dict_cursor += 1; }
            }
            KeyCode::Enter => {
                let filtered = crate::dict::filter_results(&app.dict_results, &app.dict_filter);
                if let Some(&entry_ref) = filtered.get(app.dict_cursor) {
                    let entry = entry_ref.clone();
                    let deck = if app.deck_name_input.is_empty() { None } else { Some(app.deck_name_input.as_str()) };
                    app.db.add_custom_word(&entry.hanzi, &entry.pinyin, &entry.english, 0, deck)?;
                    app.status_message = format!("Added '{}'{}.", entry.hanzi,
                        deck.map(|d| format!(" to '{}'", d)).unwrap_or_default());
                    app.status_set_at = Some(Instant::now());
                    app.dict_results.clear();
                    app.dict_filter.clear();
                    app.dict_filter_active = false;
                    app.dict_status.clear();
                    app.dict_query.clear();
                    app.dict_last_query.clear();
                    app.screen = Screen::SearchDeck;
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

    // ── Normal mode ───────────────────────────────────────────────────────────
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
            // Keep existing results visible; user can press Enter to re-search
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
        // '/' enters filter mode when results are present
        KeyCode::Char('/') if !app.dict_results.is_empty() => {
            app.dict_filter_active = true;
            app.dict_filter.clear();
            app.dict_cursor = 0;
        }
        // Arrow keys navigate results; j/k are valid query characters
        KeyCode::Up => {
            if app.dict_cursor > 0 { app.dict_cursor -= 1; }
        }
        KeyCode::Down => {
            if app.dict_cursor + 1 < app.dict_results.len() {
                app.dict_cursor += 1;
            }
        }
        KeyCode::Enter => {
            let query_changed = app.dict_query != app.dict_last_query;
            if !app.dict_results.is_empty() && !query_changed {
                // Add the highlighted result
                if let Some(entry) = app.dict_results.get(app.dict_cursor).cloned() {
                    let deck = if app.deck_name_input.is_empty() { None } else { Some(app.deck_name_input.as_str()) };
                    app.db.add_custom_word(&entry.hanzi, &entry.pinyin, &entry.english, 0, deck)?;
                    app.status_message = format!("Added '{}'{}.", entry.hanzi,
                        deck.map(|d| format!(" to '{}'", d)).unwrap_or_default());
                    app.status_set_at = Some(Instant::now());
                    app.dict_results.clear();
                    app.dict_status.clear();
                    app.dict_query.clear();
                    app.dict_last_query.clear();
                    app.screen = Screen::SearchDeck;
                }
            } else if !app.dict_query.is_empty() {
                app.dict_status = "Searching...".to_string();
                spawn_dict_lookup(app);
            }
        }
        KeyCode::Char(c) => {
            app.dict_query.push(c);
            // Keep existing results visible; status prompts user to re-search if needed
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

fn handle_content_select(app: &mut App, code: KeyCode) -> Result<()> {
    let n = app.content_items.len();
    match code {
        KeyCode::Up | KeyCode::Char('k') => {
            if app.content_cursor > 0 { app.content_cursor -= 1; }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.content_cursor + 1 < n { app.content_cursor += 1; }
        }
        KeyCode::Char('g') => {
            if app.last_char == Some('g') {
                app.content_cursor = 0;
            }
        }
        KeyCode::Char('G') => {
            if n > 0 { app.content_cursor = n - 1; }
        }
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
        KeyCode::Enter => {
            if app.content_items.iter().any(|i| i.selected) {
                app.load_review_session()?;
                if app.review_queue.is_empty() {
                    app.status_message = "No cards due — all caught up!".to_string();
                    app.status_set_at = Some(Instant::now());
                } else {
                    app.screen = Screen::Review;
                }
            }
        }
        KeyCode::Esc => { app.screen = Screen::ModeSelect; }
        _ => {}
    }
    Ok(())
}

fn handle_about(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => {
            app.screen = Screen::MainMenu;
        }
        _ => {}
    }
}

fn handle_confirm(app: &mut App, code: KeyCode, action: ClearAction) -> Result<()> {
    match code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            match action {
                ClearAction::CustomWords => {
                    app.db.clear_custom_words()?;
                    app.status_message = "Custom words cleared.".to_string();
                    app.screen = Screen::MainMenu;
                }
                ClearAction::Progress => {
                    app.db.reset_progress()?;
                    app.status_message = "All progress reset.".to_string();
                    app.screen = Screen::MainMenu;
                }
                ClearAction::Deck(name) => {
                    app.db.delete_deck(&name)?;
                    app.status_message = format!("Deck '{}' deleted.", name);
                    app.load_decks()?;
                    app.deck_cursor = 0;
                    app.screen = Screen::AddToDeck;
                }
                ClearAction::Word(word_id, hanzi) => {
                    app.db.delete_word(word_id)?;
                    app.status_message = format!("Deleted '{}'.", hanzi);
                    app.do_search()?;
                    app.screen = Screen::SearchDeck;
                }
            }
            app.status_set_at = Some(Instant::now());
        }
        _ => {
            // cancelled — return to wherever made sense
            match action {
                ClearAction::Deck(_)   => { app.screen = Screen::AddToDeck; }
                ClearAction::Word(..)  => { app.screen = Screen::SearchDeck; }
                _ => { app.screen = Screen::MainMenu; }
            }
        }
    }
    Ok(())
}
