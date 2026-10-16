use lumen_core::markup::strip_markup;
use lumen_library::model::{Block, BlockKind, Unit};
use lumen_library::slides::{Lang, SlideOpts, unit_slides};

#[test]
fn test_unit_slides_both_languages() {
    let unit = Unit {
        key: "test-1".into(),
        label: "Sample 1".into(),
        title_gu: None,
        title_en: None,
        blocks: vec![Block {
            gu: Some("ગુજરાતી લખાણ એક.".into()),
            en: Some("English text one.".into()),
            translit: None,
            notes: None,
            kind: BlockKind::Paragraph,
        }],
    };

    let opts = SlideOpts {
        lang: Lang::Both,
        blocks_per_slide: 1,
        max_chars: 200,
    };

    let slides = unit_slides(&unit, opts);
    assert_eq!(slides.len(), 1);
    assert_eq!(slides[0].0, "Sample 1");
    // Both languages separated by a blank line
    let expected = "ગુજરાતી લખાણ એક.\n\nEnglish text one.";
    assert_eq!(slides[0].1, expected);
}

#[test]
fn test_stanzas_never_split_mid_line() {
    let unit = Unit {
        key: "kirtan-1".into(),
        label: "Pad 1".into(),
        title_gu: None,
        title_en: None,
        blocks: vec![Block {
            gu: Some("પંક્તિ એક લાંબી છે.\nપંક્તિ બે પણ લાંબી છે.\nપંક્તિ ત્રણ પણ ઘણી લાંબી છે.".into()),
            en: None,
            translit: None,
            notes: None,
            kind: BlockKind::Stanza,
        }],
    };

    // Set max_chars small enough that it has to split
    let opts = SlideOpts {
        lang: Lang::Gu,
        blocks_per_slide: 1,
        max_chars: 40,
    };

    let slides = unit_slides(&unit, opts);
    assert!(slides.len() > 1);
    for (_label, text) in slides {
        // Every chunk must contain complete lines, never cutting words mid-line
        for line in text.lines() {
            assert!(
                line.ends_with('.') || line.is_empty(),
                "stanza line was cut mid-line: {:?}",
                line
            );
        }
    }
}

#[test]
fn test_paragraphs_split_at_sentence_boundaries() {
    let text_gu = "પ્રથમ વાક્ય અહીં પૂરું થાય છે. બીજું વાક્ય પણ અહીં પૂરું થાય છે. ત્રીજું વાક્ય આ રહ્યું.";
    let unit = Unit {
        key: "vach-1".into(),
        label: "Vachanamrut 1".into(),
        title_gu: None,
        title_en: None,
        blocks: vec![Block {
            gu: Some(text_gu.into()),
            en: None,
            translit: None,
            notes: None,
            kind: BlockKind::Paragraph,
        }],
    };

    let opts = SlideOpts {
        lang: Lang::Gu,
        blocks_per_slide: 1,
        max_chars: 45,
    };

    let slides = unit_slides(&unit, opts);
    assert!(slides.len() >= 2);
    for (_label, text) in slides {
        assert!(
            text.ends_with('.'),
            "slide should end at sentence boundary: {:?}",
            text
        );
    }
}

#[test]
fn test_slides_markup_escaped() {
    // If text contains markdown characters like * or [ or _, they must be escaped!
    let unit = Unit {
        key: "test-esc".into(),
        label: "Escape Test".into(),
        title_gu: None,
        title_en: None,
        blocks: vec![Block {
            gu: Some("ટેક્સ્ટ *સ્ટાર* અને [બ્રેકેટ]".into()),
            en: Some("Text *star* and [bracket] and _under_.".into()),
            translit: None,
            notes: None,
            kind: BlockKind::Paragraph,
        }],
    };

    let opts = SlideOpts {
        lang: Lang::En,
        blocks_per_slide: 1,
        max_chars: 200,
    };

    let slides = unit_slides(&unit, opts);
    assert_eq!(slides.len(), 1);
    let escaped_text = &slides[0].1;
    // Escaped text should contain backslashes before special chars
    assert!(escaped_text.contains("\\*"));
    assert!(escaped_text.contains("\\["));
    assert!(escaped_text.contains("\\_"));

    // Stripping markup should recover the exact original text!
    let stripped = strip_markup(escaped_text);
    assert_eq!(stripped, "Text *star* and [bracket] and _under_.");
}
