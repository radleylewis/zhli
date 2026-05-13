use anyhow::Result;
use scraper::{Html, Selector};
use std::sync::OnceLock;

static ROW_SEL: OnceLock<Selector> = OnceLock::new();
static HANZI_A_SEL: OnceLock<Selector> = OnceLock::new();
static PINYIN_A_SEL: OnceLock<Selector> = OnceLock::new();
static MPT_SEL: OnceLock<Selector> = OnceLock::new();
static DEFS_SEL: OnceLock<Selector> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct DictEntry {
    pub hanzi: String,
    pub pinyin: String,
    pub english: String,
}

/// MDBG returns 10 results per page; fetch up to this many pages.
const MAX_PAGES: usize = 3;

/// Look up a word on MDBG (CC-CEDICT based). Query can be hanzi, pinyin, or English.
/// Fetches up to MAX_PAGES pages and combines results.
pub fn lookup(query: &str) -> Result<Vec<DictEntry>> {
    let mut all_results: Vec<DictEntry> = Vec::new();

    // Use "contains" match for ASCII queries (English/pinyin), "begins with" for Chinese
    let wdqm = if query.chars().any(|c| c as u32 > 0x7F) {
        "1"
    } else {
        "3"
    };

    for page in 0..MAX_PAGES {
        let body = ureq::get("https://www.mdbg.net/chinese/dictionary")
            .query("page", "worddict")
            .query("wdrst", &(page * 10).to_string())
            .query("wdqb", query)
            .query("wdqm", wdqm)
            .call()?
            .into_string()?;

        let page_results = parse_page(&body)?;
        let was_full = page_results.len() >= 10;

        for entry in page_results {
            // Deduplicate across pages by (hanzi, pinyin)
            if !all_results
                .iter()
                .any(|e| e.hanzi == entry.hanzi && e.pinyin == entry.pinyin)
            {
                all_results.push(entry);
            }
        }

        if !was_full {
            break; // Last page was partial — no more results
        }
    }

    sort_by_relevance(&mut all_results, query);
    Ok(all_results)
}

/// Filter already-fetched results by a substring (hanzi, pinyin, or English).
pub fn filter_results<'a>(results: &'a [DictEntry], filter: &str) -> Vec<&'a DictEntry> {
    if filter.is_empty() {
        return results.iter().collect();
    }
    let f = filter.to_lowercase();
    results
        .iter()
        .filter(|e| {
            e.hanzi.contains(filter)
                || e.pinyin.to_lowercase().contains(&f)
                || e.english.to_lowercase().contains(&f)
        })
        .collect()
}

fn parse_page(html: &str) -> Result<Vec<DictEntry>> {
    let doc = Html::parse_document(html);

    // mpt1–mpt4 are tone classes; neutral tone may be mpt0/mpt5. Match any mpt* class.
    // We only want spans inside the FIRST <a> of each div (simplified form).
    let row_sel = ROW_SEL
        .get_or_init(|| Selector::parse("table.wordresults tr.row").expect("valid selector"));
    let hanzi_a_sel =
        HANZI_A_SEL.get_or_init(|| Selector::parse("div.hanzi a").expect("valid selector"));
    let pinyin_a_sel =
        PINYIN_A_SEL.get_or_init(|| Selector::parse("div.pinyin a").expect("valid selector"));
    let mpt_sel =
        MPT_SEL.get_or_init(|| Selector::parse("span[class^='mpt']").expect("valid selector"));
    let defs_sel = DEFS_SEL.get_or_init(|| Selector::parse("div.defs").expect("valid selector"));

    let mut results = Vec::new();

    for row in doc.select(row_sel) {
        // Take only the FIRST <a> inside div.hanzi (simplified form, not traditional)
        let hanzi: String = row
            .select(hanzi_a_sel)
            .next()
            .map(|a| {
                a.select(mpt_sel)
                    .map(|s| s.text().collect::<String>())
                    .collect()
            })
            .unwrap_or_default();

        // Same for pinyin — first <a> gives simplified form's reading
        let pinyin: String = row
            .select(pinyin_a_sel)
            .next()
            .map(|a| {
                a.select(mpt_sel)
                    .map(|s| s.text().collect::<String>())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_default();

        let english: String = row
            .select(defs_sel)
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        if !hanzi.is_empty() && !pinyin.is_empty() {
            results.push(DictEntry {
                hanzi,
                pinyin,
                english,
            });
        }
    }

    Ok(results)
}

/// Sort entries so the most relevant result for `query` appears first.
/// Priority (descending):
///   1. English is an exact case-insensitive match
///   2. English starts with the query (case-insensitive)
///   3. Shorter hanzi (more specific / less compound)
fn sort_by_relevance(results: &mut [DictEntry], query: &str) {
    let q = query.to_lowercase();
    results.sort_by_key(|e| {
        let eng = e.english.to_lowercase();
        let exact = usize::from(eng != q);
        let starts = usize::from(!eng.starts_with(&q));
        // Prefer shorter English definitions — primary/standalone entries ("Australia")
        // sort before longer contextual ones ("Australia (slang for …)")
        let eng_len = e.english.len();
        (exact, starts, eng_len)
    });
}
