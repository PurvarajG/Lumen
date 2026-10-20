//! One app-owned shaped-text renderer and a bounded GPU LRU shared by viewports.
use egui::{Context, Pos2, Rect, Response, TextureHandle, Ui, Vec2};
use lumen_core::{TextAlign, VerticalAlign};
use lumen_text::{TextRenderer, TextRequest};
use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    sync::{Arc, Mutex},
};

const GPU_BUDGET: usize = 128 * 1024 * 1024;
const IMAGES_PER_FRAME: usize = 4;
pub type SharedText = Arc<Mutex<TextImages>>;
struct Entry {
    texture: TextureHandle,
    bytes: usize,
    touched: u64,
}
pub struct TextImages {
    renderer: TextRenderer,
    images: HashMap<u64, Entry>,
    previous: HashMap<u64, u64>,
    bytes: usize,
    tick: u64,
    remaining: usize,
    /// Installed English and Gujarati families, listed the first time Settings needs them.
    choices: Option<(Vec<String>, Vec<String>)>,
    /// Why the chosen fonts were replaced by the defaults, if they were.
    notice: Option<String>,
}
impl Default for TextImages {
    fn default() -> Self {
        Self {
            renderer: TextRenderer::new(),
            images: HashMap::new(),
            previous: HashMap::new(),
            bytes: 0,
            tick: 0,
            remaining: IMAGES_PER_FRAME,
            choices: None,
            notice: None,
        }
    }
}
impl TextImages {
    pub fn install(ctx: &Context) -> SharedText {
        let shared = Arc::new(Mutex::new(Self::default()));
        ctx.data_mut(|data| data.insert_temp(egui::Id::new("shaped-text"), shared.clone()));
        shared
    }
    pub fn begin_frame(&mut self) {
        self.remaining = IMAGES_PER_FRAME;
    }
    pub fn font_notice(&self) -> String {
        format!(
            "English font: {} · Gujarati font: {}",
            self.renderer.latin_face(),
            self.renderer.gujarati_face()
        )
    }
    /// Switches the slide fonts. Slides already drawn keep their pixels until dropped, so the
    /// GPU cache goes too (its key does not include the font).
    pub fn set_faces(&mut self, fonts: &crate::settings::SlideFonts) {
        self.notice = self
            .renderer
            .set_faces(fonts.latin.as_deref(), fonts.gujarati.as_deref());
        self.images.clear();
        self.previous.clear();
        self.bytes = 0;
    }
    fn evict_for(&mut self, bytes: usize) {
        while self.bytes + bytes > GPU_BUDGET {
            let Some(key) = self
                .images
                .iter()
                .min_by_key(|(_, entry)| entry.touched)
                .map(|(key, _)| *key)
            else {
                break;
            };
            if let Some(entry) = self.images.remove(&key) {
                self.bytes -= entry.bytes;
            }
            self.previous.retain(|_, previous| *previous != key);
        }
    }
    fn texture(
        &mut self,
        ctx: &Context,
        slot: u64,
        req: &TextRequest<'_>,
    ) -> Option<TextureHandle> {
        let key = request_hash(req);
        self.tick += 1;
        if let Some(entry) = self.images.get_mut(&key) {
            entry.touched = self.tick;
            self.previous.insert(slot, key);
            return Some(entry.texture.clone());
        }
        if self.remaining > 0 {
            self.remaining -= 1;
            if let Ok(image) = self.renderer.render(req) {
                let bytes = image.pixels.len();
                if bytes <= GPU_BUDGET {
                    self.evict_for(bytes);
                    let texture = ctx.load_texture(
                        format!("shaped-{key}"),
                        egui::ColorImage::from_rgba_unmultiplied(
                            [image.width as usize, image.height as usize],
                            &image.pixels,
                        ),
                        egui::TextureOptions::LINEAR,
                    );
                    self.images.insert(
                        key,
                        Entry {
                            texture: texture.clone(),
                            bytes,
                            touched: self.tick,
                        },
                    );
                    self.bytes += bytes;
                    self.previous.insert(slot, key);
                    return Some(texture);
                }
            }
        }
        ctx.request_repaint();
        ctx.request_repaint_of(egui::ViewportId::ROOT);
        self.previous
            .get(&slot)
            .and_then(|key| self.images.get(key))
            .map(|entry| entry.texture.clone())
    }
    fn choices(&mut self) -> (Vec<String>, Vec<String>) {
        self.choices
            .take()
            .unwrap_or_else(|| (self.renderer.families('a'), self.renderer.families('ક')))
    }
}
/// The "Slide fonts" part of Settings; true when a choice changed (and was applied).
pub fn font_settings(
    ui: &mut Ui,
    fonts: &mut crate::settings::SlideFonts,
    shared: &SharedText,
) -> bool {
    let Ok(mut images) = shared.lock() else {
        return false;
    };
    let (english, gujarati) = images.choices();
    let default_gujarati = images.renderer.default_gujarati_face().to_owned();
    let gujarati_default_label = if default_gujarati.eq_ignore_ascii_case("ChitraUni") {
        "ChitraUni (default)".to_owned()
    } else {
        format!("{default_gujarati} (ChitraUni not installed)")
    };
    ui.add_space(crate::style::GAP);
    ui.label(
        egui::RichText::new("SLIDE FONTS")
            .color(crate::style::MUTED)
            .strong(),
    );
    let mut changed = false;
    for (id, label, current, default_label, default_name, families) in [
        (
            "slide-font-english",
            "English",
            &mut fonts.latin,
            "Poppins (default)".to_owned(),
            "Poppins".to_owned(),
            &english,
        ),
        (
            "slide-font-gujarati",
            "Gujarati",
            &mut fonts.gujarati,
            gujarati_default_label,
            default_gujarati,
            &gujarati,
        ),
    ] {
        ui.label(label);
        let shown = current.clone().unwrap_or_else(|| default_label.clone());
        egui::ComboBox::from_id_salt(id)
            .selected_text(shown)
            .show_ui(ui, |ui| {
                changed |= ui.selectable_value(current, None, &default_label).changed();
                for family in families.iter().filter(|f| **f != default_name) {
                    changed |= ui
                        .selectable_value(current, Some(family.clone()), family)
                        .changed();
                }
            });
    }
    images.choices = Some((english, gujarati));
    if changed {
        images.set_faces(fonts);
    }
    ui.label(
        egui::RichText::new(images.font_notice())
            .color(crate::style::TEXT_DIM)
            .size(crate::style::CAPTION),
    );
    if let Some(notice) = &images.notice {
        ui.label(egui::RichText::new(notice).color(crate::style::WARN));
    }
    changed
}
fn request_hash(req: &TextRequest<'_>) -> u64 {
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    for span in req.spans {
        span.text.hash(&mut hash);
        span.bold.hash(&mut hash);
        span.italic.hash(&mut hash);
        span.underline.hash(&mut hash);
        span.color.hash(&mut hash);
    }
    req.width_px.hash(&mut hash);
    req.height_px.hash(&mut hash);
    req.font_px.to_bits().hash(&mut hash);
    req.color.hash(&mut hash);
    req.outline.map(|(w, c)| (w.to_bits(), c)).hash(&mut hash);
    req.shadow.hash(&mut hash);
    (req.align as u8).hash(&mut hash);
    (req.valign as u8).hash(&mut hash);
    req.line_height.to_bits().hash(&mut hash);
    req.shrink_to_fit.hash(&mut hash);
    hash.finish()
}
pub fn paint(ui: &Ui, rect: Rect, req: &TextRequest<'_>, opacity: f32, clip: Rect) {
    let shared = ui
        .ctx()
        .data(|data| data.get_temp::<SharedText>(egui::Id::new("shaped-text")));
    if let Some(shared) = shared {
        let slot = ui
            .id()
            .with((
                ui.ctx().viewport_id(),
                rect.min.x.to_bits(),
                rect.min.y.to_bits(),
                rect.width().to_bits(),
                rect.height().to_bits(),
            ))
            .value();
        if let Ok(mut cache) = shared.lock()
            && let Some(texture) = cache.texture(ui.ctx(), slot, req)
        {
            let mut painter = ui.painter().with_clip_rect(clip.intersect(rect));
            painter.set_opacity(opacity);
            painter.image(
                texture.id(),
                rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                crate::style::WHITE,
            );
        }
    }
}
pub fn contains_gujarati(text: &str) -> bool {
    text.chars()
        .any(|ch| ('\u{0a80}'..='\u{0aff}').contains(&ch))
}
pub fn gu_label(ui: &mut Ui, text: &str, size: f32) -> Response {
    if !contains_gujarati(text) {
        let plain = lumen_text::fold_unsupported(text, lumen_text::poppins_covers);
        return ui.label(egui::RichText::new(plain).size(size));
    }
    shaped_label(ui, text, size, false, false)
}
pub fn gu_selectable(ui: &mut Ui, selected: bool, text: &str, size: f32) -> Response {
    if !contains_gujarati(text) {
        let plain = lumen_text::fold_unsupported(text, lumen_text::poppins_covers);
        return ui
            .add_sized(
                [ui.available_width(), crate::style::CONTROL],
                egui::Button::selectable(
                    selected,
                    (
                        egui::RichText::new(plain.as_ref()).size(size),
                        egui::Atom::grow(),
                    ),
                )
                .truncate(),
            )
            .on_hover_text(plain);
    }
    shaped_label(ui, text, size, true, selected)
}
fn shaped_label(ui: &mut Ui, text: &str, size: f32, selectable: bool, selected: bool) -> Response {
    let width = if selectable {
        ui.available_width()
    } else {
        (text
            .lines()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(1) as f32
            * size
            * 0.8
            + 8.0)
            .min(ui.available_width())
    }
    .max(1.0);
    let lines = text.lines().count().max(1) as f32;
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(width, (size * 1.7 * lines).max(28.0)),
        if selectable {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        },
    );
    if selectable && (selected || response.hovered() || response.has_focus()) {
        ui.painter().rect_filled(rect, 3.0, crate::style::RAISED);
        if response.has_focus() {
            ui.painter().rect_stroke(
                rect,
                3.0,
                egui::Stroke::new(2.0_f32, crate::style::SAFFRON),
                egui::StrokeKind::Inside,
            );
        }
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            selected,
            text,
        )
    });
    let spans = lumen_core::markup::parse_markup(text);
    let pixels = ui.ctx().pixels_per_point();
    paint(
        ui,
        rect,
        &TextRequest {
            spans: &spans,
            width_px: (rect.width() * pixels).round().clamp(1.0, 4096.0) as u32,
            height_px: (rect.height() * pixels).round().clamp(1.0, 4096.0) as u32,
            font_px: (size * pixels).clamp(4.0, 600.0),
            color: crate::style::TEXT.to_array(),
            outline: None,
            shadow: false,
            align: TextAlign::Left,
            valign: VerticalAlign::Middle,
            line_height: 1.3,
            shrink_to_fit: !selectable,
        },
        1.0,
        ui.clip_rect(),
    );
    response.on_hover_text(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gujarati_detection_includes_mixed_script_only_in_gujarati_block() {
        assert!(contains_gujarati("Vachanamrut વચનામૃત"));
        assert!(!contains_gujarati("Vachanamrut हिन्दी"));
    }
    #[test]
    fn physical_request_hash_distinguishes_scale_and_style() {
        let spans = lumen_core::markup::parse_markup("શ્રી સ્વામિનારાયણ");
        let mut req = TextRequest {
            spans: &spans,
            width_px: 200,
            height_px: 100,
            font_px: 24.0,
            color: [255; 4],
            outline: None,
            shadow: false,
            align: TextAlign::Left,
            valign: VerticalAlign::Top,
            line_height: 1.2,
            shrink_to_fit: true,
        };
        let original = request_hash(&req);
        req.width_px *= 2;
        assert_ne!(original, request_hash(&req));
        req.width_px /= 2;
        req.shadow = true;
        assert_ne!(original, request_hash(&req));
    }
    #[test]
    fn text_work_is_bounded_and_previous_image_remains_until_ready() {
        let ctx = Context::default();
        let mut cache = TextImages::default();
        let spans = lumen_core::markup::parse_markup("શ્રી સ્વામિનારાયણ");
        let mut req = TextRequest {
            spans: &spans,
            width_px: 64,
            height_px: 32,
            font_px: 12.0,
            color: [255; 4],
            outline: None,
            shadow: false,
            align: TextAlign::Left,
            valign: VerticalAlign::Top,
            line_height: 1.2,
            shrink_to_fit: true,
        };
        let previous = cache.texture(&ctx, 7, &req).unwrap();
        for width in 65..68 {
            req.width_px = width;
            assert!(cache.texture(&ctx, width as u64, &req).is_some());
        }
        req.width_px = 80;
        let fallback = cache.texture(&ctx, 7, &req).unwrap();
        assert_eq!(fallback.id(), previous.id());
        assert_eq!(cache.images.len(), 4);
        assert_eq!(cache.remaining, 0);
        cache.begin_frame();
        assert_ne!(cache.texture(&ctx, 7, &req).unwrap().id(), previous.id());
    }
    #[test]
    fn gpu_cache_evicts_least_recent_and_releases_previous_slots() {
        let ctx = Context::default();
        let mut cache = TextImages::default();
        for (key, touched) in [(1, 2), (2, 1)] {
            let texture = ctx.load_texture(
                format!("test-{key}"),
                egui::ColorImage::new([1, 1], vec![crate::style::WHITE]),
                Default::default(),
            );
            cache.images.insert(
                key,
                Entry {
                    texture,
                    bytes: GPU_BUDGET / 2,
                    touched,
                },
            );
            cache.previous.insert(key, key);
        }
        cache.bytes = GPU_BUDGET;
        cache.evict_for(4);
        assert!(cache.images.contains_key(&1));
        assert!(!cache.images.contains_key(&2));
        assert!(!cache.previous.contains_key(&2));
        assert!(cache.bytes + 4 <= GPU_BUDGET);
    }
}
