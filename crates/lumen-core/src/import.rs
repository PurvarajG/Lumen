use uuid::Uuid;

use crate::{Document, Slide, markup::escape_markup};

pub fn document_from_text(name: &str, text: &str, theme_id: Uuid) -> Document {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut doc = Document::new(name);
    let mut paragraph = Vec::new();
    let mut paragraphs = Vec::new();
    for line in normalized.lines() {
        if line.trim().is_empty() {
            if !paragraph.is_empty() {
                paragraphs.push(std::mem::take(&mut paragraph));
            }
        } else {
            paragraph.push(line.trim_end().to_owned());
        }
    }
    if !paragraph.is_empty() {
        paragraphs.push(paragraph);
    }
    doc.slides = paragraphs
        .into_iter()
        .enumerate()
        .map(|(index, lines)| {
            let mut slide = Slide::new(theme_id);
            slide.label = format!("Slide {}", index + 1);
            // Escape imported text so literal *, _, [ survive markup parsing
            slide.text = lines
                .iter()
                .map(|line| escape_markup(line))
                .collect::<Vec<_>>()
                .join("\n");
            slide
        })
        .collect();
    doc
}
