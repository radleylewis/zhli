mod deck;
mod menu;
mod review;
mod stats;

use ratatui::{
    Frame,
    layout::Rect,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Paragraph},
};

use crate::ui::app_state::{App, Screen};

pub(super) const RED:    Color = Color::Rgb(220, 50,  50);
pub(super) const GREEN:  Color = Color::Rgb(80,  200, 80);
pub(super) const YELLOW: Color = Color::Rgb(220, 180, 40);
pub(super) const BLUE:   Color = Color::Rgb(80,  140, 220);
pub(super) const CYAN:   Color = Color::Rgb(80,  210, 200);
pub(super) const GRAY:   Color = Color::Rgb(120, 120, 120);
pub(super) const WHITE:  Color = Color::Rgb(230, 230, 230);
pub(super) const BG:     Color = Color::Rgb(18,  18,  28);
pub(super) const BG2:    Color = Color::Rgb(28,  28,  42);
pub(super) const ACCENT: Color = Color::Rgb(180, 80,  220);
pub(super) const GOLD:   Color = Color::Rgb(220, 170, 40);
pub(super) const ORANGE: Color = Color::Rgb(220, 140, 40);
pub(super) const WARN:   Color = Color::Rgb(200, 100, 50);

pub(super) fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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

pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();
    f.render_widget(Block::default().style(Style::default().bg(BG)), area);

    match app.screen {
        Screen::MainMenu        => menu::render_main_menu(f, app, area),
        Screen::ModeSelect      => menu::render_mode_select(f, app, area),
        Screen::ContentSelect   => menu::render_content_select(f, app, area),
        Screen::Review          => review::render_review(f, app, area),
        Screen::Stats           => stats::render_stats(f, app, area),
        Screen::AddToDeck       => deck::render_add_to_deck(f, app, area),
        Screen::AddCustomWord   => deck::render_add_custom_word(f, app, area),
        Screen::SearchDeck      => deck::render_search_deck(f, app, area),
        Screen::EditWord        => deck::render_edit_word(f, app, area),
        Screen::About           => menu::render_about(f, area),
        Screen::Confirm(ref action) => menu::render_confirm(f, action, area),
    }

    // Bottom bar: command mode takes priority over status message
    let bar = Rect { x: 0, y: area.height.saturating_sub(1), width: area.width, height: 1 };
    if let Some(ref cmd) = app.cmd_buffer {
        let p = Paragraph::new(format!(":{cmd}"))
            .style(Style::default().fg(WHITE));
        f.render_widget(p, bar);
    } else if !app.status_message.is_empty() {
        let p = Paragraph::new(app.status_message.as_str())
            .style(Style::default().fg(GREEN).bg(Color::Rgb(20, 40, 20)));
        f.render_widget(p, bar);
    }
}
