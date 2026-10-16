use scraper::{ElementRef, Html, Node, Selector};
use unicode_normalization::UnicodeNormalization;

/// Cleans and NFC-normalizes text.
///
/// Replaces `&nbsp;` with space, replaces `&emsp;` with space,
/// trims whitespace or condenses sequential spaces while preserving newlines.
pub fn clean_text(s: &str, preserve_newlines: bool) -> String {
    let replaced = s
        .replace("&nbsp;", " ")
        .replace("&#160;", " ")
        .replace("&emsp;", " ")
        .replace(['\u{00A0}', '\u{2003}'], " ");

    let nfc: String = replaced.nfc().collect();

    if preserve_newlines {
        let lines: Vec<String> = nfc
            .lines()
            .map(|l| {
                let words: Vec<&str> = l.split_whitespace().collect();
                words.join(" ")
            })
            .collect();
        lines.join("\n").trim().to_string()
    } else {
        let words: Vec<&str> = nfc.split_whitespace().collect();
        words.join(" ")
    }
}

/// Recursively extract text from an element, ignoring specified tag names (e.g. "sup", "audio", "script").
pub fn element_text_ignoring(elem: &ElementRef, ignore_tags: &[&str]) -> String {
    let mut out = String::new();
    collect_text_recursive(elem, ignore_tags, &mut out);
    clean_text(&out, false)
}

/// Recursively extract text preserving newlines from `<br>` or existing linebreaks.
pub fn element_text_preserving_lines(elem: &ElementRef, ignore_tags: &[&str]) -> String {
    let mut out = String::new();
    collect_text_recursive_lines(elem, ignore_tags, &mut out);
    clean_text(&out, true)
}

fn collect_text_recursive(elem: &ElementRef, ignore_tags: &[&str], out: &mut String) {
    for child in elem.children() {
        match child.value() {
            Node::Text(t) => {
                out.push_str(t);
            }
            Node::Element(e) => {
                if ignore_tags.contains(&e.name()) {
                    continue;
                }
                if let Some(child_elem) = ElementRef::wrap(child) {
                    collect_text_recursive(&child_elem, ignore_tags, out);
                }
            }
            _ => {}
        }
    }
}

fn collect_text_recursive_lines(elem: &ElementRef, ignore_tags: &[&str], out: &mut String) {
    for child in elem.children() {
        match child.value() {
            Node::Text(t) => {
                out.push_str(t);
            }
            Node::Element(e) => {
                if ignore_tags.contains(&e.name()) {
                    continue;
                }
                if e.name() == "br" {
                    out.push('\n');
                    continue;
                }
                if let Some(child_elem) = ElementRef::wrap(child) {
                    collect_text_recursive_lines(&child_elem, ignore_tags, out);
                }
            }
            _ => {}
        }
    }
}

/// Helper to select elements safely using a pre-parsed selector string.
pub fn select_all<'a>(html: &'a Html, selector_str: &str) -> Vec<ElementRef<'a>> {
    let Ok(sel) = Selector::parse(selector_str) else {
        return Vec::new();
    };
    html.select(&sel).collect()
}

/// Helper to select elements inside an ElementRef.
pub fn select_within<'a>(elem: &'a ElementRef<'a>, selector_str: &str) -> Vec<ElementRef<'a>> {
    let Ok(sel) = Selector::parse(selector_str) else {
        return Vec::new();
    };
    elem.select(&sel).collect()
}

/// Helper to select the first element matching selector.
pub fn select_first<'a>(html: &'a Html, selector_str: &str) -> Option<ElementRef<'a>> {
    let sel = Selector::parse(selector_str).ok()?;
    html.select(&sel).next()
}

/// Helper to select the first element inside an ElementRef.
pub fn select_first_within<'a>(
    elem: &'a ElementRef<'a>,
    selector_str: &str,
) -> Option<ElementRef<'a>> {
    let sel = Selector::parse(selector_str).ok()?;
    elem.select(&sel).next()
}
