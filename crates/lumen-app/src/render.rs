use std::{
    collections::HashMap,
    path::PathBuf,
    time::{Duration, Instant},
};

use crate::thumbnails::PreviewState;
use egui::{Color32, Pos2, Rect, Stroke, TextureHandle, Vec2};
use lumen_core::{
    Backdrop, Background, LiveSlide, MediaFit, OutputState, Project, PushDirection, Runtime,
    SlideObject, TextAlign, TextStyle, Transition, VerticalAlign, markup::Span,
};

#[derive(Clone)]
pub struct OutputTransition {
    pub previous: OutputState,
    pub previous_media: Option<TextureHandle>,
    pub started: Instant,
    pub kind: Transition,
    /// Holds the previous picture (progress 0) until the new media has a frame.
    pub waiting_for_media: bool,
}

impl OutputTransition {
    pub fn progress(&self) -> f32 {
        if self.waiting_for_media {
            return 0.0;
        }
        transition_progress(transition_millis(&self.kind), self.started.elapsed())
    }
    pub fn is_active(&self) -> bool {
        self.waiting_for_media || self.progress() < 1.0
    }
}

struct Scene<'a> {
    live: Option<&'a LiveSlide>,
    blackout: bool,
    clear_text: bool,
    media_cleared: bool,
    background: Option<&'a Background>,
    empty_color: Color32,
    media: Option<&'a TextureHandle>,
    images: &'a HashMap<PathBuf, TextureHandle>,
}

pub fn fitted(rect: Rect, canvas: Vec2) -> Rect {
    let scale = (rect.width() / canvas.x)
        .min(rect.height() / canvas.y)
        .max(0.0);
    Rect::from_center_size(rect.center(), canvas * scale)
}

pub fn color(rgba: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3])
}

#[allow(clippy::too_many_arguments)]
pub fn paint_text(
    ui: &egui::Ui,
    spans: &[Span],
    rect: Rect,
    style: &TextStyle,
    default_size: f32,
    default_color: [u8; 4],
    align: TextAlign,
    vertical_align: VerticalAlign,
) {
    paint_text_layer(
        ui,
        spans,
        rect,
        style,
        default_size,
        default_color,
        align,
        vertical_align,
        1.0,
        rect,
    );
}

#[allow(clippy::too_many_arguments)]
fn paint_text_layer(
    ui: &egui::Ui,
    spans: &[Span],
    rect: Rect,
    style: &TextStyle,
    default_size: f32,
    default_color: [u8; 4],
    align: TextAlign,
    vertical_align: VerticalAlign,
    opacity: f32,
    clip_rect: Rect,
) {
    let pixels = ui.ctx().pixels_per_point();
    let request = lumen_text::TextRequest {
        spans,
        width_px: (rect.width() * pixels).round().clamp(1.0, 4096.0) as u32,
        height_px: (rect.height() * pixels).round().clamp(1.0, 4096.0) as u32,
        font_px: (style.font_size.unwrap_or(default_size) * pixels).clamp(4.0, 600.0),
        color: style.color.unwrap_or(default_color),
        outline: style.outline.map(|(width, color)| {
            (
                (width * rect.width() / 1920.0 * pixels).clamp(0.0, 20.0),
                color,
            )
        }),
        shadow: style.shadow,
        align,
        valign: vertical_align,
        line_height: 1.2,
        shrink_to_fit: true,
    };
    crate::text::paint(ui, rect, &request, opacity, clip_rect);
}

fn normalized_rect(canvas: Rect, rect: &lumen_core::Rect) -> Rect {
    Rect::from_min_max(
        Pos2::new(
            canvas.left() + canvas.width() * rect.x,
            canvas.top() + canvas.height() * rect.y,
        ),
        Pos2::new(
            canvas.left() + canvas.width() * (rect.x + rect.w),
            canvas.top() + canvas.height() * (rect.y + rect.h),
        ),
    )
}

pub fn transition_progress(millis: u32, elapsed: Duration) -> f32 {
    if millis == 0 {
        return 1.0;
    }
    (elapsed.as_secs_f32() * 1000.0 / millis as f32).clamp(0.0, 1.0)
}

pub fn transition_frame(kind: &Transition, t: f32) -> (f32, f32, Vec2, Vec2) {
    let t = t.clamp(0.0, 1.0);
    let t = t * t * (3.0 - 2.0 * t);
    match kind {
        Transition::Cut => (0.0, 1.0, Vec2::ZERO, Vec2::ZERO),
        Transition::Fade { .. } => (1.0 - t, t, Vec2::ZERO, Vec2::ZERO),
        Transition::FadeThroughBlack { .. } => (
            (1.0 - 2.0 * t).clamp(0.0, 1.0),
            (2.0 * t - 1.0).clamp(0.0, 1.0),
            Vec2::ZERO,
            Vec2::ZERO,
        ),
        Transition::Push { direction, .. } => {
            let vector = match direction {
                PushDirection::Left => Vec2::new(-1.0, 0.0),
                PushDirection::Right => Vec2::new(1.0, 0.0),
                PushDirection::Up => Vec2::new(0.0, -1.0),
                PushDirection::Down => Vec2::new(0.0, 1.0),
            };
            (1.0, 1.0, vector * t, vector * (t - 1.0))
        }
    }
}

fn painter(ui: &egui::Ui, clip_rect: Rect, opacity: f32) -> egui::Painter {
    let mut painter = ui.painter().with_clip_rect(clip_rect);
    painter.set_opacity(opacity);
    painter
}

fn paint_scene(
    ui: &egui::Ui,
    rect: Rect,
    scene: &Scene<'_>,
    clip_rect: Rect,
    opacity: f32,
    draw_outer: bool,
) {
    let painter = painter(ui, clip_rect, opacity);
    let canvas = fitted(rect, Vec2::new(16.0, 9.0));
    if draw_outer {
        painter.rect_filled(rect, 0.0, crate::style::BLACK);
    }
    if scene.blackout || scene.live.is_none() {
        painter.rect_filled(canvas, 0.0, crate::style::BLACK);
        return;
    }
    let Some(live) = scene.live else {
        return;
    };
    paint_background(ui, canvas, scene, clip_rect, opacity);
    paint_slide_layer(ui, canvas, live, scene, clip_rect, opacity);
}

fn paint_background(ui: &egui::Ui, canvas: Rect, scene: &Scene<'_>, clip_rect: Rect, opacity: f32) {
    let painter = painter(ui, clip_rect, opacity);
    let background = match scene.background {
        Some(Background::Solid(rgba)) if rgba[3] != 0 => color(*rgba),
        _ => scene.empty_color,
    };
    painter.rect_filled(canvas, 0.0, background);
    let pdf = if let Some(Background::Pdf { path, page }) = scene.background {
        crate::pdf::texture(ui, path, *page, canvas)
    } else {
        None
    };
    if !scene.media_cleared
        && matches!(
            scene.background,
            Some(Background::Image { .. } | Background::Video { .. } | Background::Pdf { .. })
        )
        && let Some(texture) = pdf.as_ref().or(scene.media)
    {
        // PDF and deck pages always fit; only photos and videos have a scale choice.
        let fit = scene
            .background
            .and_then(Background::fit)
            .unwrap_or_default();
        let (rect, uv) = media_rect_uv(texture.size_vec2(), canvas, fit);
        painter.image(texture.id(), rect, uv, crate::style::WHITE);
    }
}

/// Where a picture of `texture` size goes on `canvas`, and which part of it (UV) is shown.
pub fn media_rect_uv(texture: Vec2, canvas: Rect, fit: MediaFit) -> (Rect, Rect) {
    let all = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
    if texture.x <= 0.0 || texture.y <= 0.0 {
        return (Rect::from_center_size(canvas.center(), Vec2::ZERO), all);
    }
    match fit {
        MediaFit::Fit => (fitted(canvas, texture), all),
        MediaFit::Stretch => (canvas, all),
        MediaFit::Fill => {
            // Scale to cover, then show the centred part of the picture that fits the canvas.
            let scale = (canvas.width() / texture.x).max(canvas.height() / texture.y);
            let shown = Vec2::new(
                canvas.width() / (texture.x * scale),
                canvas.height() / (texture.y * scale),
            );
            let uv = Rect::from_center_size(Pos2::new(0.5, 0.5), shown);
            (canvas, uv)
        }
    }
}

fn bottom_gradient(canvas: Rect, text_rect: &lumen_core::Rect) -> egui::Mesh {
    let top = canvas.top() + canvas.height() * (text_rect.y - 0.06).clamp(0.0, 0.70);
    let mut mesh = egui::Mesh::default();
    for (pos, color) in [
        (Pos2::new(canvas.left(), top), Color32::TRANSPARENT),
        (Pos2::new(canvas.right(), top), Color32::TRANSPARENT),
        (canvas.left_bottom(), crate::style::CAPTION_GRADIENT),
        (canvas.right_bottom(), crate::style::CAPTION_GRADIENT),
    ] {
        mesh.colored_vertex(pos, color);
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 3, 2);
    mesh
}

fn paint_slide_layer(
    ui: &egui::Ui,
    canvas: Rect,
    live: &LiveSlide,
    scene: &Scene<'_>,
    clip_rect: Rect,
    opacity: f32,
) {
    if !scene.clear_text {
        if live.theme.backdrop == Backdrop::BottomGradient {
            painter(ui, clip_rect, opacity).add(egui::Shape::mesh(bottom_gradient(
                canvas,
                &live.slide.text_rect,
            )));
        }
        let target = normalized_rect(canvas, &live.slide.text_rect);
        let scale = canvas.width() / 1920.0;
        let mut style = live.slide.text_style.clone();
        style.font_size = style.font_size.map(|size| size * scale);
        paint_text_layer(
            ui,
            &lumen_core::markup::parse_markup(&live.slide.text),
            target,
            &style,
            live.theme.font_size * scale,
            live.theme.text_color,
            live.theme.align,
            live.theme.vertical_align,
            opacity,
            clip_rect,
        );
    }
    for object in &live.slide.objects {
        paint_object(
            ui,
            canvas,
            object,
            &live.theme,
            scene.images,
            opacity,
            clip_rect,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_object(
    ui: &egui::Ui,
    canvas: Rect,
    object: &SlideObject,
    theme: &lumen_core::Theme,
    images: &HashMap<PathBuf, TextureHandle>,
    opacity: f32,
    clip_rect: Rect,
) {
    let target = normalized_rect(canvas, object.rect());
    let painter = painter(ui, clip_rect, opacity);
    match object {
        SlideObject::Shape {
            color: fill,
            corner_radius,
            ..
        } => {
            painter.rect_filled(
                target,
                *corner_radius * canvas.width().min(canvas.height()),
                color(*fill),
            );
        }
        SlideObject::Text {
            text, style, align, ..
        } => {
            let scale = canvas.width() / 1920.0;
            let mut style = style.clone();
            style.font_size = style.font_size.map(|size| size * scale);
            paint_text_layer(
                ui,
                &lumen_core::markup::parse_markup(text),
                target,
                &style,
                theme.font_size * scale,
                theme.text_color,
                *align,
                VerticalAlign::Middle,
                opacity,
                clip_rect,
            );
        }
        SlideObject::Image { path, .. } => {
            if let Some(texture) = images.get(path) {
                let image_rect = fitted(target, texture.size_vec2());
                painter.image(
                    texture.id(),
                    image_rect,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    crate::style::WHITE,
                );
            } else {
                painter.rect_filled(target, 0.0, crate::style::RAISED);
                painter.rect_stroke(
                    target,
                    0.0,
                    Stroke::new(1.0_f32, crate::style::LINE),
                    egui::StrokeKind::Inside,
                );
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_transition_layers(
    ui: &egui::Ui,
    rect: Rect,
    kind: &Transition,
    progress: f32,
    previous: &OutputState,
    previous_media: Option<&TextureHandle>,
    next: &OutputState,
    media: Option<&TextureHandle>,
    images: &HashMap<PathBuf, TextureHandle>,
) {
    let t = progress.clamp(0.0, 1.0);
    let canvas = fitted(rect, Vec2::new(16.0, 9.0));
    let previous_scene = Scene {
        live: previous.live.as_ref(),
        blackout: previous.blackout,
        clear_text: previous.clear_text,
        media_cleared: previous.media_cleared,
        background: previous.effective_background(),
        empty_color: crate::style::BLACK,
        media: previous_media,
        images,
    };
    let next_scene = Scene {
        live: next.live.as_ref(),
        blackout: next.blackout,
        clear_text: next.clear_text,
        media_cleared: next.media_cleared,
        background: next.effective_background(),
        empty_color: crate::style::BLACK,
        media,
        images,
    };
    painter(ui, rect, 1.0).rect_filled(rect, 0.0, crate::style::BLACK);
    // Keep a shared video/PDF/still steady while only caption layers transition.
    if !previous.blackout
        && !next.blackout
        && previous.media_cleared == next.media_cleared
        && previous.effective_background() == next.effective_background()
    {
        paint_background(ui, canvas, &next_scene, rect, 1.0);
        let (old_alpha, new_alpha, old_offset, new_offset) = transition_frame(kind, t);
        for (scene, opacity, offset) in [
            (&previous_scene, old_alpha, old_offset),
            (&next_scene, new_alpha, new_offset),
        ] {
            if opacity > 0.0
                && let Some(live) = scene.live
            {
                let delta = Vec2::new(canvas.width() * offset.x, canvas.height() * offset.y);
                paint_slide_layer(ui, canvas.translate(delta), live, scene, canvas, opacity);
            }
        }
        return;
    }
    match kind {
        Transition::Fade { .. } => {
            paint_scene(ui, rect, &previous_scene, rect, 1.0, false);
            paint_scene(ui, rect, &next_scene, rect, t, false);
        }
        Transition::FadeThroughBlack { .. } => {
            let (old_alpha, new_alpha, _, _) = transition_frame(kind, t);
            if old_alpha > 0.0 {
                paint_scene(ui, rect, &previous_scene, rect, old_alpha, false);
            }
            if new_alpha > 0.0 {
                paint_scene(ui, rect, &next_scene, rect, new_alpha, false);
            }
        }
        Transition::Push { .. } => {
            let (_, _, old_offset, new_offset) = transition_frame(kind, t);
            let old_delta = Vec2::new(
                canvas.width() * old_offset.x,
                canvas.height() * old_offset.y,
            );
            let new_delta = Vec2::new(
                canvas.width() * new_offset.x,
                canvas.height() * new_offset.y,
            );
            paint_scene(
                ui,
                rect.translate(old_delta),
                &previous_scene,
                rect,
                1.0,
                false,
            );
            paint_scene(ui, rect.translate(new_delta), &next_scene, rect, 1.0, false);
        }
        Transition::Cut => paint_scene(ui, rect, &next_scene, rect, 1.0, false),
    }
}

/// Shared audience and live-monitor renderer. Decorations always come from the current output.
#[allow(clippy::too_many_arguments)]
pub fn paint_audience(
    ui: &egui::Ui,
    rect: Rect,
    output: &OutputState,
    project: &Project,
    runtime: &Runtime,
    now: u32,
    media: Option<&TextureHandle>,
    transition: Option<&OutputTransition>,
    images: &HashMap<PathBuf, TextureHandle>,
) {
    if let Some(transition) = transition.filter(|t| t.is_active()) {
        // A waiting Cut still holds the old picture, so it paints as a zero-length fade.
        let kind = match &transition.kind {
            Transition::Cut => &Transition::Fade { millis: 0 },
            kind => kind,
        };
        paint_transition_layers(
            ui,
            rect,
            kind,
            transition.progress(),
            &transition.previous,
            transition.previous_media.as_ref(),
            output,
            media,
            images,
        );
    } else {
        let scene = Scene {
            live: output.live.as_ref(),
            blackout: output.blackout,
            clear_text: output.clear_text,
            media_cleared: output.media_cleared,
            background: output.effective_background(),
            empty_color: crate::style::BLACK,
            media,
            images,
        };
        paint_scene(ui, rect, &scene, rect, 1.0, true);
    }
    if output.blackout {
        return;
    }
    paint_decorations(ui, rect, output, project, runtime, now, images);
}

pub fn transition_millis(kind: &Transition) -> u32 {
    match kind {
        Transition::Cut => 0,
        Transition::Fade { millis }
        | Transition::FadeThroughBlack { millis }
        | Transition::Push { millis, .. } => *millis,
    }
}

fn paint_decorations(
    ui: &egui::Ui,
    rect: Rect,
    output: &OutputState,
    project: &Project,
    runtime: &Runtime,
    now: u32,
    images: &HashMap<PathBuf, TextureHandle>,
) {
    let canvas = fitted(rect, Vec2::new(16.0, 9.0));
    for (prop, theme) in output
        .prop_snapshots
        .iter()
        .zip(&output.prop_theme_snapshots)
    {
        for object in &prop.objects {
            paint_object(ui, canvas, object, theme, images, 1.0, rect);
        }
    }
    let Some(message) = runtime.render_output_message(project, now) else {
        return;
    };
    let banner = Rect::from_min_max(
        Pos2::new(canvas.left(), canvas.bottom() - canvas.height() * 0.17),
        canvas.right_bottom(),
    );
    painter(ui, rect, 1.0).rect_filled(banner, 0.0, crate::style::PANEL);
    paint_text(
        ui,
        &lumen_core::markup::parse_markup(&message),
        banner.shrink(canvas.width() * 0.02),
        &TextStyle::default(),
        canvas.width() * 0.035,
        crate::style::WHITE.to_array(),
        TextAlign::Left,
        VerticalAlign::Middle,
    );
}

pub fn paint_preview(
    ui: &egui::Ui,
    rect: Rect,
    live: Option<&LiveSlide>,
    media: Option<&TextureHandle>,
    images: &HashMap<PathBuf, TextureHandle>,
) {
    let scene = Scene {
        live,
        blackout: false,
        clear_text: false,
        media_cleared: false,
        background: live.map(|l| &l.slide.background),
        empty_color: crate::style::CANVAS,
        media,
        images,
    };
    paint_scene(ui, rect, &scene, rect, 1.0, true);
}

pub fn preview_frame_with_images(
    ui: &mut egui::Ui,
    live: Option<&LiveSlide>,
    blackout: bool,
    clear_text: bool,
    media: Option<&TextureHandle>,
    images: &HashMap<PathBuf, TextureHandle>,
    height: f32,
) -> Rect {
    let width = (height * 16.0 / 9.0).min(ui.available_width());
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(width, height.min(width * 9.0 / 16.0)),
        egui::Sense::hover(),
    );
    let scene = Scene {
        live,
        blackout,
        clear_text,
        media_cleared: false,
        background: live.map(|l| &l.slide.background),
        empty_color: crate::style::CANVAS,
        media,
        images,
    };
    paint_scene(ui, rect, &scene, rect, 1.0, true);
    ui.painter().rect_stroke(
        rect,
        0.0,
        Stroke::new(1.0_f32, crate::style::LINE),
        egui::StrokeKind::Inside,
    );
    rect
}

/// Warning glyph in the Phosphor font.
const WARNING_ICON: char = '\u{e4e0}';

/// Under this size (thumbnail in a running-order row) a placeholder shows only its icon.
fn placeholder_is_compact(size: Vec2) -> bool {
    size.x < 120.0 || size.y < 64.0
}

/// Headline and optional second line of a placeholder, e.g. ("Loading preview…", Some(name)).
fn placeholder_copy(file_name: &str, state: &PreviewState) -> (String, Option<String>) {
    match state {
        PreviewState::Loading => ("Loading preview…".into(), Some(file_name.into())),
        PreviewState::Failed(_) => (format!("Can't preview {file_name}"), None),
        PreviewState::Missing => (format!("File not found: {file_name}"), None),
    }
}

/// Fills the preview surface for a media cue that has no thumbnail, so it is never plain black.
pub fn media_placeholder(ui: &egui::Ui, rect: Rect, file_name: &str, state: &PreviewState) {
    let painter = ui.painter().with_clip_rect(rect);
    painter.rect_filled(rect, 0.0, crate::style::RAISED);
    let loading = matches!(state, PreviewState::Loading);
    let icon_size = if placeholder_is_compact(rect.size()) {
        rect.height().min(rect.width()) * 0.4
    } else {
        20.0
    };
    let (headline, detail) = placeholder_copy(file_name, state);
    let compact = placeholder_is_compact(rect.size());
    let line_height = crate::style::CAPTION * 1.4;
    let lines = if compact {
        0.0
    } else if detail.is_some() {
        2.0
    } else {
        1.0
    };
    let gap = if compact {
        0.0
    } else {
        crate::style::GAP * 0.75
    };
    let block = icon_size + gap + lines * line_height;
    let icon_rect = Rect::from_center_size(
        Pos2::new(
            rect.center().x,
            rect.center().y - block / 2.0 + icon_size / 2.0,
        ),
        Vec2::splat(icon_size),
    );
    if loading {
        egui::Spinner::new()
            .size(icon_size)
            .color(crate::style::TEXT_DIM)
            .paint_at(ui, icon_rect);
    } else {
        painter.text(
            icon_rect.center(),
            egui::Align2::CENTER_CENTER,
            WARNING_ICON,
            egui::FontId::new(icon_size, egui::FontFamily::Name("Phosphor".into())),
            crate::style::WARN,
        );
    }
    if compact {
        return;
    }
    let width = (rect.width() - 16.0).max(1.0);
    let mut y = icon_rect.bottom() + gap;
    for (text, color) in [
        Some((headline, crate::style::TEXT)),
        detail.map(|d| (d, crate::style::TEXT_DIM)),
    ]
    .into_iter()
    .flatten()
    {
        let mut job = egui::text::LayoutJob::simple_singleline(
            text,
            egui::FontId::proportional(crate::style::CAPTION),
            color,
        );
        job.wrap = egui::text::TextWrapping::truncate_at_width(width);
        let galley = painter.layout_job(job);
        painter.galley(
            Pos2::new(rect.center().x - galley.size().x / 2.0, y),
            galley,
            color,
        );
        y += line_height;
    }
}

/// Stage text uses the same shaping path as audience text, including Latin spans.
pub fn stage_text(ui: &mut egui::Ui, text: &str, size: f32, height: f32) {
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), height),
        egui::Sense::hover(),
    );
    paint_text(
        ui,
        &lumen_core::markup::parse_markup(text),
        rect,
        &TextStyle::default(),
        size,
        crate::style::TEXT.to_array(),
        TextAlign::Left,
        VerticalAlign::Top,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_fit_places_a_4_3_picture_on_a_16_9_canvas() {
        let canvas = Rect::from_min_size(Pos2::new(10.0, 20.0), Vec2::new(1600.0, 900.0));
        let tex = Vec2::new(1440.0, 1080.0);
        let (rect, uv) = media_rect_uv(tex, canvas, MediaFit::Fit);
        assert_eq!(rect.size(), Vec2::new(1200.0, 900.0));
        assert_eq!(rect.center(), canvas.center());
        assert_eq!(uv, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)));
        let (rect, uv) = media_rect_uv(tex, canvas, MediaFit::Stretch);
        assert_eq!(rect, canvas);
        assert_eq!(uv, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)));
        // Fill covers the canvas and keeps the middle 75% of the height.
        let (rect, uv) = media_rect_uv(tex, canvas, MediaFit::Fill);
        assert_eq!(rect, canvas);
        assert!((uv.width() - 1.0).abs() < 1e-6);
        assert!((uv.height() - 0.75).abs() < 1e-6);
        assert!((uv.center().y - 0.5).abs() < 1e-6);
    }

    #[test]
    fn media_fill_crops_a_portrait_picture_to_a_band() {
        let canvas = Rect::from_min_size(Pos2::ZERO, Vec2::new(1920.0, 1080.0));
        let (rect, uv) = media_rect_uv(Vec2::new(1080.0, 1920.0), canvas, MediaFit::Fill);
        assert_eq!(rect, canvas);
        assert!((uv.width() - 1.0).abs() < 1e-6);
        // 1920×1080 of a 1080×1920 picture scaled to 1920 wide shows 1080 of 3413 rows.
        assert!((uv.height() - 1080.0 / (1920.0 * 1920.0 / 1080.0)).abs() < 1e-5);
    }

    #[test]
    fn media_fit_draws_nothing_for_an_empty_texture() {
        let canvas = Rect::from_min_size(Pos2::ZERO, Vec2::new(1920.0, 1080.0));
        for fit in [MediaFit::Fit, MediaFit::Fill, MediaFit::Stretch] {
            let (rect, _) = media_rect_uv(Vec2::ZERO, canvas, fit);
            assert_eq!(rect.size(), Vec2::ZERO);
            assert!(rect.center().x.is_finite() && rect.center().y.is_finite());
        }
    }

    #[test]
    fn bottom_gradient_covers_width_and_scales_with_canvas() {
        for scale in [1.0, 0.25] {
            let canvas =
                Rect::from_min_size(Pos2::new(30.0, 40.0), Vec2::new(1920.0, 1080.0) * scale);
            let text = lumen_core::Rect {
                x: 0.06,
                y: 0.60,
                w: 0.88,
                h: 0.34,
            };
            let mesh = bottom_gradient(canvas, &text);
            assert_eq!(mesh.vertices.len(), 4);
            assert_eq!(mesh.indices.len(), 6);
            assert_eq!(mesh.vertices[0].pos.x, canvas.left());
            assert_eq!(mesh.vertices[1].pos.x, canvas.right());
            assert!(
                (mesh.vertices[0].pos.y - (canvas.top() + 0.54 * canvas.height())).abs() < 0.01
            );
            assert_eq!(mesh.vertices[0].color.a(), 0);
            assert_eq!(mesh.vertices[2].pos.y, canvas.bottom());
            assert_eq!(mesh.vertices[2].color.a(), 217);
            let low_text = lumen_core::Rect { y: 0.99, ..text };
            let low_mesh = bottom_gradient(canvas, &low_text);
            assert!(
                (canvas.bottom() - low_mesh.vertices[0].pos.y) >= canvas.height() * 0.30 - 0.01
            );
        }
    }

    #[test]
    fn caption_transitions_paint_shared_video_once_at_full_opacity() {
        let mut project = Project::demo();
        let doc = &mut project.documents[0];
        doc.slides[0].background = Background::Video {
            path: "service.mp4".into(),
            looped: true,
            muted: true,
            fit: lumen_core::MediaFit::default(),
        };
        doc.slides[1].background = Background::Solid([0, 0, 0, 0]);
        let mut runtime = Runtime::default();
        runtime
            .trigger(
                &project,
                project.documents[0].id,
                project.documents[0].slides[0].id,
            )
            .unwrap();
        let mut previous = runtime.output.clone();
        previous.live.as_mut().unwrap().slide.text.clear();
        runtime
            .trigger(
                &project,
                project.documents[0].id,
                project.documents[0].slides[1].id,
            )
            .unwrap();
        runtime.output.live.as_mut().unwrap().slide.text.clear();
        for kind in [
            Transition::Fade { millis: 500 },
            Transition::FadeThroughBlack { millis: 500 },
            Transition::Push {
                millis: 500,
                direction: PushDirection::Left,
            },
        ] {
            let ctx = egui::Context::default();
            let texture = ctx.load_texture(
                "test-video",
                egui::ColorImage::filled([16, 9], crate::style::WHITE),
                egui::TextureOptions::LINEAR,
            );
            let frame = ctx.run(Default::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    paint_transition_layers(
                        ui,
                        Rect::from_min_size(Pos2::ZERO, Vec2::new(640.0, 360.0)),
                        &kind,
                        0.5,
                        &previous,
                        None,
                        &runtime.output,
                        Some(&texture),
                        &HashMap::new(),
                    );
                });
            });
            let images: Vec<_> = frame
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    egui::Shape::Mesh(mesh) if mesh.texture_id == texture.id() => Some(mesh),
                    _ => None,
                })
                .collect();
            assert_eq!(images.len(), 1, "{kind:?}");
            assert!(images[0].vertices.iter().all(|v| v.color.a() == 255));
        }
    }

    #[test]
    fn waiting_transition_holds_the_previous_scene_even_for_cut() {
        let mut project = Project::demo();
        let red = Color32::from_rgb(255, 0, 0);
        let blue = Color32::from_rgb(0, 0, 255);
        project.documents[0].slides[0].background = Background::Solid([255, 0, 0, 255]);
        project.documents[0].slides[1].background = Background::Solid([0, 0, 255, 255]);
        let (doc, first, second) = (
            project.documents[0].id,
            project.documents[0].slides[0].id,
            project.documents[0].slides[1].id,
        );
        let mut runtime = Runtime::default();
        runtime.trigger(&project, doc, first).unwrap();
        let previous = runtime.output.clone();
        runtime.trigger(&project, doc, second).unwrap();
        let transition = OutputTransition {
            previous,
            previous_media: None,
            started: Instant::now() - Duration::from_secs(5),
            kind: Transition::Cut,
            waiting_for_media: true,
        };
        assert!(transition.is_active());
        assert_eq!(transition.progress(), 0.0);
        let ctx = egui::Context::default();
        let frame = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                paint_audience(
                    ui,
                    Rect::from_min_size(Pos2::ZERO, Vec2::new(640.0, 360.0)),
                    &runtime.output,
                    &project,
                    &runtime,
                    0,
                    None,
                    Some(&transition),
                    &HashMap::new(),
                );
            });
        });
        let fills: Vec<Color32> = frame
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Rect(r) => Some(r.fill),
                _ => None,
            })
            .collect();
        assert!(fills.contains(&red), "previous scene must be fully painted");
        assert!(
            !fills.contains(&blue),
            "next scene must be invisible while waiting"
        );
        let released = OutputTransition {
            waiting_for_media: false,
            ..transition
        };
        assert!(!released.is_active());
    }

    #[test]
    fn placeholder_copy_names_the_file_in_every_state() {
        assert_eq!(
            placeholder_copy("a.mov", &PreviewState::Loading),
            ("Loading preview…".into(), Some("a.mov".into()))
        );
        assert_eq!(
            placeholder_copy("a.mov", &PreviewState::Failed("boom".into())),
            ("Can't preview a.mov".into(), None)
        );
        assert_eq!(
            placeholder_copy("a.mov", &PreviewState::Missing),
            ("File not found: a.mov".into(), None)
        );
        // A running-order thumbnail is icon only; a grid tile has room for text.
        assert!(placeholder_is_compact(Vec2::new(78.0, 44.0)));
        assert!(!placeholder_is_compact(Vec2::new(160.0, 90.0)));
    }

    #[test]
    fn placeholder_is_never_empty_in_any_size_or_state() {
        let ctx = egui::Context::default();
        crate::style::install(&ctx);
        for size in [Vec2::new(78.0, 44.0), Vec2::new(300.0, 169.0)] {
            for state in [
                PreviewState::Loading,
                PreviewState::Failed("x".into()),
                PreviewState::Missing,
            ] {
                let frame = ctx.run(Default::default(), |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        media_placeholder(
                            ui,
                            Rect::from_min_size(Pos2::ZERO, size),
                            "a.mov",
                            &state,
                        );
                    });
                });
                let fills = frame.shapes.iter().any(
                    |s| matches!(&s.shape, egui::Shape::Rect(r) if r.fill == crate::style::RAISED),
                );
                assert!(fills, "{state:?} at {size:?} must paint the panel");
            }
        }
    }

    #[test]
    fn transition_layers_are_normalized_and_clamped() {
        let (old, new, old_offset, new_offset) =
            transition_frame(&Transition::Fade { millis: 500 }, 0.25);
        // smoothstep(0.25) = 0.15625
        assert!((old - 0.84375).abs() < 0.001);
        assert!((new - 0.15625).abs() < 0.001);
        let (_, mid, _, _) = transition_frame(&Transition::Fade { millis: 500 }, 0.5);
        assert!((mid - 0.5).abs() < 0.001);
        assert_eq!(old_offset, Vec2::ZERO);
        assert_eq!(new_offset, Vec2::ZERO);
        assert_eq!(transition_frame(&Transition::Cut, -1.0).0, 0.0);
        assert_eq!(transition_frame(&Transition::Cut, 2.0).1, 1.0);
    }

    #[test]
    fn zero_duration_finishes_instantly_and_progress_is_clamped() {
        assert_eq!(transition_progress(0, Duration::ZERO), 1.0);
        assert_eq!(transition_progress(100, Duration::ZERO), 0.0);
        assert_eq!(transition_progress(100, Duration::from_millis(200)), 1.0);
        assert_eq!(transition_progress(100, Duration::from_millis(50)), 0.5);
    }

    #[test]
    fn push_offsets_reach_canvas_edges_and_clamp() {
        let cases = [
            (PushDirection::Left, Vec2::new(-1.0, 0.0)),
            (PushDirection::Right, Vec2::new(1.0, 0.0)),
            (PushDirection::Up, Vec2::new(0.0, -1.0)),
            (PushDirection::Down, Vec2::new(0.0, 1.0)),
        ];
        for (direction, vector) in cases {
            let transition = Transition::Push {
                millis: 100,
                direction,
            };
            let (old_at_start, _, _, new_at_start) = transition_frame(&transition, 0.0);
            let (_, new_at_end, old_at_end, _) = transition_frame(&transition, 1.0);
            assert_eq!(old_at_start, 1.0);
            assert_eq!(new_at_start, -vector);
            assert_eq!(old_at_end, vector);
            assert_eq!(new_at_end, 1.0);
        }
        assert_eq!(
            transition_frame(
                &Transition::Push {
                    millis: 100,
                    direction: PushDirection::Right
                },
                -1.0
            )
            .2,
            Vec2::ZERO
        );
        assert_eq!(
            transition_frame(
                &Transition::Push {
                    millis: 100,
                    direction: PushDirection::Right
                },
                2.0
            )
            .3,
            Vec2::ZERO
        );
    }

    #[test]
    fn normalized_rect_maps_fractional_bounds_to_canvas() {
        let canvas = Rect::from_min_size(Pos2::new(100.0, 50.0), Vec2::new(800.0, 450.0));
        let rect = lumen_core::Rect {
            x: 0.25,
            y: 0.2,
            w: 0.5,
            h: 0.4,
        };
        assert_eq!(
            normalized_rect(canvas, &rect),
            Rect::from_min_max(Pos2::new(300.0, 140.0), Pos2::new(700.0, 320.0))
        );
    }
}
