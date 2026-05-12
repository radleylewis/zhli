use anyhow::Result;
use rusqlite::{Connection, params, params_from_iter, types::Value, OptionalExtension};
use chrono::Utc;

use crate::srs::{CardDirection};

pub struct Database {
    pub conn: Connection,
}

impl Database {
    pub fn open(path: &std::path::Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        let db = Database { conn };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> Result<()> {
        self.conn.execute_batch("
            PRAGMA journal_mode=DELETE;
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
                created_at    TEXT NOT NULL,
                suspended     INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS reviews (
                id          INTEGER PRIMARY KEY,
                card_id     INTEGER NOT NULL REFERENCES cards(id),
                grade       INTEGER NOT NULL,
                time_ms     INTEGER NOT NULL,
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

            CREATE TABLE IF NOT EXISTS word_deck_memberships (
                word_id   INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
                deck_name TEXT    NOT NULL,
                PRIMARY KEY (word_id, deck_name)
            );

            INSERT OR IGNORE INTO word_deck_memberships (word_id, deck_name)
                SELECT id, custom_deck FROM words
                WHERE custom_deck IS NOT NULL AND custom_deck != '';

            DELETE FROM reviews WHERE card_id IN (
                SELECT id FROM cards WHERE id NOT IN (
                    SELECT MIN(id) FROM cards GROUP BY word_id, direction
                )
            );
            DELETE FROM cards WHERE id NOT IN (
                SELECT MIN(id) FROM cards GROUP BY word_id, direction
            );

            CREATE UNIQUE INDEX IF NOT EXISTS idx_cards_word_dir
                ON cards(word_id, direction);

            DELETE FROM reviews WHERE card_id IN (
                SELECT c.id FROM cards c WHERE c.word_id IN (
                    SELECT id FROM words WHERE id NOT IN (
                        SELECT MIN(id) FROM words GROUP BY hanzi, pinyin
                    )
                )
            );
            DELETE FROM cards WHERE word_id IN (
                SELECT id FROM words WHERE id NOT IN (
                    SELECT MIN(id) FROM words GROUP BY hanzi, pinyin
                )
            );
            DELETE FROM words WHERE id NOT IN (
                SELECT MIN(id) FROM words GROUP BY hanzi, pinyin
            );

            CREATE UNIQUE INDEX IF NOT EXISTS idx_words_hanzi_pinyin
                ON words(hanzi, pinyin);
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

        for &(hanzi, pinyin, english, level) in crate::data::hsk_words::HSK_WORDS.iter() {
            stmt.execute(params![hanzi, pinyin, english, level])?;
        }

        // Create cards for each word × direction, due immediately
        let word_ids: Vec<i64> = {
            let mut s = self.conn.prepare("SELECT id FROM words")?;
            let rows = s.query_map([], |r| r.get(0))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
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
        selected_levels: &[u8],
        selected_decks: &[&str],
        directions: &[CardDirection],
        limit: usize,
    ) -> Result<Vec<CardRow>> {
        if selected_levels.is_empty() && selected_decks.is_empty() {
            return Ok(vec![]);
        }

        let now = Utc::now().to_rfc3339();
        let mut params: Vec<Value> = Vec::new();
        let mut conditions: Vec<String> = Vec::new();

        if !selected_levels.is_empty() {
            let ph = placeholders(selected_levels.len());
            for &l in selected_levels {
                params.push(Value::Integer(i64::from(l)));
            }
            params.push(Value::Text(now));
            // HSK words: apply SRS due-date filter.
            conditions.push(format!("(w.level IN ({ph}) AND c.due_at <= ?)"));
        }

        for &deck in selected_decks {
            // Deck = practice mode: no due-date filter, always available.
            // A word can be in multiple decks; EXISTS avoids row duplication.
            conditions.push(
                "(EXISTS (SELECT 1 FROM word_deck_memberships wdm \
                          WHERE wdm.word_id = w.id AND wdm.deck_name = ?))".to_string()
            );
            params.push(Value::Text(deck.to_string()));
        }

        let dir_ph = placeholders(directions.len());
        for d in directions {
            params.push(Value::Text(d.as_str().to_string()));
        }
        params.push(Value::Integer(limit as i64));

        let where_clause = conditions.join(" OR ");
        let sql = format!(
            "SELECT c.id, w.id, c.direction, c.ease_factor, c.interval_days,
                    c.repetitions, w.hanzi, w.pinyin, w.english, w.level
             FROM cards c JOIN words w ON w.id = c.word_id
             WHERE ({where_clause})
               AND c.direction IN ({dir_ph})
               AND c.suspended = 0
             ORDER BY RANDOM() LIMIT ?"
        );

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(params), |r| {
            Ok(CardRow {
                id:           r.get(0)?,
                word_id:      r.get(1)?,
                direction:    r.get(2)?,
                ease_factor:  r.get(3)?,
                interval_days: r.get(4)?,
                repetitions:  r.get(5)?,
                hanzi:        r.get(6)?,
                pinyin:       r.get(7)?,
                english:      r.get(8)?,
                level:        r.get(9)?,
            })
        })?.collect::<rusqlite::Result<Vec<_>>>()?;
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
            "INSERT OR IGNORE INTO word_deck_memberships (word_id, deck_name) VALUES (?1, ?2)",
            params![word_id, deck_name],
        )?;
        Ok(())
    }

    pub fn list_decks(&self) -> Result<Vec<String>> {
        // Query custom_decks table so newly created empty decks appear immediately
        let mut stmt = self.conn.prepare("SELECT name FROM custom_decks ORDER BY name")?;
        let names = stmt.query_map([], |r| r.get(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(names)
    }

    pub fn create_deck(&self, name: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO custom_decks (name) VALUES (?1)",
            params![name],
        )?;
        Ok(())
    }

    pub fn delete_deck(&self, name: &str) -> Result<()> {
        self.conn.execute("DELETE FROM word_deck_memberships WHERE deck_name = ?1", params![name])?;
        self.conn.execute("DELETE FROM custom_decks WHERE name = ?1", params![name])?;
        Ok(())
    }

    pub fn words_in_deck(&self, deck_name: &str) -> Result<Vec<WordRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT w.id, w.hanzi, w.pinyin, w.english, w.level,
                    (SELECT GROUP_CONCAT(wdm2.deck_name, char(31))
                     FROM word_deck_memberships wdm2
                     WHERE wdm2.word_id = w.id
                     ORDER BY wdm2.deck_name),
                    COALESCE((SELECT MIN(c.suspended) FROM cards c WHERE c.word_id = w.id), 0)
             FROM words w
             JOIN word_deck_memberships wdm ON wdm.word_id = w.id
             WHERE wdm.deck_name = ?1
             ORDER BY w.level, w.hanzi"
        )?;
        let rows = stmt.query_map(params![deck_name], |r| {
            let deck_list: Option<String> = r.get(5)?;
            Ok(WordRow {
                id: r.get(0)?, hanzi: r.get(1)?, pinyin: r.get(2)?,
                english: r.get(3)?, level: r.get(4)?,
                decks: deck_list.map(|s| s.split('\x1F').map(|d| d.to_string()).collect())
                                 .unwrap_or_default(),
                suspended: r.get::<_, i64>(6)? == 1,
            })
        })?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn suspend_word(&self, word_id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE cards SET suspended = 1 WHERE word_id = ?1",
            params![word_id],
        )?;
        Ok(())
    }

    pub fn delete_word(&self, word_id: i64) -> Result<()> {
        self.conn.execute(
            "DELETE FROM reviews WHERE card_id IN (SELECT id FROM cards WHERE word_id = ?1)",
            params![word_id],
        )?;
        self.conn.execute("DELETE FROM cards WHERE word_id = ?1", params![word_id])?;
        self.conn.execute("DELETE FROM words WHERE id = ?1", params![word_id])?;
        Ok(())
    }

    pub fn add_custom_word(&self, hanzi: &str, pinyin: &str, english: &str, level: u8, deck: Option<&str>) -> Result<()> {
        let existing: Option<(i64, u8)> = self.conn.query_row(
            "SELECT id, level FROM words WHERE hanzi = ?1 AND pinyin = ?2",
            params![hanzi, pinyin], |r| Ok((r.get(0)?, r.get(1)?)),
        ).optional()?;

        if let Some((word_id, existing_level)) = existing {
            if existing_level > 0 {
                // HSK word — leave it in its HSK level, cannot belong to a deck simultaneously.
                anyhow::bail!("'{}' is already in HSK {}.", hanzi, existing_level);
            }
            if let Some(deck_name) = deck {
                // Custom word — assign it to the requested deck instead of failing.
                return self.add_word_to_deck(word_id, deck_name);
            }
            anyhow::bail!("'{}' ({}) is already in the dictionary", hanzi, pinyin);
        }

        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO words (hanzi, pinyin, english, level) VALUES (?1,?2,?3,?4)",
            params![hanzi, pinyin, english, level],
        )?;
        let word_id = self.conn.last_insert_rowid();
        let mut stmt = self.conn.prepare(
            "INSERT INTO cards (word_id, direction, ease_factor, interval_days, repetitions, due_at, created_at)
             VALUES (?1, ?2, 2.5, 0, 0, ?3, ?3)"
        )?;
        for dir in &["zh_to_pinyin", "zh_to_en", "en_to_zh", "pinyin_to_zh"] {
            stmt.execute(params![word_id, dir, now])?;
        }
        if let Some(deck_name) = deck {
            self.add_word_to_deck(word_id, deck_name)?;
        }
        Ok(())
    }

    pub fn clear_custom_words(&self) -> Result<()> {
        self.conn.execute_batch("
            DELETE FROM reviews
                WHERE card_id IN (
                    SELECT id FROM cards
                    WHERE word_id IN (SELECT id FROM words WHERE level = 0)
                );
            DELETE FROM cards WHERE word_id IN (SELECT id FROM words WHERE level = 0);
            DELETE FROM words WHERE level = 0;
            DELETE FROM word_deck_memberships WHERE deck_name IN (SELECT name FROM custom_decks);
            DELETE FROM custom_decks;
        ")?;
        Ok(())
    }

    pub fn reset_progress(&self) -> Result<()> {
        self.conn.execute("DELETE FROM reviews", [])?;
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE cards SET ease_factor=2.5, interval_days=0, repetitions=0, due_at=?1",
            params![now],
        )?;
        Ok(())
    }

    pub fn search_words(&self, query: &str) -> Result<Vec<WordRow>> {
        let pat = format!("%{}%", query);
        let mut stmt = self.conn.prepare(
            "SELECT w.id, w.hanzi, w.pinyin, w.english, w.level,
                    (SELECT GROUP_CONCAT(wdm.deck_name, char(31))
                     FROM word_deck_memberships wdm
                     WHERE wdm.word_id = w.id
                     ORDER BY wdm.deck_name),
                    COALESCE((SELECT MIN(c.suspended) FROM cards c WHERE c.word_id = w.id), 0)
             FROM words w
             WHERE w.hanzi LIKE ?1 OR w.pinyin LIKE ?1 OR w.english LIKE ?1
             LIMIT 50"
        )?;
        let rows = stmt.query_map(params![pat], |r| {
            let deck_list: Option<String> = r.get(5)?;
            Ok(WordRow {
                id: r.get(0)?,
                hanzi: r.get(1)?,
                pinyin: r.get(2)?,
                english: r.get(3)?,
                level: r.get(4)?,
                decks: deck_list.map(|s| s.split('\x1F').map(|d| d.to_string()).collect())
                                 .unwrap_or_default(),
                suspended: r.get::<_, i64>(6)? == 1,
            })
        })?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn unsuspend_word(&self, word_id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE cards SET suspended = 0 WHERE word_id = ?1",
            params![word_id],
        )?;
        Ok(())
    }

    pub fn remove_word_from_deck(&self, word_id: i64, deck_name: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM word_deck_memberships WHERE word_id = ?1 AND deck_name = ?2",
            params![word_id, deck_name],
        )?;
        Ok(())
    }

    pub fn update_word(&self, id: i64, hanzi: &str, pinyin: &str, english: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE words SET hanzi = ?1, pinyin = ?2, english = ?3 WHERE id = ?4",
            params![hanzi, pinyin, english, id],
        )?;
        Ok(())
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

        // Per-level breakdown: HSK levels + per-deck custom word stats.
        // Deck stats join via word_deck_memberships so one word can appear in multiple decks.
        let mut level_stmt = self.conn.prepare(
            "SELECT w.level, CAST(NULL AS TEXT),
                    COUNT(*),
                    SUM(CASE WHEN c.interval_days>=21 THEN 1 ELSE 0 END),
                    SUM(CASE WHEN c.repetitions>0    THEN 1 ELSE 0 END)
             FROM cards c JOIN words w ON w.id=c.word_id
             WHERE w.level > 0
             GROUP BY w.level
             UNION ALL
             SELECT 0, d.name,
                    COUNT(*),
                    SUM(CASE WHEN c.interval_days>=21 THEN 1 ELSE 0 END),
                    SUM(CASE WHEN c.repetitions>0    THEN 1 ELSE 0 END)
             FROM custom_decks d
             JOIN word_deck_memberships wdm ON wdm.deck_name = d.name
             JOIN words w ON w.id = wdm.word_id AND w.level = 0
             JOIN cards c ON c.word_id = w.id
             GROUP BY d.name
             UNION ALL
             SELECT 0, '(no deck)',
                    COUNT(*),
                    SUM(CASE WHEN c.interval_days>=21 THEN 1 ELSE 0 END),
                    SUM(CASE WHEN c.repetitions>0    THEN 1 ELSE 0 END)
             FROM cards c JOIN words w ON w.id=c.word_id
             WHERE w.level = 0
               AND NOT EXISTS (SELECT 1 FROM word_deck_memberships WHERE word_id = w.id)
             HAVING COUNT(*) > 0
             ORDER BY 1, 2"
        )?;
        let level_stats: Vec<LevelStat> = level_stmt.query_map([], |r| {
            Ok(LevelStat {
                level: r.get(0)?,
                deck_name: r.get(1)?,
                card_count: r.get(2)?,
                mature: r.get(3)?,
                reviewed: r.get(4)?,
            })
        })?.collect::<rusqlite::Result<Vec<_>>>()?;

        // Weakest words: group by word, take the lowest EF across all directions,
        // only include words that have at least one review recorded.
        // (repetitions resets to 0 on failure so cannot be used as the filter)
        let mut weak_stmt = self.conn.prepare(
            "SELECT w.hanzi, w.pinyin, MIN(c.ease_factor) as ef
             FROM cards c JOIN words w ON w.id = c.word_id
             WHERE EXISTS (SELECT 1 FROM reviews r WHERE r.card_id = c.id)
             GROUP BY w.id
             ORDER BY ef ASC LIMIT 10"
        )?;
        let weakest: Vec<WeakCard> = weak_stmt.query_map([], |r| {
            Ok(WeakCard {
                hanzi: r.get(0)?,
                pinyin: r.get(1)?,
                ease_factor: r.get(2)?,
            })
        })?.collect::<rusqlite::Result<Vec<_>>>()?;

        // Last 7 days review counts
        let mut daily_stmt = self.conn.prepare(
            "SELECT substr(reviewed_at,1,10) as day, COUNT(*) as cnt
             FROM reviews
             WHERE reviewed_at >= datetime('now','-7 days')
             GROUP BY day ORDER BY day"
        )?;
        let daily_reviews: Vec<(String, i64)> = daily_stmt.query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })?.collect::<rusqlite::Result<Vec<_>>>()?;

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
        let days: Vec<String> = stmt.query_map([], |r| r.get(0))?.collect::<rusqlite::Result<Vec<_>>>()?;

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

    pub fn save_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn load_setting(&self, key: &str) -> Result<Option<String>> {
        match self.conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |r| r.get(0),
        ) {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

}

fn placeholders(n: usize) -> String {
    std::iter::repeat_n("?", n).collect::<Vec<_>>().join(",")
}


// ── Data types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CardRow {
    pub id: i64,
    pub word_id: i64,
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
    pub decks: Vec<String>,
    pub suspended: bool,
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
    pub deck_name: Option<String>, // set for custom-deck rows (level = 0)
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
