//! Shaped slide text rendering for Lumen Presenter.

use std::{
    collections::{HashMap, VecDeque},
    fs,
    path::Path,
    sync::Arc,
};

use cosmic_text::{
    Align, Attrs, Buffer, Color, Family, FontSystem, Metrics, Renderer, Shaping, Style, SwashCache,
    UnderlineStyle, Weight, Wrap, render_decoration,
};
use lumen_core::{TextAlign, VerticalAlign, markup::Span};
use thiserror::Error;

const DEFAULT_CACHE_BUDGET: usize = 64 * 1024 * 1024;
const MAX_DIMENSION: u32 = 4096;
const CHITRA_LIMIT: u64 = 8 * 1024 * 1024;

const POPPINS_REGULAR: &[u8] = include_bytes!("../fonts/Poppins-Regular.ttf");
const POPPINS_MEDIUM: &[u8] = include_bytes!("../fonts/Poppins-Medium.ttf");
const POPPINS_SEMIBOLD: &[u8] = include_bytes!("../fonts/Poppins-SemiBold.ttf");
const POPPINS_BOLD: &[u8] = include_bytes!("../fonts/Poppins-Bold.ttf");
const POPPINS_ITALIC: &[u8] = include_bytes!("../fonts/Poppins-Italic.ttf");
const POPPINS_BOLD_ITALIC: &[u8] = include_bytes!("../fonts/Poppins-BoldItalic.ttf");
const NOTO_REGULAR: &[u8] = include_bytes!("../fonts/NotoSansGujarati-Regular.ttf");
const NOTO_BOLD: &[u8] = include_bytes!("../fonts/NotoSansGujarati-Bold.ttf");

/// One styled text request. Dimensions and font size are measured in physical pixels.
#[derive(Clone, Debug)]
pub struct TextRequest<'a> {
    pub spans: &'a [Span],
    pub width_px: u32,
    pub height_px: u32,
    pub font_px: f32,
    pub color: [u8; 4],
    pub outline: Option<(f32, [u8; 4])>,
    pub shadow: bool,
    pub align: TextAlign,
    pub valign: VerticalAlign,
    pub line_height: f32,
    pub shrink_to_fit: bool,
}

/// Unmultiplied RGBA pixels, ready for `ColorImage::from_rgba_unmultiplied`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RgbaImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum TextError {
    #[error("invalid text box dimensions {width}x{height}; expected 1..=4096")]
    InvalidDimensions { width: u32, height: u32 },
    #[error("font size {0} is invalid; expected finite 4..=600")]
    InvalidFontSize(f32),
    #[error("line height {0} is invalid; expected finite 0.5..=4")]
    InvalidLineHeight(f32),
    #[error("outline width {0} is invalid; expected finite 0..=20")]
    InvalidOutline(f32),
    #[error("rendered image allocation overflow")]
    ImageTooLarge,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct SpanKey {
    text: String,
    bold: bool,
    italic: bool,
    underline: bool,
    color: Option<[u8; 4]>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct RequestKey {
    spans: Vec<SpanKey>,
    width_px: u32,
    height_px: u32,
    font_px: u32,
    color: [u8; 4],
    outline: Option<(u32, [u8; 4])>,
    shadow: bool,
    align: u8,
    valign: u8,
    line_height: u32,
    shrink_to_fit: bool,
}

impl RequestKey {
    fn from_request(req: &TextRequest<'_>) -> Self {
        Self {
            spans: req
                .spans
                .iter()
                .map(|span| SpanKey {
                    text: span.text.clone(),
                    bold: span.bold,
                    italic: span.italic,
                    underline: span.underline,
                    color: span.color,
                })
                .collect(),
            width_px: req.width_px,
            height_px: req.height_px,
            font_px: req.font_px.to_bits(),
            color: req.color,
            outline: req.outline.map(|(width, color)| (width.to_bits(), color)),
            shadow: req.shadow,
            align: match req.align {
                TextAlign::Left => 0,
                TextAlign::Center => 1,
                TextAlign::Right => 2,
            },
            valign: match req.valign {
                VerticalAlign::Top => 0,
                VerticalAlign::Middle => 1,
                VerticalAlign::Bottom => 2,
            },
            line_height: req.line_height.to_bits(),
            shrink_to_fit: req.shrink_to_fit,
        }
    }
}

/// Owns the font database, rasterizer and a byte-bounded LRU image cache.
pub struct TextRenderer {
    fonts: FontSystem,
    swash: SwashCache,
    cache: HashMap<RequestKey, Arc<RgbaImage>>,
    lru: VecDeque<RequestKey>,
    cache_bytes: usize,
    cache_budget: usize,
    gujarati_face: &'static str,
    gujarati_scale: f32,
    #[cfg(test)]
    face_names: HashMap<cosmic_text::fontdb::ID, String>,
}

impl Default for TextRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl TextRenderer {
    pub fn new() -> Self {
        Self::with_cache_budget(DEFAULT_CACHE_BUDGET)
    }

    pub fn with_cache_budget(cache_budget: usize) -> Self {
        let mut db = cosmic_text::fontdb::Database::new();
        for bytes in [
            POPPINS_REGULAR,
            POPPINS_MEDIUM,
            POPPINS_SEMIBOLD,
            POPPINS_BOLD,
            POPPINS_ITALIC,
            POPPINS_BOLD_ITALIC,
            NOTO_REGULAR,
            NOTO_BOLD,
        ] {
            db.load_font_data(bytes.to_vec());
        }
        let chitra_face = load_chitra_fonts(&mut db);
        let gujarati_face = if chitra_face {
            "ChitraUni"
        } else {
            "Noto Sans Gujarati"
        };
        let gujarati_scale = measured_gujarati_scale(&db, gujarati_face);
        #[cfg(test)]
        let face_names = db
            .faces()
            .flat_map(|face| {
                let name = face
                    .families
                    .first()
                    .map(|(family, _)| family.clone())
                    .unwrap_or_default();
                [(face.id, name)]
            })
            .collect::<HashMap<_, _>>();
        db.set_sans_serif_family("Poppins");
        let fonts = FontSystem::new_with_locale_and_db("en-US".into(), db);
        Self {
            fonts,
            swash: SwashCache::new(),
            cache: HashMap::new(),
            lru: VecDeque::new(),
            cache_bytes: 0,
            cache_budget,
            gujarati_face,
            gujarati_scale,
            #[cfg(test)]
            face_names,
        }
    }

    /// Gujarati typeface used by this renderer.
    pub fn gujarati_face(&self) -> &'static str {
        self.gujarati_face
    }

    pub fn render(&mut self, req: &TextRequest<'_>) -> Result<Arc<RgbaImage>, TextError> {
        validate(req)?;
        let key = RequestKey::from_request(req);
        if let Some(image) = self.cache.get(&key).cloned() {
            self.lru.retain(|entry| entry != &key);
            self.lru.push_back(key);
            return Ok(image);
        }
        let (used_font, total_height, buffer) = self.layout(req);
        let mut ink = vec![0; checked_pixels(req.width_px, req.height_px)?];
        let y_offset = match req.valign {
            VerticalAlign::Top => 0,
            VerticalAlign::Middle => ((req.height_px as f32 - total_height) / 2.0).max(0.0) as i32,
            VerticalAlign::Bottom => (req.height_px as f32 - total_height).max(0.0) as i32,
        };
        let width = req.width_px as i32;
        let height = req.height_px as i32;
        let rgba = &mut ink;
        let default_color = to_color(req.color);
        for run in buffer.layout_runs() {
            for glyph in run.glyphs {
                let color = glyph.color_opt.unwrap_or(default_color);
                let physical = glyph.physical((0.0, run.line_y), 1.0);
                self.swash.with_pixels(
                    &mut self.fonts,
                    physical.cache_key,
                    color,
                    |x, y, coverage| {
                        let px = physical.x + x;
                        let py = physical.y + y + y_offset;
                        if px < 0 || py < 0 || px >= width || py >= height {
                            return;
                        }
                        let alpha = (u16::from(coverage.a()) * u16::from(color.a()) / 255) as u8;
                        blend(
                            rgba,
                            (py as u32 * req.width_px + px as u32) as usize * 4,
                            [color.r(), color.g(), color.b(), alpha],
                        );
                    },
                );
            }
            let mut canvas = CanvasRenderer {
                pixels: rgba,
                width: req.width_px,
                height: req.height_px,
                y_offset,
            };
            render_decoration(&mut canvas, &run, default_color);
        }
        let mut pixels = vec![0; ink.len()];
        if req.shadow {
            draw_shadow(&ink, &mut pixels, req.width_px, req.height_px, used_font);
        }
        if let Some((outline_width, outline_color)) = req.outline {
            draw_outline(
                &ink,
                &mut pixels,
                req.width_px,
                req.height_px,
                outline_width.ceil() as i32,
                outline_color,
            );
        }
        composite(&mut pixels, &ink);
        let image = Arc::new(RgbaImage {
            width: req.width_px,
            height: req.height_px,
            pixels,
        });
        let bytes = image.pixels.len();
        if bytes <= self.cache_budget {
            while self.cache_bytes + bytes > self.cache_budget {
                let Some(oldest) = self.lru.pop_front() else {
                    break;
                };
                if let Some(removed) = self.cache.remove(&oldest) {
                    self.cache_bytes -= removed.pixels.len();
                }
            }
            self.cache_bytes += bytes;
            self.lru.push_back(key.clone());
            self.cache.insert(key, image.clone());
        }
        Ok(image)
    }

    /// Return the selected font size and laid-out text height in pixels.
    pub fn measure(&mut self, req: &TextRequest<'_>) -> (f32, u32) {
        if validate(req).is_err() {
            return (req.font_px, 0);
        }
        let (used, height, _) = self.layout(req);
        (used, height.ceil() as u32)
    }

    pub fn cache_len(&self) -> usize {
        self.cache.len()
    }

    fn layout(&mut self, req: &TextRequest<'_>) -> (f32, f32, Buffer) {
        let (size, height, buffer) = self.layout_at(req, req.font_px);
        if !req.shrink_to_fit || height <= req.height_px as f32 {
            return (size, height, buffer);
        }
        let min_size = req.font_px * 0.4;
        let mut used = req.font_px;
        let mut layout = (size, height, buffer);
        while layout.1 > req.height_px as f32 && used > min_size {
            used = (used - 0.5).max(min_size);
            layout = self.layout_at(req, used);
        }
        (used, layout.1, layout.2)
    }

    fn layout_at(&mut self, req: &TextRequest<'_>, font_px: f32) -> (f32, f32, Buffer) {
        let line_px = font_px * req.line_height;
        let mut buffer = Buffer::new(&mut self.fonts, Metrics::new(font_px, line_px));
        buffer.set_size(Some(req.width_px as f32), None);
        buffer.set_wrap(Wrap::WordOrGlyph);
        let spans = styled_runs(req, self.gujarati_face, font_px, self.gujarati_scale);
        let default = Attrs::new().family(Family::Name("Poppins"));
        buffer.set_rich_text(
            spans,
            &default,
            Shaping::Advanced,
            Some(to_align(req.align)),
        );
        buffer.shape_until_scroll(&mut self.fonts, false);
        let height = buffer.layout_runs().map(|run| run.line_height).sum::<f32>();
        (font_px, height, buffer)
    }

    #[cfg(test)]
    fn shape_for_test(&mut self, text: &str, bold: bool, italic: bool) -> ShapeReport {
        let span = Span {
            text: text.into(),
            bold,
            italic,
            underline: false,
            color: None,
        };
        let req = TextRequest {
            spans: std::slice::from_ref(&span),
            width_px: 1000,
            height_px: 200,
            font_px: 48.0,
            color: [255; 4],
            outline: None,
            shadow: false,
            align: TextAlign::Left,
            valign: VerticalAlign::Top,
            line_height: 1.2,
            shrink_to_fit: false,
        };
        let (_, _, buffer) = self.layout(&req);
        let mut report = ShapeReport { glyphs: Vec::new() };
        for run in buffer.layout_runs() {
            for glyph in run.glyphs {
                report.glyphs.push(ShapedGlyph {
                    face: self
                        .face_names
                        .get(&glyph.font_id)
                        .cloned()
                        .unwrap_or_default(),
                    glyph_id: glyph.glyph_id,
                });
            }
        }
        report
    }
}

// Gujarati has no Latin x-height; measure the unmarked consonant body instead.
fn measured_gujarati_scale(db: &cosmic_text::fontdb::Database, family: &str) -> f32 {
    use cosmic_text::skrifa::{
        FontRef, MetadataProvider,
        instance::{LocationRef, Size},
    };
    let height = |name: &str, ch: char| -> Option<f32> {
        let id = db.query(&cosmic_text::fontdb::Query {
            families: &[cosmic_text::fontdb::Family::Name(name)],
            ..Default::default()
        })?;
        db.with_face_data(id, |data, index| {
            let font = FontRef::from_index(data, index).ok()?;
            let glyph = font.charmap().map(ch)?;
            let bounds = font
                .glyph_metrics(Size::new(1.0), LocationRef::default())
                .bounds(glyph)?;
            Some(bounds.y_max - bounds.y_min)
        })?
    };
    match (height("Poppins", 'x'), height(family, 'ક')) {
        (Some(latin), Some(gujarati)) if gujarati > 0.0 => (latin / gujarati).clamp(1.0, 1.6),
        _ => 1.0,
    }
}

fn load_chitra_fonts(db: &mut cosmic_text::fontdb::Database) -> bool {
    let mut loaded = false;
    for dir in [
        user_font_dir(),
        Some(std::path::PathBuf::from("/Library/Fonts")),
    ]
    .into_iter()
    .flatten()
    {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !name.to_ascii_lowercase().contains("chitra") {
                continue;
            }
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if metadata.len() > CHITRA_LIMIT {
                continue;
            }
            let Ok(bytes) = fs::read(&path) else {
                continue;
            };
            let mut candidate = cosmic_text::fontdb::Database::new();
            candidate.load_font_data(bytes.clone());
            let families = candidate
                .faces()
                .flat_map(|face| face.families.iter().map(|(f, _)| f.clone()))
                .collect::<Vec<_>>();
            if !families
                .iter()
                .any(|family| family.eq_ignore_ascii_case("ChitraUni"))
            {
                continue;
            }
            db.load_font_data(bytes);
            loaded = true;
            let _ = path;
        }
    }
    loaded
}

fn user_font_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").map(|home| Path::new(&home).join("Library/Fonts"))
}

fn styled_runs<'a>(
    req: &'a TextRequest<'_>,
    gujarati_face: &'static str,
    font_px: f32,
    gujarati_scale: f32,
) -> Vec<(&'a str, Attrs<'static>)> {
    let mut result = Vec::new();
    for span in req.spans {
        let attrs_for = |gujarati: bool| {
            let family = if gujarati { gujarati_face } else { "Poppins" };
            let weight = if span.bold {
                Weight::BOLD
            } else {
                Weight::NORMAL
            };
            let mut attrs = Attrs::new().family(Family::Name(family)).weight(weight);
            if gujarati {
                attrs = attrs.metrics(Metrics::new(
                    font_px * gujarati_scale,
                    font_px * req.line_height,
                ));
            }
            if span.italic {
                attrs = attrs.style(Style::Italic);
            }
            if let Some(color) = span.color {
                attrs = attrs.color(to_color(color));
            }
            if span.underline {
                attrs = attrs.underline(UnderlineStyle::Single);
            }
            attrs
        };
        let mut start = 0;
        let mut current = None;
        for (index, ch) in span.text.char_indices() {
            let gujarati = ('\u{0A80}'..='\u{0AFF}').contains(&ch);
            if current.is_some_and(|value| value != gujarati) {
                result.push((
                    &span.text[start..index],
                    attrs_for(current.unwrap_or(false)),
                ));
                start = index;
            }
            current = Some(gujarati);
        }
        if start < span.text.len() {
            result.push((&span.text[start..], attrs_for(current.unwrap_or(false))));
        }
    }
    result
}

fn validate(req: &TextRequest<'_>) -> Result<(), TextError> {
    if req.width_px == 0
        || req.height_px == 0
        || req.width_px > MAX_DIMENSION
        || req.height_px > MAX_DIMENSION
    {
        return Err(TextError::InvalidDimensions {
            width: req.width_px,
            height: req.height_px,
        });
    }
    if !req.font_px.is_finite() || !(4.0..=600.0).contains(&req.font_px) {
        return Err(TextError::InvalidFontSize(req.font_px));
    }
    if !req.line_height.is_finite() || !(0.5..=4.0).contains(&req.line_height) {
        return Err(TextError::InvalidLineHeight(req.line_height));
    }
    if let Some((width, _)) = req.outline
        && (!width.is_finite() || !(0.0..=20.0).contains(&width))
    {
        return Err(TextError::InvalidOutline(width));
    }
    Ok(())
}

fn checked_pixels(width: u32, height: u32) -> Result<usize, TextError> {
    (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or(TextError::ImageTooLarge)
}

fn to_color(color: [u8; 4]) -> Color {
    Color::rgba(color[0], color[1], color[2], color[3])
}
fn to_align(align: TextAlign) -> Align {
    match align {
        TextAlign::Left => Align::Left,
        TextAlign::Center => Align::Center,
        TextAlign::Right => Align::Right,
    }
}

struct CanvasRenderer<'a> {
    pixels: &'a mut [u8],
    width: u32,
    height: u32,
    y_offset: i32,
}

impl Renderer for CanvasRenderer<'_> {
    fn rectangle(&mut self, x: i32, y: i32, width: u32, height: u32, color: Color) {
        for dy in 0..height as i32 {
            for dx in 0..width as i32 {
                let px = x + dx;
                let py = y + self.y_offset + dy;
                if px >= 0 && py >= 0 && px < self.width as i32 && py < self.height as i32 {
                    blend(
                        self.pixels,
                        (py as u32 * self.width + px as u32) as usize * 4,
                        color.as_rgba(),
                    );
                }
            }
        }
    }

    fn glyph(&mut self, _glyph: cosmic_text::PhysicalGlyph, _color: Color) {}
}

fn blend(pixels: &mut [u8], index: usize, color: [u8; 4]) {
    let Some(dst) = pixels.get_mut(index..index + 4) else {
        return;
    };
    let source_alpha = u32::from(color[3]);
    let dest_alpha = u32::from(dst[3]);
    let out_alpha = source_alpha + (dest_alpha * (255 - source_alpha) + 127) / 255;
    if out_alpha == 0 {
        return;
    }
    for channel in 0..3 {
        let src = u32::from(color[channel]) * source_alpha;
        let dest = u32::from(dst[channel]) * dest_alpha;
        dst[channel] =
            ((src + (dest * (255 - source_alpha) + 127) / 255 + out_alpha / 2) / out_alpha) as u8;
    }
    dst[3] = out_alpha as u8;
}

fn draw_outline(
    ink: &[u8],
    output: &mut [u8],
    width: u32,
    height: u32,
    radius: i32,
    color: [u8; 4],
) {
    if radius <= 0 {
        return;
    }
    let source = alpha_plane(ink);
    let expanded = max_filter(&source, width as usize, height as usize, radius as usize);
    for (pixel_index, alpha) in expanded.into_iter().enumerate() {
        let i = pixel_index * 4;
        if alpha > ink[i + 3] {
            blend(
                output,
                i,
                [
                    color[0],
                    color[1],
                    color[2],
                    (u16::from(alpha) * u16::from(color[3]) / 255) as u8,
                ],
            );
        }
    }
}

fn draw_shadow(ink: &[u8], output: &mut [u8], width: u32, height: u32, font_px: f32) {
    let radius = (font_px / 18.0).round().clamp(1.0, 8.0) as i32;
    let dx = (font_px / 12.0).round().clamp(1.0, 12.0) as i32;
    let dy = dx;
    let blurred = box_blur(
        &alpha_plane(ink),
        width as usize,
        height as usize,
        radius as usize,
    );
    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let sx = x - dx;
            let sy = y - dy;
            if sx >= 0 && sy >= 0 && sx < width as i32 && sy < height as i32 {
                let alpha = blurred[sy as usize * width as usize + sx as usize] / 2;
                let i = (y as u32 * width + x as u32) as usize * 4;
                blend(output, i, [0, 0, 0, alpha]);
            }
        }
    }
}

fn alpha_plane(rgba: &[u8]) -> Vec<u8> {
    rgba.as_chunks::<4>()
        .0
        .iter()
        .map(|pixel| pixel[3])
        .collect()
}

fn max_filter(input: &[u8], width: usize, height: usize, radius: usize) -> Vec<u8> {
    let mut horizontal = vec![0; input.len()];
    let mut output = vec![0; input.len()];
    for y in 0..height {
        let row = &input[y * width..(y + 1) * width];
        filter_max_line(row, &mut horizontal[y * width..(y + 1) * width], radius);
    }
    for x in 0..width {
        let column = (0..height)
            .map(|y| horizontal[y * width + x])
            .collect::<Vec<_>>();
        let mut filtered = vec![0; height];
        filter_max_line(&column, &mut filtered, radius);
        for (y, alpha) in filtered.into_iter().enumerate() {
            output[y * width + x] = alpha;
        }
    }
    output
}

fn filter_max_line(input: &[u8], output: &mut [u8], radius: usize) {
    let mut candidates = VecDeque::<usize>::new();
    let mut next = 0;
    for (position, pixel) in output.iter_mut().enumerate() {
        let right = position
            .saturating_add(radius)
            .min(input.len().saturating_sub(1));
        while next <= right && next < input.len() {
            while candidates.back().is_some_and(|&i| input[i] <= input[next]) {
                candidates.pop_back();
            }
            candidates.push_back(next);
            next += 1;
        }
        while candidates
            .front()
            .is_some_and(|&i| i.saturating_add(radius) < position)
        {
            candidates.pop_front();
        }
        *pixel = candidates.front().map_or(0, |&i| input[i]);
    }
}

fn box_blur(input: &[u8], width: usize, height: usize, radius: usize) -> Vec<u8> {
    let mut horizontal = vec![0; input.len()];
    let mut output = vec![0; input.len()];
    for y in 0..height {
        let row = &input[y * width..(y + 1) * width];
        let filtered = &mut horizontal[y * width..(y + 1) * width];
        let mut sum = row[..=radius.min(width.saturating_sub(1))]
            .iter()
            .map(|&n| u32::from(n))
            .sum::<u32>();
        for x in 0..width {
            let left = x.saturating_sub(radius);
            let right = x.saturating_add(radius).min(width - 1);
            filtered[x] = (sum / (right - left + 1) as u32) as u8;
            if x >= radius {
                sum -= u32::from(row[x - radius]);
            }
            if let Some(&next) = row.get(x + radius + 1) {
                sum += u32::from(next);
            }
        }
    }
    for x in 0..width {
        let mut sum = (0..=radius.min(height.saturating_sub(1)))
            .map(|y| u32::from(horizontal[y * width + x]))
            .sum::<u32>();
        for y in 0..height {
            let top = y.saturating_sub(radius);
            let bottom = y.saturating_add(radius).min(height - 1);
            output[y * width + x] = (sum / (bottom - top + 1) as u32) as u8;
            if y >= radius {
                sum -= u32::from(horizontal[(y - radius) * width + x]);
            }
            if y + radius + 1 < height {
                sum += u32::from(horizontal[(y + radius + 1) * width + x]);
            }
        }
    }
    output
}

fn composite(output: &mut [u8], foreground: &[u8]) {
    for (i, pixel) in foreground.as_chunks::<4>().0.iter().enumerate() {
        if pixel[3] != 0 {
            blend(output, i * 4, [pixel[0], pixel[1], pixel[2], pixel[3]]);
        }
    }
}

#[cfg(test)]
#[derive(Debug)]
struct ShapeReport {
    glyphs: Vec<ShapedGlyph>,
}
#[cfg(test)]
#[derive(Debug)]
struct ShapedGlyph {
    face: String,
    glyph_id: u16,
}

#[cfg(test)]
mod tests {
    use super::*;
    use lumen_core::{TextAlign, VerticalAlign, markup::Span};

    fn span(text: &str) -> Span {
        Span {
            text: text.into(),
            bold: false,
            italic: false,
            underline: false,
            color: None,
        }
    }

    fn request<'a>(spans: &'a [Span]) -> TextRequest<'a> {
        TextRequest {
            spans,
            width_px: 400,
            height_px: 240,
            font_px: 48.0,
            color: [255; 4],
            outline: None,
            shadow: false,
            align: TextAlign::Center,
            valign: VerticalAlign::Middle,
            line_height: 1.2,
            shrink_to_fit: false,
        }
    }

    #[test]
    fn gujarati_conjuncts_shape_as_ligatures_with_active_face() {
        let mut renderer = TextRenderer::new();
        for word in ["ક્ષ", "શ્રી"] {
            let shaped = renderer.shape_for_test(word, false, false);
            assert!(
                shaped.glyphs.len() < word.chars().count(),
                "{word}: {:?}",
                shaped.glyphs
            );
            assert!(
                shaped
                    .glyphs
                    .iter()
                    .all(|g| g.face.contains(renderer.gujarati_face()))
            );
            assert!(shaped.glyphs.iter().all(|g| g.glyph_id > 0));
        }
    }

    #[test]
    fn mixed_line_uses_latin_and_gujarati_faces() {
        let mut renderer = TextRenderer::new();
        let shaped = renderer.shape_for_test("Vachanamrut વચનામૃત", false, false);
        assert!(shaped.glyphs.iter().any(|g| g.face.contains("Poppins")));
        assert!(
            shaped
                .glyphs
                .iter()
                .any(|g| g.face.contains(renderer.gujarati_face()))
        );
    }

    #[test]
    fn gujarati_sample_rasterizes_visible_ink() {
        let mut renderer = TextRenderer::new();
        let spans = [Span {
            bold: true,
            ..span("વચનામૃત\nશ્રી સ્વામિનારાયણ")
        }];
        let mut req = request(&spans);
        req.width_px = 1280;
        req.height_px = 720;
        req.font_px = 96.0;
        req.outline = Some((2.0, [0, 0, 0, 255]));
        req.shadow = true;
        let image = renderer.render(&req).unwrap();
        assert!(image.pixels.as_chunks::<4>().0.iter().any(|p| p[3] > 0));
        assert!(
            image
                .pixels
                .as_chunks::<4>()
                .0
                .iter()
                .any(|p| p[0] > 100 && p[1] > 100 && p[2] > 100 && p[3] > 0)
        );
    }

    #[test]
    fn wraps_at_box_width_and_shrink_fits() {
        let mut renderer = TextRenderer::new();
        let spans = [span("Vachanamrut વચનામૃત Vachanamrut વચનામૃત")];
        let mut req = request(&spans);
        req.width_px = 180;
        let (_, wrapped_height) = renderer.measure(&req);
        req.width_px = 1000;
        let (_, unwrapped_height) = renderer.measure(&req);
        assert!(wrapped_height > unwrapped_height);
        req.width_px = 1000;
        req.height_px = 40;
        req.shrink_to_fit = true;
        let (used, fitted_height) = renderer.measure(&req);
        assert!(used < req.font_px, "used={used}, height={wrapped_height}");
        assert!(
            fitted_height <= req.height_px,
            "used={used}, height={fitted_height}"
        );
    }

    #[test]
    fn alignment_and_vertical_alignment_position_ink() {
        let mut renderer = TextRenderer::new();
        let spans = [span("Hello")];
        let mut req = request(&spans);
        req.align = TextAlign::Left;
        req.valign = VerticalAlign::Top;
        let left_top = renderer.render(&req).unwrap();
        let a = ink_bounds(&left_top);
        req.align = TextAlign::Center;
        req.valign = VerticalAlign::Middle;
        let center_middle = renderer.render(&req).unwrap();
        let b = ink_bounds(&center_middle);
        req.align = TextAlign::Right;
        req.valign = VerticalAlign::Bottom;
        let right_bottom = renderer.render(&req).unwrap();
        let c = ink_bounds(&right_bottom);
        assert!(a.0 < b.0 && b.0 < c.0);
        assert!(a.1 < b.1 && b.1 < c.1);
    }

    #[test]
    fn decorations_and_span_styles_change_pixels() {
        let mut renderer = TextRenderer::new();
        let regular = [span("Text")];
        let bold = [Span {
            bold: true,
            ..span("Text")
        }];
        let colored = [Span {
            color: Some([255, 0, 0, 255]),
            ..span("Text")
        }];
        let mut base_req = request(&regular);
        base_req.align = TextAlign::Left;
        base_req.valign = VerticalAlign::Top;
        let base = renderer.render(&base_req).unwrap();
        let mut bold_req = request(&bold);
        bold_req.align = TextAlign::Left;
        bold_req.valign = VerticalAlign::Top;
        let bold_img = renderer.render(&bold_req).unwrap();
        assert_ne!(base.pixels, bold_img.pixels);
        let mut color_req = request(&colored);
        color_req.align = TextAlign::Left;
        color_req.valign = VerticalAlign::Top;
        let color_img = renderer.render(&color_req).unwrap();
        assert!(
            color_img
                .pixels
                .as_chunks::<4>()
                .0
                .iter()
                .any(|p| p[0] > 0 && p[1] == 0 && p[2] == 0)
        );
        let mut translucent_req = request(&regular);
        translucent_req.color = [255, 255, 255, 128];
        let translucent = renderer.render(&translucent_req).unwrap();
        assert!(
            translucent
                .pixels
                .as_chunks::<4>()
                .0
                .iter()
                .any(|p| p[3] > 0)
        );
        assert!(
            translucent
                .pixels
                .as_chunks::<4>()
                .0
                .iter()
                .all(|p| p[3] <= 128)
        );
        let underlined = [Span {
            underline: true,
            ..span("Text")
        }];
        let mut underline_req = request(&underlined);
        underline_req.align = TextAlign::Left;
        underline_req.valign = VerticalAlign::Top;
        assert_ne!(base.pixels, renderer.render(&underline_req).unwrap().pixels);
        for (outline, shadow) in [(Some((4.0, [0, 255, 0, 255])), false), (None, true)] {
            let mut effects_req = request(&regular);
            effects_req.align = TextAlign::Left;
            effects_req.valign = VerticalAlign::Top;
            effects_req.outline = outline;
            effects_req.shadow = shadow;
            let effects = renderer.render(&effects_req).unwrap();
            assert_ne!(base.pixels, effects.pixels);
            assert!(
                effects
                    .pixels
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .zip(base.pixels.as_chunks::<4>().0.iter())
                    .any(|(effect, plain)| effect[3] > 0 && plain[3] == 0)
            );
        }
    }

    #[test]
    fn cache_reuses_arcs_and_evicts_by_byte_budget() {
        let mut renderer = TextRenderer::with_cache_budget(400 * 240 * 4);
        let spans = [span("First")];
        let req = request(&spans);
        let a = renderer.render(&req).unwrap();
        let b = renderer.render(&req).unwrap();
        assert!(Arc::ptr_eq(&a, &b));
        let other = [span("Second")];
        let _ = renderer.render(&request(&other)).unwrap();
        assert_eq!(renderer.cache_len(), 1);
    }

    #[test]
    fn invalid_inputs_return_errors() {
        let mut renderer = TextRenderer::new();
        let spans = [span("Invalid")];
        let mut req = request(&spans);
        req.width_px = 0;
        assert!(renderer.render(&req).is_err());
        req.width_px = 100;
        req.font_px = f32::NAN;
        assert!(renderer.render(&req).is_err());
        req.font_px = 40.0;
        req.line_height = 0.0;
        assert!(renderer.render(&req).is_err());
    }

    #[test]
    #[ignore = "requires the commercial ChitraUni font installed locally"]
    fn chitrauni_face_is_used_when_installed() {
        let mut renderer = TextRenderer::new();
        assert_eq!(renderer.gujarati_face(), "ChitraUni");
        let shaped = renderer.shape_for_test("વચનામૃત", false, false);
        assert!(shaped.glyphs.iter().all(|g| g.face.contains("ChitraUni")));
    }

    #[test]
    #[ignore = "run with --release to record the rendering benchmark"]
    fn benchmark_1920_by_1080_gujarati() {
        let mut renderer = TextRenderer::new();
        let text = "વચનામૃતનું જ્ઞાન જીવનને પ્રકાશિત કરે છે\n".repeat(6);
        let spans = [span(&text)];
        let mut req = request(&spans);
        req.width_px = 1920;
        req.height_px = 1080;
        req.font_px = 72.0;
        let start = std::time::Instant::now();
        let _ = renderer.render(&req).unwrap();
        eprintln!(
            "1920x1080 Gujarati render: {:.2} ms",
            start.elapsed().as_secs_f64() * 1000.0
        );
    }

    fn ink_bounds(image: &RgbaImage) -> (u32, u32, u32, u32) {
        let mut min_x = image.width;
        let mut min_y = image.height;
        let mut max_x = 0;
        let mut max_y = 0;
        for (index, pixel) in image.pixels.as_chunks::<4>().0.iter().enumerate() {
            if pixel[3] > 0 {
                let x = index as u32 % image.width;
                let y = index as u32 / image.width;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
        (min_x, min_y, max_x, max_y)
    }
}

#[cfg(test)]
mod gujarati_scale_tests {
    use super::*;
    use cosmic_text::skrifa::{
        FontRef, MetadataProvider,
        instance::{LocationRef, Size},
    };

    #[test]
    fn mixed_line_matches_gujarati_body_to_latin_x_height() {
        let mut renderer = TextRenderer::new();
        assert!((1.0..=1.6).contains(&renderer.gujarati_scale));
        let spans = lumen_core::markup::parse_markup("xક");
        let request = TextRequest {
            spans: &spans,
            width_px: 800,
            height_px: 200,
            font_px: 80.0,
            color: [255; 4],
            outline: None,
            shadow: false,
            align: TextAlign::Left,
            valign: VerticalAlign::Top,
            line_height: 1.4,
            shrink_to_fit: false,
        };
        let (_, _, buffer) = renderer.layout(&request);
        let heights: Vec<f32> = buffer
            .layout_runs()
            .flat_map(|r| r.glyphs)
            .map(|glyph| {
                renderer
                    .fonts
                    .db()
                    .with_face_data(glyph.font_id, |data, index| {
                        let font = FontRef::from_index(data, index).unwrap();
                        let bounds = font
                            .glyph_metrics(Size::new(glyph.font_size), LocationRef::default())
                            .bounds(cosmic_text::skrifa::GlyphId::new(u32::from(glyph.glyph_id)))
                            .unwrap();
                        bounds.y_max - bounds.y_min
                    })
                    .unwrap()
            })
            .collect();
        assert_eq!(heights.len(), 2);
        assert!(
            (heights[1] / heights[0] - 1.0).abs() <= 0.10,
            "mixed glyph heights: {heights:?}"
        );
    }
}
