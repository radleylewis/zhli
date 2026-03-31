use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, BorderType, Clear, Gauge, List, ListItem, Paragraph, Wrap},
};

use crate::ui::app_state::{App, ReviewPhase, Screen};
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

pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();
    // Background
    f.render_widget(Block::default().style(Style::default().bg(BG)), area);

    match app.screen {
        Screen::MainMenu   => render_main_menu(f, app, area),
        Screen::LevelSelect => render_level_select(f, app, area),
        Screen::ModeSelect  => render_mode_select(f, app, area),
        Screen::Review      => render_review(f, app, area),
        Screen::Stats       => render_stats(f, app, area),
        Screen::AddToDeck   => render_add_to_deck(f, app, area),
        Screen::SearchDeck  => render_search_deck(f, app, area),
    }

    // Status bar
    if !app.status_message.is_empty() {
        let bar = Rect { x: 0, y: area.height.saturating_sub(1), width: area.width, height: 1 };
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
    let gold = Color::Rgb(220, 170, 40);
    let logo = vec![
        Line::from(Span::styled("        ██        ", Style::default().fg(gold))),
        Line::from(Span::styled("  ╔═════██══════╗  ", Style::default().fg(gold))),
        Line::from(Span::styled("  ║     ██      ║  ", Style::default().fg(gold))),
        Line::from(Span::styled("  ╚═════════════╝  ", Style::default().fg(gold))),
        Line::from(Span::styled("        ██        ", Style::default().fg(gold))),
        Line::from(Span::styled("Command Line Chinese Trainer!", Style::default().fg(GRAY))),
    ];
    let logo_p = Paragraph::new(logo).alignment(Alignment::Center);
    f.render_widget(logo_p, chunks[0]);

    let items_data = [
        ("📚", "Study / Review Cards", CYAN),
        ("⚙️", "Select Level & Mode",  YELLOW),
        ("📊", "Stats & Report Card",  GREEN),
        ("➕", "Add Word to Deck",     BLUE),
        ("🔍", "Search & Browse",      WHITE),
        ("❌", "Quit",                 GRAY),
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

fn render_level_select(f: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(60, 70, area);
    f.render_widget(Clear, popup);

    let block = Block::default()
        .title(" Select HSK Level ")
        .title_style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT))
        .style(Style::default().bg(BG));
    f.render_widget(block, popup);

    let inner = Rect {
        x: popup.x + 2,
        y: popup.y + 2,
        width: popup.width.saturating_sub(4),
        height: popup.height.saturating_sub(4),
    };

    let level_info = [
        (1u8, "~150 words", "Beginner",     CYAN),
        (2,   "~300 words", "Elementary",   GREEN),
        (3,   "~600 words", "Intermediate", YELLOW),
        (4,   "~1200 words","Upper-Int.",   Color::Rgb(220,140,40)),
        (5,   "~2500 words","Advanced",     RED),
        (6,   "~5000 words","Mastery",      ACCENT),
    ];

    let mut lines = vec![
        Line::from(Span::styled("  Level includes all prior levels\n", Style::default().fg(GRAY))),
        Line::from(""),
    ];

    for (lvl, words, label, color) in &level_info {
        let selected = app.level_cursor == *lvl;
        let prefix = if selected { "▶ " } else { "  " };
        let style = if selected {
            Style::default().fg(*color).bg(BG2).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(GRAY)
        };
        lines.push(Line::from(Span::styled(
            format!("{}HSK {}  {:12}  {}", prefix, lvl, words, label),
            style,
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  ↑↓ Navigate  •  Enter Confirm  •  Esc Back",
        Style::default().fg(GRAY),
    )));

    let p = Paragraph::new(lines);
    f.render_widget(p, inner);
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

    // Progress bar
    let total = app.review_queue.len();
    let done  = app.current_card_idx;
    let pct   = if total > 0 { done * 100 / total } else { 0 };
    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::NONE))
        .gauge_style(Style::default().fg(ACCENT).bg(BG2))
        .percent(pct as u16)
        .label(format!("{}/{} cards  •  ✓{}/{}", done, total, app.session_correct, app.session_total));
    f.render_widget(gauge, chunks[0]);

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
        card.level.to_string(),
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
    let t1 = Color::Rgb(80,  210, 200);  // flat
    let t2 = Color::Rgb(80,  200, 80);   // rising
    let t3 = Color::Rgb(220, 180, 40);   // dipping
    let t4 = Color::Rgb(220, 80,  80);   // falling

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
        let color = match ls.level {
            1 => CYAN, 2 => GREEN, 3 => YELLOW,
            4 => Color::Rgb(220,140,40), 5 => RED, _ => ACCENT,
        };
        ListItem::new(Line::from(vec![
            Span::styled(format!("  HSK {} ", ls.level), Style::default().fg(color).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{:░<10} {:3}% seen", bar, seen_pct), Style::default().fg(GRAY)),
            Span::styled(format!("  {} learned", ls.mature), Style::default().fg(GREEN)),
        ]))
    }).collect();

    let level_list = List::new(level_items)
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
            .border_style(Style::default().fg(BLUE))
            .title(" Level Breakdown ")
            .title_bottom(Line::from(Span::styled(
                " seen = reviewed once  learned = scheduled 21+ days ",
                Style::default().fg(GRAY),
            ))));
    f.render_widget(level_list, left_chunks[1]);

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

fn render_add_to_deck(f: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(70, 80, area);
    f.render_widget(Clear, popup);

    let block = Block::default()
        .title(" ➕ Add Word to Custom Deck ")
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

    let lines = vec![
        Line::from(Span::styled("Search for a word then press Tab to add it to a deck", Style::default().fg(GRAY))),
        Line::from(""),
        Line::from(Span::styled("Deck name (Enter to confirm):", Style::default().fg(YELLOW))),
        Line::from(Span::styled(format!("  {}█", app.deck_name_input), Style::default().fg(WHITE))),
        Line::from(""),
        Line::from(Span::styled("Press Tab to go to search", Style::default().fg(GRAY))),
        Line::from(""),
        Line::from(Span::styled("  Esc  Back  •  Tab  Search", Style::default().fg(GRAY))),
    ];

    let p = Paragraph::new(lines);
    f.render_widget(p, inner);
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

    // Results
    let items: Vec<ListItem> = app.search_results.iter().enumerate().map(|(i, w)| {
        let selected = i == app.search_cursor;
        let style = if selected {
            Style::default().fg(CYAN).bg(BG2).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(WHITE)
        };
        let deck_tag = w.custom_deck.as_deref().map(|d| format!(" [{}]", d)).unwrap_or_default();
        let prefix = if selected { "▶ " } else { "  " };
        ListItem::new(Line::from(Span::styled(
            format!("{}{}  {}  {}  HSK{}{}", prefix, w.hanzi, w.pinyin, w.english, w.level, deck_tag),
            style,
        )))
    }).collect();

    let list = List::new(items)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(BLUE))
            .title(format!(" Results ({}) ", app.search_results.len())));
    f.render_widget(list, chunks[1]);

    let deck_name = if app.deck_name_input.is_empty() { "My Deck" } else { &app.deck_name_input };
    let footer = Paragraph::new(format!(
        "Type to search  •  ↑↓/jk Navigate  •  Tab Add to \"{}\"  •  Esc Back",
        deck_name,
    ))
    .style(Style::default().fg(GRAY))
    .alignment(Alignment::Center);
    f.render_widget(footer, chunks[2]);
}
