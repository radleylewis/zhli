use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Gauge, Paragraph, Wrap},
    Frame,
};

use super::{centered_rect, ACCENT, BG, BG2, BLUE, CYAN, GOLD, GRAY, GREEN, RED, WHITE, YELLOW};
use crate::srs::CardDirection;
use crate::ui::app_state::{App, ReviewPhase};

pub(super) fn render_review(f: &mut Frame, app: &App, area: Rect) {
    if app.review_queue.is_empty() || app.current_card_idx >= app.review_queue.len() {
        render_session_complete(f, app, area);
        return;
    }

    let card = &app.review_queue[app.current_card_idx];
    let direction = CardDirection::from_str(&card.direction);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // progress bar
            Constraint::Length(3),  // header
            Constraint::Length(11), // card prompt
            Constraint::Length(4),  // input
            Constraint::Length(8),  // answer/grading
            Constraint::Min(2),     // footer
        ])
        .margin(2)
        .split(area);

    // Progress bar + stats row
    let total = app.review_queue.len();
    let done = app.current_card_idx;
    let pct = (done * 100).checked_div(total).unwrap_or(0);
    let wrong = app.session_total.saturating_sub(app.session_correct);
    let prog_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(2)])
        .split(chunks[0]);
    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::NONE))
        .gauge_style(Style::default().fg(ACCENT).bg(BG2))
        .percent(pct as u16);
    f.render_widget(gauge, prog_chunks[0]);
    let stats_line = Line::from(vec![
        Span::styled(
            format!("{done} / {total}"),
            Style::default().fg(WHITE).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  │  ", Style::default().fg(GRAY)),
        Span::styled("✓ ", Style::default().fg(GREEN)),
        Span::styled(
            app.session_correct.to_string(),
            Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  ✗ ", Style::default().fg(RED)),
        Span::styled(
            wrong.to_string(),
            Style::default().fg(RED).add_modifier(Modifier::BOLD),
        ),
    ]);
    f.render_widget(
        Paragraph::new(stats_line).alignment(Alignment::Center),
        prog_chunks[1],
    );

    // Mode box
    let mode_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT))
        .title(" Mode ")
        .title_style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD));
    let mode_inner = mode_block.inner(chunks[1]);
    f.render_widget(mode_block, chunks[1]);
    let mode_p = Paragraph::new(direction.display_name())
        .style(Style::default().fg(CYAN).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    f.render_widget(mode_p, mode_inner);

    // Card prompt
    let is_hanzi = matches!(
        &direction,
        CardDirection::ZhToPinyin | CardDirection::ZhToEn
    );
    let raw_prompt = match &direction {
        CardDirection::ZhToPinyin => card.hanzi.as_str(),
        CardDirection::ZhToEn => card.hanzi.as_str(),
        CardDirection::EnToZh => card.english.as_str(),
        CardDirection::PinyinToZh => card.pinyin.as_str(),
    };
    let spaced;
    let prompt_text: &str = if is_hanzi {
        spaced = raw_prompt
            .chars()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join("  ");
        &spaced
    } else {
        raw_prompt
    };

    let card_h = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(10), Constraint::Length(9)])
        .split(chunks[2]);

    // HSK level box
    let hsk_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(GOLD))
        .title(" HSK ")
        .title_style(Style::default().fg(GOLD).add_modifier(Modifier::BOLD));
    let hsk_inner = hsk_block.inner(card_h[1]);
    f.render_widget(hsk_block, card_h[1]);
    let hsk_pad = hsk_inner.height.saturating_sub(1) as usize / 2;
    let mut hsk_lines: Vec<Line> = (0..hsk_pad).map(|_| Line::from("")).collect();
    hsk_lines.push(Line::from(Span::styled(
        if card.level == 0 {
            "Other".to_string()
        } else {
            card.level.to_string()
        },
        Style::default().fg(GOLD).add_modifier(Modifier::BOLD),
    )));
    f.render_widget(
        Paragraph::new(hsk_lines).alignment(Alignment::Center),
        hsk_inner,
    );

    // Word prompt block
    let card_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(BLUE))
        .title(format!(" {} ", direction.prompt_label()))
        .title_style(Style::default().fg(BLUE));

    let card_inner = card_block.inner(card_h[0]);
    f.render_widget(card_block, card_h[0]);

    let inner_height = card_inner.height as usize;
    let text_lines = if is_hanzi {
        1
    } else {
        prompt_text
            .len()
            .div_ceil(card_inner.width.max(1) as usize)
            .max(1)
    };
    let padding = inner_height.saturating_sub(text_lines) / 2;
    let mut prompt_lines: Vec<Line> = (0..padding).map(|_| Line::from("")).collect();
    prompt_lines.push(Line::from(Span::styled(
        prompt_text,
        Style::default().fg(WHITE).add_modifier(Modifier::BOLD),
    )));

    let prompt_p = Paragraph::new(prompt_lines)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    f.render_widget(prompt_p, card_inner);

    // Input box
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if app.review_phase == ReviewPhase::Prompt {
            Style::default().fg(CYAN)
        } else {
            Style::default().fg(GRAY)
        })
        .title(" Your Answer ")
        .title_style(Style::default().fg(CYAN));

    let cursor = if app.review_phase == ReviewPhase::Prompt {
        "█"
    } else {
        ""
    };
    let input_text = format!("{}{}", app.input_buffer, cursor);
    let input_p = Paragraph::new(input_text)
        .block(input_block)
        .style(Style::default().fg(WHITE));
    f.render_widget(input_p, chunks[3]);

    // Answer / grading area
    match app.review_phase {
        ReviewPhase::Prompt => {
            if matches!(direction, CardDirection::ZhToPinyin) {
                render_tone_legend(f, chunks[4]);
            } else {
                let hint = Paragraph::new("Type your answer and press Enter to check")
                    .style(Style::default().fg(GRAY))
                    .alignment(Alignment::Center);
                f.render_widget(hint, chunks[4]);
            }
        }
        ReviewPhase::ShowAnswer => {
            render_answer_panel(f, app, card, chunks[4]);
        }
    }

    // Footer
    let footer = if app.review_phase == ReviewPhase::Prompt {
        "Enter Submit  •  Esc Skip"
    } else {
        "←/h →/l Navigate  •  Enter Confirm  •  0-5 Grade  •  y Yank  •  s Suspend  •  i Lookup  •  Esc Menu"
    };
    let footer_p = Paragraph::new(footer)
        .style(Style::default().fg(GRAY))
        .alignment(Alignment::Center);
    f.render_widget(footer_p, chunks[5]);
}

fn render_tone_legend(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .title(" Tone Guide ")
        .title_style(Style::default().fg(GRAY))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(GRAY));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let tones = Line::from(vec![
        Span::styled(
            "  1 ",
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        ),
        Span::styled("ā  flat ", Style::default().fg(CYAN)),
        Span::styled(
            "   2 ",
            Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
        ),
        Span::styled("á  rise ", Style::default().fg(GREEN)),
        Span::styled(
            "   3 ",
            Style::default().fg(YELLOW).add_modifier(Modifier::BOLD),
        ),
        Span::styled("ǎ  dip  ", Style::default().fg(YELLOW)),
        Span::styled(
            "   4 ",
            Style::default().fg(RED).add_modifier(Modifier::BOLD),
        ),
        Span::styled("à  fall", Style::default().fg(RED)),
    ]);
    let hint = Line::from(Span::styled(
        "  Type a1  a2  a3  a4  or paste ā á ǎ à directly",
        Style::default().fg(GRAY),
    ));

    let lines = vec![Line::from(""), tones, Line::from(""), hint];
    f.render_widget(Paragraph::new(lines), inner);
}

fn render_answer_panel(f: &mut Frame, app: &App, card: &crate::db::CardRow, area: Rect) {
    let feedback_color = if app.last_score >= 1.0 {
        GREEN
    } else if app.last_score >= 0.5 {
        YELLOW
    } else {
        RED
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Length(3)])
        .split(area);

    let answer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(feedback_color))
        .title(format!(" {} ", app.feedback.as_str()))
        .title_style(
            Style::default()
                .fg(feedback_color)
                .add_modifier(Modifier::BOLD),
        );
    let answer_inner = answer_block.inner(chunks[0]);
    f.render_widget(answer_block, chunks[0]);

    let spaced_hanzi = card
        .hanzi
        .chars()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join("  ");
    let answer_lines = vec![
        Line::from(Span::styled(
            spaced_hanzi,
            Style::default().fg(WHITE).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            card.pinyin.as_str(),
            Style::default().fg(CYAN),
        )),
        Line::from(Span::styled(
            card.english.as_str(),
            Style::default().fg(GRAY),
        )),
    ];
    let answer_p = Paragraph::new(answer_lines).alignment(Alignment::Center);
    f.render_widget(answer_p, answer_inner);

    let grade_items = [
        "[0] Blackout",
        "[1] Wrong",
        "[2] Hard",
        "[3] Okay",
        "[4] Good",
        "[5] Perfect",
    ];
    let grade_colors = [RED, RED, YELLOW, YELLOW, GREEN, GREEN];

    let grade_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(GRAY))
        .title(" Rate your recall ")
        .title_style(Style::default().fg(GRAY));
    let grade_inner = grade_block.inner(chunks[1]);
    f.render_widget(grade_block, chunks[1]);

    let per = grade_inner.width / 6;
    let grade_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![Constraint::Length(per); 6])
        .split(grade_inner);

    for (i, (label, color)) in grade_items.iter().zip(grade_colors.iter()).enumerate() {
        let selected = i == app.grade_cursor;
        let style = if selected {
            Style::default()
                .fg(*color)
                .bg(BG2)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default().fg(*color)
        };
        let p = Paragraph::new(*label)
            .style(style)
            .alignment(Alignment::Center);
        f.render_widget(p, grade_layout[i]);
    }
}

fn render_session_complete(f: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(60, 55, area);
    f.render_widget(Clear, popup);

    let accuracy = (app.session_correct * 100)
        .checked_div(app.session_total)
        .unwrap_or(0);
    let wrong = app.session_total.saturating_sub(app.session_correct);

    let elapsed = app
        .session_start
        .map(|s| {
            let secs = s.elapsed().as_secs();
            if secs >= 60 {
                format!("{}m {}s", secs / 60, secs % 60)
            } else {
                format!("{secs}s")
            }
        })
        .unwrap_or_default();

    let border_color = if accuracy >= 80 {
        GREEN
    } else if accuracy >= 50 {
        YELLOW
    } else {
        RED
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Session Complete",
            Style::default()
                .fg(border_color)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Reviewed:  ", Style::default().fg(GRAY)),
            Span::styled(
                app.session_total.to_string(),
                Style::default().fg(WHITE).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Correct:   ", Style::default().fg(GRAY)),
            Span::styled(
                format!("{} ({}%)", app.session_correct, accuracy),
                Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Incorrect: ", Style::default().fg(GRAY)),
            Span::styled(
                wrong.to_string(),
                Style::default().fg(if wrong > 0 { RED } else { GRAY }),
            ),
        ]),
    ];

    if !elapsed.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("  Time:      ", Style::default().fg(GRAY)),
            Span::styled(elapsed, Style::default().fg(CYAN)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  No more cards due — come back later.",
        Style::default().fg(GRAY),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Esc / Enter / Q  return to menu",
        Style::default().fg(GRAY),
    )));

    let p = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .title(" Done ")
            .style(Style::default().bg(BG)),
    );
    f.render_widget(p, popup);
}
