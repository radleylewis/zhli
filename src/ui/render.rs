use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, BorderType, Clear, Gauge, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::ui::app_state::{App, ClearAction, ContentKind, ReviewPhase, Screen};
use crate::srs::CardDirection;

const RED:    Color = Color::Rgb(220, 50,  50);
const GREEN:  Color = Color::Rgb(80,  200, 80);
const YELLOW: Color = Color::Rgb(220, 180, 40);
const BLUE:   Color = Color::Rgb(80,  140, 220);
const CYAN:   Color = Color::Rgb(80,  210, 200);
const GRAY:   Color = Color::Rgb(120, 120, 120);
const WHITE:  Color = Color::Rgb(230, 230, 230);
const BG:     Color = Color::Rgb(18,  18,  28);
const BG2:    Color = Color::Rgb(28,  28,  42);
const ACCENT: Color = Color::Rgb(180, 80,  220);
const GOLD:   Color = Color::Rgb(220, 170, 40);
const ORANGE: Color = Color::Rgb(220, 140, 40);
const WARN:   Color = Color::Rgb(200, 100, 50);

pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();
    // Background
    f.render_widget(Block::default().style(Style::default().bg(BG)), area);

    match app.screen {
        Screen::MainMenu        => render_main_menu(f, app, area),
        Screen::ModeSelect      => render_mode_select(f, app, area),
        Screen::ContentSelect   => render_content_select(f, app, area),
        Screen::Review          => render_review(f, app, area),
        Screen::Stats           => render_stats(f, app, area),
        Screen::AddToDeck       => render_add_to_deck(f, app, area),
        Screen::AddCustomWord   => render_add_custom_word(f, app, area),
        Screen::SearchDeck      => render_search_deck(f, app, area),
        Screen::EditWord        => render_edit_word(f, app, area),
        Screen::About           => render_about(f, app, area),
        Screen::Confirm(ref action) => render_confirm(f, action, area),
    }

    // Bottom bar: command mode takes priority over status message
    let bar = Rect { x: 0, y: area.height.saturating_sub(1), width: area.width, height: 1 };
    if let Some(ref cmd) = app.cmd_buffer {
        let p = Paragraph::new(format!(":{cmd}"))
            .style(Style::default().fg(WHITE));
        f.render_widget(p, bar);
    } else if !app.status_message.is_empty() {
        let p = Paragraph::new(app.status_message.as_str())
            .style(Style::default().fg(GREEN).bg(Color::Rgb(20,40,20)));
        f.render_widget(p, bar);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn render_main_menu(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .margin(2)
        .split(area);

    // Logo — 中 (zhōng)
    let logo = vec![
        Line::from(Span::styled("        ██        ", Style::default().fg(GOLD))),
        Line::from(Span::styled("  ╔═════██══════╗  ", Style::default().fg(GOLD))),
        Line::from(Span::styled("  ║     ██      ║  ", Style::default().fg(GOLD))),
        Line::from(Span::styled("  ╚═════════════╝  ", Style::default().fg(GOLD))),
        Line::from(Span::styled("        ██        ", Style::default().fg(GOLD))),
        Line::from(Span::styled("Command Line Chinese Trainer!", Style::default().fg(GRAY))),
    ];
    let logo_p = Paragraph::new(logo).alignment(Alignment::Center);
    f.render_widget(logo_p, chunks[0]);

    let items_data = [
        ("📚", "Study / Review Cards",    CYAN),
        ("📊", "Stats & Report Card",     GREEN),
        ("➕", "Add Word to Deck",        BLUE),
        ("🔍", "Search & Browse",         WHITE),
        ("📖", "About / Rules / Algo",    ACCENT),
        ("🗑️", "Clear Custom Words",      WARN),
        ("⚠️", "Reset All Progress",      RED),
        ("❌", "Quit",                    GRAY),
    ];

    let items: Vec<ListItem> = items_data.iter().enumerate().map(|(i, (icon, label, color))| {
        let style = if i == app.menu_cursor {
            Style::default().fg(*color).bg(BG2).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(GRAY)
        };
        let prefix = if i == app.menu_cursor { "▶ " } else { "  " };
        ListItem::new(Line::from(vec![
            Span::styled(format!("{prefix}{icon} {label}"), style),
        ]))
    }).collect();

    let list = List::new(items)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(ACCENT))
            .title(" Menu ")
            .title_style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)));

    f.render_widget(list, chunks[1]);

    // Footer
    let footer = Paragraph::new("↑↓ Navigate  •  Enter Select  •  Q Quit")
        .style(Style::default().fg(GRAY))
        .alignment(Alignment::Center);
    f.render_widget(footer, chunks[2]);
}

fn render_mode_select(f: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(65, 60, area);
    f.render_widget(Clear, popup);

    let block = Block::default()
        .title(" Select Study Mode ")
        .title_style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT))
        .style(Style::default().bg(BG));
    f.render_widget(block, popup);

    let inner = Rect {
        x: popup.x + 2, y: popup.y + 2,
        width: popup.width.saturating_sub(4),
        height: popup.height.saturating_sub(4),
    };

    let directions = [
        (CardDirection::ZhToPinyin, "Chinese → Pinyin",  "See hanzi, write pinyin"),
        (CardDirection::ZhToEn,     "Chinese → English", "See hanzi, write meaning"),
        (CardDirection::EnToZh,     "English → Chinese", "See English, write hanzi"),
        (CardDirection::PinyinToZh, "Pinyin  → Chinese", "See pinyin, write hanzi"),
    ];

    let mut lines = vec![
        Line::from(Span::styled("  Choose a study mode for this session:", Style::default().fg(GRAY))),
        Line::from(""),
    ];

    for (i, (_dir, label, hint)) in directions.iter().enumerate() {
        let selected = i == app.mode_cursor;
        let radio = if selected { "●" } else { "○" };
        let style = if selected {
            Style::default().fg(CYAN).bg(BG2).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(GRAY)
        };
        let prefix = if selected { "▶ " } else { "  " };
        lines.push(Line::from(Span::styled(
            format!("{}{} {}  — {}", prefix, radio, label, hint),
            style,
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  ↑↓ Navigate  •  Enter Select",
        Style::default().fg(GRAY),
    )));

    let p = Paragraph::new(lines);
    f.render_widget(p, inner);
}

pub fn render_review(f: &mut Frame, app: &App, area: Rect) {
    if app.review_queue.is_empty() || app.current_card_idx >= app.review_queue.len() {
        render_session_complete(f, app, area);
        return;
    }

    let card = &app.review_queue[app.current_card_idx];
    let direction = CardDirection::from_str(&card.direction);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),   // progress bar
            Constraint::Length(3),   // header
            Constraint::Length(11),  // card prompt (taller for readability)
            Constraint::Length(4),   // input
            Constraint::Length(8),   // answer/grading (fixed so no blank lines)
            Constraint::Min(2),      // footer (absorbs remaining space)
        ])
        .margin(2)
        .split(area);

    // Progress bar + stats row
    let total = app.review_queue.len();
    let done  = app.current_card_idx;
    let pct   = if total > 0 { done * 100 / total } else { 0 };
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
        Span::styled(format!("{done} / {total}"), Style::default().fg(WHITE).add_modifier(Modifier::BOLD)),
        Span::styled("  │  ", Style::default().fg(GRAY)),
        Span::styled("✓ ", Style::default().fg(GREEN)),
        Span::styled(app.session_correct.to_string(), Style::default().fg(GREEN).add_modifier(Modifier::BOLD)),
        Span::styled("  ✗ ", Style::default().fg(RED)),
        Span::styled(wrong.to_string(), Style::default().fg(RED).add_modifier(Modifier::BOLD)),
    ]);
    f.render_widget(Paragraph::new(stats_line).alignment(Alignment::Center), prog_chunks[1]);

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
    let is_hanzi = matches!(&direction, CardDirection::ZhToPinyin | CardDirection::ZhToEn);
    let raw_prompt = match &direction {
        CardDirection::ZhToPinyin  => card.hanzi.as_str(),
        CardDirection::ZhToEn      => card.hanzi.as_str(),
        CardDirection::EnToZh      => card.english.as_str(),
        CardDirection::PinyinToZh  => card.pinyin.as_str(),
    };
    // Space out hanzi characters so they read larger in the terminal
    let spaced;
    let prompt_text: &str = if is_hanzi {
        spaced = raw_prompt.chars().map(|c| c.to_string()).collect::<Vec<_>>().join("  ");
        &spaced
    } else {
        raw_prompt
    };

    // Split card area: word on the left, HSK level box on the right
    let card_h = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(10), Constraint::Length(9)])
        .split(chunks[2]);

    // HSK level box (right side)
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
        if card.level == 0 { "Other".to_string() } else { card.level.to_string() },
        Style::default().fg(GOLD).add_modifier(Modifier::BOLD),
    )));
    f.render_widget(Paragraph::new(hsk_lines).alignment(Alignment::Center), hsk_inner);

    // Word prompt block (left side)
    let card_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(BLUE))
        .title(format!(" {} ", direction.prompt_label()))
        .title_style(Style::default().fg(BLUE));

    let card_inner = card_block.inner(card_h[0]);
    f.render_widget(card_block, card_h[0]);

    // Vertically center the prompt
    let inner_height = card_inner.height as usize;
    let text_lines = if is_hanzi { 1 } else { prompt_text.len().div_ceil(card_inner.width.max(1) as usize).max(1) };
    let padding = inner_height.saturating_sub(text_lines) / 2;
    let mut prompt_lines: Vec<Line> = (0..padding).map(|_| Line::from("")).collect();
    prompt_lines.push(Line::from(Span::styled(prompt_text, Style::default().fg(WHITE).add_modifier(Modifier::BOLD))));

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

    let cursor = if app.review_phase == ReviewPhase::Prompt { "█" } else { "" };
    let input_text = format!("{}{}", app.input_buffer, cursor);
    let input_p = Paragraph::new(input_text)
        .block(input_block)
        .style(Style::default().fg(WHITE));
    f.render_widget(input_p, chunks[3]);

    // Answer / grading area
    match app.review_phase {
        ReviewPhase::Prompt => {
            let is_pinyin_dir = matches!(direction, CardDirection::ZhToPinyin | CardDirection::PinyinToZh);
            if is_pinyin_dir {
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
        "←/h →/l Navigate  •  Enter Confirm  •  0-5 Grade  •  y Yank  •  i Lookup  •  Esc Menu"
    };
    let footer_p = Paragraph::new(footer)
        .style(Style::default().fg(GRAY))
        .alignment(Alignment::Center);
    f.render_widget(footer_p, chunks[5]);
}

fn render_tone_legend(f: &mut Frame, area: Rect) {
    // Tone colours: 1=cyan (flat), 2=green (rising), 3=yellow (dip), 4=red (falling)
    let t1 = CYAN;
    let t2 = GREEN;
    let t3 = YELLOW;
    let t4 = RED;

    let block = Block::default()
        .title(" Tone Guide ")
        .title_style(Style::default().fg(GRAY))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(GRAY));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let tones = Line::from(vec![
        Span::styled("  1 ", Style::default().fg(t1).add_modifier(Modifier::BOLD)),
        Span::styled("ā  flat ", Style::default().fg(t1)),
        Span::styled("   2 ", Style::default().fg(t2).add_modifier(Modifier::BOLD)),
        Span::styled("á  rise ", Style::default().fg(t2)),
        Span::styled("   3 ", Style::default().fg(t3).add_modifier(Modifier::BOLD)),
        Span::styled("ǎ  dip  ", Style::default().fg(t3)),
        Span::styled("   4 ", Style::default().fg(t4).add_modifier(Modifier::BOLD)),
        Span::styled("à  fall", Style::default().fg(t4)),
    ]);
    let hint = Line::from(Span::styled(
        "  Type a1  a2  a3  a4  or paste ā á ǎ à directly",
        Style::default().fg(GRAY),
    ));

    let lines = vec![Line::from(""), tones, Line::from(""), hint];
    f.render_widget(Paragraph::new(lines), inner);
}

fn render_answer_panel(f: &mut Frame, app: &App, card: &crate::db::CardRow, area: Rect) {
    let feedback_color = if app.last_score >= 1.0 { GREEN }
        else if app.last_score >= 0.5 { YELLOW }
        else { RED };

    // 5 lines: answer info box (border+3 content+border)
    // 3 lines: grade buttons box (border+1 content+border)
    // = exactly 8 lines, zero blanks
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Length(3)])
        .split(area);

    // ── Answer info box ──────────────────────────────────────────────────────
    let answer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(feedback_color))
        .title(format!(" {} ", app.feedback.as_str()))
        .title_style(Style::default().fg(feedback_color).add_modifier(Modifier::BOLD));
    let answer_inner = answer_block.inner(chunks[0]);
    f.render_widget(answer_block, chunks[0]);

    // Show hanzi  •  pinyin  •  english
    let spaced_hanzi = card.hanzi.chars().map(|c| c.to_string()).collect::<Vec<_>>().join("  ");
    let answer_lines = vec![
        Line::from(Span::styled(spaced_hanzi, Style::default().fg(WHITE).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled(card.pinyin.as_str(), Style::default().fg(CYAN))),
        Line::from(Span::styled(card.english.as_str(), Style::default().fg(GRAY))),
    ];
    let answer_p = Paragraph::new(answer_lines).alignment(Alignment::Center);
    f.render_widget(answer_p, answer_inner);

    // ── Grade buttons (single border, one content line) ──────────────────────
    let grade_items = ["[0] Blackout", "[1] Wrong", "[2] Hard", "[3] Okay", "[4] Good", "[5] Perfect"];
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
            Style::default().fg(*color).bg(BG2).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default().fg(*color)
        };
        let p = Paragraph::new(*label).style(style).alignment(Alignment::Center);
        f.render_widget(p, grade_layout[i]);
    }
}

fn render_session_complete(f: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(60, 50, area);
    f.render_widget(Clear, popup);

    let accuracy = if app.session_total > 0 {
        app.session_correct * 100 / app.session_total
    } else { 0 };

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled("  🎉 Session Complete!", Style::default().fg(GREEN).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(Span::styled(format!("  Cards reviewed: {}", app.session_total), Style::default().fg(WHITE))),
        Line::from(Span::styled(format!("  Correct:        {} ({}%)", app.session_correct, accuracy), Style::default().fg(CYAN))),
        Line::from(""),
        Line::from(Span::styled("  No more cards due right now.", Style::default().fg(GRAY))),
        Line::from(Span::styled("  Come back later for the next review.", Style::default().fg(GRAY))),
        Line::from(""),
        Line::from(Span::styled("  Press Esc / Enter / Q to return to menu", Style::default().fg(GRAY))),
    ];

    let p = Paragraph::new(lines)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(GREEN))
            .title(" Done ")
            .style(Style::default().bg(BG)));
    f.render_widget(p, popup);
}

pub fn render_stats(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" 📊 Report Card ")
        .title_style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT))
        .style(Style::default().bg(BG));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let Some(stats) = &app.stats_cache else {
        let p = Paragraph::new("Loading stats...").style(Style::default().fg(GRAY));
        f.render_widget(p, inner);
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    // Left column
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(12), Constraint::Min(8)])
        .split(chunks[0]);

    let retention = if stats.total_reviews > 0 {
        (stats.avg_grade / 5.0 * 100.0) as u16
    } else { 0 };

    let kv = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  🔥 Streak:        ", Style::default().fg(GRAY)),
            Span::styled(format!("{} days", stats.streak), Style::default().fg(YELLOW).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  📅 Reviews today: ", Style::default().fg(GRAY)),
            Span::styled(stats.reviews_today.to_string(), Style::default().fg(CYAN)),
        ]),
        Line::from(vec![
            Span::styled("  📦 Total reviews: ", Style::default().fg(GRAY)),
            Span::styled(stats.total_reviews.to_string(), Style::default().fg(WHITE)),
        ]),
        Line::from(vec![
            Span::styled("  ⏰ Cards due:     ", Style::default().fg(GRAY)),
            Span::styled(stats.cards_due.to_string(), Style::default().fg(RED)),
        ]),
        Line::from(vec![
            Span::styled("  🌱 Learned cards: ", Style::default().fg(GRAY)),
            Span::styled(format!("{}/{}", stats.mature_cards, stats.total_cards), Style::default().fg(GREEN)),
        ]),
        Line::from(vec![
            Span::styled("  📈 Retention:     ", Style::default().fg(GRAY)),
            Span::styled(format!("{}%", retention), Style::default().fg(CYAN)),
        ]),
    ];

    let kv_p = Paragraph::new(kv)
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
            .border_style(Style::default().fg(BLUE)).title(" Overview "));
    f.render_widget(kv_p, left_chunks[0]);

    // Level breakdown
    let level_items: Vec<ListItem> = stats.level_stats.iter().map(|ls| {
        let seen_pct = if ls.card_count > 0 { ls.reviewed * 100 / ls.card_count } else { 0 };
        let bar = "█".repeat((seen_pct / 10) as usize);
        let (label, color) = if let Some(deck) = &ls.deck_name {
            let truncated = if deck.len() > 12 { format!("{}…", &deck[..11]) } else { deck.clone() };
            (format!("{:<12}", truncated), ACCENT)
        } else {
            let color = match ls.level {
                1 => CYAN, 2 => GREEN, 3 => YELLOW,
                4 => ORANGE, 5 => RED, _ => ACCENT,
            };
            (format!("HSK {:<8}", ls.level), color)
        };
        ListItem::new(Line::from(vec![
            Span::styled(format!("  {}", label), Style::default().fg(color).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{:░<10} {:3}% seen", bar, seen_pct), Style::default().fg(GRAY)),
            Span::styled(format!("  {} learned", ls.mature), Style::default().fg(GREEN)),
        ]))
    }).collect();

    let level_list = List::new(level_items)
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
            .border_style(Style::default().fg(BLUE))
            .title(" Level Breakdown ")
            .title_bottom(Line::from(Span::styled(
                " seen = reviewed once  learned = scheduled 21+ days  ↑↓ scroll ",
                Style::default().fg(GRAY),
            ))));
    let mut level_state = ListState::default();
    if !stats.level_stats.is_empty() {
        let clamped = app.stats_scroll.min(stats.level_stats.len() - 1);
        level_state.select(Some(clamped));
    }
    f.render_stateful_widget(level_list, left_chunks[1], &mut level_state);

    // Right column: weakest words + EF legend + daily activity
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(8), Constraint::Length(8)])
        .split(chunks[1]);

    let weak_items: Vec<ListItem> = stats.weakest.iter().map(|w| {
        let ef_color = if w.ease_factor < 1.8 { RED }
            else if w.ease_factor < 2.2 { YELLOW }
            else { GREEN };
        ListItem::new(Line::from(vec![
            Span::styled(format!("  {} ", w.hanzi), Style::default().fg(WHITE).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{} ", w.pinyin), Style::default().fg(CYAN)),
            Span::styled(format!("[EF:{:.1}]", w.ease_factor), Style::default().fg(ef_color)),
        ]))
    }).collect();

    let weak_list = List::new(weak_items)
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
            .border_style(Style::default().fg(RED))
            .title(" 🔴 Weakest Words "));
    f.render_widget(weak_list, right_chunks[0]);

    // EF legend (stacked lines)
    let legend_lines = vec![
        Line::from(Span::styled("  EF = Ease Factor", Style::default().fg(WHITE).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled("  Multiplier controlling how fast intervals grow.", Style::default().fg(GRAY))),
        Line::from(Span::styled("  Starts at 2.5, floor 1.3, no ceiling.", Style::default().fg(GRAY))),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ■ ", Style::default().fg(GREEN)),
            Span::styled("easy       ", Style::default().fg(GREEN)),
            Span::styled("EF ≥ 2.2", Style::default().fg(GRAY)),
        ]),
        Line::from(vec![
            Span::styled("  ■ ", Style::default().fg(YELLOW)),
            Span::styled("struggling ", Style::default().fg(YELLOW)),
            Span::styled("EF < 2.2", Style::default().fg(GRAY)),
        ]),
        Line::from(vec![
            Span::styled("  ■ ", Style::default().fg(RED)),
            Span::styled("hard       ", Style::default().fg(RED)),
            Span::styled("EF < 1.8", Style::default().fg(GRAY)),
        ]),
    ];
    let legend_p = Paragraph::new(legend_lines)
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
            .border_style(Style::default().fg(GRAY))
            .title(" Legend "));
    f.render_widget(legend_p, right_chunks[1]);

    // Daily activity (last 7 days)
    let max_daily = stats.daily_reviews.iter().map(|(_, c)| *c).max().unwrap_or(1).max(1);
    let daily_lines: Vec<Line> = stats.daily_reviews.iter().map(|(day, cnt)| {
        let bar_len = (cnt * 20 / max_daily) as usize;
        let bar = "▓".repeat(bar_len);
        let short_day = &day[5..]; // MM-DD
        Line::from(vec![
            Span::styled(format!("  {} ", short_day), Style::default().fg(GRAY)),
            Span::styled(format!("{:20} {}", bar, cnt), Style::default().fg(CYAN)),
        ])
    }).collect();

    let daily_p = Paragraph::new(daily_lines)
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
            .border_style(Style::default().fg(GREEN)).title(" 7-Day Activity "));
    f.render_widget(daily_p, right_chunks[2]);
}

fn render_content_select(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(6), Constraint::Length(2)])
        .margin(2)
        .split(area);

    let selected_count = app.content_items.iter().filter(|i| i.selected).count();
    let (count_text, count_color) = if selected_count == 0 {
        ("  —  nothing selected — press Space or 'a'".to_string(), WARN)
    } else {
        (format!("  —  {} item(s) selected", selected_count), GRAY)
    };
    let title = Paragraph::new(Line::from(vec![
        Span::styled("Select Content", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled(count_text, Style::default().fg(count_color)),
    ])).alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);

    let level_colors = [CYAN, GREEN, YELLOW, ORANGE, RED, ACCENT];

    let items: Vec<ListItem> = app.content_items.iter().enumerate().map(|(i, item)| {
        let is_cursor = i == app.content_cursor;
        let check = if item.selected { "[✓]" } else { "[ ]" };
        let (label, color) = match &item.kind {
            ContentKind::Hsk(l) => {
                let (words, diff) = match l {
                    1 => ("~150 words", "Beginner"),
                    2 => ("~300 words", "Elementary"),
                    3 => ("~600 words", "Intermediate"),
                    4 => ("~1200 words", "Upper-Int."),
                    5 => ("~2500 words", "Advanced"),
                    _ => ("~5000 words", "Mastery"),
                };
                let c = level_colors[(*l as usize).saturating_sub(1).min(5)];
                (format!("HSK {}  {:13} {}", l, words, diff), c)
            }
            ContentKind::Deck(d) => (format!("📂  {}", d), WHITE),
        };
        let style = if is_cursor {
            Style::default().fg(color).bg(BG2).add_modifier(Modifier::BOLD)
        } else if item.selected {
            Style::default().fg(color)
        } else {
            Style::default().fg(GRAY)
        };
        let prefix = if is_cursor { "▶ " } else { "  " };
        ListItem::new(Line::from(Span::styled(format!("{prefix}{check}  {label}"), style)))
    }).collect();

    let mut list_state = ListState::default();
    list_state.select(Some(app.content_cursor));
    let list = List::new(items)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(ACCENT))
            .title(" Content "));
    f.render_stateful_widget(list, chunks[1], &mut list_state);

    let footer = Paragraph::new("↑↓/jk Navigate  •  Space Toggle  •  a Select all  •  n Deselect all  •  Enter Start  •  Esc Back")
        .style(Style::default().fg(GRAY))
        .alignment(Alignment::Center);
    f.render_widget(footer, chunks[2]);
}

fn render_about(f: &mut Frame, _app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .margin(2)
        .split(area);

    // ── Left: Rules + Author ─────────────────────────────────────────────────
    let left_lines = vec![
        Line::from(Span::styled(" ZHLI — Command Line Chinese Trainer", Style::default().fg(GOLD).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(Span::styled(" Author", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled("  Radley E. Sidwell-Lewis", Style::default().fg(WHITE))),
        Line::from(Span::styled("  https://github.com/radleylewis", Style::default().fg(GRAY))),
        Line::from(Span::styled("  License: GPL-3.0", Style::default().fg(GRAY))),
        Line::from(""),
        Line::from(Span::styled(" How to Study", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled("  1. Select a study mode (direction of recall).", Style::default().fg(WHITE))),
        Line::from(Span::styled("  2. Toggle HSK levels and/or custom decks.", Style::default().fg(WHITE))),
        Line::from(Span::styled("  3. Press Enter to start the session.", Style::default().fg(WHITE))),
        Line::from(Span::styled("  4. Type your answer and press Enter.", Style::default().fg(WHITE))),
        Line::from(Span::styled("  5. Grade your recall honestly (0–5).", Style::default().fg(WHITE))),
        Line::from(""),
        Line::from(Span::styled(" Study Modes", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))),
        Line::from(vec![
            Span::styled("  Chinese → Pinyin  ", Style::default().fg(CYAN)),
            Span::styled("See hanzi, write pinyin", Style::default().fg(GRAY)),
        ]),
        Line::from(vec![
            Span::styled("  Chinese → English ", Style::default().fg(CYAN)),
            Span::styled("See hanzi, write meaning", Style::default().fg(GRAY)),
        ]),
        Line::from(vec![
            Span::styled("  English → Chinese ", Style::default().fg(CYAN)),
            Span::styled("See English, write hanzi", Style::default().fg(GRAY)),
        ]),
        Line::from(vec![
            Span::styled("  Pinyin  → Chinese ", Style::default().fg(CYAN)),
            Span::styled("See pinyin, write hanzi", Style::default().fg(GRAY)),
        ]),
        Line::from(""),
        Line::from(Span::styled(" Grading Scale", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))),
        Line::from(vec![Span::styled("  0 Blackout  ", Style::default().fg(RED)),   Span::styled("Complete blank", Style::default().fg(GRAY))]),
        Line::from(vec![Span::styled("  1 Wrong     ", Style::default().fg(RED)),   Span::styled("Incorrect, recalled on seeing answer", Style::default().fg(GRAY))]),
        Line::from(vec![Span::styled("  2 Hard      ", Style::default().fg(YELLOW)),Span::styled("Incorrect but felt close", Style::default().fg(GRAY))]),
        Line::from(vec![Span::styled("  3 Okay      ", Style::default().fg(YELLOW)),Span::styled("Correct with real effort", Style::default().fg(GRAY))]),
        Line::from(vec![Span::styled("  4 Good      ", Style::default().fg(GREEN)), Span::styled("Correct after brief hesitation", Style::default().fg(GRAY))]),
        Line::from(vec![Span::styled("  5 Perfect   ", Style::default().fg(GREEN)), Span::styled("Instant, effortless recall", Style::default().fg(GRAY))]),
    ];

    let left = Paragraph::new(left_lines)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(ACCENT))
            .title(" About "))
        .wrap(Wrap { trim: false });
    f.render_widget(left, chunks[0]);

    // ── Right: SM-2 Algorithm ────────────────────────────────────────────────
    let algo_lines = vec![
        Line::from(Span::styled(" SM-2 Spaced Repetition Algorithm", Style::default().fg(GOLD).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(Span::styled(" Overview", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled("  Each card is scheduled based on how well you", Style::default().fg(WHITE))),
        Line::from(Span::styled("  recalled it. Easier cards are shown less often;", Style::default().fg(WHITE))),
        Line::from(Span::styled("  harder cards come back sooner.", Style::default().fg(WHITE))),
        Line::from(""),
        Line::from(Span::styled(" Key Variables", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))),
        Line::from(vec![Span::styled("  EF  ", Style::default().fg(CYAN).add_modifier(Modifier::BOLD)), Span::styled("Ease Factor — interval multiplier (floor 1.3, default 2.5)", Style::default().fg(WHITE))]),
        Line::from(vec![Span::styled("  I   ", Style::default().fg(CYAN).add_modifier(Modifier::BOLD)), Span::styled("Interval — days until next review", Style::default().fg(WHITE))]),
        Line::from(vec![Span::styled("  n   ", Style::default().fg(CYAN).add_modifier(Modifier::BOLD)), Span::styled("Repetitions — consecutive correct answers", Style::default().fg(WHITE))]),
        Line::from(""),
        Line::from(Span::styled(" Scheduling (grade ≥ 3 = correct)", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled("  n=0  →  I = 1 day", Style::default().fg(WHITE))),
        Line::from(Span::styled("  n=1  →  I = 6 days", Style::default().fg(WHITE))),
        Line::from(Span::styled("  n>1  →  I = I × EF  (min 1 day)", Style::default().fg(WHITE))),
        Line::from(""),
        Line::from(Span::styled(" On failure (grade < 3)", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled("  Repetitions reset to 0.", Style::default().fg(WHITE))),
        Line::from(vec![Span::styled("  2 Hard   ", Style::default().fg(YELLOW)), Span::styled("→  12 h  (0.5 days)", Style::default().fg(GRAY))]),
        Line::from(vec![Span::styled("  1 Wrong  ", Style::default().fg(RED)),    Span::styled("→  ~2.5 h  (0.1 days)", Style::default().fg(GRAY))]),
        Line::from(vec![Span::styled("  0 Blackout ", Style::default().fg(RED)),  Span::styled("→  ~1 h  (0.04 days)", Style::default().fg(GRAY))]),
        Line::from(""),
        Line::from(Span::styled(" Ease Factor Update (every review)", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled("  EF += 0.1 - (5-q)(0.08 + (5-q)×0.02)", Style::default().fg(WHITE))),
        Line::from(Span::styled("  where q is the grade (0–5).", Style::default().fg(GRAY))),
        Line::from(Span::styled("  EF never drops below 1.3.", Style::default().fg(GRAY))),
        Line::from(""),
        Line::from(Span::styled(" Deck Practice Mode", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled("  Deck words bypass the SRS due-date filter —", Style::default().fg(WHITE))),
        Line::from(Span::styled("  all cards in the deck are always available.", Style::default().fg(WHITE))),
        Line::from(Span::styled("  A word can belong to multiple decks.", Style::default().fg(WHITE))),
        Line::from(Span::styled("  HSK words added to a deck stay in their HSK level.", Style::default().fg(GRAY))),
        Line::from(Span::styled("  SRS scheduling still applies when grading.", Style::default().fg(GRAY))),
        Line::from(""),
        Line::from(Span::styled("  Esc / Enter / Q  Return to menu", Style::default().fg(GRAY))),
    ];

    let right = Paragraph::new(algo_lines)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(BLUE))
            .title(" Algorithm "))
        .wrap(Wrap { trim: false });
    f.render_widget(right, chunks[1]);
}

fn render_confirm(f: &mut Frame, action: &ClearAction, area: Rect) {
    let popup = centered_rect(55, 30, area);
    f.render_widget(Clear, popup);

    let (title, warning, detail) = match action {
        ClearAction::CustomWords => (
            " 🗑️  Clear Custom Words ".to_string(),
            "This will permanently delete all custom words and decks.".to_string(),
            "HSK words and their progress are not affected.".to_string(),
        ),
        ClearAction::Progress => (
            " ⚠️  Reset All Progress ".to_string(),
            "This will delete every review and reset all cards.".to_string(),
            "Ease factors and intervals return to defaults.".to_string(),
        ),
        ClearAction::Deck(name) => (
            " 🗑️  Delete Deck ".to_string(),
            format!("Delete deck \"{}\"?", name),
            "Words in this deck will lose their deck assignment.".to_string(),
        ),
        ClearAction::Word(_, hanzi) => (
            " 🗑️  Delete Word ".to_string(),
            format!("Delete \"{}\" and all its review history?", hanzi),
            "This cannot be undone.".to_string(),
        ),
    };

    let border_color = match action {
        ClearAction::CustomWords => WARN,
        ClearAction::Progress    => RED,
        ClearAction::Deck(_)     => WARN,
        ClearAction::Word(..)    => RED,
    };

    let block = Block::default()
        .title(title)
        .title_style(Style::default().fg(border_color).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(BG));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(warning, Style::default().fg(WHITE).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled(detail,  Style::default().fg(GRAY))),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Press ", Style::default().fg(GRAY)),
            Span::styled("Y", Style::default().fg(RED).add_modifier(Modifier::BOLD)),
            Span::styled(" to confirm  or  any other key to cancel", Style::default().fg(GRAY)),
        ]),
    ];
    f.render_widget(Paragraph::new(lines).alignment(Alignment::Center), inner);
}

fn render_add_to_deck(f: &mut Frame, app: &App, area: Rect) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(8), Constraint::Length(3), Constraint::Length(2)])
        .margin(2)
        .split(area);

    let title = Paragraph::new(Line::from(vec![
        Span::styled("📂 Decks", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled("  —  select a deck to add words, or create a new one", Style::default().fg(GRAY)),
    ])).alignment(Alignment::Center);
    f.render_widget(title, outer[0]);

    // Split main area: left = deck list, right = words preview
    let panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(outer[1]);

    // Left: deck list
    let mut items: Vec<ListItem> = Vec::new();
    for i in 0..=app.available_decks.len() {
        let selected = i == app.deck_cursor && !app.creating_new_deck;
        let (label, color) = if i == 0 {
            ("  ＋  New Deck".to_string(), GREEN)
        } else {
            (format!("  📂  {}", app.available_decks[i - 1]), WHITE)
        };
        let style = if selected {
            Style::default().fg(color).bg(BG2).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(GRAY)
        };
        let prefix = if selected { "▶" } else { " " };
        items.push(ListItem::new(Line::from(Span::styled(format!("{prefix}{label}"), style))));
    }
    let list = List::new(items)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(if app.creating_new_deck { GRAY } else { ACCENT }))
            .title(" Decks "));
    f.render_widget(list, panels[0]);

    // Right: words in hovered deck
    let preview_title = if app.deck_cursor > 0 && !app.creating_new_deck {
        format!(" {} words ", app.deck_preview.len())
    } else {
        " Words ".to_string()
    };
    let preview_items: Vec<ListItem> = app.deck_preview.iter().map(|w| {
        ListItem::new(Line::from(vec![
            Span::styled(format!("  {} ", w.hanzi), Style::default().fg(WHITE).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{} ", w.pinyin), Style::default().fg(CYAN)),
            Span::styled(if w.level == 0 { "Other ".to_string() } else { format!("HSK{} ", w.level) }, Style::default().fg(GOLD)),
            Span::styled(w.english.as_str(), Style::default().fg(GRAY)),
        ]))
    }).collect();
    let preview_list = List::new(preview_items)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(GRAY))
            .title(preview_title));
    f.render_widget(preview_list, panels[1]);

    // New deck name input
    let input_border = if app.creating_new_deck { ACCENT } else { GRAY };
    let input_title  = if app.creating_new_deck { " New Deck Name " } else { " New Deck Name (navigate to ＋) " };
    let input_text   = if app.creating_new_deck { format!("{}█", app.deck_name_input) } else { String::new() };
    let input_p = Paragraph::new(input_text)
        .block(Block::default().title(input_title).title_style(Style::default().fg(input_border))
            .borders(Borders::ALL).border_type(BorderType::Rounded)
            .border_style(Style::default().fg(input_border)))
        .style(Style::default().fg(WHITE));
    f.render_widget(input_p, outer[2]);

    let footer = if app.creating_new_deck {
        "Type deck name  •  Enter Create & go to search  •  Esc Cancel"
    } else {
        "↑↓/jk Navigate  •  Enter Select  •  d/Del Delete  •  Esc Back"
    };
    f.render_widget(
        Paragraph::new(footer).style(Style::default().fg(GRAY)).alignment(Alignment::Center),
        outer[3],
    );
}

fn render_search_deck(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(10), Constraint::Length(2)])
        .margin(2)
        .split(area);

    // Search bar
    let search_block = Block::default()
        .title(" 🔍 Search Words (hanzi, pinyin, english) ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(CYAN));
    let search_p = Paragraph::new(format!("{}█", app.search_query))
        .block(search_block)
        .style(Style::default().fg(WHITE));
    f.render_widget(search_p, chunks[0]);

    // Results (+ one extra item: "＋ Add custom word")
    let add_item_idx = app.search_results.len();
    let mut items: Vec<ListItem> = app.search_results.iter().enumerate().map(|(i, w)| {
        let selected = i == app.search_cursor;
        let style = if selected {
            Style::default().fg(CYAN).bg(BG2).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(WHITE)
        };
        let target_deck = if app.deck_name_input.is_empty() { "My Deck" } else { &app.deck_name_input };
        let in_target = w.decks.iter().any(|d| d == target_deck);
        let deck_tag = if in_target {
            " ✓".to_string()
        } else if w.decks.is_empty() {
            String::new()
        } else {
            format!(" [{}]", w.decks.join(", "))
        };
        let suspended_tag = if w.suspended { " [suspended]" } else { "" };
        let prefix = if selected { "▶ " } else { "  " };
        let tag_style = if in_target { Style::default().fg(GREEN) } else { style };
        let level_str = if w.level == 0 { "Other".to_string() } else { format!("HSK{}", w.level) };
        ListItem::new(Line::from(vec![
            Span::styled(format!("{}{}  {}  {}  {}", prefix, w.hanzi, w.pinyin, w.english, level_str), style),
            Span::styled(deck_tag, tag_style),
            Span::styled(suspended_tag, Style::default().fg(WARN)),
        ]))
    }).collect();

    // "＋ Add custom word" entry at the bottom
    let add_selected = app.search_cursor == add_item_idx;
    let add_style = if add_selected {
        Style::default().fg(GREEN).bg(BG2).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(GREEN)
    };
    let add_label = if app.search_query.is_empty() {
        "  ＋  Add custom word".to_string()
    } else {
        format!("  ＋  Add \"{}\" as custom word", app.search_query)
    };
    items.push(ListItem::new(Line::from(Span::styled(add_label, add_style))));

    let results_title = if app.search_query.is_empty() {
        " Type to search — or ↓ to add a custom word ".to_string()
    } else {
        format!(" Results ({}) ", app.search_results.len())
    };
    let list = List::new(items)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(BLUE))
            .title(results_title));
    f.render_widget(list, chunks[1]);

    let deck_name = if app.deck_name_input.is_empty() { "My Deck" } else { &app.deck_name_input };
    let remove_hint = if !app.deck_name_input.is_empty() { "  •  R Remove from deck" } else { "" };
    let footer = Paragraph::new(format!(
        "Type to search  •  ↑↓ Navigate  •  Enter Add to \"{}\"  •  E Edit  •  S Suspend  •  D Delete{}  •  Esc Back",
        deck_name, remove_hint,
    ))
    .style(Style::default().fg(GRAY))
    .alignment(Alignment::Center);
    f.render_widget(footer, chunks[2]);
}

fn render_add_custom_word(f: &mut Frame, app: &App, area: Rect) {
    let has_filter = app.dict_filter_active || !app.dict_filter.is_empty();
    let constraints: Vec<Constraint> = if has_filter {
        vec![
            Constraint::Length(3), // query input
            Constraint::Length(1), // status line
            Constraint::Length(1), // filter bar
            Constraint::Min(5),    // results list
            Constraint::Length(1), // footer
        ]
    } else {
        vec![
            Constraint::Length(3), // query input
            Constraint::Length(1), // status line
            Constraint::Min(5),    // results list
            Constraint::Length(1), // footer
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .margin(2)
        .split(area);

    // Chunk indices shift when the filter bar is present
    let (idx_status, idx_filter, idx_list, idx_footer) = if has_filter {
        (1, 2, 3, 4)
    } else {
        (1, usize::MAX, 2, 3)
    };

    // Query input
    let is_searching = app.dict_status == "Searching...";
    let query_border = if is_searching { YELLOW } else { GOLD };
    let query_p = Paragraph::new(format!("{}█", app.dict_query))
        .block(Block::default()
            .title(" ＋ Add Custom Word — hanzi, pinyin, or English ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(query_border)))
        .style(Style::default().fg(WHITE));
    f.render_widget(query_p, chunks[0]);

    // Status line
    let status_color = if app.dict_status.starts_with("Error") { RED }
        else if is_searching { YELLOW }
        else { GRAY };
    let status_p = Paragraph::new(app.dict_status.as_str())
        .style(Style::default().fg(status_color))
        .alignment(Alignment::Center);
    f.render_widget(status_p, chunks[idx_status]);

    // Filter bar (only when active)
    if has_filter {
        let filter_text = format!("/{}{}", app.dict_filter, if app.dict_filter_active { "█" } else { "" });
        let filter_p = Paragraph::new(filter_text)
            .style(Style::default().fg(if app.dict_filter_active { CYAN } else { GRAY }));
        f.render_widget(filter_p, chunks[idx_filter]);
    }

    // Results list — show only entries matching the active filter
    let filtered: Vec<&crate::dict::DictEntry> =
        crate::dict::filter_results(&app.dict_results, &app.dict_filter);

    let items: Vec<ListItem> = filtered.iter().enumerate().map(|(i, e)| {
        let selected = i == app.dict_cursor;
        let bg = if selected { BG2 } else { BG };
        let prefix = if selected { "▶ " } else { "  " };
        let line1 = Line::from(vec![
            Span::styled(format!("{}{}", prefix, e.hanzi),
                Style::default().fg(if selected { CYAN } else { GOLD })
                    .bg(bg).add_modifier(if selected { Modifier::BOLD } else { Modifier::empty() })),
            Span::styled(format!("  {}", e.pinyin),
                Style::default().fg(if selected { WHITE } else { GRAY }).bg(bg)),
        ]);
        let line2 = Line::from(Span::styled(
            format!("    {}", e.english),
            Style::default().fg(if selected { Color::Rgb(200,200,200) } else { GRAY }).bg(bg),
        ));
        ListItem::new(vec![line1, line2])
    }).collect();

    let results_title = if filtered.is_empty() && app.dict_results.is_empty() {
        " Results ".to_string()
    } else if has_filter {
        format!(" Results  {}/{}  (filtered from {}) ", app.dict_cursor + 1, filtered.len(), app.dict_results.len())
    } else {
        format!(" Results  {}/{}  ↑↓ to scroll ", app.dict_cursor + 1, app.dict_results.len())
    };
    let list_border = if has_filter { CYAN } else { BLUE };
    let list = List::new(items)
        .block(Block::default()
            .title(results_title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(list_border)))
        .highlight_style(Style::default().bg(BG2));

    let mut list_state = ListState::default();
    if !filtered.is_empty() {
        list_state.select(Some(app.dict_cursor));
    }
    f.render_stateful_widget(list, chunks[idx_list], &mut list_state);

    // Footer
    let footer_text = if app.dict_filter_active {
        "type to filter  •  ↑↓  Scroll  •  Enter  Add selected  •  Esc  Clear filter"
    } else if app.dict_results.is_empty() {
        "Enter  Search online  •  Esc  Back"
    } else {
        "↑↓  Scroll  •  /  Filter results  •  Enter  Add  •  edit query + Enter  Re-search  •  Esc  Back"
    };
    let footer = Paragraph::new(footer_text)
        .style(Style::default().fg(GRAY))
        .alignment(Alignment::Center);
    f.render_widget(footer, chunks[idx_footer]);
}

fn render_edit_word(f: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(70, 55, area);
    f.render_widget(Clear, popup);

    let block = Block::default()
        .title(" ✏  Edit Custom Word ")
        .title_style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT))
        .style(Style::default().bg(BG));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .margin(1)
        .split(inner);

    let labels = ["Hanzi", "Pinyin", "English"];
    for (i, (label, chunk)) in labels.iter().zip(chunks.iter()).enumerate() {
        let focused = i == app.edit_field;
        let border_color = if focused { CYAN } else { GRAY };
        let text = format!("{}{}", app.edit_bufs[i], if focused { "█" } else { "" });
        let p = Paragraph::new(text)
            .block(Block::default()
                .title(format!(" {} ", label))
                .title_style(Style::default().fg(border_color))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color)))
            .style(Style::default().fg(WHITE));
        f.render_widget(p, *chunk);
    }

    let footer = Paragraph::new("Tab/↑↓ Switch field  •  Enter Save  •  Esc Cancel")
        .style(Style::default().fg(GRAY))
        .alignment(Alignment::Center);
    f.render_widget(footer, chunks[3]);
}
