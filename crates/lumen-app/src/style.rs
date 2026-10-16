use egui::{Color32, FontData, FontDefinitions, FontFamily, FontId, RichText, Stroke, TextStyle};

pub const CANVAS: Color32 = Color32::from_rgb(16, 19, 23);
pub const PANEL: Color32 = Color32::from_rgb(23, 27, 32);
pub const SURFACE: Color32 = Color32::from_rgb(31, 36, 42);
pub const LINE: Color32 = Color32::from_rgb(49, 57, 64);
pub const TEXT: Color32 = Color32::from_rgb(236, 239, 234);
pub const MUTED: Color32 = Color32::from_rgb(148, 159, 168);
pub const ACCENT: Color32 = Color32::from_rgb(192, 236, 147);
pub const LIVE: Color32 = Color32::from_rgb(249, 130, 118);

pub fn install(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Dark);
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "Geist".into(),
        FontData::from_static(include_bytes!("../assets/fonts/Geist-Regular.ttf")).into(),
    );
    fonts
        .families
        .get_mut(&FontFamily::Proportional)
        .unwrap()
        .insert(0, "Geist".into());
    ctx.set_fonts(fonts);
    ctx.style_mut(|s| {
        s.text_styles
            .insert(TextStyle::Heading, FontId::proportional(20.0));
        s.text_styles
            .insert(TextStyle::Body, FontId::proportional(14.0));
        s.text_styles
            .insert(TextStyle::Button, FontId::proportional(13.0));
        s.text_styles
            .insert(TextStyle::Small, FontId::proportional(11.0));
        s.spacing.item_spacing = egui::vec2(9.0, 10.0);
        s.spacing.button_padding = egui::vec2(12.0, 8.0);
        s.spacing.interact_size.y = 30.0;
        s.spacing.text_edit_width = 240.0;
        s.animation_time = 0.13;
    });
    // Set the dark theme's visuals too: OS theme updates must not restore a
    // light palette over the live-control workbench.
    ctx.set_visuals_of(egui::Theme::Dark, {
        let mut v = egui::Visuals::dark();
        v.override_text_color = Some(TEXT);
        v.panel_fill = PANEL;
        v.window_fill = PANEL;
        v.extreme_bg_color = CANVAS;
        v.faint_bg_color = SURFACE;
        v.selection.bg_fill = Color32::from_rgb(54, 72, 47);
        v.selection.stroke = Stroke::new(1.0_f32, ACCENT);
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, LINE);
        v.widgets.inactive.weak_bg_fill = SURFACE;
        v.widgets.inactive.bg_fill = SURFACE;
        v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, LINE);
        v.widgets.hovered.weak_bg_fill = Color32::from_rgb(45, 53, 58);
        v.widgets.hovered.bg_fill = Color32::from_rgb(45, 53, 58);
        v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, MUTED);
        v.widgets.active.bg_fill = Color32::from_rgb(62, 79, 52);
        v.widgets.active.weak_bg_fill = Color32::from_rgb(62, 79, 52);
        v.widgets.active.bg_stroke = Stroke::new(1.0_f32, ACCENT);
        v
    });
}

pub fn panel_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(PANEL)
        .inner_margin(16)
        .stroke(Stroke::new(1.0_f32, LINE))
}

pub fn section(ui: &mut egui::Ui, text: &str) {
    ui.add_space(5.0);
    ui.label(RichText::new(text).size(13.0).strong().color(MUTED));
    ui.add_space(3.0);
}

pub fn primary(text: &str) -> egui::Button<'_> {
    egui::Button::new(RichText::new(text).strong().color(CANVAS))
        .fill(ACCENT)
        .stroke(Stroke::NONE)
        .corner_radius(5)
}
