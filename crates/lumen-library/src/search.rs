use crate::model::{Book, to_nfc};
use crate::parse::helpers::clean_text;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hit {
    pub book: String,
    pub unit_key: String,
    pub block_index: usize,
    pub snippet: String,
}

#[derive(Debug, Clone)]
struct IndexedUnit {
    book_id: String,
    unit_key: String,
    label: String,
    label_lower: String,
    label_abbrevs: Vec<String>,
    title_gu_lower: Option<String>,
    title_en_lower: Option<String>,
    title_display: Option<String>,
    blocks: Vec<IndexedBlock>,
}

#[derive(Debug, Clone)]
struct IndexedBlock {
    index: usize,
    gu: Option<(String, String)>,       // (original, lower)
    en: Option<(String, String)>,       // (original, lower)
    translit: Option<(String, String)>, // (original, lower)
}

#[derive(Debug, Clone)]
pub struct SearchIndex {
    units: Vec<IndexedUnit>,
}

impl SearchIndex {
    pub fn new(books: &[Book]) -> Self {
        let mut units = Vec::new();

        for book in books {
            for unit in &book.units {
                let label_nfc = to_nfc(&unit.label);
                let label_lower = label_nfc.to_lowercase();
                let label_abbrevs = generate_label_abbrevs(&label_nfc);

                let title_gu_lower = unit.title_gu.as_ref().map(|t| to_nfc(t).to_lowercase());
                let title_en_lower = unit.title_en.as_ref().map(|t| to_nfc(t).to_lowercase());
                let title_display = unit.title_en.clone().or_else(|| unit.title_gu.clone());

                let mut indexed_blocks = Vec::with_capacity(unit.blocks.len());
                for (idx, b) in unit.blocks.iter().enumerate() {
                    let gu = b.gu.as_ref().map(|s| {
                        let n = to_nfc(s);
                        let l = n.to_lowercase();
                        (n, l)
                    });
                    let en = b.en.as_ref().map(|s| {
                        let n = to_nfc(s);
                        let l = n.to_lowercase();
                        (n, l)
                    });
                    let translit = b.translit.as_ref().map(|s| {
                        let n = to_nfc(s);
                        let l = n.to_lowercase();
                        (n, l)
                    });

                    indexed_blocks.push(IndexedBlock {
                        index: idx,
                        gu,
                        en,
                        translit,
                    });
                }

                units.push(IndexedUnit {
                    book_id: book.id.clone(),
                    unit_key: unit.key.clone(),
                    label: label_nfc,
                    label_lower,
                    label_abbrevs,
                    title_gu_lower,
                    title_en_lower,
                    title_display,
                    blocks: indexed_blocks,
                });
            }
        }

        Self { units }
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<Hit> {
        let q = to_nfc(query.trim()).to_lowercase();
        if q.is_empty() || limit == 0 {
            return Vec::new();
        }

        let mut label_hits: Vec<Hit> = Vec::new();
        let mut title_hits: Vec<Hit> = Vec::new();
        let mut text_hits: Vec<Hit> = Vec::new();

        for u in &self.units {
            // 1. Label match
            let matches_label = u.label_lower.contains(&q)
                || u.label_abbrevs.iter().any(|abbrev| abbrev.contains(&q));

            if matches_label {
                label_hits.push(Hit {
                    book: u.book_id.clone(),
                    unit_key: u.unit_key.clone(),
                    block_index: 0,
                    snippet: u.label.clone(),
                });
            }

            // 2. Title match
            let matches_title = u
                .title_gu_lower
                .as_ref()
                .map(|t| t.contains(&q))
                .unwrap_or(false)
                || u.title_en_lower
                    .as_ref()
                    .map(|t| t.contains(&q))
                    .unwrap_or(false);

            if matches_title {
                title_hits.push(Hit {
                    book: u.book_id.clone(),
                    unit_key: u.unit_key.clone(),
                    block_index: 0,
                    snippet: u.title_display.clone().unwrap_or_else(|| u.label.clone()),
                });
            }

            // 3. Text match
            for b in &u.blocks {
                let hit_opt = [&b.en, &b.gu, &b.translit].into_iter().find_map(|opt| {
                    opt.as_ref().and_then(|(orig, lower)| {
                        lower.find(&q).map(|pos| Hit {
                            book: u.book_id.clone(),
                            unit_key: u.unit_key.clone(),
                            block_index: b.index,
                            snippet: make_snippet(orig, pos, q.len()),
                        })
                    })
                });

                if let Some(hit) = hit_opt {
                    text_hits.push(hit);
                }
            }
        }

        let mut results = Vec::with_capacity(limit);
        for hit in label_hits.into_iter().chain(title_hits).chain(text_hits) {
            results.push(hit);
            if results.len() >= limit {
                break;
            }
        }

        results
    }
}

pub fn search(books: &[Book], query: &str, limit: usize) -> Vec<Hit> {
    SearchIndex::new(books).search(query, limit)
}

fn generate_label_abbrevs(label: &str) -> Vec<String> {
    let lower = label.to_lowercase();
    let mut abbrevs = Vec::new();

    // e.g. "gadhada i-1" -> "ga 1-1", "gi-1", "gi 1", "g 1-1"
    if lower.contains("gadhada i-") {
        abbrevs.push(lower.replace("gadhada i-", "ga 1-"));
        abbrevs.push(lower.replace("gadhada i-", "gi-"));
        abbrevs.push(lower.replace("gadhada i-", "gi "));
        abbrevs.push(lower.replace("gadhada i-", "g 1-"));
    } else if lower.contains("gadhada ii-") {
        abbrevs.push(lower.replace("gadhada ii-", "ga 2-"));
        abbrevs.push(lower.replace("gadhada ii-", "gii-"));
        abbrevs.push(lower.replace("gadhada ii-", "gii "));
        abbrevs.push(lower.replace("gadhada ii-", "g 2-"));
    } else if lower.contains("gadhada iii-") {
        abbrevs.push(lower.replace("gadhada iii-", "ga 3-"));
        abbrevs.push(lower.replace("gadhada iii-", "giii-"));
        abbrevs.push(lower.replace("gadhada iii-", "giii "));
        abbrevs.push(lower.replace("gadhada iii-", "g 3-"));
    }

    // Replace hyphens with spaces and vice versa
    if lower.contains('-') {
        abbrevs.push(lower.replace('-', " "));
    }

    abbrevs
}

fn make_snippet(text: &str, byte_pos: usize, match_len: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut char_pos = 0;
    let mut cur_bytes = 0;
    for (i, &c) in chars.iter().enumerate() {
        if cur_bytes >= byte_pos {
            char_pos = i;
            break;
        }
        cur_bytes += c.len_utf8();
    }

    let match_char_len = text[byte_pos..byte_pos + match_len].chars().count();
    let start = char_pos.saturating_sub(30);
    let end = (char_pos + match_char_len + 30).min(chars.len());

    let mut snippet = String::new();
    if start > 0 {
        snippet.push_str("...");
    }
    for c in &chars[start..end] {
        snippet.push(*c);
    }
    if end < chars.len() {
        snippet.push_str("...");
    }

    clean_text(&snippet, false)
}
