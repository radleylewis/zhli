use chrono::{Utc, Duration};
use anyhow::Result;
use crate::db::{CardRow, Database};

// SM-2 scheduling parameters
const INTERVAL_FIRST:    f64 = 1.0;  // days after first correct review
const INTERVAL_SECOND:   f64 = 6.0;  // days after second correct review
const INTERVAL_HARD:     f64 = 0.5;  // ~12 hours (grade 2: wrong but easy to recall)
const INTERVAL_WRONG:    f64 = 0.1;  // ~2.5 hours (grade 1: incorrect)
const INTERVAL_BLACKOUT: f64 = 0.04; // ~1 hour (grade 0: complete blank)
const MIN_EASE_FACTOR:   f64 = 1.3;

/// SM-2 quality grades (0-5)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewGrade {
    Blackout  = 0,  // User has no idea
    Wrong     = 1,  // incorrect but remembered on seeing answer
    Hard      = 2,  // incorrect but easy to recall
    Okay      = 3,  // correct with difficulty
    Good      = 4,  // correct after hesitation
    Perfect   = 5,  // perfect recall
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
            Self::ZhToPinyin  => "zh_to_pinyin",
            Self::ZhToEn      => "zh_to_en",
            Self::EnToZh      => "en_to_zh",
            Self::PinyinToZh  => "pinyin_to_zh",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "zh_to_pinyin" => Self::ZhToPinyin,
            "zh_to_en"     => Self::ZhToEn,
            "en_to_zh"     => Self::EnToZh,
            _              => Self::PinyinToZh,
        }
    }

    pub fn prompt_label(&self) -> &'static str {
        match self {
            Self::ZhToPinyin  => "Write the Pinyin",
            Self::ZhToEn      => "Write the English meaning",
            Self::EnToZh      => "Write the Chinese (Hanzi)",
            Self::PinyinToZh  => "Write the Chinese (Hanzi)",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::ZhToPinyin  => "Chinese → Pinyin",
            Self::ZhToEn      => "Chinese → English",
            Self::EnToZh      => "English → Chinese",
            Self::PinyinToZh  => "Pinyin → Chinese",
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
    let base   = "aaaaeeeeiiiioooouuuuuuuu";
    let base_chars: Vec<char> = base.chars().collect();
    let mut out = String::new();
    for ch in pinyin.chars() {
        if let Some(idx) = toned.chars().position(|t| t == ch) {
            out.push(base_chars[idx]);
        } else if ch == 'ü' || ch == 'v' {
            // plain ü (from v→ü conversion) and bare 'v' both normalise to 'u'
            out.push('u');
        } else {
            out.push(ch.to_ascii_lowercase());
        }
    }
    out
}

pub fn normalize_pinyin(s: &str) -> String {
    strip_tones(s).split_whitespace().collect::<Vec<_>>().join("")
}

/// Convert a single syllable like "hao3" → "hǎo", "zhong1" → "zhōng".
/// Handles 'v' as ü (e.g. "lv4" → "lǜ").
fn numbered_syllable_to_toned(syl: &str) -> String {
    let s = syl.trim();
    let last = match s.chars().last() {
        Some(c) => c,
        None    => return s.to_string(),
    };
    let (base, tone): (&str, u8) = match last {
        '1' => (&s[..s.len()-1], 1),
        '2' => (&s[..s.len()-1], 2),
        '3' => (&s[..s.len()-1], 3),
        '4' => (&s[..s.len()-1], 4),
        '5' => (&s[..s.len()-1], 0),
        _   => (s, 0),
    };
    if tone == 0 { return base.replace('v', "ü"); }

    // Normalise ü written as 'v'
    let base = base.replace('v', "ü");
    let chars: Vec<char> = base.chars().collect();

    // Placement rules: a/e first, then 'o' in 'ou', then last vowel
    let vowels = ['a', 'e', 'i', 'o', 'u', 'ü'];
    let mark_pos = chars.iter().position(|&c| c == 'a' || c == 'e')
        .or_else(|| {
            chars.windows(2).position(|w| w[0] == 'o' && w[1] == 'u')
        })
        .or_else(|| chars.iter().rposition(|c| vowels.contains(c)));

    let Some(pos) = mark_pos else { return base; };

    let toned_char = match (chars[pos], tone) {
        ('a', 1) => 'ā', ('a', 2) => 'á', ('a', 3) => 'ǎ', ('a', 4) => 'à',
        ('e', 1) => 'ē', ('e', 2) => 'é', ('e', 3) => 'ě', ('e', 4) => 'è',
        ('i', 1) => 'ī', ('i', 2) => 'í', ('i', 3) => 'ǐ', ('i', 4) => 'ì',
        ('o', 1) => 'ō', ('o', 2) => 'ó', ('o', 3) => 'ǒ', ('o', 4) => 'ò',
        ('u', 1) => 'ū', ('u', 2) => 'ú', ('u', 3) => 'ǔ', ('u', 4) => 'ù',
        ('ü', 1) => 'ǖ', ('ü', 2) => 'ǘ', ('ü', 3) => 'ǚ', ('ü', 4) => 'ǜ',
        (c, _)   => c,
    };

    chars.iter().enumerate()
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
    syllables.iter().map(|syl| numbered_syllable_to_toned(syl)).collect::<Vec<_>>().join(" ")
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
            let variants: Vec<String> = card.english
                .split(['/', ',', ';', '、'])
                .map(|v| normalize_english(v.trim()))
                .filter(|v| !v.is_empty())
                .collect();

            if variants.iter().any(|v| v == &given) {
                (1.0, "✓ Correct!".to_string())
            } else if !given.is_empty() && given.len() > 2
                && variants.iter().any(|v| v.contains(given.as_str()) || given.contains(v.as_str()))
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
    db.update_card_srs(card.id, result.ease_factor, result.interval_days, result.repetitions, &result.due_at)?;
    db.record_review(card.id, grade as i32, time_ms)?;
    Ok(())
}
