use lumen_core::{
    Background, CueAction, MediaFit, Project, Rect, Slide, SlideObject, TextAlign, TextStyle,
    Transition,
};
use uuid::Uuid;

pub fn slide(ui: &mut egui::Ui, slide: &mut Slide, project: &mut Project) -> bool {
    let mut changed = false;
    ui.heading("Slide inspector");
    changed |= ui.text_edit_singleline(&mut slide.label).changed();
    let saffron = crate::style::SAFFRON;
    ui.label(format!(
        "Text · **bold**  *italic*  __underline__  [#{:02X}{:02X}{:02X}]colour[/]",
        saffron.r(),
        saffron.g(),
        saffron.b()
    ));
    changed |= ui
        .add(egui::TextEdit::multiline(&mut slide.text).desired_rows(5))
        .changed();
    if let Some(theme) = project
        .themes
        .iter_mut()
        .find(|theme| theme.id == slide.theme_id)
    {
        let mut enabled = theme.backdrop == lumen_core::Backdrop::BottomGradient;
        if ui.checkbox(&mut enabled, "Bottom gradient").changed() {
            theme.backdrop = if enabled {
                lumen_core::Backdrop::BottomGradient
            } else {
                lumen_core::Backdrop::None
            };
            changed = true;
        }
    }
    ui.collapsing("Text box (%)", |ui| {
        changed |= rect_editor(ui, &mut slide.text_rect)
    });
    ui.collapsing("Text style", |ui| {
        if let Some(size) = &mut slide.text_style.font_size {
            let mut reset = false;
            ui.horizontal(|ui| {
                changed |= ui
                    .add(
                        egui::DragValue::new(size)
                            .range(8.0..=400.0)
                            .prefix("Size "),
                    )
                    .changed();
                reset = ui.small_button("Reset").clicked();
            });
            if reset {
                slide.text_style.font_size = None;
                changed = true;
            }
        } else if ui.button("Override font size").clicked() {
            slide.text_style.font_size = Some(48.0);
            changed = true;
        }
        if let Some(color) = &mut slide.text_style.color {
            let mut reset = false;
            ui.horizontal(|ui| {
                let mut c = rgba(*color);
                if ui.color_edit_button_srgba(&mut c).changed() {
                    *color = bytes(c);
                    changed = true;
                }
                reset = ui.small_button("Reset").clicked();
            });
            if reset {
                slide.text_style.color = None;
                changed = true;
            }
        } else if ui.button("Override text colour").clicked() {
            slide.text_style.color = Some(bytes(crate::style::WHITE));
            changed = true;
        }
        changed |= ui
            .checkbox(&mut slide.text_style.shadow, "Shadow")
            .changed();
        if let Some((width, color)) = &mut slide.text_style.outline {
            let mut reset = false;
            ui.horizontal(|ui| {
                changed |= ui
                    .add(
                        egui::DragValue::new(width)
                            .range(0.0..=20.0)
                            .prefix("Outline "),
                    )
                    .changed();
                let mut c = rgba(*color);
                if ui.color_edit_button_srgba(&mut c).changed() {
                    *color = bytes(c);
                    changed = true;
                }
                reset = ui.small_button("Reset").clicked();
            });
            if reset {
                slide.text_style.outline = None;
                changed = true;
            }
        } else if ui.button("Add outline").clicked() {
            slide.text_style.outline = Some((2.0, bytes(crate::style::BLACK)));
            changed = true;
        }
    });
    ui.collapsing("Background", |ui| match &mut slide.background {
        Background::Solid(c) => {
            ui.label("Solid");
            let mut col = rgba(*c);
            if ui.color_edit_button_srgba(&mut col).changed() {
                *c = bytes(col);
                changed = true;
            }
        }
        Background::Pdf { path, page } => {
            ui.label(format!("PDF: {} — Page {}", path.display(), page));
        }
        Background::Image { path, fit } => {
            ui.label("Image path");
            let mut s = path.to_string_lossy().into_owned();
            if ui.text_edit_singleline(&mut s).changed() {
                *path = s.into();
                changed = true;
            }
            changed |= scale_combo(ui, fit);
        }
        Background::Video {
            path,
            looped,
            muted,
            fit,
        } => {
            ui.label("Video path");
            let mut s = path.to_string_lossy().into_owned();
            if ui.text_edit_singleline(&mut s).changed() {
                *path = s.into();
                changed = true;
            }
            changed |= ui.checkbox(looped, "Loop").changed();
            changed |= ui.checkbox(muted, "Muted").changed();
            changed |= scale_combo(ui, fit);
        }
    });
    ui.collapsing("Transition", |ui| {
        let mut kind = match slide.transition.clone() {
            Transition::Cut => 0,
            Transition::Fade { .. } => 1,
            Transition::FadeThroughBlack { .. } => 2,
            Transition::Push { .. } => 3,
        };
        egui::ComboBox::from_id_salt("transition-kind")
            .selected_text(["Show default", "Fade", "Fade through black", "Push"][kind])
            .show_ui(ui, |ui| {
                for (i, n) in ["Show default", "Fade", "Fade through black", "Push"]
                    .iter()
                    .enumerate()
                {
                    ui.selectable_value(&mut kind, i, *n);
                }
            });
        let mut millis = match slide.transition.clone() {
            Transition::Cut => 500,
            Transition::Fade { millis }
            | Transition::FadeThroughBlack { millis }
            | Transition::Push { millis, .. } => millis,
        };
        if kind > 0 {
            changed |= ui
                .add(
                    egui::DragValue::new(&mut millis)
                        .range(0..=10_000)
                        .suffix(" ms"),
                )
                .changed();
        }
        let mut direction = match slide.transition.clone() {
            Transition::Push { direction, .. } => direction,
            _ => lumen_core::PushDirection::Left,
        };
        if kind == 3 {
            egui::ComboBox::from_id_salt("push-direction")
                .selected_text(format!("{direction:?}"))
                .show_ui(ui, |ui| {
                    for d in [
                        lumen_core::PushDirection::Left,
                        lumen_core::PushDirection::Right,
                        lumen_core::PushDirection::Up,
                        lumen_core::PushDirection::Down,
                    ] {
                        ui.selectable_value(&mut direction, d, format!("{d:?}"));
                    }
                });
        }
        let next = match kind {
            0 => Transition::Cut,
            1 => Transition::Fade { millis },
            2 => Transition::FadeThroughBlack { millis },
            _ => Transition::Push { millis, direction },
        };
        if next != slide.transition {
            slide.transition = next;
            changed = true;
        }
    });
    ui.collapsing("Audio cue", |ui| {
        if let Some(audio) = &mut slide.audio {
            let mut path = audio.path.to_string_lossy().into_owned();
            if ui.text_edit_singleline(&mut path).changed() {
                audio.path = path.into();
                changed = true;
            }
            changed |= ui
                .add(egui::Slider::new(&mut audio.volume, 0.0..=1.0).text("Volume"))
                .changed();
            if ui.button("Remove audio").clicked() {
                slide.audio = None;
                changed = true;
            }
        } else if ui.button("Add audio cue").clicked() {
            slide.audio = Some(lumen_core::AudioCue {
                path: "".into(),
                volume: 1.0,
            });
            changed = true;
        }
    });
    ui.collapsing("Objects", |ui| changed |= objects(ui, &mut slide.objects));
    ui.collapsing("Cue actions", |ui| {
        changed |= actions(ui, &mut slide.actions, project)
    });
    changed
}

pub fn actions(ui: &mut egui::Ui, actions: &mut Vec<CueAction>, project: &Project) -> bool {
    let mut changed = false;
    let mut remove = None;
    let count = actions.len();
    for i in 0..count {
        ui.horizontal(|ui| {
            ui.label(format!("{}.", i + 1));
            changed |= action_editor(ui, &mut actions[i], project);
            if ui.small_button("Up").clicked() && i > 0 {
                actions.swap(i, i - 1);
                changed = true;
            }
            if ui.small_button("Down").clicked() && i + 1 < count {
                actions.swap(i, i + 1);
                changed = true;
            }
            if ui.small_button("Remove").clicked() {
                remove = Some(i);
            }
        });
    }
    if let Some(i) = remove {
        actions.remove(i);
        changed = true;
    }
    if ui.button("Add action").clicked() {
        actions.push(CueAction::ClearSlide);
        changed = true;
    }
    changed
}
fn action_editor(ui: &mut egui::Ui, a: &mut CueAction, p: &Project) -> bool {
    let names = [
        "Blackout",
        "Clear slide",
        "Clear all",
        "Clear media",
        "Clear audio",
        "Clear messages",
        "All overlays off",
        "Clear message",
        "Stop audio",
        "Stage message",
        "Start timer",
        "Pause timer",
        "Reset timer",
        "Show overlay",
        "Hide overlay",
        "Show message",
        "Play deck",
        "Stop deck",
    ];
    let idx = action_index(a);
    let mut selected = idx;
    egui::ComboBox::from_id_salt(("action", a as *const _ as usize))
        .selected_text(names[idx])
        .show_ui(ui, |ui| {
            for (i, n) in names.iter().enumerate() {
                ui.selectable_value(&mut selected, i, *n);
            }
        });
    let mut changed = false;
    if selected != idx {
        let available = match selected {
            10..=12 => !p.timers.is_empty(),
            13..=14 => !p.props.is_empty(),
            15 => !p.messages.is_empty(),
            _ => true,
        };
        if available {
            *a = default_action(selected, p);
            changed = true;
        } else {
            ui.small("Create a matching timer, overlay, or message before adding this action.");
        }
    }
    let reference_kind = match a {
        CueAction::StartTimer { .. }
        | CueAction::PauseTimer { .. }
        | CueAction::ResetTimer { .. } => 1,
        CueAction::TriggerProp { .. } => 2,
        CueAction::HideProp { .. } => 2,
        CueAction::ShowMessage { .. } => 3,
        _ => 0,
    };
    match a {
        CueAction::Blackout(v) | CueAction::ClearText(v) => {
            changed |= ui.checkbox(v, "On").changed()
        }
        CueAction::StageMessage(s) => changed |= ui.text_edit_singleline(s).changed(),
        CueAction::PlayDeck { path } => {
            let mut text = path.to_string_lossy().into_owned();
            if ui.text_edit_singleline(&mut text).changed() {
                *path = text.into();
                changed = true;
            }
        }
        CueAction::StartTimer { id }
        | CueAction::PauseTimer { id }
        | CueAction::ResetTimer { id }
        | CueAction::TriggerProp { id }
        | CueAction::HideProp { id }
        | CueAction::ShowMessage { id } => {
            let list: Vec<(Uuid, String)> = match reference_kind {
                1 => p.timers.iter().map(|x| (x.id, x.name.clone())).collect(),
                2 => p.props.iter().map(|x| (x.id, x.name.clone())).collect(),
                _ => p.messages.iter().map(|x| (x.id, x.name.clone())).collect(),
            };
            let current = *id;
            let mut chosen = current;
            egui::ComboBox::from_id_salt(("ref", current))
                .selected_text(
                    list.iter()
                        .find(|(x, _)| *x == current)
                        .map(|x| x.1.as_str())
                        .unwrap_or("Choose reference"),
                )
                .show_ui(ui, |ui| {
                    for (candidate, n) in list {
                        ui.selectable_value(&mut chosen, candidate, n);
                    }
                });
            if chosen != current {
                *id = chosen;
                changed = true;
            }
        }
        _ => {}
    }
    changed
}
fn action_index(a: &CueAction) -> usize {
    match a {
        CueAction::Blackout(_) => 0,
        CueAction::ClearText(_) | CueAction::ClearSlide => 1,
        CueAction::ClearAll => 2,
        CueAction::ClearMedia => 3,
        CueAction::ClearAudio => 4,
        CueAction::ClearMessages => 5,
        CueAction::ClearProps => 6,
        CueAction::ClearMessage => 7,
        CueAction::StopAudio => 8,
        CueAction::StageMessage(_) => 9,
        CueAction::StartTimer { .. } => 10,
        CueAction::PauseTimer { .. } => 11,
        CueAction::ResetTimer { .. } => 12,
        CueAction::TriggerProp { .. } => 13,
        CueAction::HideProp { .. } => 14,
        CueAction::ShowMessage { .. } => 15,
        CueAction::PlayDeck { .. } => 16,
        CueAction::StopDeck => 17,
    }
}
fn default_action(i: usize, p: &Project) -> CueAction {
    match i {
        0 => CueAction::Blackout(true),
        1 => CueAction::ClearSlide,
        2 => CueAction::ClearAll,
        3 => CueAction::ClearMedia,
        4 => CueAction::ClearAudio,
        5 => CueAction::ClearMessages,
        6 => CueAction::ClearProps,
        7 => CueAction::ClearMessage,
        8 => CueAction::StopAudio,
        9 => CueAction::StageMessage(String::new()),
        10 => CueAction::StartTimer {
            id: p.timers.first().map_or_else(Uuid::new_v4, |x| x.id),
        },
        11 => CueAction::PauseTimer {
            id: p.timers.first().map_or_else(Uuid::new_v4, |x| x.id),
        },
        12 => CueAction::ResetTimer {
            id: p.timers.first().map_or_else(Uuid::new_v4, |x| x.id),
        },
        13 => CueAction::TriggerProp {
            id: p.props.first().map_or_else(Uuid::new_v4, |x| x.id),
        },
        14 => CueAction::HideProp {
            id: p.props.first().map_or_else(Uuid::new_v4, |x| x.id),
        },
        15 => CueAction::ShowMessage {
            id: p.messages.first().map_or_else(Uuid::new_v4, |x| x.id),
        },
        16 => CueAction::PlayDeck {
            path: Default::default(),
        },
        _ => CueAction::StopDeck,
    }
}

pub fn objects(ui: &mut egui::Ui, objects: &mut Vec<SlideObject>) -> bool {
    let mut changed = false;
    let mut remove = None;
    for (i, obj) in objects.iter_mut().enumerate() {
        ui.push_id(i, |ui| {
            ui.horizontal(|ui| {
                ui.label(match obj {
                    SlideObject::Text { .. } => "Text",
                    SlideObject::Image { .. } => "Image",
                    SlideObject::Shape { .. } => "Shape",
                });
                if ui.small_button("Remove").clicked() {
                    remove = Some(i);
                }
            });
            match obj {
                SlideObject::Text {
                    rect,
                    text,
                    style,
                    align,
                    ..
                } => {
                    changed |= rect_editor(ui, rect);
                    changed |= ui.text_edit_singleline(text).changed();
                    changed |= ui.checkbox(&mut style.shadow, "Shadow").changed();
                    egui::ComboBox::from_id_salt("align")
                        .selected_text(format!("{align:?}"))
                        .show_ui(ui, |ui| {
                            for a in [TextAlign::Left, TextAlign::Center, TextAlign::Right] {
                                if ui.selectable_value(align, a, format!("{a:?}")).changed() {
                                    changed = true;
                                }
                            }
                        });
                }
                SlideObject::Image { rect, path, .. } => {
                    changed |= rect_editor(ui, rect);
                    let mut s = path.to_string_lossy().into_owned();
                    if ui.text_edit_singleline(&mut s).changed() {
                        *path = s.into();
                        changed = true;
                    }
                }
                SlideObject::Shape {
                    rect,
                    color,
                    corner_radius,
                    ..
                } => {
                    changed |= rect_editor(ui, rect);
                    let mut c = rgba(*color);
                    if ui.color_edit_button_srgba(&mut c).changed() {
                        *color = bytes(c);
                        changed = true;
                    }
                    changed |= ui
                        .add(egui::DragValue::new(corner_radius).range(0.0..=0.5))
                        .changed();
                }
            }
        });
    }
    if let Some(i) = remove {
        objects.remove(i);
        changed = true;
    }
    ui.menu_button("Add object", |ui| {
        for label in ["Text", "Shape", "Image"] {
            if ui.button(label).clicked() {
                let id = Uuid::new_v4();
                let r = Rect::default();
                objects.push(match label {
                    "Text" => SlideObject::Text {
                        id,
                        rect: r,
                        text: "Text".into(),
                        style: TextStyle::default(),
                        align: TextAlign::Center,
                    },
                    "Shape" => SlideObject::Shape {
                        id,
                        rect: r,
                        color: bytes(crate::style::WHITE),
                        corner_radius: 0.0,
                    },
                    _ => SlideObject::Image {
                        id,
                        rect: r,
                        path: "".into(),
                    },
                });
                changed = true;
            }
        }
    });
    changed
}
fn rect_editor(ui: &mut egui::Ui, r: &mut Rect) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        let mut x = unit_to_percent(r.x);
        if ui
            .add(
                egui::DragValue::new(&mut x)
                    .range(0.0..=unit_to_percent(1.0 - r.w))
                    .speed(0.5)
                    .prefix("X")
                    .suffix("%"),
            )
            .changed()
        {
            r.x = percent_to_unit(x);
            changed = true;
        }
        let mut y = unit_to_percent(r.y);
        if ui
            .add(
                egui::DragValue::new(&mut y)
                    .range(0.0..=unit_to_percent(1.0 - r.h))
                    .speed(0.5)
                    .prefix("Y")
                    .suffix("%"),
            )
            .changed()
        {
            r.y = percent_to_unit(y);
            changed = true;
        }
        let mut w = unit_to_percent(r.w);
        if ui
            .add(
                egui::DragValue::new(&mut w)
                    .range(0.1..=unit_to_percent(1.0 - r.x))
                    .speed(0.5)
                    .prefix("W")
                    .suffix("%"),
            )
            .changed()
        {
            r.w = percent_to_unit(w);
            changed = true;
        }
        let mut h = unit_to_percent(r.h);
        if ui
            .add(
                egui::DragValue::new(&mut h)
                    .range(0.1..=unit_to_percent(1.0 - r.y))
                    .speed(0.5)
                    .prefix("H")
                    .suffix("%"),
            )
            .changed()
        {
            r.h = percent_to_unit(h);
            changed = true;
        }
    });
    changed
}
fn unit_to_percent(value: f32) -> f32 {
    value * 100.0
}
fn percent_to_unit(value: f32) -> f32 {
    value / 100.0
}

fn rgba(c: [u8; 4]) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])
}
fn bytes(c: egui::Color32) -> [u8; 4] {
    [c.r(), c.g(), c.b(), c.a()]
}
pub fn fit_label(fit: MediaFit) -> &'static str {
    match fit {
        MediaFit::Fit => "Fit (show bars)",
        MediaFit::Fill => "Fill screen (crop)",
        MediaFit::Stretch => "Stretch",
    }
}

/// How a photo or video that is not 16:9 meets the screen; true when it changed.
fn scale_combo(ui: &mut egui::Ui, fit: &mut MediaFit) -> bool {
    let before = *fit;
    ui.horizontal(|ui| {
        ui.label("Scale");
        egui::ComboBox::from_id_salt("media-fit")
            .selected_text(fit_label(*fit))
            .show_ui(ui, |ui| {
                for choice in [MediaFit::Fit, MediaFit::Fill, MediaFit::Stretch] {
                    ui.selectable_value(fit, choice, fit_label(choice));
                }
            });
    });
    *fit != before
}

#[cfg(test)]
mod tests {
    use super::{percent_to_unit, unit_to_percent};
    #[test]
    fn rect_percent_conversion_round_trips() {
        for v in [0.0, 0.05, 0.5, 1.0] {
            assert!((percent_to_unit(unit_to_percent(v)) - v).abs() < f32::EPSILON);
        }
    }
}
