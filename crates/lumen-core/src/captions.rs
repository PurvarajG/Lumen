use unicode_segmentation::UnicodeSegmentation;

/// Wrap plain caption text into chunks of at most three lines, preserving grapheme clusters.
pub fn caption_chunks(text: &str, max_chars: usize) -> Vec<String> {
    let limit = max_chars.max(1);
    let mut lines = Vec::new();
    for source_line in text.lines() {
        let mut line = String::new();
        let mut columns = 0;
        for word in source_line.split_whitespace() {
            if !line.is_empty() {
                if columns + 1 + word.chars().count() > limit {
                    lines.push(std::mem::take(&mut line));
                    columns = 0;
                } else {
                    line.push(' ');
                    columns += 1;
                }
            }
            for cluster in word.graphemes(true) {
                let width = cluster.chars().count();
                if columns > 0 && columns + width > limit {
                    lines.push(std::mem::take(&mut line));
                    columns = 0;
                }
                line.push_str(cluster);
                columns += width;
            }
        }
        if !line.is_empty() {
            lines.push(line);
        }
    }
    lines.chunks(3).map(|lines| lines.join("\n")).collect()
}
