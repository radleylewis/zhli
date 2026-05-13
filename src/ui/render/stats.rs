use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, BorderType, List, ListItem, ListState, Paragraph},
};
use chrono::{Datelike, Utc};

use crate::ui::app_state::App;
use super::{ACCENT, BG, BLUE, CYAN, GRAY, GREEN, ORANGE, RED, WHITE, YELLOW};

pub(super) fn render_stats(f: &mut Frame, app: &App, area: Rect) {
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

    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(20), Constraint::Length(9)])
        .split(inner);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(vert[0]);

    // Left column
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(12), Constraint::Min(8)])
        .split(chunks[0]);

    let retention = if stats.total_reviews > 0 {
        (stats.avg_grade / 5.0 * 100.0).round() as u16
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

    // Right column
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

    let max_daily = stats.daily_reviews.iter().map(|(_, c)| *c).max().unwrap_or(1).max(1);
    let daily_lines: Vec<Line> = stats.daily_reviews.iter().map(|(day, cnt)| {
        let bar_len = (cnt * 20 / max_daily) as usize;
        let bar = "▓".repeat(bar_len);
        let short_day = &day[5..];
        Line::from(vec![
            Span::styled(format!("  {} ", short_day), Style::default().fg(GRAY)),
            Span::styled(format!("{:20} {}", bar, cnt), Style::default().fg(CYAN)),
        ])
    }).collect();

    let daily_p = Paragraph::new(daily_lines)
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
            .border_style(Style::default().fg(GREEN)).title(" 7-Day Activity "));
    f.render_widget(daily_p, right_chunks[2]);

    render_heatmap(f, app, vert[1]);
}

fn render_heatmap(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" Activity Heatmap (12 weeks) ")
        .title_style(Style::default().fg(GRAY))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(GRAY));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let today = Utc::now().date_naive();
    let days_since_monday = today.weekday().num_days_from_monday() as i64;
    let this_monday = today - chrono::Duration::days(days_since_monday);
    let start = this_monday - chrono::Duration::weeks(11);
    let weeks: usize = 12;
    let day_labels = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];

    let lines: Vec<Line> = day_labels.iter().enumerate().map(|(dow, label)| {
        let mut spans = vec![Span::styled(format!("{label} "), Style::default().fg(GRAY))];
        for week in 0..weeks {
            let date = start + chrono::Duration::days((week * 7 + dow) as i64);
            if date > today {
                spans.push(Span::raw("  "));
                continue;
            }
            let key = date.format("%Y-%m-%d").to_string();
            let count = app.heatmap.get(&key).copied().unwrap_or(0);
            let color = match count {
                0     => Color::Rgb(35, 45, 35),
                1..=2 => Color::Rgb(0, 100, 40),
                3..=6 => Color::Rgb(0, 170, 70),
                _     => Color::Rgb(50, 230, 100),
            };
            spans.push(Span::styled("█ ", Style::default().fg(color)));
        }
        Line::from(spans)
    }).collect();

    f.render_widget(Paragraph::new(lines), inner);
}
