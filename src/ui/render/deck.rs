use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, BorderType, Clear, List, ListItem, Paragraph},
};

use crate::ui::app_state::App;
use super::{ACCENT, BG, BG2, BLUE, CYAN, GOLD, GRAY, GREEN, RED, WARN, WHITE, YELLOW, centered_rect};

pub(super) fn render_add_to_deck(f: &mut Frame, app: &App, area: Rect) {
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

    let panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(outer[1]);

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

    let (input_border, input_title, input_text) = if app.creating_new_deck {
        (ACCENT, " New Deck Name ", format!("{}█", app.deck_name_input))
    } else if app.renaming_deck {
        (YELLOW, " Rename Deck ", format!("{}█", app.deck_name_input))
    } else {
        (GRAY, " New Deck Name (navigate to ＋) ", String::new())
    };
    let input_p = Paragraph::new(input_text)
        .block(Block::default().title(input_title).title_style(Style::default().fg(input_border))
            .borders(Borders::ALL).border_type(BorderType::Rounded)
            .border_style(Style::default().fg(input_border)))
        .style(Style::default().fg(WHITE));
    f.render_widget(input_p, outer[2]);

    let footer = if app.creating_new_deck {
        "Type deck name  •  Enter Create & go to search  •  Esc Cancel"
    } else if app.renaming_deck {
        "Type new name  •  Enter Save  •  Esc Cancel"
    } else {
        "↑↓/jk Navigate  •  Enter Select  •  r Rename  •  d/Del Delete  •  Esc Back"
    };
    f.render_widget(
        Paragraph::new(footer).style(Style::default().fg(GRAY)).alignment(Alignment::Center),
        outer[3],
    );
}

pub(super) fn render_search_deck(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(10), Constraint::Length(2)])
        .margin(2)
        .split(area);

    let search_block = Block::default()
        .title(" 🔍 Search Words (hanzi, pinyin, english) ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(CYAN));
    let search_p = Paragraph::new(format!("{}█", app.search_query))
        .block(search_block)
        .style(Style::default().fg(WHITE));
    f.render_widget(search_p, chunks[0]);

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

pub(super) fn render_add_custom_word(f: &mut Frame, app: &App, area: Rect) {
    let has_filter = app.dict_filter_active || !app.dict_filter.is_empty();
    let constraints: Vec<Constraint> = if has_filter {
        vec![
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(5),
            Constraint::Length(1),
        ]
    } else {
        vec![
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(5),
            Constraint::Length(1),
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .margin(2)
        .split(area);

    let (idx_status, idx_filter, idx_list, idx_footer) = if has_filter {
        (1, 2, 3, 4)
    } else {
        (1, usize::MAX, 2, 3)
    };

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

    let status_color = if app.dict_status.starts_with("Error") { RED }
        else if is_searching { YELLOW }
        else { GRAY };
    let status_p = Paragraph::new(app.dict_status.as_str())
        .style(Style::default().fg(status_color))
        .alignment(Alignment::Center);
    f.render_widget(status_p, chunks[idx_status]);

    if has_filter {
        let filter_text = format!("/{}{}", app.dict_filter, if app.dict_filter_active { "█" } else { "" });
        let filter_p = Paragraph::new(filter_text)
            .style(Style::default().fg(if app.dict_filter_active { CYAN } else { GRAY }));
        f.render_widget(filter_p, chunks[idx_filter]);
    }

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
            Style::default().fg(if selected { WHITE } else { GRAY }).bg(bg),
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

    let mut list_state = ratatui::widgets::ListState::default();
    if !filtered.is_empty() {
        list_state.select(Some(app.dict_cursor));
    }
    f.render_stateful_widget(list, chunks[idx_list], &mut list_state);

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

pub(super) fn render_edit_word(f: &mut Frame, app: &App, area: Rect) {
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
