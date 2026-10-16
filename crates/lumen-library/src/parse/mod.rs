pub mod helpers;

use crate::error::LibraryError;
use crate::model::{Block, BlockKind, Unit, UnitRef, to_nfc};
use helpers::{
    clean_text, element_text_ignoring, element_text_preserving_lines, select_all, select_first,
    select_first_within, select_within,
};
use scraper::{ElementRef, Html};
use std::collections::HashMap;

fn gu_to_ascii_digits(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '૦' => '0',
            '૧' => '1',
            '૨' => '2',
            '૩' => '3',
            '૪' => '4',
            '૫' => '5',
            '૬' => '6',
            '૭' => '7',
            '૮' => '8',
            '૯' => '9',
            _ => c,
        })
        .collect()
}

// ============================================================================
// 1. Vachanamrut
// ============================================================================

/// Parse a single Vachanamrut page (either Gujarati format=gu or English format=en).
pub fn parse_vachanamrut_page(
    html_str: &str,
    lang: &str,
    unit: &UnitRef,
) -> Result<Unit, LibraryError> {
    let html = Html::parse_document(html_str);

    let vach_text = select_first(&html, "div#vach_text").ok_or_else(|| LibraryError::Layout {
        book: unit.book.clone(),
        unit: unit.key.clone(),
        detail: "missing div#vach_text container".into(),
    })?;

    let label_elem = select_first_within(&vach_text, "h3.pra_secno");
    let label = label_elem
        .map(|e| clean_text(&e.text().collect::<String>(), false))
        .unwrap_or_else(|| unit.key.clone());

    let is_gu = lang == "gu";
    let title_sel = if is_gu { "h1.title_gu" } else { "h1.title_en" };
    let title_elem = select_first_within(&vach_text, title_sel);
    let title = title_elem.map(|e| element_text_ignoring(&e, &["sup", "span"]));

    // Footnotes extraction from #tab-footnotes
    let mut footnotes: HashMap<String, String> = HashMap::new();
    let fn_items = select_all(&html, "#tab-footnotes .footnote-item p");
    for item in fn_items {
        let fn_id = item
            .value()
            .attr("id")
            .unwrap_or_default()
            .replace("fn_", "");
        let fn_text = clean_text(&item.text().collect::<String>(), false);
        if !fn_id.is_empty() && !fn_text.is_empty() {
            let ascii_id = gu_to_ascii_digits(&fn_id);
            footnotes.insert(fn_id, fn_text.clone());
            footnotes.insert(ascii_id, fn_text);
        }
    }

    let mut blocks = Vec::new();

    // Check for opening cite verse
    let cite_sel = if is_gu {
        "cite.text_gu"
    } else {
        "cite.text_en"
    };
    for cite in select_within(&vach_text, cite_sel) {
        let text = element_text_preserving_lines(&cite, &["sup", "audio"]);
        if !text.is_empty() {
            blocks.push(Block {
                gu: if is_gu { Some(text.clone()) } else { None },
                en: if !is_gu { Some(text) } else { None },
                translit: None,
                notes: None,
                kind: BlockKind::Verse,
            });
        }
    }

    // Text paragraphs
    let p_sel = if is_gu { "p.text_gu" } else { "p.text_en" };
    let p_elems = select_within(&vach_text, p_sel);
    for p in p_elems {
        // Collect referenced footnote numbers
        let sups = select_within(&p, "sup");
        let mut note_parts = Vec::new();
        for sup in sups {
            let num = clean_text(&sup.text().collect::<String>(), false);
            let ascii_num = gu_to_ascii_digits(&num);
            if let Some(note_text) = footnotes.get(&num).or_else(|| footnotes.get(&ascii_num)) {
                note_parts.push(note_text.clone());
            }
        }
        let notes = if note_parts.is_empty() {
            None
        } else {
            Some(note_parts.join("; "))
        };

        let text = element_text_ignoring(&p, &["sup", "audio", "script"]);
        if !text.is_empty() {
            blocks.push(Block {
                gu: if is_gu { Some(text.clone()) } else { None },
                en: if !is_gu { Some(text) } else { None },
                translit: None,
                notes,
                kind: BlockKind::Paragraph,
            });
        }
    }

    if blocks.is_empty() {
        return Err(LibraryError::Layout {
            book: unit.book.clone(),
            unit: unit.key.clone(),
            detail: format!("no blocks found using selector {p_sel}"),
        });
    }

    Ok(Unit {
        key: to_nfc(&unit.key),
        label: to_nfc(&label),
        title_gu: if is_gu {
            title.as_ref().map(|t| to_nfc(t))
        } else {
            None
        },
        title_en: if !is_gu {
            title.as_ref().map(|t| to_nfc(t))
        } else {
            None
        },
        blocks,
    })
}

/// Merge Gujarati and English Vachanamrut units paragraph-by-paragraph by position.
/// If counts differ, keeps both sequences without guessing alignment (gu-only blocks then en-only blocks)
/// and records a warning string.
pub fn merge_vachanamrut_units(gu: Unit, en: Unit) -> (Unit, Option<String>) {
    let key = gu.key;
    let label = if !en.label.is_empty() && en.label != key {
        en.label
    } else {
        gu.label
    };
    let title_gu = gu.title_gu;
    let title_en = en.title_en;

    if gu.blocks.len() == en.blocks.len() {
        let mut merged = Vec::with_capacity(gu.blocks.len());
        for (g, e) in gu.blocks.into_iter().zip(en.blocks) {
            let notes = match (g.notes, e.notes) {
                (Some(n1), Some(n2)) => Some(format!("{n1}\n{n2}")),
                (Some(n1), None) => Some(n1),
                (None, Some(n2)) => Some(n2),
                (None, None) => None,
            };
            merged.push(Block {
                gu: g.gu,
                en: e.en,
                translit: None,
                notes,
                kind: g.kind,
            });
        }
        (
            Unit {
                key,
                label,
                title_gu,
                title_en,
                blocks: merged,
            },
            None,
        )
    } else {
        let warning = format!(
            "Vachanamrut unit '{key}' paragraph count mismatch: gu has {}, en has {}",
            gu.blocks.len(),
            en.blocks.len()
        );
        let mut combined = Vec::with_capacity(gu.blocks.len() + en.blocks.len());
        combined.extend(gu.blocks);
        combined.extend(en.blocks);
        (
            Unit {
                key,
                label,
                title_gu,
                title_en,
                blocks: combined,
            },
            Some(warning),
        )
    }
}

/// Parse both Gujarati and English Vachanamrut pages and merge them.
pub fn parse_vachanamrut(
    gu_html: &str,
    en_html: &str,
    unit: &UnitRef,
) -> Result<Unit, LibraryError> {
    let gu_unit = parse_vachanamrut_page(gu_html, "gu", unit)?;
    let en_unit = parse_vachanamrut_page(en_html, "en", unit)?;
    let (merged, _warning) = merge_vachanamrut_units(gu_unit, en_unit);
    Ok(merged)
}

// ============================================================================
// 2. Swamini Vato
// ============================================================================

pub fn parse_vato(html_str: &str, unit: &UnitRef) -> Result<Unit, LibraryError> {
    let html = Html::parse_document(html_str);

    let cards = select_all(&html, "div.vat_card");
    if cards.is_empty() {
        return Err(LibraryError::Layout {
            book: unit.book.clone(),
            unit: unit.key.clone(),
            detail: "missing div.vat_card containers".into(),
        });
    }

    // Try to find matching card by unit.key or use the first card
    let matching_card = cards
        .iter()
        .find(|card| {
            if unit.key.is_empty() {
                return true;
            }
            let info_en = select_first_within(card, "div.vatinfo_en")
                .map(|e| clean_text(&e.text().collect::<String>(), false))
                .unwrap_or_default();
            let info_gu = select_first_within(card, "div.vatinfo_gu")
                .map(|e| clean_text(&e.text().collect::<String>(), false))
                .unwrap_or_default();
            let id_gu = select_first_within(card, "div.vat_wrap[id^=\"vat_gu_\"]")
                .and_then(|e| e.value().attr("id"))
                .unwrap_or_default();

            info_en.contains(&unit.key)
                || info_gu.contains(&unit.key)
                || id_gu.ends_with(&format!("_{}", unit.key))
                || id_gu == format!("vat_gu_{}", unit.key)
        })
        .or_else(|| cards.first())
        .ok_or_else(|| LibraryError::Layout {
            book: unit.book.clone(),
            unit: unit.key.clone(),
            detail: "no matching vat_card found".into(),
        })?;

    let label = select_first_within(matching_card, "div.vatinfo_en b")
        .or_else(|| select_first_within(matching_card, "div.vatinfo_gu b"))
        .or_else(|| select_first_within(matching_card, "div.vatinfo_en"))
        .or_else(|| select_first_within(matching_card, "div.vatinfo_gu"))
        .map(|e| clean_text(&e.text().collect::<String>(), false))
        .unwrap_or_else(|| unit.key.clone());

    let gu_paragraphs = select_all_within_wrap(
        matching_card,
        "div.vat_wrap[id^=\"vat_gu_\"] div.vat p.text_gu",
    );
    let en_paragraphs = select_all_within_wrap(
        matching_card,
        "div.vat_wrap[id^=\"vat_en_\"] div.vat p.text_en",
    );
    let trans_paragraphs = select_all_within_wrap(
        matching_card,
        "div.vat_wrap[id^=\"vat_trans_\"] div.vat p.text_trans",
    );

    let notes = select_first_within(matching_card, "div.footnote p.text_en")
        .map(|e| clean_text(&e.text().collect::<String>(), false));

    let max_len = gu_paragraphs
        .len()
        .max(en_paragraphs.len())
        .max(trans_paragraphs.len());
    if max_len == 0 {
        return Err(LibraryError::Layout {
            book: unit.book.clone(),
            unit: unit.key.clone(),
            detail: "no paragraphs found in vat_card".into(),
        });
    }

    let mut blocks = Vec::new();
    for i in 0..max_len {
        let gu = gu_paragraphs.get(i).cloned();
        let en = en_paragraphs.get(i).cloned();
        let translit = trans_paragraphs.get(i).cloned();
        blocks.push(Block {
            gu,
            en,
            translit,
            notes: if i == max_len - 1 {
                notes.clone()
            } else {
                None
            },
            kind: BlockKind::Paragraph,
        });
    }

    Ok(Unit {
        key: to_nfc(&unit.key),
        label: to_nfc(&label),
        title_gu: None,
        title_en: None,
        blocks,
    })
}

fn select_all_within_wrap(parent: &ElementRef, selector_str: &str) -> Vec<String> {
    let mut out = Vec::new();
    for p in select_within(parent, selector_str) {
        let text = element_text_ignoring(&p, &["sup", "audio", "script"]);
        if !text.is_empty() {
            out.push(to_nfc(&text));
        }
    }
    out
}

// ============================================================================
// 3. Kirtan Muktavali
// ============================================================================

pub fn parse_kirtan(html_str: &str, unit: &UnitRef) -> Result<Unit, LibraryError> {
    let html = Html::parse_document(html_str);

    let title_gu = select_first(&html, "h1#title_gu")
        .map(|e| clean_text(&e.text().collect::<String>(), false));
    let title_en = select_first(&html, "h1#title_en")
        .map(|e| clean_text(&e.text().collect::<String>(), false));

    let gu_line_elems = select_all(&html, "div#text_gu p.k_line_gu");
    let en_line_elems = select_all(&html, "div#text_en p.k_line_en");

    if gu_line_elems.is_empty() && en_line_elems.is_empty() {
        return Err(LibraryError::Layout {
            book: unit.book.clone(),
            unit: unit.key.clone(),
            detail: "missing div#text_gu or p.k_line_gu elements".into(),
        });
    }

    let gu_lines: Vec<String> = gu_line_elems
        .iter()
        .map(|e| clean_text(&element_text_ignoring(e, &["sup", "audio"]), false))
        .filter(|s| !s.is_empty())
        .collect();

    let en_lines: Vec<String> = en_line_elems
        .iter()
        .map(|e| clean_text(&element_text_ignoring(e, &["sup", "audio"]), false))
        .filter(|s| !s.is_empty())
        .collect();

    // Group Gujarati lines into stanzas (couplets or separated by refrains)
    let mut gu_stanzas: Vec<String> = Vec::new();
    let mut current_gu = Vec::new();
    for line in &gu_lines {
        current_gu.push(line.clone());
        let trimmed = line.trim();
        // A refrain or numbered marker marks the end of a stanza
        if trimmed.contains('꠶') || trimmed.ends_with("ટેક") || current_gu.len() >= 2 {
            gu_stanzas.push(current_gu.join("\n"));
            current_gu.clear();
        }
    }
    if !current_gu.is_empty() {
        gu_stanzas.push(current_gu.join("\n"));
    }

    // Group English lines similarly
    let mut en_stanzas: Vec<String> = Vec::new();
    let mut current_en = Vec::new();
    for line in &en_lines {
        current_en.push(line.clone());
        let trimmed = line.trim();
        if trimmed.ends_with("tek")
            || trimmed.ends_with("...")
            || current_en.len() >= 2
            || (!current_en.is_empty()
                && (trimmed.ends_with('1') || trimmed.ends_with('2') || trimmed.ends_with('3')))
        {
            en_stanzas.push(current_en.join("\n"));
            current_en.clear();
        }
    }
    if !current_en.is_empty() {
        en_stanzas.push(current_en.join("\n"));
    }

    let max_stanzas = gu_stanzas.len().max(en_stanzas.len());
    let mut blocks = Vec::with_capacity(max_stanzas);

    for i in 0..max_stanzas {
        let gu = gu_stanzas.get(i).cloned();
        let en = en_stanzas.get(i).cloned();
        let is_refrain = i == 0
            && gu
                .as_ref()
                .map(|g| g.contains("ટેક") || g.contains('꠶'))
                .unwrap_or(false);
        blocks.push(Block {
            gu,
            en,
            translit: None,
            notes: None,
            kind: if is_refrain {
                BlockKind::Refrain
            } else {
                BlockKind::Stanza
            },
        });
    }

    let label = if !unit.key.is_empty() {
        unit.key.clone()
    } else {
        title_en.clone().unwrap_or_else(|| "Kirtan".into())
    };

    Ok(Unit {
        key: to_nfc(&unit.key),
        label: to_nfc(&label),
        title_gu: title_gu.map(|t| to_nfc(&t)),
        title_en: title_en.map(|t| to_nfc(&t)),
        blocks,
    })
}

// ============================================================================
// 4. Satsang Diksha
// ============================================================================

pub fn parse_diksha(html_str: &str, unit: &UnitRef) -> Result<Unit, LibraryError> {
    let html = Html::parse_document(html_str);

    let wraps = select_all(&html, "div.shloks_wrap");
    if wraps.is_empty() {
        return Err(LibraryError::Layout {
            book: unit.book.clone(),
            unit: unit.key.clone(),
            detail: "missing div.shloks_wrap containers".into(),
        });
    }

    let matching = wraps
        .iter()
        .find(|w| {
            if unit.key.is_empty() {
                return true;
            }
            let num = select_first_within(w, "div.shlok_num")
                .map(|e| clean_text(&e.text().collect::<String>(), false))
                .unwrap_or_default();
            let id = w.value().attr("id").unwrap_or_default();
            num == unit.key
                || id.ends_with(&format!("_{}", unit.key))
                || id == format!("shlok_wrap_{}", unit.key)
        })
        .or_else(|| wraps.first())
        .ok_or_else(|| LibraryError::Layout {
            book: unit.book.clone(),
            unit: unit.key.clone(),
            detail: "no matching shlok found".into(),
        })?;

    let shlok_num = select_first_within(matching, "div.shlok_num")
        .map(|e| clean_text(&e.text().collect::<String>(), false))
        .unwrap_or_else(|| unit.key.clone());

    let shlok_hi = select_first_within(matching, "div.sanskrit_html p.shlok_hi")
        .or_else(|| select_first_within(matching, "p.shlok_hi"))
        .map(|e| clean_text(&e.text().collect::<String>(), false));

    let shlok_gu = select_first_within(matching, "div.gujarati_sans_html p.shlok_gu")
        .or_else(|| select_first_within(matching, "p.shlok_gu"))
        .map(|e| clean_text(&e.text().collect::<String>(), false));

    let shlok_trans = select_first_within(matching, "div.sanskrit_trans_html p.shlok_trans")
        .or_else(|| select_first_within(matching, "p.shlok_trans"))
        .map(|e| clean_text(&e.text().collect::<String>(), false));

    let meaning_gu = select_first_within(matching, "div.gujarati_html p.meaning_gu")
        .or_else(|| select_first_within(matching, "p.meaning_gu"))
        .map(|e| clean_text(&e.text().collect::<String>(), false));

    let meaning_en = select_first_within(matching, "div.english_html p.meaning_en")
        .or_else(|| select_first_within(matching, "p.meaning_en"))
        .map(|e| clean_text(&e.text().collect::<String>(), false));

    let mut blocks = Vec::new();

    // 1. Shlok block
    if shlok_gu.is_some() || shlok_hi.is_some() || shlok_trans.is_some() {
        blocks.push(Block {
            gu: shlok_gu,
            en: None,
            translit: shlok_trans.or(shlok_hi),
            notes: None,
            kind: BlockKind::Shlok,
        });
    }

    // 2. Meaning block
    if meaning_gu.is_some() || meaning_en.is_some() {
        blocks.push(Block {
            gu: meaning_gu,
            en: meaning_en,
            translit: None,
            notes: None,
            kind: BlockKind::Paragraph,
        });
    }

    if blocks.is_empty() {
        return Err(LibraryError::Layout {
            book: unit.book.clone(),
            unit: unit.key.clone(),
            detail: "no shlok or meaning blocks extracted".into(),
        });
    }

    Ok(Unit {
        key: to_nfc(&unit.key),
        label: to_nfc(&format!("Shlok {shlok_num}")),
        title_gu: None,
        title_en: None,
        blocks,
    })
}

// ============================================================================
// 5. Nishkulanand Kavya
// ============================================================================

pub fn parse_kavya(html_str: &str, unit: &UnitRef) -> Result<Unit, LibraryError> {
    let html = Html::parse_document(html_str);

    let title_elem = select_first(&html, "h1.title");
    let title = title_elem.map(|e| clean_text(&e.text().collect::<String>(), false));

    let section_elem = select_first(&html, "h2.section");
    let section = section_elem.map(|e| clean_text(&e.text().collect::<String>(), false));

    let verse_elems = select_all(&html, "p.line.kavya-verse, p.line_uni");
    let trans_elems = select_all(&html, "p.line_en, p.line_trans");
    let meaning_elems = select_all(&html, "p.meaning_en");

    if verse_elems.is_empty() && trans_elems.is_empty() {
        return Err(LibraryError::Layout {
            book: unit.book.clone(),
            unit: unit.key.clone(),
            detail: "no kavya verse lines found".into(),
        });
    }

    let mut blocks = Vec::new();
    let max_len = verse_elems.len().max(trans_elems.len());

    for i in 0..max_len {
        let gu = verse_elems
            .get(i)
            .map(|e| clean_text(&element_text_ignoring(e, &["sup", "audio"]), false))
            .filter(|s| !s.is_empty());
        let translit = trans_elems
            .get(i)
            .map(|e| clean_text(&element_text_ignoring(e, &["sup", "audio"]), false))
            .filter(|s| !s.is_empty());
        let en = meaning_elems
            .get(i)
            .map(|e| clean_text(&element_text_ignoring(e, &["sup", "audio"]), false))
            .filter(|s| !s.is_empty());

        if gu.is_some() || translit.is_some() || en.is_some() {
            blocks.push(Block {
                gu,
                en,
                translit,
                notes: None,
                kind: BlockKind::Verse,
            });
        }
    }

    Ok(Unit {
        key: to_nfc(&unit.key),
        label: to_nfc(&section.unwrap_or_else(|| unit.key.clone())),
        title_gu: title.as_ref().map(|t| to_nfc(t)),
        title_en: None,
        blocks,
    })
}

// ============================================================================
// 6. Bhaktachintamani
// ============================================================================

pub fn parse_chintamani(html_str: &str, unit: &UnitRef) -> Result<Unit, LibraryError> {
    let html = Html::parse_document(html_str);

    let title_gu = select_first(&html, "h1.title_uni")
        .map(|e| clean_text(&e.text().collect::<String>(), false));
    let title_en = select_first(&html, "h1.title_trans")
        .map(|e| clean_text(&e.text().collect::<String>(), false));

    let gu_elems = select_all(&html, "p.line_uni");
    let trans_elems = select_all(&html, "p.line_trans");

    // Fallback: extract from initialCache if DOM is empty
    if gu_elems.is_empty() && trans_elems.is_empty() {
        if let Some(cache_unit) = parse_chintamani_initial_cache(html_str, unit) {
            return Ok(cache_unit);
        }
        return Err(LibraryError::Layout {
            book: unit.book.clone(),
            unit: unit.key.clone(),
            detail: "missing line_uni elements or initialCache".into(),
        });
    }

    let mut blocks = Vec::new();
    let max_len = gu_elems.len().max(trans_elems.len());

    for i in 0..max_len {
        let gu = gu_elems
            .get(i)
            .map(|e| clean_text(&element_text_ignoring(e, &["sup", "audio"]), false))
            .filter(|s| !s.is_empty());
        let translit = trans_elems
            .get(i)
            .map(|e| clean_text(&element_text_ignoring(e, &["sup", "audio"]), false))
            .filter(|s| !s.is_empty());

        if gu.is_some() || translit.is_some() {
            blocks.push(Block {
                gu,
                en: None,
                translit,
                notes: None,
                kind: BlockKind::Verse,
            });
        }
    }

    Ok(Unit {
        key: to_nfc(&unit.key),
        label: to_nfc(&format!("Prakaran {}", unit.key)),
        title_gu: title_gu.map(|t| to_nfc(&t)),
        title_en: title_en.map(|t| to_nfc(&t)),
        blocks,
    })
}

fn parse_chintamani_initial_cache(html: &str, unit: &UnitRef) -> Option<Unit> {
    let marker = "var initialCache = \"";
    let start = html.find(marker)? + marker.len();
    let rest = &html[start..];
    let end = rest.find("\";")?;
    let escaped_json = &rest[..end];

    let unescaped = escaped_json
        .replace("\\\"", "\"")
        .replace("\\\\", "\\")
        .replace("\\/", "/")
        .replace("\\n", "\n");

    let val: serde_json::Value = serde_json::from_str(&unescaped).ok()?;
    let title_gu = val["title_gu"].as_str().map(to_nfc);
    let title_trans = val["title_trans"].as_str().map(to_nfc);

    let gu_html_str = val["gujarati"].as_str().unwrap_or_default();
    let gu_doc = Html::parse_fragment(gu_html_str);
    let gu_lines: Vec<String> = select_all(&gu_doc, "p.line_uni")
        .into_iter()
        .map(|e| clean_text(&e.text().collect::<String>(), false))
        .filter(|s| !s.is_empty())
        .collect();

    let trans_html_str = val["trans"].as_str().unwrap_or_default();
    let trans_doc = Html::parse_fragment(trans_html_str);
    let trans_lines: Vec<String> = select_all(&trans_doc, "p.line_trans")
        .into_iter()
        .map(|e| clean_text(&e.text().collect::<String>(), false))
        .filter(|s| !s.is_empty())
        .collect();

    let max_len = gu_lines.len().max(trans_lines.len());
    let mut blocks = Vec::with_capacity(max_len);
    for i in 0..max_len {
        blocks.push(Block {
            gu: gu_lines.get(i).cloned(),
            en: None,
            translit: trans_lines.get(i).cloned(),
            notes: None,
            kind: BlockKind::Verse,
        });
    }

    Some(Unit {
        key: to_nfc(&unit.key),
        label: to_nfc(&format!("Prakaran {}", unit.key)),
        title_gu,
        title_en: title_trans,
        blocks,
    })
}

// ============================================================================
// 7. Harililamrut
// ============================================================================

pub fn parse_harililamrut(html_str: &str, unit: &UnitRef) -> Result<Unit, LibraryError> {
    let html = Html::parse_document(html_str);

    let title = select_first(&html, "h1.title_gu, h2.toc_sub")
        .map(|e| clean_text(&e.text().collect::<String>(), false));

    let mut verse_elems = select_all(&html, "p.line.hla-verse");
    if verse_elems.is_empty() {
        verse_elems = select_all(&html, "span.hla-verse-text");
    }
    if verse_elems.is_empty() {
        return Err(LibraryError::Layout {
            book: unit.book.clone(),
            unit: unit.key.clone(),
            detail: "no harililamrut verse elements found".into(),
        });
    }

    let mut blocks = Vec::new();
    for elem in verse_elems {
        let text = clean_text(&element_text_ignoring(&elem, &["sup", "audio"]), false);
        if !text.is_empty() {
            blocks.push(Block {
                gu: Some(to_nfc(&text)),
                en: None,
                translit: None,
                notes: None,
                kind: BlockKind::Verse,
            });
        }
    }

    Ok(Unit {
        key: to_nfc(&unit.key),
        label: to_nfc(&title.clone().unwrap_or_else(|| unit.key.clone())),
        title_gu: title.map(|t| to_nfc(&t)),
        title_en: None,
        blocks,
    })
}

// ============================================================================
// 8. Yogi Gita
// ============================================================================

pub fn parse_yogi_gita(html_str: &str, unit: &UnitRef) -> Result<Unit, LibraryError> {
    let html = Html::parse_document(html_str);

    let sec_title = select_first(&html, "h1.sec_title")
        .map(|e| clean_text(&e.text().collect::<String>(), false));

    let gu_elems = select_all(&html, "div.gu_text p, div.gu_text li.item, div.gu_text li");
    let en_elems = select_all(&html, "div.en_text p, div.en_text li.item, div.en_text li");

    if gu_elems.is_empty() && en_elems.is_empty() {
        return Err(LibraryError::Layout {
            book: unit.book.clone(),
            unit: unit.key.clone(),
            detail: "no gu_text or en_text elements found".into(),
        });
    }

    let mut blocks = Vec::new();
    let max_len = gu_elems.len().max(en_elems.len());

    for i in 0..max_len {
        let gu = gu_elems
            .get(i)
            .map(|e| clean_text(&element_text_ignoring(e, &["sup", "audio"]), false))
            .filter(|s| !s.is_empty());
        let en = en_elems
            .get(i)
            .map(|e| clean_text(&element_text_ignoring(e, &["sup", "audio"]), false))
            .filter(|s| !s.is_empty());

        if gu.is_some() || en.is_some() {
            blocks.push(Block {
                gu,
                en,
                translit: None,
                notes: None,
                kind: BlockKind::Paragraph,
            });
        }
    }

    Ok(Unit {
        key: to_nfc(&unit.key),
        label: to_nfc(&sec_title.clone().unwrap_or_else(|| unit.key.clone())),
        title_gu: sec_title.map(|t| to_nfc(&t)),
        title_en: None,
        blocks,
    })
}

// ============================================================================
// 9. Swaminarayan Siddhant Karika
// ============================================================================

pub fn parse_karika(html_str: &str, unit: &UnitRef) -> Result<Unit, LibraryError> {
    let html = Html::parse_document(html_str);

    let title =
        select_first(&html, "p.hi_title").map(|e| clean_text(&e.text().collect::<String>(), false));

    let verse_elems = select_all(&html, "div#text_body p.hi_center, p.hi_center");
    if verse_elems.is_empty() {
        return Err(LibraryError::Layout {
            book: unit.book.clone(),
            unit: unit.key.clone(),
            detail: "no hi_center verse elements found".into(),
        });
    }

    let mut blocks = Vec::new();
    for elem in verse_elems {
        let text = clean_text(&element_text_ignoring(&elem, &["sup", "audio"]), false);
        if !text.is_empty() {
            blocks.push(Block {
                gu: None,
                en: None,
                translit: Some(to_nfc(&text)),
                notes: None,
                kind: BlockKind::Shlok,
            });
        }
    }

    Ok(Unit {
        key: to_nfc(&unit.key),
        label: to_nfc(&title.clone().unwrap_or_else(|| unit.key.clone())),
        title_gu: None,
        title_en: title.map(|t| to_nfc(&t)),
        blocks,
    })
}

// ============================================================================
// 10. Aksharamrutam
// ============================================================================

pub fn parse_aksharamrutam(html_str: &str, unit: &UnitRef) -> Result<Unit, LibraryError> {
    let html = Html::parse_document(html_str);

    let vat_elems = select_all(&html, "p.vat_text_gu");
    if vat_elems.is_empty() {
        return Err(LibraryError::Layout {
            book: unit.book.clone(),
            unit: unit.key.clone(),
            detail: "missing p.vat_text_gu elements".into(),
        });
    }

    // Try to find matching vat by unit.key or use the first
    let matching = vat_elems
        .iter()
        .find(|e| {
            if unit.key.is_empty() {
                return true;
            }
            let id = e.value().attr("id").unwrap_or_default();
            id.ends_with(&format!("vat_{}", unit.key)) || id.contains(&unit.key)
        })
        .or_else(|| vat_elems.first())
        .ok_or_else(|| LibraryError::Layout {
            book: unit.book.clone(),
            unit: unit.key.clone(),
            detail: "no matching aksharamrutam vat found".into(),
        })?;

    let text = clean_text(
        &element_text_ignoring(matching, &["sup", "b", "audio"]),
        false,
    );
    if text.is_empty() {
        return Err(LibraryError::Layout {
            book: unit.book.clone(),
            unit: unit.key.clone(),
            detail: "empty vat_text_gu paragraph".into(),
        });
    }

    let block = Block {
        gu: Some(to_nfc(&text)),
        en: None,
        translit: None,
        notes: None,
        kind: BlockKind::Paragraph,
    };

    Ok(Unit {
        key: to_nfc(&unit.key),
        label: to_nfc(&format!("Vat {}", unit.key)),
        title_gu: None,
        title_en: None,
        blocks: vec![block],
    })
}

// ============================================================================
// 11. Purushottam Bolya Prite
// ============================================================================

pub fn parse_pbp(html_str: &str, unit: &UnitRef) -> Result<Unit, LibraryError> {
    let html = Html::parse_document(html_str);

    let title = select_first(&html, "h1.chp_title")
        .map(|e| clean_text(&e.text().collect::<String>(), false));

    let p_elems = select_all(&html, "p.gu_indent, p.gu_center");
    if p_elems.is_empty() {
        return Err(LibraryError::Layout {
            book: unit.book.clone(),
            unit: unit.key.clone(),
            detail: "missing p.gu_indent elements".into(),
        });
    }

    let mut blocks = Vec::new();
    for p in p_elems {
        let text = clean_text(&element_text_ignoring(&p, &["sup", "audio"]), false);
        if !text.is_empty() {
            blocks.push(Block {
                gu: Some(to_nfc(&text)),
                en: None,
                translit: None,
                notes: None,
                kind: BlockKind::Paragraph,
            });
        }
    }

    Ok(Unit {
        key: to_nfc(&unit.key),
        label: to_nfc(&title.clone().unwrap_or_else(|| unit.key.clone())),
        title_gu: title.map(|t| to_nfc(&t)),
        title_en: None,
        blocks,
    })
}

// ============================================================================
// 12. Sadguru Aksharanand Swamini Vato
// ============================================================================

pub fn parse_asv(html_str: &str, unit: &UnitRef) -> Result<Unit, LibraryError> {
    let html = Html::parse_document(html_str);

    let title = select_first(&html, "h1.chp_title")
        .map(|e| clean_text(&e.text().collect::<String>(), false));

    let p_elems = select_all(&html, "div.vat_wrap p.gu_indent, p.gu_indent");
    if p_elems.is_empty() {
        return Err(LibraryError::Layout {
            book: unit.book.clone(),
            unit: unit.key.clone(),
            detail: "missing asv paragraph elements".into(),
        });
    }

    let mut blocks = Vec::new();
    for p in p_elems {
        let text = clean_text(&element_text_ignoring(&p, &["sup", "audio"]), false);
        if !text.is_empty() {
            blocks.push(Block {
                gu: Some(to_nfc(&text)),
                en: None,
                translit: None,
                notes: None,
                kind: BlockKind::Paragraph,
            });
        }
    }

    Ok(Unit {
        key: to_nfc(&unit.key),
        label: to_nfc(&title.clone().unwrap_or_else(|| unit.key.clone())),
        title_gu: title.map(|t| to_nfc(&t)),
        title_en: None,
        blocks,
    })
}

// ============================================================================
// 13. Shrimad Bhagvad Gita
// ============================================================================

pub fn parse_gita(html_str: &str, unit: &UnitRef) -> Result<Unit, LibraryError> {
    let html = Html::parse_document(html_str);

    let title = select_first(&html, "h3.chapter_title")
        .map(|e| clean_text(&e.text().collect::<String>(), false));

    let sanskrit_elems = select_all(&html, "p.sanskrit");
    let trans_elems = select_all(&html, "p.transliteration");
    let eng_elems = select_all(&html, "p.translation");

    if sanskrit_elems.is_empty() && eng_elems.is_empty() {
        return Err(LibraryError::Layout {
            book: unit.book.clone(),
            unit: unit.key.clone(),
            detail: "missing gita verse elements".into(),
        });
    }

    let mut blocks = Vec::new();
    let max_len = sanskrit_elems.len().max(eng_elems.len());

    for i in 0..max_len {
        let translit = sanskrit_elems
            .get(i)
            .or_else(|| trans_elems.get(i))
            .map(|e| clean_text(&element_text_ignoring(e, &["sup", "audio"]), false))
            .filter(|s| !s.is_empty());
        let en = eng_elems
            .get(i)
            .map(|e| clean_text(&element_text_ignoring(e, &["sup", "audio"]), false))
            .filter(|s| !s.is_empty());

        if translit.is_some() || en.is_some() {
            blocks.push(Block {
                gu: None,
                en,
                translit,
                notes: None,
                kind: BlockKind::Shlok,
            });
        }
    }

    Ok(Unit {
        key: to_nfc(&unit.key),
        label: to_nfc(&title.clone().unwrap_or_else(|| unit.key.clone())),
        title_gu: None,
        title_en: title.map(|t| to_nfc(&t)),
        blocks,
    })
}

// ============================================================================
// 14. Shrimad Bhagavata Purana
// ============================================================================

pub fn parse_bhagvat(html_str: &str, unit: &UnitRef) -> Result<Unit, LibraryError> {
    let html = Html::parse_document(html_str);

    let adhyay =
        select_first(&html, "h2.adhyay").map(|e| clean_text(&e.text().collect::<String>(), false));

    let verse_elems = select_all(&html, "p.hi_left");
    if verse_elems.is_empty() {
        return Err(LibraryError::Layout {
            book: unit.book.clone(),
            unit: unit.key.clone(),
            detail: "missing bhagvat verse elements".into(),
        });
    }

    let mut blocks = Vec::new();
    for elem in verse_elems {
        let text = clean_text(&element_text_ignoring(&elem, &["sup", "audio"]), false);
        if !text.is_empty() {
            blocks.push(Block {
                gu: None,
                en: None,
                translit: Some(to_nfc(&text)),
                notes: None,
                kind: BlockKind::Shlok,
            });
        }
    }

    Ok(Unit {
        key: to_nfc(&unit.key),
        label: to_nfc(&adhyay.clone().unwrap_or_else(|| unit.key.clone())),
        title_gu: None,
        title_en: adhyay.map(|t| to_nfc(&t)),
        blocks,
    })
}

// ============================================================================
// 15. Sanskar Kunj Shloks
// ============================================================================

pub fn parse_shloks(html_str: &str, unit: &UnitRef) -> Result<Unit, LibraryError> {
    let html = Html::parse_document(html_str);

    let shloka_gu_elems = select_all(&html, "p.shloka_gu");
    let shloka_sa_elems = select_all(&html, "p.shloka_sa");
    let meaning_gu_elems = select_all(&html, "p.meaning_gu");
    let meaning_en_elems = select_all(&html, "p.meaning_en");

    if shloka_gu_elems.is_empty() && shloka_sa_elems.is_empty() {
        return Err(LibraryError::Layout {
            book: unit.book.clone(),
            unit: unit.key.clone(),
            detail: "missing shloka elements in shloks collection".into(),
        });
    }

    let mut blocks = Vec::new();
    let max_len = shloka_gu_elems.len().max(shloka_sa_elems.len());

    for i in 0..max_len {
        let gu = shloka_gu_elems
            .get(i)
            .map(|e| clean_text(&element_text_ignoring(e, &["sup", "audio"]), false))
            .filter(|s| !s.is_empty());
        let sa = shloka_sa_elems
            .get(i)
            .map(|e| clean_text(&element_text_ignoring(e, &["sup", "audio"]), false))
            .filter(|s| !s.is_empty());
        let mg = meaning_gu_elems
            .get(i)
            .map(|e| clean_text(&element_text_ignoring(e, &["sup", "audio"]), false))
            .filter(|s| !s.is_empty());
        let me = meaning_en_elems
            .get(i)
            .map(|e| clean_text(&element_text_ignoring(e, &["sup", "audio"]), false))
            .filter(|s| !s.is_empty());

        if gu.is_some() || sa.is_some() {
            blocks.push(Block {
                gu: gu.clone(),
                en: None,
                translit: sa,
                notes: None,
                kind: BlockKind::Shlok,
            });
        }
        if mg.is_some() || me.is_some() {
            blocks.push(Block {
                gu: mg,
                en: me,
                translit: None,
                notes: None,
                kind: BlockKind::Paragraph,
            });
        }
    }

    Ok(Unit {
        key: to_nfc(&unit.key),
        label: to_nfc("Shloks Collection"),
        title_gu: Some(to_nfc("સંસ્કાર કુંજ")),
        title_en: Some(to_nfc("Sanskar Kunj")),
        blocks,
    })
}
