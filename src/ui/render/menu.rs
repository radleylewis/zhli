use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use super::{
    centered_rect, ACCENT, BG, BG2, BLUE, CYAN, GOLD, GRAY, GREEN, ORANGE, RED, WARN, WHITE, YELLOW,
};
use crate::srs::CardDirection;
use crate::ui::app_state::{App, ClearAction, ContentKind};

pub(super) fn render_main_menu(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),
            Constraint::Min(10),
            Constraint::Length(2),
        ])
        .margin(2)
        .split(area);

    let logo = vec![
        Line::from(Span::styled(
            "        ██        ",
            Style::default().fg(GOLD),
        )),
        Line::from(Span::styled(
            "  ╔═════██══════╗  ",
            Style::default().fg(GOLD),
        )),
        Line::from(Span::styled(
            "  ║     ██      ║  ",
            Style::default().fg(GOLD),
        )),
        Line::from(Span::styled(
            "  ╚═════════════╝  ",
            Style::default().fg(GOLD),
        )),
        Line::from(Span::styled(
            "        ██        ",
            Style::default().fg(GOLD),
        )),
        Line::from(Span::styled(
            "Command Line Chinese Trainer!",
            Style::default().fg(GRAY),
        )),
    ];
    f.render_widget(Paragraph::new(logo).alignment(Alignment::Center), chunks[0]);

    let items_data = [
        ("📚", "Study / Review Cards", CYAN),
        ("📊", "Stats & Report Card", GREEN),
        ("➕", "Add Word to Deck", BLUE),
        ("🔍", "Search & Browse", WHITE),
        ("💤", "Suspended Words", YELLOW),
        ("📤", "Export Custom Words", BLUE),
        ("📥", "Import from JSON", BLUE),
        ("📖", "About / Rules / Algo", ACCENT),
        ("🗑️", "Clear Custom Words", WARN),
        ("⚠️", "Reset All Progress", RED),
        ("❌", "Quit", GRAY),
    ];

    let items: Vec<ListItem> = items_data
        .iter()
        .enumerate()
        .map(|(i, (icon, label, color))| {
            let style = if i == app.menu_cursor {
                Style::default()
                    .fg(*color)
                    .bg(BG2)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(GRAY)
            };
            let prefix = if i == app.menu_cursor { "▶ " } else { "  " };
            ListItem::new(Line::from(vec![Span::styled(
                format!("{prefix}{icon} {label}"),
                style,
            )]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(ACCENT))
            .title(" Menu ")
            .title_style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
    );
    f.render_widget(list, chunks[1]);

    f.render_widget(
        Paragraph::new("↑↓ Navigate  •  Enter Select  •  Q Quit")
            .style(Style::default().fg(GRAY))
            .alignment(Alignment::Center),
        chunks[2],
    );
}

pub(super) fn render_mode_select(f: &mut Frame, app: &App, area: Rect) {
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
        x: popup.x + 2,
        y: popup.y + 2,
        width: popup.width.saturating_sub(4),
        height: popup.height.saturating_sub(4),
    };

    let directions = [
        (
            CardDirection::ZhToPinyin,
            "Chinese → Pinyin",
            "See hanzi, write pinyin",
        ),
        (
            CardDirection::ZhToEn,
            "Chinese → English",
            "See hanzi, write meaning",
        ),
        (
            CardDirection::EnToZh,
            "English → Chinese",
            "See English, write hanzi",
        ),
        (
            CardDirection::PinyinToZh,
            "Pinyin  → Chinese",
            "See pinyin, write hanzi",
        ),
    ];

    let sel_count = app.selected_directions.len();
    let header = if sel_count == 0 {
        "  Select one or more modes (Space to toggle):".to_string()
    } else {
        format!("  {sel_count} mode(s) selected — Enter to continue:")
    };

    let mut lines = vec![
        Line::from(Span::styled(header, Style::default().fg(GRAY))),
        Line::from(""),
    ];

    for (i, (dir, label, hint)) in directions.iter().enumerate() {
        let is_cursor = i == app.mode_cursor;
        let is_selected = app.selected_directions.iter().any(|d| d == dir);
        let check = if is_selected { "[✓]" } else { "[ ]" };
        let style = if is_cursor {
            Style::default()
                .fg(CYAN)
                .bg(BG2)
                .add_modifier(Modifier::BOLD)
        } else if is_selected {
            Style::default().fg(CYAN)
        } else {
            Style::default().fg(GRAY)
        };
        let prefix = if is_cursor { "▶ " } else { "  " };
        lines.push(Line::from(Span::styled(
            format!("{prefix}{check} {label}  — {hint}"),
            style,
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  ↑↓/jk Navigate  •  Space Toggle  •  Enter Confirm",
        Style::default().fg(GRAY),
    )));

    let p = Paragraph::new(lines);
    f.render_widget(p, inner);
}

pub(super) fn render_content_select(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Length(2),
        ])
        .margin(2)
        .split(area);

    let selected_count = app.content_items.iter().filter(|i| i.selected).count();
    let (count_text, count_color) = if selected_count == 0 {
        (
            "  —  nothing selected — press Space or 'a'".to_string(),
            WARN,
        )
    } else {
        (format!("  —  {} item(s) selected", selected_count), GRAY)
    };
    let cram_span = if app.cram_mode {
        Span::styled(
            "  [CRAM]",
            Style::default().fg(ORANGE).add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled("", Style::default())
    };
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            "Select Content",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::styled(count_text, Style::default().fg(count_color)),
        cram_span,
    ]))
    .alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);

    let level_colors = [CYAN, GREEN, YELLOW, ORANGE, RED, ACCENT];

    let items: Vec<ListItem> = app
        .content_items
        .iter()
        .enumerate()
        .map(|(i, item)| {
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
                Style::default()
                    .fg(color)
                    .bg(BG2)
                    .add_modifier(Modifier::BOLD)
            } else if item.selected {
                Style::default().fg(color)
            } else {
                Style::default().fg(GRAY)
            };
            let prefix = if is_cursor { "▶ " } else { "  " };
            ListItem::new(Line::from(Span::styled(
                format!("{prefix}{check}  {label}"),
                style,
            )))
        })
        .collect();

    let mut list_state = ratatui::widgets::ListState::default();
    list_state.select(Some(app.content_cursor));
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(ACCENT))
            .title(" Content "),
    );
    f.render_stateful_widget(list, chunks[1], &mut list_state);

    let footer = Paragraph::new("↑↓/jk Navigate  •  Space Toggle  •  a All  •  n None  •  c Cram  •  r Refresh  •  Enter Start  •  Esc Back")
        .style(Style::default().fg(GRAY))
        .alignment(Alignment::Center);
    f.render_widget(footer, chunks[2]);
}

pub(super) fn render_about(f: &mut Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .margin(2)
        .split(area);

    let left_lines = vec![
        Line::from(Span::styled(
            " ZHLI — Command Line Chinese Trainer",
            Style::default().fg(GOLD).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            " Author",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "  Radley E. Sidwell-Lewis",
            Style::default().fg(WHITE),
        )),
        Line::from(Span::styled(
            "  https://github.com/radleylewis",
            Style::default().fg(GRAY),
        )),
        Line::from(Span::styled(
            "  License: GPL-3.0",
            Style::default().fg(GRAY),
        )),
        Line::from(""),
        Line::from(Span::styled(
            " How to Study",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "  1. Select a study mode (direction of recall).",
            Style::default().fg(WHITE),
        )),
        Line::from(Span::styled(
            "  2. Toggle HSK levels and/or custom decks.",
            Style::default().fg(WHITE),
        )),
        Line::from(Span::styled(
            "  3. Press Enter to start the session.",
            Style::default().fg(WHITE),
        )),
        Line::from(Span::styled(
            "  4. Type your answer and press Enter.",
            Style::default().fg(WHITE),
        )),
        Line::from(Span::styled(
            "  5. Grade your recall honestly (0–5).",
            Style::default().fg(WHITE),
        )),
        Line::from(""),
        Line::from(Span::styled(
            " Study Modes",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
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
        Line::from(Span::styled(
            " Grading Scale",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("  0 Blackout  ", Style::default().fg(RED)),
            Span::styled("Complete blank", Style::default().fg(GRAY)),
        ]),
        Line::from(vec![
            Span::styled("  1 Wrong     ", Style::default().fg(RED)),
            Span::styled(
                "Incorrect, recalled on seeing answer",
                Style::default().fg(GRAY),
            ),
        ]),
        Line::from(vec![
            Span::styled("  2 Hard      ", Style::default().fg(YELLOW)),
            Span::styled("Incorrect but felt close", Style::default().fg(GRAY)),
        ]),
        Line::from(vec![
            Span::styled("  3 Okay      ", Style::default().fg(YELLOW)),
            Span::styled("Correct with real effort", Style::default().fg(GRAY)),
        ]),
        Line::from(vec![
            Span::styled("  4 Good      ", Style::default().fg(GREEN)),
            Span::styled("Correct after brief hesitation", Style::default().fg(GRAY)),
        ]),
        Line::from(vec![
            Span::styled("  5 Perfect   ", Style::default().fg(GREEN)),
            Span::styled("Instant, effortless recall", Style::default().fg(GRAY)),
        ]),
    ];

    let left = Paragraph::new(left_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(ACCENT))
                .title(" About "),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(left, chunks[0]);

    let algo_lines = vec![
        Line::from(Span::styled(
            " SM-2 Spaced Repetition Algorithm",
            Style::default().fg(GOLD).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            " Overview",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "  Each card is scheduled based on how well you",
            Style::default().fg(WHITE),
        )),
        Line::from(Span::styled(
            "  recalled it. Easier cards are shown less often;",
            Style::default().fg(WHITE),
        )),
        Line::from(Span::styled(
            "  harder cards come back sooner.",
            Style::default().fg(WHITE),
        )),
        Line::from(""),
        Line::from(Span::styled(
            " Key Variables",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled(
                "  EF  ",
                Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Ease Factor — interval multiplier (floor 1.3, default 2.5)",
                Style::default().fg(WHITE),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  I   ",
                Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Interval — days until next review",
                Style::default().fg(WHITE),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  n   ",
                Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Repetitions — consecutive correct answers",
                Style::default().fg(WHITE),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            " Scheduling (grade ≥ 3 = correct)",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "  n=0  →  I = 1 day",
            Style::default().fg(WHITE),
        )),
        Line::from(Span::styled(
            "  n=1  →  I = 6 days",
            Style::default().fg(WHITE),
        )),
        Line::from(Span::styled(
            "  n>1  →  I = I × EF  (min 1 day)",
            Style::default().fg(WHITE),
        )),
        Line::from(""),
        Line::from(Span::styled(
            " On failure (grade < 3)",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "  Repetitions reset to 0.",
            Style::default().fg(WHITE),
        )),
        Line::from(vec![
            Span::styled("  2 Hard   ", Style::default().fg(YELLOW)),
            Span::styled("→  12 h  (0.5 days)", Style::default().fg(GRAY)),
        ]),
        Line::from(vec![
            Span::styled("  1 Wrong  ", Style::default().fg(RED)),
            Span::styled("→  ~2.5 h  (0.1 days)", Style::default().fg(GRAY)),
        ]),
        Line::from(vec![
            Span::styled("  0 Blackout ", Style::default().fg(RED)),
            Span::styled("→  ~1 h  (0.04 days)", Style::default().fg(GRAY)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            " Ease Factor Update (every review)",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "  EF += 0.1 - (5-q)(0.08 + (5-q)×0.02)",
            Style::default().fg(WHITE),
        )),
        Line::from(Span::styled(
            "  where q is the grade (0–5).",
            Style::default().fg(GRAY),
        )),
        Line::from(Span::styled(
            "  EF never drops below 1.3.",
            Style::default().fg(GRAY),
        )),
        Line::from(""),
        Line::from(Span::styled(
            " Deck Practice Mode",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "  Deck words bypass the SRS due-date filter —",
            Style::default().fg(WHITE),
        )),
        Line::from(Span::styled(
            "  all cards in the deck are always available.",
            Style::default().fg(WHITE),
        )),
        Line::from(Span::styled(
            "  A word can belong to multiple decks.",
            Style::default().fg(WHITE),
        )),
        Line::from(Span::styled(
            "  HSK words added to a deck stay in their HSK level.",
            Style::default().fg(GRAY),
        )),
        Line::from(Span::styled(
            "  SRS scheduling still applies when grading.",
            Style::default().fg(GRAY),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Esc / Enter / Q  Return to menu",
            Style::default().fg(GRAY),
        )),
    ];

    let right = Paragraph::new(algo_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(BLUE))
                .title(" Algorithm "),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(right, chunks[1]);
}

pub(super) fn render_suspended_words(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Length(2),
        ])
        .margin(2)
        .split(area);

    let suspended_count = app.suspended_words.len();
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            "💤 Suspended Words",
            Style::default().fg(YELLOW).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  —  {suspended_count} word(s)"),
            Style::default().fg(GRAY),
        ),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);

    if app.suspended_words.is_empty() {
        let msg = Paragraph::new("No suspended words.")
            .style(Style::default().fg(GRAY))
            .alignment(Alignment::Center);
        f.render_widget(msg, chunks[1]);
    } else {
        let items: Vec<ListItem> = app
            .suspended_words
            .iter()
            .enumerate()
            .map(|(i, w)| {
                let selected = i == app.suspended_cursor;
                let style = if selected {
                    Style::default()
                        .fg(CYAN)
                        .bg(BG2)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(WHITE)
                };
                let prefix = if selected { "▶ " } else { "  " };
                let level_str = if w.level == 0 {
                    "Other".to_string()
                } else {
                    format!("HSK{}", w.level)
                };
                let deck_str = if w.decks.is_empty() {
                    String::new()
                } else {
                    format!("  [{}]", w.decks.join(", "))
                };
                ListItem::new(Line::from(vec![Span::styled(
                    format!(
                        "{prefix}{}  {}  {}  {}{}",
                        w.hanzi, w.pinyin, w.english, level_str, deck_str
                    ),
                    style,
                )]))
            })
            .collect();

        let mut list_state = ratatui::widgets::ListState::default();
        list_state.select(Some(app.suspended_cursor));
        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(YELLOW)),
        );
        f.render_stateful_widget(list, chunks[1], &mut list_state);
    }

    let footer = Paragraph::new("↑↓/jk Navigate  •  Enter/u Unsuspend  •  Esc Back")
        .style(Style::default().fg(GRAY))
        .alignment(Alignment::Center);
    f.render_widget(footer, chunks[2]);
}

pub(super) fn render_import_file(f: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(65, 55, area);
    f.render_widget(Clear, popup);

    let block = Block::default()
        .title(" 📥  Import from JSON ")
        .title_style(Style::default().fg(BLUE).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(BLUE))
        .style(Style::default().bg(BG));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let inner_layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(2),
            Constraint::Length(1),
        ])
        .split(inner);

    let desc = Paragraph::new(vec![
        Line::from(Span::styled(
            "Import custom words from a zhli JSON export.",
            Style::default().fg(WHITE),
        )),
        Line::from(Span::styled(
            "Existing words and HSK entries are skipped.",
            Style::default().fg(GRAY),
        )),
    ]);
    f.render_widget(desc, inner_layout[0]);

    f.render_widget(
        Paragraph::new("File path:").style(Style::default().fg(GRAY)),
        inner_layout[1],
    );

    let path_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(CYAN));
    let path_inner = path_block.inner(inner_layout[2]);
    f.render_widget(path_block, inner_layout[2]);
    f.render_widget(
        Paragraph::new(app.import_path.as_str()).style(Style::default().fg(WHITE)),
        path_inner,
    );

    let hint = Paragraph::new(vec![Line::from(Span::styled(
        "Format: [{\"hanzi\":\"…\",\"pinyin\":\"…\",\"english\":\"…\",\"decks\":[…]}, …]",
        Style::default().fg(GRAY),
    ))]);
    f.render_widget(hint, inner_layout[3]);

    f.render_widget(
        Paragraph::new("Enter = Import   •   Esc = Cancel")
            .style(Style::default().fg(GRAY))
            .alignment(Alignment::Center),
        inner_layout[4],
    );
}

pub(super) fn render_confirm(f: &mut Frame, action: &ClearAction, area: Rect) {
    let popup = centered_rect(55, 30, area);
    f.render_widget(Clear, popup);

    let (title, warning, detail) = match action {
        ClearAction::CustomWords => (
            " 🗑️  Clear Custom Words ".to_string(),
            "Deletes all custom words, decks, and their review history.".to_string(),
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
        ClearAction::Progress => RED,
        ClearAction::Deck(_) => WARN,
        ClearAction::Word(..) => RED,
    };

    let block = Block::default()
        .title(title)
        .title_style(
            Style::default()
                .fg(border_color)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(BG));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            warning,
            Style::default().fg(WHITE).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(detail, Style::default().fg(GRAY))),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Press ", Style::default().fg(GRAY)),
            Span::styled("Y", Style::default().fg(RED).add_modifier(Modifier::BOLD)),
            Span::styled(
                " to confirm  or  any other key to cancel",
                Style::default().fg(GRAY),
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(lines).alignment(Alignment::Center), inner);
}
