use std::{
    collections::HashMap,
    path::PathBuf,
    time::{Duration, Instant},
};

use egui::{Color32, Pos2, Rect, Stroke, TextureHandle, Vec2};
use lumen_core::{
    Background, LiveSlide, OutputState, Project, PushDirection, Runtime, SlideObject, TextAlign,
    TextStyle, Transition, VerticalAlign, markup::Span,
};

#[derive(Clone)]
pub struct OutputTransition {
    pub previous: OutputState,
    pub previous_media: Option<TextureHandle>,
    pub started: Instant,
    pub kind: Transition,
}

struct Scene<'a> {
    live: Option<&'a LiveSlide>,
    blackout: bool,
    clear_text: bool,
    media_cleared: bool,
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
    let background = match &live.slide.background {
        Background::Solid(rgba) => color(*rgba),
        _ => color(live.theme.background_color),
    };
    painter.rect_filled(canvas, 0.0, background);
    if !scene.media_cleared
        && let Some(texture) = scene.media
        && matches!(
            live.slide.background,
            Background::Image { .. } | Background::Video { .. }
        )
    {
        let image_rect = if matches!(live.slide.background, Background::Image { .. }) {
            fitted(canvas, texture.size_vec2())
        } else {
            canvas
        };
        painter.image(
            texture.id(),
            image_rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            crate::style::WHITE,
        );
    }
    if !scene.clear_text {
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
    previous: Option<&LiveSlide>,
    previous_blackout: bool,
    previous_clear_text: bool,
    previous_media_cleared: bool,
    previous_media: Option<&TextureHandle>,
    next: Option<&LiveSlide>,
    blackout: bool,
    clear_text: bool,
    media_cleared: bool,
    media: Option<&TextureHandle>,
    images: &HashMap<PathBuf, TextureHandle>,
) {
    let t = progress.clamp(0.0, 1.0);
    let canvas = fitted(rect, Vec2::new(16.0, 9.0));
    let previous_scene = Scene {
        live: previous,
        blackout: previous_blackout,
        clear_text: previous_clear_text,
        media_cleared: previous_media_cleared,
        media: previous_media,
        images,
    };
    let next_scene = Scene {
        live: next,
        blackout,
        clear_text,
        media_cleared,
        media,
        images,
    };
    painter(ui, rect, 1.0).rect_filled(rect, 0.0, crate::style::BLACK);
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
    let active_transition = transition.filter(|transition| {
        !matches!(&transition.kind, Transition::Cut)
            && transition_progress(
                transition_millis(&transition.kind),
                transition.started.elapsed(),
            ) < 1.0
    });
    if let Some(transition) = active_transition {
        let progress = transition_progress(
            transition_millis(&transition.kind),
            transition.started.elapsed(),
        );
        paint_transition_layers(
            ui,
            rect,
            &transition.kind,
            progress,
            transition.previous.live.as_ref(),
            transition.previous.blackout,
            transition.previous.clear_text,
            transition.previous.media_cleared,
            transition.previous_media.as_ref(),
            output.live.as_ref(),
            output.blackout,
            output.clear_text,
            output.media_cleared,
            media,
            images,
        );
    } else {
        let scene = Scene {
            live: output.live.as_ref(),
            blackout: output.blackout,
            clear_text: output.clear_text,
            media_cleared: output.media_cleared,
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

fn transition_millis(kind: &Transition) -> u32 {
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
    fn transition_layers_are_normalized_and_clamped() {
        let (old, new, old_offset, new_offset) =
            transition_frame(&Transition::Fade { millis: 500 }, 0.25);
        assert!((old - 0.75).abs() < 0.001);
        assert!((new - 0.25).abs() < 0.001);
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
