use std::time::Instant;
use anyhow::Result;
use crate::db::{CardRow, Database, Stats};
use crate::srs::{CardDirection, ReviewGrade};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Screen {
    MainMenu,
    LevelSelect,
    ModeSelect,
    Review,
    Stats,
    AddToDeck,
    SearchDeck,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewPhase {
    Prompt,       // Show question, user types answer
    ShowAnswer,   // Show answer + feedback, user grades
}

pub struct App {
    pub screen: Screen,
    pub db: Database,
    // Settings
    pub selected_level: u8,
    pub selected_directions: Vec<CardDirection>,
    pub current_deck: Option<String>,
    // Review state
    pub review_queue: Vec<CardRow>,
    pub current_card_idx: usize,
    pub review_phase: ReviewPhase,
    pub input_buffer: String,
    pub feedback: String,
    pub last_score: f64,
    pub card_start: Option<Instant>,
    pub session_correct: u32,
    pub session_total: u32,
    // Stats
    pub stats_cache: Option<Stats>,
    // Menu cursors
    pub menu_cursor: usize,
    pub level_cursor: u8,
    pub mode_cursor: usize,
    pub start_review_after_mode: bool,
    pub grade_cursor: usize,
    // Deck management
    pub search_query: String,
    pub search_results: Vec<crate::db::WordRow>,
    pub search_cursor: usize,
    pub deck_name_input: String,
    pub status_message: String,
    pub status_set_at: Option<Instant>,
}

impl App {
    pub fn new(db: Database) -> Self {
        Self {
            screen: Screen::MainMenu,
            db,
            selected_level: 1,
            selected_directions: vec![
                CardDirection::ZhToPinyin,
                CardDirection::ZhToEn,
                CardDirection::EnToZh,
                CardDirection::PinyinToZh,
            ],
            current_deck: None,
            review_queue: vec![],
            current_card_idx: 0,
            review_phase: ReviewPhase::Prompt,
            input_buffer: String::new(),
            feedback: String::new(),
            last_score: 0.0,
            card_start: None,
            session_correct: 0,
            session_total: 0,
            stats_cache: None,
            menu_cursor: 0,
            level_cursor: 1,
            mode_cursor: 0,
            start_review_after_mode: false,
            grade_cursor: 1,
            search_query: String::new(),
            search_results: vec![],
            search_cursor: 0,
            deck_name_input: String::new(),
            status_message: String::new(),
            status_set_at: None,
        }
    }

    pub fn load_review_session(&mut self) -> Result<()> {
        self.review_queue = self.db.due_cards(
            self.selected_level,
            &self.selected_directions,
            50,
            self.current_deck.as_deref(),
        )?;
        // Shuffle for variety
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        self.review_queue.shuffle(&mut rng);
        self.current_card_idx = 0;
        self.review_phase = ReviewPhase::Prompt;
        self.input_buffer.clear();
        self.session_correct = 0;
        self.session_total = 0;
        self.card_start = Some(Instant::now());
        Ok(())
    }

    pub fn advance_card(&mut self) {
        self.current_card_idx += 1;
        self.review_phase = ReviewPhase::Prompt;
        self.input_buffer.clear();
        self.feedback.clear();
        self.card_start = Some(Instant::now());
    }

    pub fn refresh_stats(&mut self) -> Result<()> {
        self.stats_cache = Some(self.db.stats()?);
        Ok(())
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.card_start
            .map(|s| s.elapsed().as_millis() as u64)
            .unwrap_or(0)
    }

    pub fn submit_answer(&mut self) -> Result<()> {
        if let Some(card) = self.review_queue.get(self.current_card_idx).cloned() {
            let direction = crate::srs::CardDirection::from_str(&card.direction);
            let (score, feedback) = crate::srs::grade_answer(&direction, &card, &self.input_buffer);
            self.last_score = score;
            self.feedback = feedback;
            self.review_phase = ReviewPhase::ShowAnswer;
            // 1.0 = correct  → suggest Good (4)
            // 0.5-1.0 = partial (e.g. right syllable, wrong tone) → suggest Hard (2, incorrect)
            // 0.0 = wrong    → suggest Wrong (1)
            self.grade_cursor = if score >= 1.0 { 4 } else if score >= 0.5 { 2 } else { 1 };
        }
        Ok(())
    }

    pub fn apply_grade(&mut self, grade: ReviewGrade) -> Result<()> {
        let elapsed = self.elapsed_ms();
        if let Some(card) = self.review_queue.get(self.current_card_idx).cloned() {
            crate::srs::apply_review(&self.db, &card, grade, elapsed)?;
            self.session_total += 1;
            if self.last_score >= 0.5 { self.session_correct += 1; }
        }
        self.advance_card();
        Ok(())
    }

    pub fn do_search(&mut self) -> Result<()> {
        self.search_results = self.db.search_words(&self.search_query)?;
        self.search_cursor = 0;
        Ok(())
    }

    pub fn add_selected_to_deck(&mut self) -> Result<()> {
        if let Some(word) = self.search_results.get(self.search_cursor) {
            let deck = if self.deck_name_input.is_empty() {
                "My Deck".to_string()
            } else {
                self.deck_name_input.clone()
            };
            self.db.add_word_to_deck(word.id, &deck)?;
            self.status_message = format!("Added '{}' to deck '{}'", word.hanzi, deck);
            self.status_set_at = Some(Instant::now());
        }
        Ok(())
    }
}
