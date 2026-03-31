use anyhow::Result;
use rusqlite::{Connection, params};
use std::path::PathBuf;
use chrono::Utc;

use crate::srs::{CardDirection};

pub struct Database {
    pub conn: Connection,
}

impl Database {
    pub fn open() -> Result<Self> {
        let path = db_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&path)?;
        let db = Database { conn };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> Result<()> {
        self.conn.execute_batch("
            PRAGMA journal_mode=WAL;
            PRAGMA foreign_keys=ON;

            CREATE TABLE IF NOT EXISTS words (
                id          INTEGER PRIMARY KEY,
                hanzi       TEXT NOT NULL,
                pinyin      TEXT NOT NULL,
                english     TEXT NOT NULL,
                level       INTEGER NOT NULL,
                custom_deck TEXT
            );

            CREATE TABLE IF NOT EXISTS cards (
                id            INTEGER PRIMARY KEY,
                word_id       INTEGER NOT NULL REFERENCES words(id),
                direction     TEXT NOT NULL,
                ease_factor   REAL NOT NULL DEFAULT 2.5,
                interval_days REAL NOT NULL DEFAULT 0,
                repetitions   INTEGER NOT NULL DEFAULT 0,
                due_at        TEXT NOT NULL,
                last_grade    INTEGER,
                created_at    TEXT NOT NULL
            );

            CREATE UNIQUE INDEX IF NOT EXISTS idx_cards_word_dir
                ON cards(word_id, direction);

            CREATE TABLE IF NOT EXISTS reviews (
                id         INTEGER PRIMARY KEY,
                card_id    INTEGER NOT NULL REFERENCES cards(id),
                grade      INTEGER NOT NULL,
                time_ms    INTEGER NOT NULL,
                reviewed_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS custom_decks (
                id   INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE
            );

            CREATE TABLE IF NOT EXISTS settings (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
        ")?;
        Ok(())
    }

    pub fn seed_words(&self) -> Result<()> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM words", [], |r| r.get(0)
        )?;
        if count > 0 { return Ok(()); }

        let now = Utc::now().to_rfc3339();
        let mut stmt = self.conn.prepare(
            "INSERT OR IGNORE INTO words (hanzi, pinyin, english, level) VALUES (?1,?2,?3,?4)"
        )?;

        for (hanzi, pinyin, english, level) in crate::data::hsk_words::HSK_WORDS {
            stmt.execute(params![hanzi, pinyin, english, level])?;
        }

        // Create cards for each word × direction, due immediately
        let word_ids: Vec<i64> = {
            let mut s = self.conn.prepare("SELECT id FROM words")?;
            let rows = s.query_map([], |r| r.get(0))?;
            let ids = rows.filter_map(|r| r.ok()).collect();
            ids
        };

        let directions = ["zh_to_pinyin", "zh_to_en", "en_to_zh", "pinyin_to_zh"];
        let mut card_stmt = self.conn.prepare(
            "INSERT OR IGNORE INTO cards (word_id, direction, ease_factor, interval_days, repetitions, due_at, created_at)
             VALUES (?1, ?2, 2.5, 0, 0, ?3, ?3)"
        )?;

        for wid in &word_ids {
            for dir in &directions {
                card_stmt.execute(params![wid, dir, now])?;
            }
        }

        Ok(())
    }

    // ── Card queries ──────────────────────────────────────────────────────────

    pub fn due_cards(
        &self,
        max_level: u8,
        directions: &[CardDirection],
        limit: usize,
        custom_deck: Option<&str>,
    ) -> Result<Vec<CardRow>> {
        let now = Utc::now().to_rfc3339();
        let dir_list = directions.iter().map(|d| format!("'{}'", d.as_str())).collect::<Vec<_>>().join(",");

        let sql = if let Some(deck) = custom_deck {
            format!(
                "SELECT c.id, c.direction, c.ease_factor, c.interval_days,
                        c.repetitions, w.hanzi, w.pinyin, w.english, w.level
                 FROM cards c JOIN words w ON w.id = c.word_id
                 WHERE w.level <= {max_level}
                   AND c.direction IN ({dir_list})
                   AND c.due_at <= '{now}'
                   AND w.custom_deck = '{deck}'
                 ORDER BY RANDOM() LIMIT {limit}"
            )
        } else {
            format!(
                "SELECT c.id, c.direction, c.ease_factor, c.interval_days,
                        c.repetitions, w.hanzi, w.pinyin, w.english, w.level
                 FROM cards c JOIN words w ON w.id = c.word_id
                 WHERE w.level <= {max_level}
                   AND c.direction IN ({dir_list})
                   AND c.due_at <= '{now}'
                 ORDER BY RANDOM() LIMIT {limit}"
            )
        };

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([], |r| {
            Ok(CardRow {
                id: r.get(0)?,
                direction: r.get(1)?,
                ease_factor: r.get(2)?,
                interval_days: r.get(3)?,
                repetitions: r.get(4)?,
                hanzi: r.get(5)?,
                pinyin: r.get(6)?,
                english: r.get(7)?,
                level: r.get(8)?,
            })
        })?.filter_map(|r| r.ok()).collect();
        Ok(rows)
    }

    pub fn update_card_srs(&self, card_id: i64, ef: f64, interval: f64, reps: i32, due: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE cards SET ease_factor=?1, interval_days=?2, repetitions=?3, due_at=?4 WHERE id=?5",
            params![ef, interval, reps, due, card_id],
        )?;
        Ok(())
    }

    pub fn record_review(&self, card_id: i64, grade: i32, time_ms: u64) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO reviews (card_id, grade, time_ms, reviewed_at) VALUES (?1,?2,?3,?4)",
            params![card_id, grade, time_ms as i64, now],
        )?;
        Ok(())
    }

    // ── Custom deck ──────────────────────────────────────────────────────────

    pub fn add_word_to_deck(&self, word_id: i64, deck_name: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO custom_decks (name) VALUES (?1)",
            params![deck_name],
        )?;
        self.conn.execute(
            "UPDATE words SET custom_deck=?1 WHERE id=?2",
            params![deck_name, word_id],
        )?;
        Ok(())
    }

    pub fn search_words(&self, query: &str) -> Result<Vec<WordRow>> {
        let pat = format!("%{}%", query);
        let mut stmt = self.conn.prepare(
            "SELECT id, hanzi, pinyin, english, level, custom_deck FROM words
             WHERE hanzi LIKE ?1 OR pinyin LIKE ?1 OR english LIKE ?1
             LIMIT 50"
        )?;
        let rows = stmt.query_map(params![pat], |r| {
            Ok(WordRow {
                id: r.get(0)?,
                hanzi: r.get(1)?,
                pinyin: r.get(2)?,
                english: r.get(3)?,
                level: r.get(4)?,
                custom_deck: r.get(5)?,
            })
        })?.filter_map(|r| r.ok()).collect();
        Ok(rows)
    }

    // ── Stats ────────────────────────────────────────────────────────────────

    pub fn stats(&self) -> Result<Stats> {
        let total_reviews: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM reviews", [], |r| r.get(0)
        )?;

        let today = Utc::now().format("%Y-%m-%d").to_string();
        let reviews_today: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM reviews WHERE reviewed_at LIKE ?1",
            params![format!("{}%", today)], |r| r.get(0)
        )?;

        let avg_grade: f64 = self.conn.query_row(
            "SELECT COALESCE(AVG(grade),0) FROM reviews", [], |r| r.get(0)
        )?;

        let cards_due: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM cards WHERE due_at <= ?1",
            params![Utc::now().to_rfc3339()], |r| r.get(0)
        )?;

        let total_cards: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM cards", [], |r| r.get(0)
        )?;

        let mature_cards: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM cards WHERE interval_days >= 21", [], |r| r.get(0)
        )?;

        // Streak: count consecutive days with at least 1 review
        let streak = self.calculate_streak()?;

        // Per-level breakdown
        let mut level_stmt = self.conn.prepare(
            "SELECT w.level, COUNT(*),
                    SUM(CASE WHEN c.interval_days>=21 THEN 1 ELSE 0 END),
                    SUM(CASE WHEN c.repetitions>0    THEN 1 ELSE 0 END)
             FROM cards c JOIN words w ON w.id=c.word_id
             GROUP BY w.level ORDER BY w.level"
        )?;
        let level_stats: Vec<LevelStat> = level_stmt.query_map([], |r| {
            Ok(LevelStat {
                level: r.get(0)?,
                card_count: r.get(1)?,
                mature: r.get(2)?,
                reviewed: r.get(3)?,
            })
        })?.filter_map(|r| r.ok()).collect();

        // Weakest words (lowest ease factor with at least 1 rep)
        let mut weak_stmt = self.conn.prepare(
            "SELECT w.hanzi, w.pinyin, c.ease_factor
             FROM cards c JOIN words w ON w.id=c.word_id
             WHERE c.repetitions > 0
             ORDER BY c.ease_factor ASC LIMIT 10"
        )?;
        let weakest: Vec<WeakCard> = weak_stmt.query_map([], |r| {
            Ok(WeakCard {
                hanzi: r.get(0)?,
                pinyin: r.get(1)?,
                ease_factor: r.get(2)?,
            })
        })?.filter_map(|r| r.ok()).collect();

        // Last 7 days review counts
        let mut daily_stmt = self.conn.prepare(
            "SELECT substr(reviewed_at,1,10) as day, COUNT(*) as cnt
             FROM reviews
             WHERE reviewed_at >= datetime('now','-7 days')
             GROUP BY day ORDER BY day"
        )?;
        let daily_reviews: Vec<(String, i64)> = daily_stmt.query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })?.filter_map(|r| r.ok()).collect();

        Ok(Stats {
            total_reviews,
            reviews_today,
            avg_grade,
            cards_due,
            total_cards,
            mature_cards,
            streak,
            level_stats,
            weakest,
            daily_reviews,
        })
    }

    fn calculate_streak(&self) -> Result<i64> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT substr(reviewed_at,1,10) as day FROM reviews ORDER BY day DESC"
        )?;
        let days: Vec<String> = stmt.query_map([], |r| r.get(0))?.filter_map(|r| r.ok()).collect();

        let today = Utc::now().format("%Y-%m-%d").to_string();
        let mut streak = 0i64;
        let mut expected = today.clone();

        for day in &days {
            if day == &expected {
                streak += 1;
                // decrement expected by 1 day (naive string arithmetic via chrono)
                if let Ok(d) = chrono::NaiveDate::parse_from_str(&expected, "%Y-%m-%d") {
                    expected = (d - chrono::Duration::days(1)).format("%Y-%m-%d").to_string();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        Ok(streak)
    }
}

fn db_path() -> Result<PathBuf> {
    let mut p = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    p.push("zhli");
    p.push("data.db");
    Ok(p)
}

// ── Data types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CardRow {
    pub id: i64,
    pub direction: String,
    pub ease_factor: f64,
    pub interval_days: f64,
    pub repetitions: i32,
    pub hanzi: String,
    pub pinyin: String,
    pub english: String,
    pub level: u8,
}

#[derive(Debug, Clone)]
pub struct WordRow {
    pub id: i64,
    pub hanzi: String,
    pub pinyin: String,
    pub english: String,
    pub level: u8,
    pub custom_deck: Option<String>,
}

#[derive(Debug)]
pub struct Stats {
    pub total_reviews: i64,
    pub reviews_today: i64,
    pub avg_grade: f64,
    pub cards_due: i64,
    pub total_cards: i64,
    pub mature_cards: i64,
    pub streak: i64,
    pub level_stats: Vec<LevelStat>,
    pub weakest: Vec<WeakCard>,
    pub daily_reviews: Vec<(String, i64)>,
}

#[derive(Debug)]
pub struct LevelStat {
    pub level: u8,
    pub card_count: i64,
    pub mature: i64,   // interval >= 21 days
    pub reviewed: i64, // repetitions > 0
}

#[derive(Debug)]
pub struct WeakCard {
    pub hanzi: String,
    pub pinyin: String,
    pub ease_factor: f64,
}
