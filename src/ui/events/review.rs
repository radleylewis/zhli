use anyhow::Result;
use crossterm::event::KeyCode;
use std::time::Instant;

use super::apply_grade;
use crate::srs::ReviewGrade;
use crate::ui::app_state::{App, ReviewPhase, Screen};

pub(super) fn handle_review(app: &mut App, code: KeyCode) -> Result<()> {
    if app.review_queue.is_empty() || app.current_card_idx >= app.review_queue.len() {
        match code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.screen = Screen::MainMenu;
            }
            _ => {}
        }
        return Ok(());
    }

    match app.review_phase {
        ReviewPhase::Prompt => match code {
            KeyCode::Enter => {
                app.submit_answer()?;
            }
            KeyCode::Backspace => {
                app.input_buffer.pop();
            }
            KeyCode::Esc => {
                app.feedback = "Skipped".to_string();
                app.last_score = 0.0;
                app.grade_cursor = 1;
                app.review_phase = ReviewPhase::ShowAnswer;
            }
            KeyCode::Char(c) => {
                app.input_buffer.push(c);
            }
            _ => {}
        },
        ReviewPhase::ShowAnswer => match code {
            KeyCode::Left | KeyCode::Char('h') if app.grade_cursor > 0 => {
                app.grade_cursor -= 1;
            }
            KeyCode::Right | KeyCode::Char('l') if app.grade_cursor < 5 => {
                app.grade_cursor += 1;
            }
            KeyCode::Char('0') => {
                apply_grade(app, ReviewGrade::Blackout);
            }
            KeyCode::Char('1') => {
                apply_grade(app, ReviewGrade::Wrong);
            }
            KeyCode::Char('2') => {
                apply_grade(app, ReviewGrade::Hard);
            }
            KeyCode::Char('3') => {
                apply_grade(app, ReviewGrade::Okay);
            }
            KeyCode::Char('4') => {
                apply_grade(app, ReviewGrade::Good);
            }
            KeyCode::Char('5') => {
                apply_grade(app, ReviewGrade::Perfect);
            }
            KeyCode::Enter => {
                let grade = match app.grade_cursor {
                    0 => ReviewGrade::Blackout,
                    1 => ReviewGrade::Wrong,
                    2 => ReviewGrade::Hard,
                    3 => ReviewGrade::Okay,
                    4 => ReviewGrade::Good,
                    _ => ReviewGrade::Perfect,
                };
                apply_grade(app, grade);
            }
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
            KeyCode::Char('s') => {
                if let Some(card) = app.review_queue.get(app.current_card_idx) {
                    let word_id = card.word_id;
                    let hanzi = card.hanzi.clone();
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
            KeyCode::Char('i') => {
                if let Some(card) = app.review_queue.get(app.current_card_idx) {
                    let encoded: String = card
                        .hanzi
                        .bytes()
                        .map(|b| match b {
                            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                                (b as char).to_string()
                            }
                            b => format!("%{b:02X}"),
                        })
                        .collect();
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
        },
    }
    Ok(())
}
