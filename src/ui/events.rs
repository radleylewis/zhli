use anyhow::Result;
use arboard::Clipboard;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Instant;

use crate::ui::app_state::{App, ReviewPhase, Screen};
use crate::srs::{CardDirection, ReviewGrade};

/// Returns true if the app should quit
pub fn handle_event(app: &mut App, event: Event) -> Result<bool> {
    let Event::Key(KeyEvent { code, modifiers, .. }) = event else { return Ok(false); };

    // Global quit
    if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
        return Ok(true);
    }

    match app.screen.clone() {
        Screen::MainMenu   => {
            if handle_main_menu(app, code) {
                return Ok(true);
            }
        }
        Screen::LevelSelect => handle_level_select(app, code),
        Screen::ModeSelect  => handle_mode_select(app, code),
        Screen::Review      => handle_review(app, code)?,
        Screen::Stats       => handle_stats(app, code),
        Screen::AddToDeck   => handle_add_to_deck(app, code)?,
        Screen::SearchDeck  => handle_search_deck(app, code)?,
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
            if app.menu_cursor < 5 { app.menu_cursor += 1; }
        }
        KeyCode::Enter => {
            match app.menu_cursor {
                0 => {
                    // Study: go through level + mode selection first
                    app.start_review_after_mode = true;
                    app.screen = Screen::LevelSelect;
                }
                1 => {
                    // Configure level & mode without starting review
                    app.start_review_after_mode = false;
                    app.screen = Screen::LevelSelect;
                }
                2 => {
                    let _ = app.refresh_stats();
                    app.screen = Screen::Stats;
                }
                3 => { app.screen = Screen::AddToDeck; }
                4 => {
                    app.search_query.clear();
                    app.search_results.clear();
                    app.screen = Screen::SearchDeck;
                }
                5 => { return true; } // Quit
                _ => {}
            }
        }
        KeyCode::Char('q') | KeyCode::Char('Q') => { return true; }
        _ => {}
    }
    false
}

fn handle_level_select(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Up | KeyCode::Char('k') => {
            if app.level_cursor > 1 { app.level_cursor -= 1; }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.level_cursor < 6 { app.level_cursor += 1; }
        }
        KeyCode::Enter => {
            app.selected_level = app.level_cursor;
            app.screen = Screen::ModeSelect;
        }
        KeyCode::Esc => { app.screen = Screen::MainMenu; }
        _ => {}
    }
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
        KeyCode::Enter => {
            // Single-select: study with exactly the highlighted direction
            app.selected_directions = vec![directions[app.mode_cursor].clone()];
            if app.start_review_after_mode {
                app.start_review_after_mode = false;
                let _ = app.load_review_session();
                app.screen = Screen::Review;
            } else {
                app.screen = Screen::MainMenu;
            }
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
                KeyCode::Char('y') => {
                    if let Some(card) = app.review_queue.get(app.current_card_idx) {
                        if let Ok(mut cb) = Clipboard::new() {
                            let _ = cb.set_text(card.hanzi.clone());
                            app.status_message = format!("Copied \"{}\" to clipboard", card.hanzi);
                            app.status_set_at = Some(Instant::now());
                        }
                    }
                }
                // Open in MDBG online dictionary
                KeyCode::Char('i') => {
                    if let Some(card) = app.review_queue.get(app.current_card_idx) {
                        let url = format!(
                            "https://www.mdbg.net/chinese/dictionary?page=worddict&wdrst=0&wdqb={}",
                            card.hanzi
                        );
                        let _ = open::that(&url);
                        app.status_message = format!("Opening \"{}\" in browser…", card.hanzi);
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
            let _ = app.refresh_stats();
        }
        _ => {}
    }
}

fn handle_add_to_deck(app: &mut App, code: KeyCode) -> Result<()> {
    match code {
        KeyCode::Esc => { app.screen = Screen::MainMenu; }
        KeyCode::Tab => {
            app.search_query.clear();
            app.search_results.clear();
            app.screen = Screen::SearchDeck;
        }
        KeyCode::Backspace => { app.deck_name_input.pop(); }
        KeyCode::Char(c) => { app.deck_name_input.push(c); }
        _ => {}
    }
    Ok(())
}

fn handle_search_deck(app: &mut App, code: KeyCode) -> Result<()> {
    match code {
        KeyCode::Esc => { app.screen = Screen::MainMenu; }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.search_cursor > 0 { app.search_cursor -= 1; }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.search_cursor + 1 < app.search_results.len() {
                app.search_cursor += 1;
            }
        }
        KeyCode::Tab => {
            app.add_selected_to_deck()?;
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            app.do_search()?;
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
            app.do_search()?;
        }
        _ => {}
    }
    Ok(())
}
