use egui::{Align, Color32, FontId, Pos2, Rect, Stroke, Vec2, text::LayoutJob};
use lumen_core::{Background, LiveSlide, TextAlign, VerticalAlign};

pub fn fitted(rect: Rect, canvas: Vec2) -> Rect {
    let scale = (rect.width() / canvas.x)
        .min(rect.height() / canvas.y)
        .max(0.0);
    let size = canvas * scale;
    Rect::from_center_size(rect.center(), size)
}

pub fn color(rgba: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3])
}

pub fn paint(
    ui: &egui::Ui,
    rect: Rect,
    live: Option<&LiveSlide>,
    blackout: bool,
    clear_text: bool,
    media: Option<&egui::TextureHandle>,
) {
    let canvas = fitted(rect, Vec2::new(16.0, 9.0));
    ui.painter().rect_filled(rect, 0.0, Color32::BLACK);
    if blackout || live.is_none() {
        ui.painter().rect_filled(canvas, 0.0, Color32::BLACK);
        return;
    }
    let Some(live) = live else {
        return;
    };
    let bg = match &live.slide.background {
        Background::Solid(c) => color(*c),
        _ => color(live.theme.background_color),
    };
    ui.painter().rect_filled(canvas, 0.0, bg);
    if let Some(tex) = media
        && matches!(
            live.slide.background,
            Background::Image { .. } | Background::Video { .. }
        )
    {
        let image_size = tex.size_vec2();
        let image_rect = if matches!(live.slide.background, Background::Image { .. }) {
            fitted(canvas, image_size)
        } else {
            canvas
        };
        ui.painter().image(
            tex.id(),
            image_rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0_f32, 1.0_f32)),
            Color32::WHITE,
        );
    }
    if clear_text {
        return;
    }
    let pad = canvas.width() * 90.0 / 1920.0;
    let target = canvas.shrink(pad);
    let wrap_width = target.width().max(1.0);
    let align = match live.theme.align {
        TextAlign::Left => Align::Min,
        TextAlign::Center => Align::Center,
        TextAlign::Right => Align::Max,
    };
    let valign = match live.theme.vertical_align {
        VerticalAlign::Top => Align::Min,
        VerticalAlign::Middle => Align::Center,
        VerticalAlign::Bottom => Align::Max,
    };
    let mut size = (live.theme.font_size * canvas.width() / 1920.0).max(1.0);
    let galley = loop {
        let mut job = LayoutJob::simple(
            live.slide.text.clone(),
            FontId::proportional(size),
            color(live.theme.text_color),
            wrap_width,
        );
        job.halign = align;
        let galley = ui.painter().layout_job(job);
        if galley.size().y <= target.height() || size <= 1.0 {
            break galley;
        }
        size = (size * 0.9).max(1.0);
    };
    let pos = galley_origin(target, galley.size(), align, valign);
    ui.painter()
        .with_clip_rect(canvas)
        .galley(pos, galley, color(live.theme.text_color));
}

fn galley_origin(target: Rect, size: Vec2, align: Align, valign: Align) -> Pos2 {
    Pos2::new(
        match align {
            Align::Min => target.left(),
            Align::Center => target.center().x,
            Align::Max => target.right(),
        },
        match valign {
            Align::Min => target.top(),
            Align::Center => target.center().y - size.y / 2.0,
            Align::Max => target.bottom() - size.y,
        },
    )
}

pub fn preview_frame(
    ui: &mut egui::Ui,
    live: Option<&LiveSlide>,
    blackout: bool,
    clear_text: bool,
    media: Option<&egui::TextureHandle>,
    height: f32,
) -> Rect {
    let width = (height * 16.0 / 9.0).min(ui.available_width());
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(width, height.min(width * 9.0 / 16.0)),
        egui::Sense::hover(),
    );
    paint(ui, rect, live, blackout, clear_text, media);
    ui.painter().rect_stroke(
        rect,
        0.0,
        Stroke::new(1.0_f32, Color32::from_gray(72)),
        egui::StrokeKind::Inside,
    );
    rect
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fit_preserves_sixteen_nine_and_centers() {
        let r = Rect::from_min_size(Pos2::new(10.0, 20.0), Vec2::new(500.0, 400.0));
        let f = fitted(r, Vec2::new(16.0, 9.0));
        assert!((f.width() / f.height() - 16.0 / 9.0).abs() < 0.001);
        assert_eq!(f.center(), r.center());
    }

    #[test]
    fn galley_origin_uses_alignment_anchor_once() {
        let target = Rect::from_min_size(Pos2::new(10.0, 20.0), Vec2::new(400.0, 200.0));
        let size = Vec2::new(120.0, 60.0);

        for (align, expected_x) in [
            (Align::Min, target.left()),
            (Align::Center, target.center().x),
            (Align::Max, target.right()),
        ] {
            let origin = galley_origin(target, size, align, Align::Center);
            assert_eq!(origin.x, expected_x);
            assert_eq!(origin.y, target.center().y - size.y / 2.0);
            let painted = match align {
                Align::Min => Rect::from_min_size(origin, size),
                Align::Center => Rect::from_center_size(origin, size),
                Align::Max => Rect::from_min_max(
                    Pos2::new(origin.x - size.x, origin.y),
                    Pos2::new(origin.x, origin.y + size.y),
                ),
            };
            let actual_anchor = match align {
                Align::Min => painted.left(),
                Align::Center => painted.center().x,
                Align::Max => painted.right(),
            };
            assert_eq!(actual_anchor, expected_x);
        }
    }
}
