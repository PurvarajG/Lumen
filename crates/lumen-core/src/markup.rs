/// Tiny inline markup for slide text.
///
/// Supported syntax:
/// - `**bold**`
/// - `*italic*`
/// - `__underline__`
/// - `[#RRGGBB]coloured[/]`
/// - Backslash escapes: `\*`, `\_`, `\[`, `\\`
///
/// Unclosed markers render literally (never error).

#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub color: Option<[u8; 4]>,
}

impl Span {
    fn new() -> Self {
        Self {
            text: String::new(),
            bold: false,
            italic: false,
            underline: false,
            color: None,
        }
    }

    fn style_eq(&self, other: &Self) -> bool {
        self.bold == other.bold
            && self.italic == other.italic
            && self.underline == other.underline
            && self.color == other.color
    }
}

/// Parse inline markup into spans.
///
/// Newlines are preserved inside span text. Adjacent same-style spans are merged.
pub fn parse_markup(input: &str) -> Vec<Span> {
    let mut spans: Vec<Span> = Vec::new();
    let mut current = Span::new();

    // State stacks for toggling
    let mut bold = false;
    let mut italic = false;
    let mut underline = false;
    let mut color_stack: Vec<[u8; 4]> = Vec::new();

    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Backslash escape
        if chars[i] == '\\' && i + 1 < len {
            let next = chars[i + 1];
            if matches!(next, '*' | '_' | '[' | '\\') {
                current.text.push(next);
                i += 2;
                continue;
            }
        }

        // Bold: **
        if chars[i] == '*' && i + 1 < len && chars[i + 1] == '*' {
            // Check if there's a matching closing **
            if !bold && has_closing(&chars, i + 2, "**") {
                push_span(&mut spans, &mut current);
                bold = true;
                update_style(&mut current, bold, italic, underline, &color_stack);
                i += 2;
                continue;
            } else if bold {
                push_span(&mut spans, &mut current);
                bold = false;
                update_style(&mut current, bold, italic, underline, &color_stack);
                i += 2;
                continue;
            }
        }

        // Italic: single *
        if chars[i] == '*' && !(i + 1 < len && chars[i + 1] == '*') {
            if !italic && has_closing(&chars, i + 1, "*") {
                push_span(&mut spans, &mut current);
                italic = true;
                update_style(&mut current, bold, italic, underline, &color_stack);
                i += 1;
                continue;
            } else if italic {
                push_span(&mut spans, &mut current);
                italic = false;
                update_style(&mut current, bold, italic, underline, &color_stack);
                i += 1;
                continue;
            }
        }

        // Underline: __
        if chars[i] == '_' && i + 1 < len && chars[i + 1] == '_' {
            if !underline && has_closing(&chars, i + 2, "__") {
                push_span(&mut spans, &mut current);
                underline = true;
                update_style(&mut current, bold, italic, underline, &color_stack);
                i += 2;
                continue;
            } else if underline {
                push_span(&mut spans, &mut current);
                underline = false;
                update_style(&mut current, bold, italic, underline, &color_stack);
                i += 2;
                continue;
            }
        }

        // Colour: [#RRGGBB]...[/]
        if chars[i] == '['
            && i + 8 < len
            && chars[i + 1] == '#'
            && let Some(color_val) = parse_color_tag(&chars, i)
            && has_closing(&chars, i + 9, "[/]")
        {
            push_span(&mut spans, &mut current);
            color_stack.push(color_val);
            update_style(&mut current, bold, italic, underline, &color_stack);
            i += 9; // skip [#RRGGBB]
            continue;
        }

        // Colour close: [/]
        if chars[i] == '['
            && i + 2 < len
            && chars[i + 1] == '/'
            && chars[i + 2] == ']'
            && !color_stack.is_empty()
        {
            push_span(&mut spans, &mut current);
            color_stack.pop();
            update_style(&mut current, bold, italic, underline, &color_stack);
            i += 3;
            continue;
        }

        current.text.push(chars[i]);
        i += 1;
    }

    push_span(&mut spans, &mut current);

    // Merge adjacent same-style spans
    merge_spans(&mut spans);

    spans
}

/// Strip markup, returning plain text with escapes resolved.
pub fn strip_markup(input: &str) -> String {
    parse_markup(input)
        .into_iter()
        .map(|span| span.text)
        .collect()
}

/// Escape text so that `strip_markup(&escape_markup(s)) == s` for any `s`.
pub fn escape_markup(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '*' | '_' | '[' | '\\' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}

// ---- internal helpers ----

fn push_span(spans: &mut Vec<Span>, current: &mut Span) {
    if !current.text.is_empty() {
        spans.push(std::mem::replace(current, Span::new()));
        // Carry style forward
        if let Some(last) = spans.last() {
            current.bold = last.bold;
            current.italic = last.italic;
            current.underline = last.underline;
            current.color = last.color;
        }
    }
}

fn update_style(
    current: &mut Span,
    bold: bool,
    italic: bool,
    underline: bool,
    color_stack: &[[u8; 4]],
) {
    current.bold = bold;
    current.italic = italic;
    current.underline = underline;
    current.color = color_stack.last().copied();
}

fn merge_spans(spans: &mut Vec<Span>) {
    let mut merged: Vec<Span> = Vec::with_capacity(spans.len());
    for span in spans.drain(..) {
        if let Some(last) = merged.last_mut()
            && last.style_eq(&span)
        {
            last.text.push_str(&span.text);
            continue;
        }
        if !span.text.is_empty() {
            merged.push(span);
        }
    }
    *spans = merged;
}

/// Check if a closing marker exists starting from position `from`.
fn has_closing(chars: &[char], from: usize, marker: &str) -> bool {
    let marker_chars: Vec<char> = marker.chars().collect();
    let mlen = marker_chars.len();
    let mut i = from;
    while i + mlen <= chars.len() {
        // Skip escaped chars
        if chars[i] == '\\' {
            i += 2;
            continue;
        }
        if chars[i..i + mlen] == marker_chars[..] {
            return true;
        }
        i += 1;
    }
    false
}

/// Try to parse `[#RRGGBB]` at position `i`. Returns the RGBA color if valid.
fn parse_color_tag(chars: &[char], i: usize) -> Option<[u8; 4]> {
    // Need [#RRGGBB] = 9 chars
    if i + 8 >= chars.len() {
        return None;
    }
    if chars[i] != '[' || chars[i + 1] != '#' || chars[i + 8] != ']' {
        return None;
    }
    let hex: String = chars[i + 2..i + 8].iter().collect();
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some([r, g, b, 255])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_single_span() {
        let spans = parse_markup("Hello world");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "Hello world");
        assert!(!spans[0].bold);
        assert!(!spans[0].italic);
        assert!(!spans[0].underline);
        assert_eq!(spans[0].color, None);
    }

    #[test]
    fn bold_text() {
        let spans = parse_markup("say **hello** world");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].text, "say ");
        assert!(!spans[0].bold);
        assert_eq!(spans[1].text, "hello");
        assert!(spans[1].bold);
        assert_eq!(spans[2].text, " world");
        assert!(!spans[2].bold);
    }

    #[test]
    fn italic_text() {
        let spans = parse_markup("say *hello* world");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[1].text, "hello");
        assert!(spans[1].italic);
    }

    #[test]
    fn underline_text() {
        let spans = parse_markup("say __hello__ world");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[1].text, "hello");
        assert!(spans[1].underline);
    }

    #[test]
    fn nested_bold_italic() {
        let spans = parse_markup("**bold *and italic* only bold**");
        assert_eq!(spans.len(), 3);
        assert!(spans[0].bold);
        assert!(!spans[0].italic);
        assert_eq!(spans[0].text, "bold ");
        assert!(spans[1].bold);
        assert!(spans[1].italic);
        assert_eq!(spans[1].text, "and italic");
        assert!(spans[2].bold);
        assert!(!spans[2].italic);
        assert_eq!(spans[2].text, " only bold");
    }

    #[test]
    fn colour_text() {
        let spans = parse_markup("normal [#FF0000]red text[/] normal");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].text, "normal ");
        assert_eq!(spans[0].color, None);
        assert_eq!(spans[1].text, "red text");
        assert_eq!(spans[1].color, Some([255, 0, 0, 255]));
        assert_eq!(spans[2].text, " normal");
        assert_eq!(spans[2].color, None);
    }

    #[test]
    fn escape_star() {
        let spans = parse_markup("hello \\* world");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "hello * world");
    }

    #[test]
    fn escape_underscore() {
        let spans = parse_markup("a \\_ b");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "a _ b");
    }

    #[test]
    fn escape_bracket() {
        let spans = parse_markup("\\[not colour]");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "[not colour]");
    }

    #[test]
    fn escape_backslash() {
        let spans = parse_markup("a \\\\ b");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "a \\ b");
    }

    #[test]
    fn unclosed_bold_renders_literally() {
        let spans = parse_markup("hello **world");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "hello **world");
        assert!(!spans[0].bold);
    }

    #[test]
    fn unclosed_italic_renders_literally() {
        let spans = parse_markup("hello *world");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "hello *world");
    }

    #[test]
    fn unclosed_underline_renders_literally() {
        let spans = parse_markup("hello __world");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "hello __world");
    }

    #[test]
    fn unclosed_colour_renders_literally() {
        let spans = parse_markup("[#FF0000]red without close");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "[#FF0000]red without close");
    }

    #[test]
    fn newlines_preserved() {
        let spans = parse_markup("line1\n**bold\nline**\nend");
        // Should have spans preserving newlines
        let full_text: String = spans.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(full_text, "line1\nbold\nline\nend");
    }

    #[test]
    fn strip_markup_removes_formatting() {
        assert_eq!(
            strip_markup("**bold** *italic* __under__ [#FF0000]red[/]"),
            "bold italic under red"
        );
    }

    #[test]
    fn strip_markup_resolves_escapes() {
        assert_eq!(strip_markup("\\* \\_ \\[ \\\\"), "* _ [ \\");
    }

    #[test]
    fn escape_then_strip_roundtrips() {
        let cases = [
            "hello",
            "**bold**",
            "*italic*",
            "__underline__",
            "[#FF0000]red[/]",
            "\\backslash\\",
            "a * b _ c [ d",
            "",
            "***triple***",
            "____quad____",
            "normal text with no markup",
        ];
        for s in &cases {
            assert_eq!(
                strip_markup(&escape_markup(s)),
                *s,
                "roundtrip failed for {:?}",
                s
            );
        }
    }

    #[test]
    fn escape_then_strip_roundtrips_unicode() {
        let cases = [
            "日本語テスト",
            "émoji 🎵 *star*",
            "café __underline__",
            "[brackets] and **stars**",
            "mixed *italic* and [#AABBCC]colour[/]",
        ];
        for s in &cases {
            assert_eq!(
                strip_markup(&escape_markup(s)),
                *s,
                "roundtrip failed for {:?}",
                s
            );
        }
    }

    #[test]
    fn escape_then_strip_roundtrips_generated_strings() {
        let alphabet = ['*', '_', '[', ']', '\\', 'a', 'é', '🎵', '\n'];
        for seed in 0_u32..512 {
            let mut value = seed.wrapping_add(1);
            let mut input = String::new();
            for _ in 0..(seed as usize % 31) {
                value = value.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                input.push(alphabet[value as usize % alphabet.len()]);
            }
            assert_eq!(
                strip_markup(&escape_markup(&input)),
                input,
                "roundtrip failed for {input:?}"
            );
        }
    }

    #[test]
    fn adjacent_same_style_merged() {
        // Two bold regions back to back should merge
        let spans = parse_markup("**a****b**");
        assert_eq!(spans.len(), 1);
        assert!(spans[0].bold);
        assert_eq!(spans[0].text, "ab");
    }

    #[test]
    fn empty_input() {
        assert!(parse_markup("").is_empty());
        assert_eq!(strip_markup(""), "");
        assert_eq!(escape_markup(""), "");
    }
}
