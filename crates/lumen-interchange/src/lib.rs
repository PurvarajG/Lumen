use std::{collections::HashMap, fs::File, io::Read, path::Path};

use lumen_core::markup::{escape_markup, strip_markup};
use lumen_core::{Document, Slide};
use thiserror::Error;
use uuid::Uuid;

const MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;
const MAX_FIELDS: usize = 100_000;
const MAX_DEPTH: usize = 32;
const MAX_RTF_BYTES: usize = 1024 * 1024;
const OPENLYRICS_NS: &str = "http://openlyrics.info/namespace/2009/song";

#[derive(Clone, Debug)]
pub struct ImportReport {
    pub document: Document,
    pub warnings: Vec<String>,
}

#[derive(Debug, Error)]
pub enum InterchangeError {
    #[error("could not {operation}: {source}")]
    Io {
        operation: String,
        #[source]
        source: std::io::Error,
    },
    #[error("unsupported interchange input: {0}")]
    Unsupported(String),
    #[error("malformed interchange input: {0}")]
    Malformed(String),
    #[error("interchange input exceeds the {maximum}-byte limit")]
    TooLarge { maximum: usize },
    #[error("interchange parser limit exceeded: {0}")]
    Limit(String),
}

pub fn import_file(path: &Path, theme_id: Uuid) -> Result<ImportReport, InterchangeError> {
    let mut file = File::open(path).map_err(|source| InterchangeError::Io {
        operation: format!("open {}", path.display()),
        source,
    })?;
    let mut bytes = Vec::new();
    file.by_ref()
        .take((MAX_INPUT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|source| InterchangeError::Io {
            operation: format!("read {}", path.display()),
            source,
        })?;
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(InterchangeError::TooLarge {
            maximum: MAX_INPUT_BYTES,
        });
    }
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("Imported document");
    match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "txt" => Ok(ImportReport {
            document: lumen_core::document_from_text(
                stem,
                std::str::from_utf8(&bytes)
                    .map_err(|_| InterchangeError::Malformed("text file is not UTF-8".into()))?,
                theme_id,
            ),
            warnings: Vec::new(),
        }),
        "xml" | "openlyrics" => import_openlyrics(
            std::str::from_utf8(&bytes)
                .map_err(|_| InterchangeError::Malformed("OpenLyrics XML is not UTF-8".into()))?,
            theme_id,
        ),
        "pro" => import_propresenter(&bytes, stem, theme_id),
        other => Err(InterchangeError::Unsupported(format!(
            "file extension .{other} is not recognized"
        ))),
    }
}

pub fn import_openlyrics(xml: &str, theme_id: Uuid) -> Result<ImportReport, InterchangeError> {
    if xml.len() > MAX_INPUT_BYTES {
        return Err(InterchangeError::TooLarge {
            maximum: MAX_INPUT_BYTES,
        });
    }
    let parsed = roxmltree::Document::parse_with_options(
        xml,
        roxmltree::ParsingOptions {
            allow_dtd: false,
            ..Default::default()
        },
    )
    .map_err(|e| InterchangeError::Malformed(format!("OpenLyrics XML: {e}")))?;
    let root = parsed.root_element();
    if root.tag_name().name() != "song" || root.tag_name().namespace() != Some(OPENLYRICS_NS) {
        return Err(InterchangeError::Unsupported(
            "expected an OpenLyrics song in the standard namespace".into(),
        ));
    }
    if root
        .attribute("version")
        .is_some_and(|v| !matches!(v, "0.8" | "0.9"))
    {
        return Err(InterchangeError::Unsupported(
            "OpenLyrics versions 0.8 and 0.9 are supported".into(),
        ));
    }
    let title = root
        .descendants()
        .find(|n| {
            n.is_element()
                && n.tag_name().name() == "title"
                && n.tag_name().namespace() == Some(OPENLYRICS_NS)
        })
        .and_then(|n| n.text())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| InterchangeError::Malformed("OpenLyrics song is missing a title".into()))?;
    let lyrics = root
        .children()
        .find(|n| {
            n.is_element()
                && n.tag_name().name() == "lyrics"
                && n.tag_name().namespace() == Some(OPENLYRICS_NS)
        })
        .ok_or_else(|| InterchangeError::Malformed("OpenLyrics song is missing lyrics".into()))?;
    let mut verses = Vec::new();
    let mut by_name: HashMap<String, Vec<usize>> = HashMap::new();
    for verse in lyrics.children().filter(|n| {
        n.is_element()
            && n.tag_name().name() == "verse"
            && n.tag_name().namespace() == Some(OPENLYRICS_NS)
    }) {
        let name = verse
            .attribute("name")
            .ok_or_else(|| {
                InterchangeError::Malformed("OpenLyrics verse is missing its name".into())
            })?
            .to_owned();
        let text = verse_text(verse);
        let index = verses.len();
        by_name.entry(name.clone()).or_default().push(index);
        verses.push((name, text));
    }
    if verses.is_empty() {
        return Err(InterchangeError::Malformed(
            "OpenLyrics lyrics contain no verses".into(),
        ));
    }
    if by_name.values().any(|indices| indices.len() > 1) {
        return Err(InterchangeError::Malformed(
            "duplicate verse names are ambiguous".into(),
        ));
    }
    let order = lyrics.children().find(|n| {
        n.is_element()
            && n.tag_name().name() == "verseOrder"
            && n.tag_name().namespace() == Some(OPENLYRICS_NS)
    });
    let ordered = if let Some(order) = order {
        let mut output = Vec::new();
        for name in order.text().unwrap_or("").split_whitespace() {
            let indices = by_name.get(name).ok_or_else(|| {
                InterchangeError::Malformed(format!("verseOrder references unknown verse '{name}'"))
            })?;
            if indices.len() != 1 {
                return Err(InterchangeError::Malformed(format!(
                    "verse name '{name}' is ambiguous"
                )));
            }
            output.push(indices[0]);
        }
        output
    } else {
        (0..verses.len()).collect()
    };
    let mut document = Document::new(title);
    for index in ordered {
        let (label, text) = &verses[index];
        document.slides.push(text_slide(label, text, theme_id));
    }
    let mut warnings = Vec::new();
    for name in ["authors", "copyright", "ccliNo", "key", "tempo", "language"] {
        if root
            .descendants()
            .any(|n| n.is_element() && n.tag_name().name() == name)
        {
            warnings.push(format!("OpenLyrics {name} metadata is not retained."));
        }
    }
    Ok(ImportReport { document, warnings })
}

fn verse_text(verse: roxmltree::Node<'_, '_>) -> String {
    fn walk(node: roxmltree::Node<'_, '_>, out: &mut String) {
        if node.is_element() && matches!(node.tag_name().name(), "chord" | "comment") {
            return;
        }
        if node.is_element() && node.tag_name().name() == "br" {
            out.push('\n');
            return;
        }
        if node.is_text() {
            out.push_str(node.text().unwrap_or(""));
            return;
        }
        for child in node.children() {
            walk(child, out);
        }
    }
    let mut out = String::new();
    for (i, line) in verse
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "lines")
        .flat_map(|n| {
            n.children()
                .filter(|c| c.is_element() && c.tag_name().name() == "line")
        })
        .enumerate()
    {
        if i > 0 {
            out.push('\n');
        }
        walk(line, &mut out);
    }
    if out.is_empty() {
        walk(verse, &mut out);
    }
    out.trim().to_owned()
}

pub fn export_openlyrics(document: &Document) -> Result<String, InterchangeError> {
    if document.name.trim().is_empty() {
        return Err(InterchangeError::Malformed("document name is empty".into()));
    }
    if document.slides.is_empty() {
        return Err(InterchangeError::Malformed(
            "document has no slides to export".into(),
        ));
    }
    let mut output = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<song xmlns=\"{OPENLYRICS_NS}\" version=\"0.9\">\n  <properties><titles><title>{}</title></titles></properties>\n  <lyrics>\n",
        xml_escape(&document.name)
    );
    let names: Vec<String> = (0..document.slides.len())
        .map(|i| format!("v{}", i + 1))
        .collect();
    for (i, slide) in document.slides.iter().enumerate() {
        let text = &strip_markup(&slide.text);
        let lines = text
            .split('\n')
            .map(xml_escape)
            .collect::<Vec<_>>()
            .join("<br/>");
        output.push_str(&format!(
            "    <verse name=\"{}\"><lines>{lines}</lines></verse>\n",
            names[i]
        ));
    }
    output.push_str(&format!(
        "    <verseOrder>{}</verseOrder>\n  </lyrics>\n</song>\n",
        names.join(" ")
    ));
    Ok(output)
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub fn import_propresenter(
    bytes: &[u8],
    fallback_name: &str,
    theme_id: Uuid,
) -> Result<ImportReport, InterchangeError> {
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(InterchangeError::TooLarge {
            maximum: MAX_INPUT_BYTES,
        });
    }
    let top = parse_fields(bytes, 0)?;
    let title = top
        .iter()
        .find(|f| f.number == 3 && f.wire == 2)
        .map(|f| {
            std::str::from_utf8(f.bytes)
                .map(str::to_owned)
                .map_err(|_| InterchangeError::Malformed("ProPresenter title is not UTF-8".into()))
        })
        .transpose()?
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| fallback_name.to_owned());
    let cues: Vec<_> = top
        .iter()
        .filter(|f| f.number == 13 && f.wire == 2)
        .collect();
    if cues.is_empty() {
        return Err(InterchangeError::Unsupported(
            "no observed top-level field-13 cue records".into(),
        ));
    }
    let mut examined = top.len();
    let mut slides = Vec::new();
    let mut recognized = 0;
    let mut unsupported = 0;
    for cue in cues {
        let mut payloads = Vec::new();
        extract_rtf(
            cue.bytes,
            0,
            &[10, 23, 2, 1, 1, 1, 13, 5],
            &mut examined,
            &mut payloads,
        )?;
        let texts = payloads
            .iter()
            .map(|b| parse_rtf(b))
            .collect::<Result<Vec<_>, _>>()?;
        if texts.is_empty() {
            unsupported += 1;
            slides.push(text_slide("Cue", "", theme_id));
        } else {
            recognized += 1;
            slides.push(text_slide("Cue", &texts.join("\n"), theme_id));
        }
    }
    if recognized == 0 {
        return Err(InterchangeError::Unsupported(
            "no cue contains the recognized ProPresenter RTF text structure".into(),
        ));
    }
    let mut warnings = vec!["Experimental ProPresenter 21.4 text-only compatibility; media, layout, formatting and actions are lost.".into()];
    if unsupported > 0 {
        warnings.push(format!("{unsupported} cue(s) had unsupported text structure and were imported blank to preserve cue positions."));
    }
    Ok(ImportReport {
        document: Document {
            id: Uuid::new_v4(),
            name: title,
            slides,
            source: None,
        },
        warnings,
    })
}

fn text_slide(label: &str, text: &str, theme_id: Uuid) -> Slide {
    let mut slide = Slide::new(theme_id);
    slide.label = label.into();
    slide.text = escape_markup(text);
    slide
}

#[derive(Clone, Copy)]
struct Field<'a> {
    number: u32,
    wire: u8,
    bytes: &'a [u8],
}

fn parse_fields(bytes: &[u8], depth: usize) -> Result<Vec<Field<'_>>, InterchangeError> {
    if depth > MAX_DEPTH {
        return Err(InterchangeError::Limit(
            "protobuf nesting exceeds 32".into(),
        ));
    }
    let mut pos = 0;
    let mut fields = Vec::new();
    while pos < bytes.len() {
        let tag = read_varint(bytes, &mut pos)?;
        let number = (tag >> 3) as u32;
        if tag >> 3 > (1 << 29) - 1 {
            return Err(InterchangeError::Malformed(
                "protobuf field number exceeds the wire-format maximum".into(),
            ));
        }
        let wire = (tag & 7) as u8;
        if number == 0 {
            return Err(InterchangeError::Malformed(
                "protobuf field number zero".into(),
            ));
        }
        let payload = match wire {
            0 => {
                let start = pos;
                read_varint(bytes, &mut pos)?;
                &bytes[start..pos]
            }
            1 => {
                let end = pos
                    .checked_add(8)
                    .filter(|e| *e <= bytes.len())
                    .ok_or_else(|| InterchangeError::Malformed("truncated fixed64 field".into()))?;
                let p = &bytes[pos..end];
                pos = end;
                p
            }
            2 => {
                let len = read_varint(bytes, &mut pos)?;
                let len = usize::try_from(len).map_err(|_| {
                    InterchangeError::Malformed("protobuf length does not fit platform".into())
                })?;
                let end = pos
                    .checked_add(len)
                    .filter(|e| *e <= bytes.len())
                    .ok_or_else(|| {
                        InterchangeError::Malformed("truncated length-delimited field".into())
                    })?;
                let p = &bytes[pos..end];
                pos = end;
                p
            }
            5 => {
                let end = pos
                    .checked_add(4)
                    .filter(|e| *e <= bytes.len())
                    .ok_or_else(|| InterchangeError::Malformed("truncated fixed32 field".into()))?;
                let p = &bytes[pos..end];
                pos = end;
                p
            }
            3 | 4 => {
                return Err(InterchangeError::Malformed(
                    "protobuf groups are unsupported".into(),
                ));
            }
            _ => {
                return Err(InterchangeError::Malformed(format!(
                    "unsupported protobuf wire type {wire}"
                )));
            }
        };
        fields.push(Field {
            number,
            wire,
            bytes: payload,
        });
        if fields.len() > MAX_FIELDS {
            return Err(InterchangeError::Limit(
                "protobuf field count exceeds 100000".into(),
            ));
        }
    }
    Ok(fields)
}

fn read_varint(bytes: &[u8], pos: &mut usize) -> Result<u64, InterchangeError> {
    let mut value = 0u64;
    for i in 0..10 {
        let byte = *bytes
            .get(*pos)
            .ok_or_else(|| InterchangeError::Malformed("truncated protobuf varint".into()))?;
        *pos += 1;
        if i == 9 && byte > 1 {
            return Err(InterchangeError::Malformed(
                "protobuf varint overflows 64 bits".into(),
            ));
        }
        value |= ((byte & 0x7f) as u64) << (i * 7);
        if byte < 0x80 {
            return Ok(value);
        }
    }
    Err(InterchangeError::Malformed(
        "invalid protobuf varint".into(),
    ))
}

fn extract_rtf<'a>(
    bytes: &'a [u8],
    depth: usize,
    path: &[u32],
    count: &mut usize,
    output: &mut Vec<&'a [u8]>,
) -> Result<(), InterchangeError> {
    if depth > MAX_DEPTH {
        return Err(InterchangeError::Limit(
            "protobuf nesting exceeds 32".into(),
        ));
    }
    let fields = parse_fields(bytes, depth)?;
    *count += fields.len();
    if *count > MAX_FIELDS {
        return Err(InterchangeError::Limit(
            "protobuf field count exceeds 100000".into(),
        ));
    }
    let Some((&head, tail)) = path.split_first() else {
        return Ok(());
    };
    for field in fields.iter().filter(|f| f.number == head && f.wire == 2) {
        if tail.is_empty() {
            if field.bytes.len() > MAX_RTF_BYTES {
                return Err(InterchangeError::Limit("RTF payload exceeds 1 MiB".into()));
            }
            if field.bytes.starts_with(b"{\\rtf") {
                output.push(field.bytes);
            }
        } else {
            extract_rtf(field.bytes, depth + 1, tail, count, output)?;
        }
    }
    Ok(())
}

fn parse_rtf(bytes: &[u8]) -> Result<String, InterchangeError> {
    let source = std::str::from_utf8(bytes)
        .map_err(|_| InterchangeError::Malformed("RTF payload is not UTF-8".into()))?;
    let mut nesting = 0usize;
    let mut escaped = false;
    for b in source.bytes() {
        if escaped {
            escaped = false;
            continue;
        }
        match b {
            b'\\' => escaped = true,
            b'{' => {
                nesting += 1;
                if nesting > MAX_DEPTH {
                    return Err(InterchangeError::Limit(
                        "RTF brace nesting exceeds 32".into(),
                    ));
                }
            }
            b'}' => {
                nesting = nesting
                    .checked_sub(1)
                    .ok_or_else(|| InterchangeError::Malformed("unbalanced RTF brace".into()))?;
            }
            _ => {}
        }
    }
    if nesting != 0 || !source.starts_with("{\\rtf") {
        return Err(InterchangeError::Malformed(
            "RTF braces or header are invalid".into(),
        ));
    }
    let raw_source = source;
    let source = normalize_rtf_unicode(raw_source);
    // rtf-parser mistakes a final literal backslash before the closing brace for an escaped brace.
    let source = source
        .strip_suffix("\\\\}")
        .map(|s| format!("{s}\\\\\\par}}"))
        .unwrap_or(source);
    let parsed = std::panic::catch_unwind(|| rtf_parser::RtfDocument::try_from(source.as_str()))
        .map_err(|_| {
            InterchangeError::Malformed(
                "RTF parser rejected payload without returning an error".into(),
            )
        })?
        .map_err(|e| InterchangeError::Malformed(format!("RTF: {e}")))?;
    let _validated_text = parsed.get_text();
    let mut text = decode_rtf_text(raw_source)?;
    if text.ends_with('\n') {
        text.pop();
    }
    Ok(text)
}

fn decode_rtf_text(source: &str) -> Result<String, InterchangeError> {
    #[derive(Clone, Copy)]
    struct State {
        ignored: bool,
    }
    let chars: Vec<char> = source.chars().collect();
    let mut stack = Vec::new();
    let mut text = String::new();
    let mut i = 0;
    let mut uc = 1usize;
    let mut skip_fallback = 0usize;
    let mut high = None;
    while i < chars.len() {
        let c = chars[i];
        if c == '{' {
            stack.push(State {
                ignored: stack.last().is_some_and(|s: &State| s.ignored),
            });
            i += 1;
            continue;
        }
        if c == '}' {
            stack
                .pop()
                .ok_or_else(|| InterchangeError::Malformed("unbalanced RTF group".into()))?;
            i += 1;
            continue;
        }
        let ignored = stack.last().is_some_and(|s| s.ignored);
        if c != '\\' {
            if skip_fallback > 0 {
                skip_fallback -= 1;
            } else if !ignored && !c.is_control() {
                text.push(c);
            }
            i += 1;
            continue;
        }
        i += 1;
        let Some(&next) = chars.get(i) else {
            return Err(InterchangeError::Malformed("trailing RTF escape".into()));
        };
        if next == '\n' || next == '\r' {
            if skip_fallback > 0 {
                skip_fallback -= 1;
            } else if !ignored {
                text.push('\n');
            }
            i += 1;
            continue;
        }
        if next == '\'' {
            let hex: String = chars.get(i + 1..i + 3).unwrap_or(&[]).iter().collect();
            let byte = u8::from_str_radix(&hex, 16)
                .map_err(|_| InterchangeError::Malformed("invalid RTF hex escape".into()))?;
            if skip_fallback > 0 {
                skip_fallback -= 1;
            } else if !ignored {
                text.push(cp1252_char(byte));
            }
            i = (i + 3).min(chars.len());
            continue;
        }
        if !next.is_ascii_alphabetic() {
            if skip_fallback > 0 {
                skip_fallback -= 1;
            } else if !ignored {
                match next {
                    '\\' | '{' | '}' => text.push(next),
                    '~' => text.push('\u{00a0}'),
                    '_' => text.push('\u{2011}'),
                    '-' => {}
                    '*' => {
                        if let Some(s) = stack.last_mut() {
                            s.ignored = true;
                        }
                    }
                    _ => {}
                }
            }
            i += 1;
            continue;
        }
        let start = i;
        while chars.get(i).is_some_and(|c| c.is_ascii_alphabetic()) {
            i += 1;
        }
        let word: String = chars[start..i].iter().collect();
        let sign = if chars.get(i) == Some(&'-') {
            i += 1;
            -1
        } else {
            1
        };
        let digits = i;
        while chars.get(i).is_some_and(|c| c.is_ascii_digit()) {
            i += 1;
        }
        let parameter = if i > digits {
            chars[digits..i]
                .iter()
                .collect::<String>()
                .parse::<i32>()
                .ok()
                .map(|n| n * sign)
        } else {
            None
        };
        if chars.get(i) == Some(&' ') {
            i += 1;
        }
        if matches!(
            word.as_str(),
            "fonttbl"
                | "colortbl"
                | "stylesheet"
                | "info"
                | "header"
                | "footer"
                | "pict"
                | "object"
                | "filetbl"
                | "listtable"
                | "listoverridetable"
                | "xmlnstbl"
                | "datastore"
        ) && let Some(group) = stack.last_mut()
        {
            group.ignored = true;
        }
        if word == "uc" {
            uc = parameter.unwrap_or(1).clamp(0, 8) as usize;
            continue;
        }
        if word == "u" {
            if let Some(unit) = parameter {
                let unit = unit.rem_euclid(65_536) as u16;
                if (0xD800..=0xDBFF).contains(&unit) {
                    high = Some(unit);
                } else if (0xDC00..=0xDFFF).contains(&unit) {
                    if let Some(first) = high.take() {
                        let scalar =
                            0x10000 + (((first as u32 - 0xD800) << 10) | (unit as u32 - 0xDC00));
                        if !ignored {
                            text.push(char::from_u32(scalar).unwrap());
                        }
                    }
                } else {
                    high = None;
                    if !ignored {
                        text.push(char::from_u32(unit as u32).unwrap());
                    }
                }
                skip_fallback = uc;
            }
            continue;
        }
        if skip_fallback > 0 {
            skip_fallback = 0;
        }
        if ignored {
            continue;
        }
        match word.as_str() {
            "par" | "line" => text.push('\n'),
            "tab" => text.push('\t'),
            "emdash" => text.push('\u{2014}'),
            "endash" => text.push('\u{2013}'),
            "lquote" => text.push('\u{2018}'),
            "rquote" => text.push('\u{2019}'),
            "ldblquote" => text.push('\u{201c}'),
            "rdblquote" => text.push('\u{201d}'),
            _ => {}
        }
    }
    if !stack.is_empty() {
        return Err(InterchangeError::Malformed("unclosed RTF group".into()));
    }
    if let Some(unit) = high {
        text.push(char::from_u32(unit as u32).unwrap_or('\u{fffd}'));
    }
    Ok(text)
}

fn normalize_rtf_unicode(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut output = String::with_capacity(source.len());
    let mut i = 0;
    let mut fallback_count = 1;
    while i < chars.len() {
        if chars[i] == '\\' && chars.get(i + 1) == Some(&'u') {
            let mut end = i + 2;
            if chars.get(end) == Some(&'-') {
                end += 1;
            }
            let digits = end;
            while chars.get(end).is_some_and(|c| c.is_ascii_digit()) {
                end += 1;
            }
            if end > digits
                && let Ok(value) = chars[digits..end].iter().collect::<String>().parse::<i32>()
            {
                let value = if chars.get(i + 2) == Some(&'-') {
                    -value
                } else {
                    value
                };
                let first = (value.rem_euclid(65_536)) as u16;
                if (0xD800..=0xDBFF).contains(&first)
                    && chars.get(end..end + 2) == Some(&['\\', 'u'])
                {
                    let mut low_end = end + 2;
                    while chars
                        .get(low_end)
                        .is_some_and(|c| c.is_ascii_digit() || *c == '-')
                    {
                        low_end += 1;
                    }
                    if let Ok(low) = chars[end + 2..low_end]
                        .iter()
                        .collect::<String>()
                        .parse::<i32>()
                    {
                        let low = low.rem_euclid(65_536) as u16;
                        if (0xDC00..=0xDFFF).contains(&low) {
                            let code =
                                0x10000 + (((first as u32 - 0xD800) << 10) | (low as u32 - 0xDC00));
                            if let Some(ch) = char::from_u32(code) {
                                output.push(ch);
                            }
                            i = low_end;
                        } else {
                            i = end;
                            if let Some(ch) = char::from_u32(first as u32) {
                                output.push(ch);
                            }
                        }
                    } else {
                        i = end;
                    }
                } else {
                    if let Some(ch) = char::from_u32(first as u32) {
                        output.push(ch);
                    }
                    i = end;
                }
                if chars.get(i) == Some(&' ') {
                    i += 1;
                }
                for _ in 0..fallback_count {
                    if i >= chars.len() || chars[i] == '}' {
                        break;
                    }
                    if chars[i] == '\\' && chars.get(i + 1) == Some(&'\'') {
                        i = (i + 4).min(chars.len());
                    } else if chars[i] == '\\' {
                        i = (i + 2).min(chars.len());
                    } else {
                        i += 1;
                    }
                }
                continue;
            }
        }
        if chars[i] == '\\' && chars.get(i + 1) == Some(&'u') && chars.get(i + 2) == Some(&'c') {
            let mut end = i + 3;
            while chars.get(end).is_some_and(|c| c.is_ascii_digit()) {
                end += 1;
            }
            if end > i + 3
                && let Ok(count) = chars[i + 3..end]
                    .iter()
                    .collect::<String>()
                    .parse::<usize>()
            {
                fallback_count = count.min(8);
            }
        }
        output.push(chars[i]);
        i += 1;
    }
    output
}

fn cp1252_char(byte: u8) -> char {
    const WINDOWS_1252: [char; 32] = [
        '\u{20ac}', '\u{81}', '\u{201a}', '\u{192}', '\u{201e}', '\u{2026}', '\u{2020}',
        '\u{2021}', '\u{2c6}', '\u{2030}', '\u{160}', '\u{2039}', '\u{152}', '\u{8d}', '\u{17d}',
        '\u{8f}', '\u{90}', '\u{2018}', '\u{2019}', '\u{201c}', '\u{201d}', '\u{2022}', '\u{2013}',
        '\u{2014}', '\u{2dc}', '\u{2122}', '\u{161}', '\u{203a}', '\u{153}', '\u{9d}', '\u{17e}',
        '\u{178}',
    ];
    if (0x80..=0x9f).contains(&byte) {
        WINDOWS_1252[(byte - 0x80) as usize]
    } else {
        char::from(byte)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn xml(body: &str) -> String {
        format!(
            r#"<song xmlns="http://openlyrics.info/namespace/2009/song" version="0.9"><properties><titles><title>A &amp; B</title></titles></properties><lyrics>{body}</lyrics></song>"#
        )
    }

    #[test]
    fn openlyrics_imports_namespace_order_repeats_and_visible_text() {
        let input = xml(
            r#"<verse name="v1"><lines>One <i>bright</i> &amp; true<br/>line<chord>C</chord><comment>aside</comment></lines></verse><verse name="c"><lines>Chorus</lines></verse><verseOrder>c v1 c</verseOrder>"#,
        );
        let report = import_openlyrics(&input, Uuid::nil()).unwrap();
        assert_eq!(report.document.name, "A & B");
        assert_eq!(
            report
                .document
                .slides
                .iter()
                .map(|s| s.text.as_str())
                .collect::<Vec<_>>(),
            ["Chorus", "One bright & true\nline", "Chorus"]
        );
        assert_ne!(report.document.slides[0].id, report.document.slides[2].id);
    }

    #[test]
    fn openlyrics_reports_missing_ambiguous_and_malformed_documents() {
        assert!(
            import_openlyrics(
                &xml(r#"<verse name="v1"><lines>x</lines></verse><verseOrder>v2</verseOrder>"#),
                Uuid::nil()
            )
            .is_err()
        );
        assert!(import_openlyrics(&xml(r#"<verse name="v1"><lines>x</lines></verse><verse name="v1"><lines>y</lines></verse><verseOrder>v1</verseOrder>"#), Uuid::nil()).is_err());
        assert!(import_openlyrics("<song>", Uuid::nil()).is_err());
        assert!(
            import_openlyrics(
                "<!DOCTYPE song [<!ENTITY x SYSTEM 'file:///etc/passwd'>]><song/>",
                Uuid::nil()
            )
            .is_err()
        );
        assert!(import_openlyrics(&" ".repeat(MAX_INPUT_BYTES + 1), Uuid::nil()).is_err());
        assert!(
            import_openlyrics(
                &xml(r#"<verse name="v"><lines>x</lines></verse>"#)
                    .replace("version=\"0.9\"", "version=\"1.0\""),
                Uuid::nil()
            )
            .is_err()
        );
    }

    #[test]
    fn openlyrics_export_roundtrips_xml_text_and_linebreaks() {
        let mut doc = lumen_core::Document::new("A & <title>");
        doc.slides
            .push(text_slide("Verse", "Text & <tag>\nsecond", Uuid::nil()));
        doc.slides.push(text_slide("Chorus", "more", Uuid::nil()));
        let xml = export_openlyrics(&doc).unwrap();
        assert!(xml.contains("version=\"0.9\""));
        assert!(xml.contains("<br/>"));
        assert!(xml.contains("<verseOrder>v1 v2</verseOrder>"));
        let roundtrip = import_openlyrics(&xml, Uuid::nil()).unwrap();
        assert_eq!(roundtrip.document.name, doc.name);
        assert_eq!(roundtrip.document.slides[0].text, doc.slides[0].text);
        assert_eq!(roundtrip.document.slides[1].text, doc.slides[1].text);
        assert!(export_openlyrics(&lumen_core::Document::new("x")).is_err());
    }

    fn varint(mut n: u64) -> Vec<u8> {
        let mut v = Vec::new();
        while n >= 128 {
            v.push((n as u8) | 0x80);
            n >>= 7;
        }
        v.push(n as u8);
        v
    }
    fn field(n: u64, wire: u8, bytes: &[u8]) -> Vec<u8> {
        let mut v = varint((n << 3) | wire as u64);
        match wire {
            2 => {
                v.extend(varint(bytes.len() as u64));
                v.extend(bytes);
            }
            0 => v.extend(bytes),
            _ => v.extend(bytes),
        }
        v
    }
    fn msg(fields: Vec<Vec<u8>>) -> Vec<u8> {
        fields.into_iter().flatten().collect()
    }
    fn cue(rtf: &[u8]) -> Vec<u8> {
        cue_many(&[rtf])
    }
    fn cue_many(rtfs: &[&[u8]]) -> Vec<u8> {
        let payload = msg(rtfs.iter().map(|rtf| field(5, 2, rtf)).collect());
        let a = field(13, 2, &payload);
        let b = field(1, 2, &a);
        let c = field(1, 2, &b);
        let d = field(1, 2, &c);
        let e = field(2, 2, &d);
        let f = field(23, 2, &e);
        field(10, 2, &f)
    }
    fn pro(cues: Vec<Vec<u8>>) -> Vec<u8> {
        let mut fields = vec![field(3, 2, b"Synthetic")];
        fields.extend(cues.into_iter().map(|c| field(13, 2, &c)));
        msg(fields)
    }
    fn rtf(s: &str) -> Vec<u8> {
        format!(r#"{{\rtf1\ansi\uc1 {s}\par}}"#).into_bytes()
    }

    #[test]
    fn propresenter_keeps_cue_order_blank_cues_and_multiple_texts() {
        let bytes = pro(vec![
            cue_many(&[&rtf("first"), &rtf("extra")]),
            field(7, 2, b"unknown"),
            cue(&rtf("second")),
        ]);
        let report = import_propresenter(&bytes, "fallback", Uuid::nil()).unwrap();
        assert_eq!(report.document.name, "Synthetic");
        assert_eq!(
            report
                .document
                .slides
                .iter()
                .map(|s| s.text.as_str())
                .collect::<Vec<_>>(),
            ["first\nextra", "", "second"]
        );
        assert!(report.warnings.iter().any(|w| w.contains("text-only")));
        assert!(report.warnings.iter().any(|w| w.contains("unsupported")));
    }

    #[test]
    fn propresenter_preserves_unicode_and_rtf_escapes() {
        let content = rtf(r#"Caf\'e9 \u8212? na\u239?ve\par \{brace\} \\"#);
        let report =
            import_propresenter(&pro(vec![cue(&content)]), "fallback", Uuid::nil()).unwrap();
        assert!(
            report.document.slides[0].text.contains("Café —"),
            "{:?}",
            report.document.slides[0].text
        );
        assert!(
            report.document.slides[0].text.contains("{brace} \\"),
            "{:?}",
            report.document.slides[0].text
        );
    }

    #[test]
    fn protobuf_framing_rejects_malformed_and_unrecognized_inputs() {
        for bytes in [vec![0x80], vec![0x0b], vec![0x00], vec![0x0a, 0x05, 0x00]] {
            assert!(import_propresenter(&bytes, "x", Uuid::nil()).is_err());
        }
        assert!(import_propresenter(b"unrelated bytes", "x", Uuid::nil()).is_err());
        assert!(parse_fields(&[0x80; 10], 0).is_err());
        assert!(parse_fields(&[0x0b], 0).is_err());
        assert!(parse_fields(&[], MAX_DEPTH + 1).is_err());
        let too_many_fields = [0x08, 0x00].repeat(MAX_FIELDS + 1);
        assert!(parse_fields(&too_many_fields, 0).is_err());
    }

    #[test]
    fn text_extension_dispatch_uses_core_importer_and_actual_read_cap() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Notes.txt");
        std::fs::write(&path, "one\n\ntwo").unwrap();
        let report = import_file(&path, Uuid::nil()).unwrap();
        assert_eq!(report.document.name, "Notes");
        assert_eq!(report.document.slides.len(), 2);
        assert!(import_file(&dir.path().join("unknown"), Uuid::nil()).is_err());
    }

    #[test]
    #[ignore = "reads parent-owned local research fixtures when manually verified"]
    fn owned_prop_fixture_matches_recorded_text() {
        let theme = Uuid::new_v4();
        let a = std::fs::read("/Users/purvarajsinhgohil/Developer/PROPRESENTER/.local/research/Lumen-Research-Fixture.pro").unwrap();
        let report = import_propresenter(&a, "fixture", theme).unwrap();
        assert_eq!(
            report
                .document
                .slides
                .iter()
                .map(|s| s.text.as_str())
                .collect::<Vec<_>>(),
            [
                "Lumen reference sample\nOriginal line two",
                "Second original cue\nCreated for interoperability testing"
            ]
        );
        let b = std::fs::read("/Users/purvarajsinhgohil/Developer/PROPRESENTER/.local/research/Lumen-Unicode-Fixture.pro").unwrap();
        let report = import_propresenter(&b, "fixture", theme).unwrap();
        assert_eq!(
            strip_markup(&report.document.slides[1].text),
            "Café — naïve\nUnicode: Ω שלום 😀\nEscapes: {brace} \\"
        );
    }
}
