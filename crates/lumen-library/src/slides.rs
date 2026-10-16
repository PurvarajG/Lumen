use crate::model::{Block, BlockKind, Unit};
use lumen_core::markup::escape_markup;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Gu,
    En,
    Both,
}

#[derive(Debug, Clone, Copy)]
pub struct SlideOpts {
    pub lang: Lang,
    pub blocks_per_slide: usize, // 1..=4
    pub max_chars: usize,
}

impl Default for SlideOpts {
    fn default() -> Self {
        Self {
            lang: Lang::Both,
            blocks_per_slide: 1,
            max_chars: 400,
        }
    }
}

pub fn unit_slides(unit: &Unit, opts: SlideOpts) -> Vec<(String, String)> {
    let blocks_per_slide = opts.blocks_per_slide.clamp(1, 4);
    let max_chars = if opts.max_chars == 0 {
        400
    } else {
        opts.max_chars
    };

    let mut chunks: Vec<String> = Vec::new();

    for block in &unit.blocks {
        let block_chunks = format_and_split_block(block, opts.lang, max_chars);
        chunks.extend(block_chunks);
    }

    if chunks.is_empty() {
        return Vec::new();
    }

    // Pack chunks into slides up to blocks_per_slide and max_chars
    let mut slides: Vec<String> = Vec::new();
    let mut current_slide: Vec<String> = Vec::new();
    let mut current_len = 0;

    for chunk in chunks {
        let chunk_len = chunk.chars().count();
        let separator_len = if current_slide.is_empty() { 0 } else { 2 }; // \n\n

        if !current_slide.is_empty()
            && (current_slide.len() >= blocks_per_slide
                || current_len + separator_len + chunk_len > max_chars)
        {
            slides.push(current_slide.join("\n\n"));
            current_slide.clear();
            current_len = 0;
        }

        current_slide.push(chunk);
        current_len += chunk_len + separator_len;
    }

    if !current_slide.is_empty() {
        slides.push(current_slide.join("\n\n"));
    }

    slides
        .into_iter()
        .map(|text| (unit.label.clone(), escape_markup(&text)))
        .collect()
}

fn format_and_split_block(block: &Block, lang: Lang, max_chars: usize) -> Vec<String> {
    match lang {
        Lang::Gu => {
            let text = block
                .gu
                .as_deref()
                .or(block.translit.as_deref())
                .or(block.en.as_deref())
                .unwrap_or_default()
                .trim();
            if text.is_empty() {
                return Vec::new();
            }
            split_text(text, block.kind, max_chars)
        }
        Lang::En => {
            let text = block
                .en
                .as_deref()
                .or(block.translit.as_deref())
                .or(block.gu.as_deref())
                .unwrap_or_default()
                .trim();
            if text.is_empty() {
                return Vec::new();
            }
            split_text(text, block.kind, max_chars)
        }
        Lang::Both => {
            let gu = block.gu.as_deref().map(str::trim).filter(|s| !s.is_empty());
            let en = block.en.as_deref().map(str::trim).filter(|s| !s.is_empty());

            match (gu, en) {
                (Some(g), Some(e)) => {
                    let combined = format!("{g}\n\n{e}");
                    if combined.chars().count() <= max_chars {
                        vec![combined]
                    } else {
                        // Split gu and en separately, or present them sequentially
                        let mut res = Vec::new();
                        res.extend(split_text(g, block.kind, max_chars));
                        res.extend(split_text(e, block.kind, max_chars));
                        res
                    }
                }
                (Some(g), None) => split_text(g, block.kind, max_chars),
                (None, Some(e)) => split_text(e, block.kind, max_chars),
                (None, None) => {
                    if let Some(tr) = block.translit.as_deref().filter(|s| !s.trim().is_empty()) {
                        split_text(tr.trim(), block.kind, max_chars)
                    } else {
                        Vec::new()
                    }
                }
            }
        }
    }
}

fn split_text(text: &str, kind: BlockKind, max_chars: usize) -> Vec<String> {
    if text.chars().count() <= max_chars {
        return vec![text.to_string()];
    }

    match kind {
        BlockKind::Stanza | BlockKind::Verse => {
            // Stanzas never split mid-line!
            let lines: Vec<&str> = text.lines().collect();
            let mut chunks = Vec::new();
            let mut current = Vec::new();
            let mut cur_len = 0;

            for line in lines {
                let line_len = line.chars().count();
                if !current.is_empty() && cur_len + 1 + line_len > max_chars {
                    chunks.push(current.join("\n"));
                    current.clear();
                    cur_len = 0;
                }
                current.push(line);
                cur_len += line_len + 1;
            }
            if !current.is_empty() {
                chunks.push(current.join("\n"));
            }
            chunks
        }
        _ => {
            // Paragraph: split at sentence boundaries
            split_paragraph(text, max_chars)
        }
    }
}

fn split_paragraph(text: &str, max_chars: usize) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut cur_sentence = String::new();
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();

    let mut i = 0;
    while i < len {
        cur_sentence.push(chars[i]);
        let is_delim = matches!(chars[i], '.' | '?' | '!' | '।' | '॥' | '\n');
        if is_delim && (i + 1 == len || chars[i + 1].is_whitespace()) {
            sentences.push(cur_sentence.trim().to_string());
            cur_sentence.clear();
        }
        i += 1;
    }
    if !cur_sentence.trim().is_empty() {
        sentences.push(cur_sentence.trim().to_string());
    }

    if sentences.is_empty() {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut current = Vec::new();
    let mut cur_len = 0;

    for s in sentences {
        let s_len = s.chars().count();
        if !current.is_empty() && cur_len + 1 + s_len > max_chars {
            chunks.push(current.join(" "));
            current.clear();
            cur_len = 0;
        }

        // If a single sentence exceeds max_chars, split on space
        if s_len > max_chars {
            if !current.is_empty() {
                chunks.push(current.join(" "));
                current.clear();
                cur_len = 0;
            }
            chunks.extend(split_by_words(&s, max_chars));
        } else {
            current.push(s);
            cur_len += s_len + 1;
        }
    }

    if !current.is_empty() {
        chunks.push(current.join(" "));
    }

    chunks
}

fn split_by_words(text: &str, max_chars: usize) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut chunks = Vec::new();
    let mut current = Vec::new();
    let mut cur_len = 0;

    for w in words {
        let w_len = w.chars().count();
        if !current.is_empty() && cur_len + 1 + w_len > max_chars {
            chunks.push(current.join(" "));
            current.clear();
            cur_len = 0;
        }
        current.push(w);
        cur_len += w_len + 1;
    }
    if !current.is_empty() {
        chunks.push(current.join(" "));
    }
    chunks
}
