use std::{env, fs, path::PathBuf};

use lumen_core::{TextAlign, VerticalAlign, markup::Span};
use lumen_text::{TextRenderer, TextRequest};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = env::args_os().nth(1).map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../../../.local/reports/b4-sample.png")
    });
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let spans = [Span {
        text: "વચનામૃત\nશ્રી સ્વામિનારાયણ".into(),
        bold: true,
        italic: false,
        underline: false,
        color: None,
    }];
    let request = TextRequest {
        spans: &spans,
        width_px: 1280,
        height_px: 720,
        font_px: 96.0,
        color: [255, 255, 255, 255],
        outline: Some((2.0, [0, 0, 0, 255])),
        shadow: true,
        align: TextAlign::Center,
        valign: VerticalAlign::Middle,
        line_height: 1.25,
        shrink_to_fit: true,
    };
    let image = TextRenderer::new().render(&request)?;
    let rgba = image::RgbaImage::from_raw(image.width, image.height, image.pixels.clone())
        .ok_or("renderer returned an invalid RGBA buffer")?;
    rgba.save(&output)?;
    println!("wrote {}", output.display());
    Ok(())
}
