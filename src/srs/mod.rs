use crate::db::{CardRow, Database};
use anyhow::Result;
use chrono::{Duration, Utc};

// SM-2 scheduling parameters
const INTERVAL_FIRST: f64 = 1.0; // days after first correct review
const INTERVAL_SECOND: f64 = 6.0; // days after second correct review
const INTERVAL_HARD: f64 = 0.5; // ~12 hours (grade 2: wrong but easy to recall)
const INTERVAL_WRONG: f64 = 0.1; // ~2.5 hours (grade 1: incorrect)
const INTERVAL_BLACKOUT: f64 = 0.04; // ~1 hour (grade 0: complete blank)
const MIN_EASE_FACTOR: f64 = 1.3;

/// SM-2 quality grades (0-5)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewGrade {
    Blackout = 0, // User has no idea
    Wrong = 1,    // incorrect but remembered on seeing answer
    Hard = 2,     // incorrect but easy to recall
    Okay = 3,     // correct with difficulty
    Good = 4,     // correct after hesitation
    Perfect = 5,  // perfect recall
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CardDirection {
    ZhToPinyin,
    ZhToEn,
    EnToZh,
    PinyinToZh,
}

impl CardDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ZhToPinyin => "zh_to_pinyin",
            Self::ZhToEn => "zh_to_en",
            Self::EnToZh => "en_to_zh",
            Self::PinyinToZh => "pinyin_to_zh",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "zh_to_pinyin" => Self::ZhToPinyin,
            "zh_to_en" => Self::ZhToEn,
            "en_to_zh" => Self::EnToZh,
            "pinyin_to_zh" => Self::PinyinToZh,
            _ => Self::PinyinToZh,
        }
    }

    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s.trim() {
            "zh_to_pinyin" => Some(Self::ZhToPinyin),
            "zh_to_en" => Some(Self::ZhToEn),
            "en_to_zh" => Some(Self::EnToZh),
            "pinyin_to_zh" => Some(Self::PinyinToZh),
            _ => None,
        }
    }

    pub fn prompt_label(&self) -> &'static str {
        match self {
            Self::ZhToPinyin => "Write the Pinyin",
            Self::ZhToEn => "Write the English meaning",
            Self::EnToZh => "Write the Chinese (Hanzi)",
            Self::PinyinToZh => "Write the Chinese (Hanzi)",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::ZhToPinyin => "Chinese → Pinyin",
            Self::ZhToEn => "Chinese → English",
            Self::EnToZh => "English → Chinese",
            Self::PinyinToZh => "Pinyin → Chinese",
        }
    }
}

/// SM-2 scheduling result
pub struct Sm2Result {
    pub ease_factor: f64,
    pub interval_days: f64,
    pub repetitions: i32,
    pub due_at: String,
}

/// Core SM-2 algorithm
pub fn sm2_schedule(card: &CardRow, grade: ReviewGrade) -> Sm2Result {
    let q = grade as i32;
    let mut ef = card.ease_factor;
    let mut interval = card.interval_days;
    let mut reps = card.repetitions;

    if q >= 3 {
        // Correct response
        interval = match reps {
            0 => INTERVAL_FIRST,
            1 => INTERVAL_SECOND,
            _ => (interval * ef).max(1.0),
        };
        reps += 1;
    } else {
        // Incorrect — reset repetitions, review again soon
        reps = 0;
        interval = match q {
            2 => INTERVAL_HARD,
            1 => INTERVAL_WRONG,
            _ => INTERVAL_BLACKOUT,
        };
    }

    // Update ease factor
    ef += 0.1 - (5.0 - q as f64) * (0.08 + (5.0 - q as f64) * 0.02);
    ef = ef.max(MIN_EASE_FACTOR);

    let due = Utc::now() + Duration::seconds((interval * 86400.0) as i64);
    Sm2Result {
        ease_factor: ef,
        interval_days: interval,
        repetitions: reps,
        due_at: due.to_rfc3339(),
    }
}

/// Pinyin grading helpers
pub fn strip_tones(pinyin: &str) -> String {
    let toned = "āáǎàēéěèīíǐìōóǒòūúǔùǖǘǚǜ";
    let base = "aaaaeeeeiiiioooouuuuüüüü";
    let base_chars: Vec<char> = base.chars().collect();
    let mut out = String::new();
    for ch in pinyin.chars() {
        if let Some(idx) = toned.chars().position(|t| t == ch) {
            out.push(base_chars[idx]);
        } else if ch == 'ü' || ch == 'v' {
            // ü and keyboard substitute 'v' both normalise to ü (distinct from u)
            out.push('ü');
        } else {
            out.push(ch.to_ascii_lowercase());
        }
    }
    out
}

pub fn normalize_pinyin(s: &str) -> String {
    strip_tones(s)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("")
}

/// Convert a single syllable like "hao3" → "hǎo", "zhong1" → "zhōng".
/// Handles 'v' as ü (e.g. "lv4" → "lǜ").
fn numbered_syllable_to_toned(syl: &str) -> String {
    let s = syl.trim();
    let last = match s.chars().last() {
        Some(c) => c,
        None => return s.to_string(),
    };
    let (base, tone): (&str, u8) = match last {
        '1' => (&s[..s.len() - 1], 1),
        '2' => (&s[..s.len() - 1], 2),
        '3' => (&s[..s.len() - 1], 3),
        '4' => (&s[..s.len() - 1], 4),
        '5' => (&s[..s.len() - 1], 0),
        _ => (s, 0),
    };
    if tone == 0 {
        return base.replace('v', "ü");
    }

    // Normalise ü written as 'v'
    let base = base.replace('v', "ü");
    let chars: Vec<char> = base.chars().collect();

    // Placement rules: a/e first, then 'o' in 'ou', then last vowel
    let vowels = ['a', 'e', 'i', 'o', 'u', 'ü'];
    let mark_pos = chars
        .iter()
        .position(|&c| c == 'a' || c == 'e')
        .or_else(|| chars.windows(2).position(|w| w[0] == 'o' && w[1] == 'u'))
        .or_else(|| chars.iter().rposition(|c| vowels.contains(c)));

    let Some(pos) = mark_pos else {
        return base;
    };

    let toned_char = match (chars[pos], tone) {
        ('a', 1) => 'ā',
        ('a', 2) => 'á',
        ('a', 3) => 'ǎ',
        ('a', 4) => 'à',
        ('e', 1) => 'ē',
        ('e', 2) => 'é',
        ('e', 3) => 'ě',
        ('e', 4) => 'è',
        ('i', 1) => 'ī',
        ('i', 2) => 'í',
        ('i', 3) => 'ǐ',
        ('i', 4) => 'ì',
        ('o', 1) => 'ō',
        ('o', 2) => 'ó',
        ('o', 3) => 'ǒ',
        ('o', 4) => 'ò',
        ('u', 1) => 'ū',
        ('u', 2) => 'ú',
        ('u', 3) => 'ǔ',
        ('u', 4) => 'ù',
        ('ü', 1) => 'ǖ',
        ('ü', 2) => 'ǘ',
        ('ü', 3) => 'ǚ',
        ('ü', 4) => 'ǜ',
        (c, _) => c,
    };

    chars
        .iter()
        .enumerate()
        .map(|(i, &c)| if i == pos { toned_char } else { c })
        .collect()
}

/// Convert a full numbered pinyin string (one or more syllables) to toned.
/// Accepts both space-separated ("ni3 hao3") and run-together ("ni3hao3").
pub fn numbered_to_toned(s: &str) -> String {
    // Split on digit boundaries so "ni3hao3" → ["ni3", "hao3"]
    let mut syllables: Vec<String> = Vec::new();
    let mut current = String::new();
    for c in s.chars() {
        current.push(c);
        if matches!(c, '1'..='5') {
            syllables.push(current.trim().to_string());
            current.clear();
        } else if c == ' ' && !current.trim().is_empty() {
            // flush on spaces too, in case user mixes styles
            let trimmed = current.trim().to_string();
            if !trimmed.is_empty() {
                syllables.push(trimmed);
            }
            current.clear();
        }
    }
    if !current.trim().is_empty() {
        syllables.push(current.trim().to_string());
    }
    syllables
        .iter()
        .map(|syl| numbered_syllable_to_toned(syl))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn normalize_english(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Grade a written answer.
/// Returns (score 0.0–1.0, feedback message).
pub fn grade_answer(direction: &CardDirection, card: &CardRow, answer: &str) -> (f64, String) {
    let answer = answer.trim();
    match direction {
        CardDirection::ZhToPinyin => {
            let correct_tones = card.pinyin.trim().to_lowercase();
            // Compact form (no spaces) for flexible matching
            let correct_compact: String = correct_tones.split_whitespace().collect();
            let correct_no_tones = normalize_pinyin(&correct_tones);

            let given_raw = answer.to_lowercase();
            // Accept numbered pinyin (e.g. "ni3 hao3" or "ni3hao3")
            let given_toned = numbered_to_toned(&given_raw);
            let given_compact: String = given_toned.split_whitespace().collect();
            let given_no_tones = normalize_pinyin(&given_toned);

            if given_compact == correct_compact || given_raw == correct_tones {
                (1.0, "✓ Perfect!".to_string())
            } else if given_no_tones == correct_no_tones {
                (0.6, "~ Close — check the tones".to_string())
            } else {
                (0.0, "✗ Incorrect".to_string())
            }
        }
        CardDirection::ZhToEn => {
            let given = normalize_english(answer);
            // Accept /, comma, semicolon, and Chinese enumeration comma as separators
            let variants: Vec<String> = card
                .english
                .split(['/', ',', ';', '、'])
                .map(|v| normalize_english(v.trim()))
                .filter(|v| !v.is_empty())
                .collect();

            if variants.iter().any(|v| v == &given) {
                (1.0, "✓ Correct!".to_string())
            } else if !given.is_empty()
                && variants
                    .iter()
                    .any(|v| v.contains(given.as_str()) || given.contains(v.as_str()))
            {
                (0.6, "~ Partially correct".to_string())
            } else {
                (0.0, "✗ Incorrect".to_string())
            }
        }
        CardDirection::EnToZh | CardDirection::PinyinToZh => {
            let given = answer.trim();
            if given == card.hanzi {
                (1.0, "✓ Correct!".to_string())
            } else {
                (0.0, "✗ Incorrect".to_string())
            }
        }
    }
}

/// Apply a review to the database
pub fn apply_review(db: &Database, card: &CardRow, grade: ReviewGrade, time_ms: u64) -> Result<()> {
    let result = sm2_schedule(card, grade);
    db.update_card_srs(
        card.id,
        result.ease_factor,
        result.interval_days,
        result.repetitions,
        &result.due_at,
    )?;
    db.record_review(card.id, grade as i32, time_ms)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(ef: f64, interval: f64, reps: i32) -> CardRow {
        CardRow {
            id: 1,
            word_id: 1,
            direction: "zh_to_en".into(),
            ease_factor: ef,
            interval_days: interval,
            repetitions: reps,
            hanzi: "你好".into(),
            pinyin: "nǐ hǎo".into(),
            english: "hello".into(),
            level: 1,
        }
    }

    fn make_card(hanzi: &str, pinyin: &str, english: &str) -> CardRow {
        CardRow {
            id: 1,
            word_id: 1,
            direction: "zh_to_en".into(),
            ease_factor: 2.5,
            interval_days: 1.0,
            repetitions: 1,
            hanzi: hanzi.into(),
            pinyin: pinyin.into(),
            english: english.into(),
            level: 1,
        }
    }

    // ── SM-2: correct grades ──────────────────────────────────────────────────

    #[test]
    fn sm2_first_correct_gives_one_day() {
        let r = sm2_schedule(&card(2.5, 0.0, 0), ReviewGrade::Good);
        assert_eq!(r.interval_days, INTERVAL_FIRST);
        assert_eq!(r.repetitions, 1);
    }

    #[test]
    fn sm2_second_correct_gives_six_days() {
        let r = sm2_schedule(&card(2.5, 1.0, 1), ReviewGrade::Good);
        assert_eq!(r.interval_days, INTERVAL_SECOND);
        assert_eq!(r.repetitions, 2);
    }

    #[test]
    fn sm2_third_correct_multiplies_by_ef() {
        let r = sm2_schedule(&card(2.5, 6.0, 2), ReviewGrade::Good);
        assert!((r.interval_days - 15.0).abs() < 0.01);
        assert_eq!(r.repetitions, 3);
    }

    #[test]
    fn sm2_okay_is_minimum_passing_grade() {
        // Grade::Okay (3) should advance the card just like Good or Perfect
        let r = sm2_schedule(&card(2.5, 0.0, 0), ReviewGrade::Okay);
        assert_eq!(r.repetitions, 1);
        assert_eq!(r.interval_days, INTERVAL_FIRST);
    }

    #[test]
    fn sm2_interval_floor_after_mature_card_with_low_ef() {
        // Even with EF at the floor, interval should never go below 1 day on a correct answer
        let r = sm2_schedule(&card(MIN_EASE_FACTOR, 1.0, 5), ReviewGrade::Good);
        assert!(
            r.interval_days >= 1.0,
            "interval dropped below 1 day: {}",
            r.interval_days
        );
    }

    // ── SM-2: failure grades ──────────────────────────────────────────────────

    #[test]
    fn sm2_hard_resets_reps_and_sets_half_day_interval() {
        let r = sm2_schedule(&card(2.5, 21.0, 5), ReviewGrade::Hard);
        assert_eq!(r.repetitions, 0);
        assert_eq!(r.interval_days, INTERVAL_HARD);
    }

    #[test]
    fn sm2_wrong_resets_reps_and_shortens_interval() {
        let r = sm2_schedule(&card(2.5, 21.0, 5), ReviewGrade::Wrong);
        assert_eq!(r.repetitions, 0);
        assert_eq!(r.interval_days, INTERVAL_WRONG);
    }

    #[test]
    fn sm2_blackout_gives_shortest_interval() {
        let r = sm2_schedule(&card(2.5, 21.0, 5), ReviewGrade::Blackout);
        assert_eq!(r.repetitions, 0);
        assert_eq!(r.interval_days, INTERVAL_BLACKOUT);
        assert!(r.interval_days < INTERVAL_WRONG);
    }

    #[test]
    fn sm2_failure_intervals_are_ordered() {
        let c = card(2.5, 21.0, 5);
        let blackout = sm2_schedule(&c, ReviewGrade::Blackout).interval_days;
        let wrong = sm2_schedule(&c, ReviewGrade::Wrong).interval_days;
        let hard = sm2_schedule(&c, ReviewGrade::Hard).interval_days;
        assert!(
            blackout < wrong,
            "blackout ({blackout}) should be shorter than wrong ({wrong})"
        );
        assert!(
            wrong < hard,
            "wrong ({wrong}) should be shorter than hard ({hard})"
        );
    }

    // ── SM-2: ease factor ─────────────────────────────────────────────────────

    #[test]
    fn sm2_perfect_raises_ef() {
        let r = sm2_schedule(&card(2.5, 1.0, 1), ReviewGrade::Perfect);
        assert!(
            r.ease_factor > 2.5,
            "EF should increase on Perfect: {}",
            r.ease_factor
        );
    }

    #[test]
    fn sm2_good_leaves_ef_nearly_unchanged() {
        // Grade 4 (Good): ef += 0.1 - 1*(0.08 + 1*0.02) = 0.1 - 0.10 = 0.0
        let r = sm2_schedule(&card(2.5, 1.0, 1), ReviewGrade::Good);
        assert!(
            (r.ease_factor - 2.5).abs() < 0.001,
            "EF should be ~2.5 on Good: {}",
            r.ease_factor
        );
    }

    #[test]
    fn sm2_okay_decreases_ef() {
        // Grade 3 (Okay): ef += 0.1 - 2*(0.08 + 2*0.02) = 0.1 - 0.24 = -0.14
        let r = sm2_schedule(&card(2.5, 1.0, 1), ReviewGrade::Okay);
        assert!(
            r.ease_factor < 2.5,
            "EF should decrease on Okay: {}",
            r.ease_factor
        );
        assert!((r.ease_factor - 2.36).abs() < 0.001);
    }

    #[test]
    fn sm2_ef_floor_is_enforced() {
        let mut c = card(1.4, 0.0, 0);
        for _ in 0..20 {
            let r = sm2_schedule(&c, ReviewGrade::Blackout);
            assert!(
                r.ease_factor >= MIN_EASE_FACTOR,
                "EF dipped below floor: {}",
                r.ease_factor
            );
            c.ease_factor = r.ease_factor;
        }
    }

    // ── SM-2: full progression ────────────────────────────────────────────────

    #[test]
    fn sm2_full_correct_progression() {
        let mut c = card(2.5, 0.0, 0);

        let r1 = sm2_schedule(&c, ReviewGrade::Good);
        assert_eq!(r1.interval_days, INTERVAL_FIRST);
        assert_eq!(r1.repetitions, 1);
        c.interval_days = r1.interval_days;
        c.ease_factor = r1.ease_factor;
        c.repetitions = r1.repetitions;

        let r2 = sm2_schedule(&c, ReviewGrade::Good);
        assert_eq!(r2.interval_days, INTERVAL_SECOND);
        assert_eq!(r2.repetitions, 2);
        c.interval_days = r2.interval_days;
        c.ease_factor = r2.ease_factor;
        c.repetitions = r2.repetitions;

        let r3 = sm2_schedule(&c, ReviewGrade::Good);
        assert!(
            r3.interval_days > INTERVAL_SECOND,
            "3rd correct should give > 6 days"
        );
        assert_eq!(r3.repetitions, 3);
    }

    #[test]
    fn sm2_failure_after_progress_restarts_reps() {
        // Get to rep 3, then fail — should reset to 0
        let mature = card(2.5, 15.0, 3);
        let r = sm2_schedule(&mature, ReviewGrade::Wrong);
        assert_eq!(r.repetitions, 0);
        assert!(r.interval_days < INTERVAL_FIRST);
    }

    // ── Pinyin conversion ─────────────────────────────────────────────────────

    #[test]
    fn numbered_to_toned_basic() {
        assert_eq!(numbered_to_toned("ni3 hao3"), "nǐ hǎo");
        assert_eq!(numbered_to_toned("zhong1"), "zhōng");
        assert_eq!(numbered_to_toned("ma1"), "mā");
        assert_eq!(numbered_to_toned("ma2"), "má");
        assert_eq!(numbered_to_toned("ma3"), "mǎ");
        assert_eq!(numbered_to_toned("ma4"), "mà");
    }

    #[test]
    fn numbered_to_toned_no_space_multisyllable() {
        assert_eq!(numbered_to_toned("ni3hao3"), "nǐ hǎo");
    }

    #[test]
    fn numbered_to_toned_v_becomes_u_umlaut() {
        assert_eq!(numbered_to_toned("lv4"), "lǜ");
        assert_eq!(numbered_to_toned("nv3"), "nǚ");
    }

    #[test]
    fn numbered_to_toned_neutral_tone() {
        assert_eq!(numbered_to_toned("ma5"), "ma");
    }

    #[test]
    fn numbered_to_toned_tone_placement_ou() {
        // In "dou", tone goes on 'o' (ou rule)
        assert_eq!(numbered_to_toned("dou4"), "dòu");
    }

    #[test]
    fn numbered_to_toned_all_finals_for_e() {
        assert_eq!(numbered_to_toned("he1"), "hē");
        assert_eq!(numbered_to_toned("le2"), "lé");
    }

    #[test]
    fn strip_tones_removes_diacritics() {
        assert_eq!(strip_tones("nǐ hǎo"), "ni hao");
        assert_eq!(strip_tones("zhōng"), "zhong");
        assert_eq!(strip_tones("ǖǘǚǜ"), "üüüü");
    }

    #[test]
    fn strip_tones_preserves_non_pinyin() {
        assert_eq!(strip_tones("hello123"), "hello123");
    }

    #[test]
    fn normalize_pinyin_removes_spaces_and_tones() {
        assert_eq!(normalize_pinyin("nǐ hǎo"), "nihao");
        assert_eq!(normalize_pinyin("zhōng guó"), "zhongguo");
    }

    // ── Grading: ZhToPinyin ───────────────────────────────────────────────────

    #[test]
    fn grade_zh_to_pinyin_exact() {
        let c = make_card("你好", "nǐ hǎo", "hello");
        let (score, _) = grade_answer(&CardDirection::ZhToPinyin, &c, "nǐ hǎo");
        assert_eq!(score, 1.0);
    }

    #[test]
    fn grade_zh_to_pinyin_numbered_with_space_accepted() {
        let c = make_card("你好", "nǐ hǎo", "hello");
        let (score, _) = grade_answer(&CardDirection::ZhToPinyin, &c, "ni3 hao3");
        assert_eq!(score, 1.0);
    }

    #[test]
    fn grade_zh_to_pinyin_numbered_no_space_accepted() {
        let c = make_card("你好", "nǐ hǎo", "hello");
        let (score, _) = grade_answer(&CardDirection::ZhToPinyin, &c, "ni3hao3");
        assert_eq!(score, 1.0);
    }

    #[test]
    fn grade_zh_to_pinyin_wrong_tones_partial_credit() {
        let c = make_card("你好", "nǐ hǎo", "hello");
        let (score, _) = grade_answer(&CardDirection::ZhToPinyin, &c, "ni hao");
        assert_eq!(score, 0.6);
    }

    #[test]
    fn grade_zh_to_pinyin_wrong_syllables_is_zero() {
        let c = make_card("你好", "nǐ hǎo", "hello");
        let (score, _) = grade_answer(&CardDirection::ZhToPinyin, &c, "zai jian");
        assert_eq!(score, 0.0);
    }

    // ── Grading: ZhToEn ──────────────────────────────────────────────────────

    #[test]
    fn grade_zh_to_en_exact() {
        let c = make_card("你好", "nǐ hǎo", "hello");
        let (score, _) = grade_answer(&CardDirection::ZhToEn, &c, "hello");
        assert_eq!(score, 1.0);
    }

    #[test]
    fn grade_zh_to_en_case_insensitive() {
        let c = make_card("你好", "nǐ hǎo", "Hello");
        let (score, _) = grade_answer(&CardDirection::ZhToEn, &c, "hello");
        assert_eq!(score, 1.0);
    }

    #[test]
    fn grade_zh_to_en_slash_variant_accepted() {
        let c = make_card("或者", "huò zhě", "or / either");
        let (score, _) = grade_answer(&CardDirection::ZhToEn, &c, "either");
        assert_eq!(score, 1.0);
    }

    #[test]
    fn grade_zh_to_en_semicolon_variant_accepted() {
        let c = make_card("或者", "huò zhě", "or; either");
        let (score, _) = grade_answer(&CardDirection::ZhToEn, &c, "either");
        assert_eq!(score, 1.0);
    }

    #[test]
    fn grade_zh_to_en_comma_variant_accepted() {
        let c = make_card("或者", "huò zhě", "or, either");
        let (score, _) = grade_answer(&CardDirection::ZhToEn, &c, "either");
        assert_eq!(score, 1.0);
    }

    #[test]
    fn grade_zh_to_en_punctuation_in_answer_stripped() {
        let c = make_card("你好", "nǐ hǎo", "hello");
        let (score, _) = grade_answer(&CardDirection::ZhToEn, &c, "Hello!");
        assert_eq!(score, 1.0);
    }

    #[test]
    fn grade_zh_to_en_wrong_is_zero() {
        let c = make_card("你好", "nǐ hǎo", "hello");
        let (score, _) = grade_answer(&CardDirection::ZhToEn, &c, "goodbye");
        assert_eq!(score, 0.0);
    }

    #[test]
    fn grade_zh_to_en_empty_answer_is_zero() {
        let c = make_card("你好", "nǐ hǎo", "hello");
        let (score, _) = grade_answer(&CardDirection::ZhToEn, &c, "");
        assert_eq!(score, 0.0);
    }

    // ── Grading: EnToZh / PinyinToZh ─────────────────────────────────────────

    #[test]
    fn grade_en_to_zh_exact() {
        let c = make_card("你好", "nǐ hǎo", "hello");
        let (score, _) = grade_answer(&CardDirection::EnToZh, &c, "你好");
        assert_eq!(score, 1.0);
    }

    #[test]
    fn grade_en_to_zh_wrong() {
        let c = make_card("你好", "nǐ hǎo", "hello");
        let (score, _) = grade_answer(&CardDirection::EnToZh, &c, "再见");
        assert_eq!(score, 0.0);
    }

    #[test]
    fn grade_pinyin_to_zh_exact() {
        let c = make_card("你好", "nǐ hǎo", "hello");
        let (score, _) = grade_answer(&CardDirection::PinyinToZh, &c, "你好");
        assert_eq!(score, 1.0);
    }

    #[test]
    fn grade_pinyin_to_zh_wrong() {
        let c = make_card("你好", "nǐ hǎo", "hello");
        let (score, _) = grade_answer(&CardDirection::PinyinToZh, &c, "再见");
        assert_eq!(score, 0.0);
    }
}
