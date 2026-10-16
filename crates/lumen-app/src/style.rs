use egui::{Color32, FontData, FontDefinitions, FontFamily, FontId, RichText, Stroke, TextStyle};

pub const CANVAS: Color32 = Color32::from_rgb(18, 19, 22);
pub const PANEL: Color32 = Color32::from_rgb(26, 28, 32);
pub const RAISED: Color32 = Color32::from_rgb(35, 38, 43);
pub const SURFACE: Color32 = RAISED;
pub const LINE: Color32 = Color32::from_rgb(46, 50, 56);
pub const TEXT: Color32 = Color32::from_rgb(236, 232, 225);
pub const TEXT_DIM: Color32 = Color32::from_rgb(163, 158, 150);
pub const TEXT_FAINT: Color32 = Color32::from_rgb(119, 114, 107);
pub const MUTED: Color32 = TEXT_DIM;
pub const SAFFRON: Color32 = Color32::from_rgb(242, 163, 58);
pub const SAFFRON_INK: Color32 = Color32::from_rgb(26, 18, 6);
pub const LIVE: Color32 = Color32::from_rgb(229, 72, 77);
pub const WHITE: Color32 = Color32::WHITE;
pub const BLACK: Color32 = Color32::BLACK;
pub const CAPTION_GRADIENT: Color32 = Color32::from_black_alpha(217);
pub const OVERLAY_PLATE: Color32 = Color32::from_black_alpha(180);
pub const OK: Color32 = Color32::from_rgb(70, 167, 88);
pub const WARN: Color32 = Color32::from_rgb(245, 184, 61);
pub const INFO: Color32 = Color32::from_rgb(91, 155, 213);
pub const CAPTION: f32 = 11.0;
pub const BODY: f32 = 13.0;
pub const HEADING: f32 = 15.0;
pub const TITLE: f32 = 20.0;
pub const CLOCK: f32 = 28.0;
pub const CONTROL: f32 = 28.0;
pub const PRIMARY: f32 = 40.0;
pub const PADDING: i8 = 12;
pub const GAP: f32 = 8.0;
pub const SECTION_GAP: f32 = 20.0;
pub const LEFT_WIDTH: f32 = 280.0;
pub const RIGHT_WIDTH: f32 = 360.0;
pub const ON_DECK_RAIL: f32 = 3.0;

pub fn install(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Dark);
    let mut fonts = FontDefinitions::default();
    for (name, bytes) in [
        (
            "Poppins",
            include_bytes!("../assets/fonts/Poppins-Regular.ttf").as_slice(),
        ),
        (
            "Poppins Medium",
            include_bytes!("../assets/fonts/Poppins-Medium.ttf").as_slice(),
        ),
        (
            "Poppins SemiBold",
            include_bytes!("../assets/fonts/Poppins-SemiBold.ttf").as_slice(),
        ),
        (
            "Poppins Bold",
            include_bytes!("../assets/fonts/Poppins-Bold.ttf").as_slice(),
        ),
        (
            "Poppins Italic",
            include_bytes!("../assets/fonts/Poppins-Italic.ttf").as_slice(),
        ),
    ] {
        fonts
            .font_data
            .insert(name.into(), FontData::from_static(bytes).into());
        fonts
            .families
            .insert(FontFamily::Name(name.into()), vec![name.into()]);
    }
    fonts
        .families
        .get_mut(&FontFamily::Proportional)
        .unwrap()
        .insert(0, "Poppins".into());
    fonts.font_data.insert(
        "Phosphor".into(),
        FontData::from_static(include_bytes!("../assets/fonts/Phosphor.ttf")).into(),
    );
    fonts
        .families
        .insert(FontFamily::Name("Phosphor".into()), vec!["Phosphor".into()]);
    // Poppins has tabular-width figures; use it for timers rather than a system monospace.
    fonts
        .families
        .get_mut(&FontFamily::Monospace)
        .unwrap()
        .insert(0, "Poppins".into());
    ctx.set_fonts(fonts);
    ctx.style_mut(|s| {
        s.text_styles.insert(
            TextStyle::Heading,
            FontId::new(TITLE, FontFamily::Name("Poppins SemiBold".into())),
        );
        s.text_styles
            .insert(TextStyle::Body, FontId::proportional(BODY));
        s.text_styles.insert(
            TextStyle::Button,
            FontId::new(BODY, FontFamily::Name("Poppins Medium".into())),
        );
        s.text_styles
            .insert(TextStyle::Small, FontId::proportional(CAPTION));
        s.spacing.item_spacing = egui::vec2(GAP, GAP);
        s.spacing.button_padding = egui::vec2(10.0, 5.0);
        s.spacing.interact_size.y = CONTROL;
        s.spacing.text_edit_width = 220.0;
        s.animation_time = 0.12;
    });
    let mut v = egui::Visuals::dark();
    v.override_text_color = Some(TEXT);
    v.panel_fill = PANEL;
    v.window_fill = PANEL;
    v.extreme_bg_color = CANVAS;
    v.faint_bg_color = SURFACE;
    v.selection.bg_fill = SAFFRON.gamma_multiply(0.15);
    v.selection.stroke = Stroke::new(2.0_f32, SAFFRON);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, LINE);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT_DIM);
    v.weak_text_color = Some(TEXT_FAINT);
    v.widgets.inactive.weak_bg_fill = RAISED;
    v.widgets.inactive.bg_fill = RAISED;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, LINE);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT);
    v.widgets.hovered.weak_bg_fill = LINE;
    v.widgets.hovered.bg_fill = LINE;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, TEXT_DIM);
    v.widgets.active.bg_fill = RAISED;
    v.widgets.active.weak_bg_fill = RAISED;
    v.widgets.active.bg_stroke = Stroke::new(2.0_f32, SAFFRON);
    v.text_cursor.stroke = Stroke::new(2.0_f32, SAFFRON);
    v.text_cursor.preview = true;
    ctx.set_visuals_of(egui::Theme::Dark, v);
}

pub fn panel_frame() -> egui::Frame {
    egui::Frame::new().fill(PANEL).inner_margin(PADDING)
}
pub fn section(ui: &mut egui::Ui, text: &str) {
    ui.add_space(SECTION_GAP);
    ui.label(RichText::new(text).size(HEADING).strong());
    ui.add_space(GAP / 2.0);
}
pub fn primary(text: &str) -> egui::Button<'_> {
    egui::Button::new(RichText::new(text).strong().color(SAFFRON_INK))
        .fill(SAFFRON)
        .stroke(Stroke::NONE)
        .corner_radius(4)
}

pub fn icon(glyph: char, size: f32) -> RichText {
    RichText::new(glyph.to_string()).font(FontId::new(size, FontFamily::Name("Phosphor".into())))
}
