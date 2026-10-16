use eframe::egui::{self, ColorImage, FontFamily, FontId, TextureHandle, TextureOptions};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Variant {
    Full,
    Compact,
    MarkOnly,
}

fn variant(present: bool, width: f32) -> Variant {
    if present {
        if width >= 1200.0 {
            Variant::Full
        } else if width >= 1000.0 {
            Variant::Compact
        } else {
            Variant::MarkOnly
        }
    } else if width >= 1200.0 {
        Variant::Compact
    } else {
        Variant::MarkOnly
    }
}

#[derive(Clone)]
struct MarkTextures {
    normal: TextureHandle,
    retina: TextureHandle,
}

pub fn lockup(ui: &mut egui::Ui, present: bool, available_width: f32, show_name: &str) {
    let variant = variant(present, available_width);
    let textures_id = egui::Id::new("lumen_brand_mark_textures");
    let textures = ui
        .ctx()
        .data(|data| data.get_temp::<MarkTextures>(textures_id));
    let textures = textures.unwrap_or_else(|| {
        let textures = MarkTextures {
            normal: decode_mark(
                ui.ctx(),
                "Lumen mark 28",
                include_bytes!("../assets/lumen-mark-28.png"),
            ),
            retina: decode_mark(
                ui.ctx(),
                "Lumen mark 56",
                include_bytes!("../assets/lumen-mark-56.png"),
            ),
        };
        ui.ctx()
            .data_mut(|data| data.insert_temp(textures_id, textures.clone()));
        textures
    });
    let texture = if ui.ctx().pixels_per_point() > 1.5 {
        &textures.retina
    } else {
        &textures.normal
    };

    let response = ui
        .with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = crate::style::GAP;
            ui.add(
                egui::Image::new(texture)
                    .fit_to_exact_size(egui::vec2(
                        crate::style::BRAND_MARK,
                        crate::style::BRAND_MARK,
                    ))
                    .alt_text("Lumen Presenter"),
            );

            if variant != Variant::MarkOnly {
                ui.label(
                    egui::RichText::new("Lumen")
                        .font(FontId::new(
                            18.0,
                            FontFamily::Name("Poppins SemiBold".into()),
                        ))
                        .color(crate::style::TEXT),
                );
            }

            if variant == Variant::Full {
                ui.label(
                    egui::RichText::new("PRESENTER")
                        .font(FontId::new(11.0, FontFamily::Name("Poppins Medium".into())))
                        .extra_letter_spacing(1.3)
                        .color(crate::style::TEXT_DIM),
                );
            }
        })
        .response;
    response.on_hover_text(format!("Lumen Presenter · {show_name}"));
}

fn decode_mark(ctx: &egui::Context, name: &str, bytes: &[u8]) -> TextureHandle {
    let image = image::load_from_memory(bytes)
        .expect("embedded Lumen brand mark must be a valid PNG")
        .to_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    ctx.load_texture(
        name,
        ColorImage::from_rgba_unmultiplied(size, image.as_raw()),
        TextureOptions::LINEAR,
    )
}

#[cfg(test)]
mod tests {
    use super::{Variant, variant};

    #[test]
    fn present_variant_uses_the_specified_width_thresholds() {
        assert_eq!(variant(true, 1200.0), Variant::Full);
        assert_eq!(variant(true, 1199.0), Variant::Compact);
        assert_eq!(variant(true, 1000.0), Variant::Compact);
        assert_eq!(variant(true, 999.0), Variant::MarkOnly);
    }

    #[test]
    fn prepare_variant_reserves_room_for_the_project_name() {
        assert_eq!(variant(false, 1200.0), Variant::Compact);
        assert_eq!(variant(false, 1199.0), Variant::MarkOnly);
    }
}
