use egui::{self, RichText};
use lumen_core::{
    CueAction, MessageTemplate, Project, Prop, Rect, Runtime, SlideObject, TextAlign, TextStyle,
    TimerDef, TimerKind,
};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimerTone {
    Normal,
    Warning,
    Live,
}
#[cfg(test)]
mod countdown_tone_tests {
    use super::{TimerKind, timer_value_for_tone};
    #[test]
    fn countdown_to_tone_uses_wall_clock_and_overrun_policy() {
        let kind = TimerKind::CountdownTo {
            hour: 1,
            minute: 0,
            second: 0,
        };
        assert_eq!(timer_value_for_tone(&kind, 0.0, 3_590, true), 10.0);
        assert_eq!(timer_value_for_tone(&kind, 0.0, 3_610, true), -10.0);
        assert_eq!(timer_value_for_tone(&kind, 0.0, 3_610, false), 0.0);
    }
}

pub fn timer_tone(value_secs: f64, warn_at_secs: Option<f64>) -> TimerTone {
    if value_secs <= 0.0 {
        TimerTone::Live
    } else if warn_at_secs.is_some_and(|at| value_secs <= at) {
        TimerTone::Warning
    } else {
        TimerTone::Normal
    }
}

fn timer_value_for_tone(kind: &TimerKind, state_value: f64, now: u32, allow_overrun: bool) -> f64 {
    match kind {
        TimerKind::CountdownTo {
            hour,
            minute,
            second,
        } => {
            let target = *hour as f64 * 3600.0 + *minute as f64 * 60.0 + *second as f64;
            let remaining = target - now as f64;
            if allow_overrun {
                remaining
            } else {
                remaining.max(0.0)
            }
        }
        _ => state_value,
    }
}

pub fn timers(ui: &mut egui::Ui, project: &mut Project, runtime: &mut Runtime, now: u32) -> bool {
    let mut changed = false;
    runtime.sync_timers(project);
    let snapshot = project.clone();
    ui.heading("Timers");
    if ui.button("Add timer").clicked() {
        let id = Uuid::new_v4();
        project.timers.push(TimerDef {
            id,
            name: "New timer".into(),
            kind: TimerKind::Countdown {
                duration_secs: 300.0,
            },
            allow_overrun: false,
            warn_at_secs: Some(30.0),
        });
        runtime.sync_timers(project);
        changed = true;
    }
    if project.timers.is_empty() {
        ui.label("No timers yet — add a timer to start a countdown.");
    }
    let mut remove = None;
    for (i, t) in project.timers.iter_mut().enumerate() {
        let used = project.documents.iter().flat_map(|d| &d.slides).flat_map(|s| &s.actions).chain(project.macros.iter().flat_map(|m| &m.actions)).any(|a| matches!(a, CueAction::StartTimer{id} | CueAction::PauseTimer{id} | CueAction::ResetTimer{id} if *id == t.id));
        let original_kind = t.kind.clone();
        ui.push_id(t.id, |ui| {
            if crate::text::contains_gujarati(&t.name) {
                crate::text::gu_label(ui, &t.name, crate::style::BODY);
            }
            ui.horizontal_wrapped(|ui| {
                changed |= ui
                    .add(egui::TextEdit::singleline(&mut t.name).desired_width(104.0))
                    .changed();
                let value = runtime.timer_display(t, now);
                let value_secs = timer_value_for_tone(
                    &t.kind,
                    runtime
                        .timer_states
                        .get(&t.id)
                        .map_or(0.0, |s| s.value_secs),
                    now,
                    t.allow_overrun,
                );
                let tone = timer_tone(value_secs, t.warn_at_secs);
                let color = match tone {
                    TimerTone::Normal => crate::style::TEXT,
                    TimerTone::Warning => crate::style::WARN,
                    TimerTone::Live => crate::style::LIVE,
                };
                ui.label(RichText::new(value).monospace().size(18.0).color(color));
                if ui
                    .button(
                        if runtime.timer_states.get(&t.id).is_some_and(|s| s.running) {
                            "Pause"
                        } else {
                            "Start"
                        },
                    )
                    .clicked()
                {
                    let action = if runtime.timer_states.get(&t.id).is_some_and(|s| s.running) {
                        CueAction::PauseTimer { id: t.id }
                    } else {
                        CueAction::StartTimer { id: t.id }
                    };
                    let _ = runtime.apply_actions(&snapshot, &[action]);
                }
                if ui.button("Reset").clicked() {
                    let _ = runtime.apply_actions(&snapshot, &[CueAction::ResetTimer { id: t.id }]);
                }
                ui.menu_button(crate::style::icon('\u{e1fe}', 16.0), |ui| {
                    if ui
                        .add_enabled(!used, egui::Button::new("Delete timer"))
                        .clicked()
                    {
                        remove = Some(i);
                        ui.close();
                    }
                });
            });
            if used {
                ui.small("Used by a slide or macro action; remove those references first.");
            }
            egui::ComboBox::from_id_salt(("timer-kind", t.id))
                .selected_text(match &t.kind {
                    TimerKind::Countdown { .. } => "Countdown duration",
                    TimerKind::CountdownTo { .. } => "Countdown to time",
                    TimerKind::Elapsed => "Elapsed",
                })
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(
                            matches!(&t.kind, TimerKind::Countdown { .. }),
                            "Countdown duration",
                        )
                        .clicked()
                    {
                        t.kind = TimerKind::Countdown {
                            duration_secs: 300.0,
                        };
                        changed = true;
                    }
                    if ui
                        .selectable_label(
                            matches!(&t.kind, TimerKind::CountdownTo { .. }),
                            "Countdown to time",
                        )
                        .clicked()
                    {
                        t.kind = TimerKind::CountdownTo {
                            hour: 12,
                            minute: 0,
                            second: 0,
                        };
                        changed = true;
                    }
                    if ui
                        .selectable_label(matches!(&t.kind, TimerKind::Elapsed), "Elapsed")
                        .clicked()
                    {
                        t.kind = TimerKind::Elapsed;
                        changed = true;
                    }
                });
            match &mut t.kind {
                TimerKind::Countdown { duration_secs } => {
                    changed |= ui
                        .add(
                            egui::DragValue::new(duration_secs)
                                .range(0.0..=86400.0)
                                .suffix(" sec"),
                        )
                        .changed();
                }
                TimerKind::CountdownTo {
                    hour,
                    minute,
                    second,
                } => {
                    ui.horizontal_wrapped(|ui| {
                        changed |= ui
                            .add(egui::DragValue::new(hour).range(0..=23).prefix("HH "))
                            .changed();
                        changed |= ui
                            .add(egui::DragValue::new(minute).range(0..=59).prefix("MM "))
                            .changed();
                        changed |= ui
                            .add(egui::DragValue::new(second).range(0..=59).prefix("SS "))
                            .changed();
                    });
                }
                TimerKind::Elapsed => {}
            }
            changed |= ui.checkbox(&mut t.allow_overrun, "Allow overrun").changed();
            let mut warn = t.warn_at_secs.unwrap_or(0.0);
            let had = t.warn_at_secs.is_some();
            ui.horizontal_wrapped(|ui| {
                let mut enabled = had;
                if ui.checkbox(&mut enabled, "Warn at").changed() {
                    t.warn_at_secs = enabled.then_some(warn);
                    changed = true;
                }
                if enabled
                    && ui
                        .add(
                            egui::DragValue::new(&mut warn)
                                .range(0.0..=86400.0)
                                .suffix(" sec"),
                        )
                        .changed()
                {
                    t.warn_at_secs = Some(warn);
                    changed = true;
                }
            });
        });
        if t.kind != original_kind
            && let Some(state) = runtime.timer_states.get_mut(&t.id)
        {
            state.running = false;
            state.value_secs = match &t.kind {
                TimerKind::Countdown { duration_secs } => *duration_secs,
                TimerKind::CountdownTo { .. } | TimerKind::Elapsed => 0.0,
            };
        }
    }
    if let Some(i) = remove {
        let id = project.timers.remove(i).id;
        runtime.timer_states.remove(&id);
        changed = true;
    }
    changed
}

pub fn messages(ui: &mut egui::Ui, project: &mut Project, runtime: &mut Runtime, now: u32) -> bool {
    let mut changed = false;
    let snapshot = project.clone();
    ui.heading("Messages");
    if ui.button("Add message").clicked() {
        project.messages.push(MessageTemplate {
            id: Uuid::new_v4(),
            name: "New message".into(),
            text: String::new(),
        });
        changed = true;
    }
    if project.messages.is_empty() {
        ui.label("No messages yet — add a message to show one on screen.");
    }
    let mut remove = None;
    for (i, m) in project.messages.iter_mut().enumerate() {
        let used = project
            .documents
            .iter()
            .flat_map(|d| &d.slides)
            .flat_map(|s| &s.actions)
            .chain(project.macros.iter().flat_map(|m| &m.actions))
            .any(|a| matches!(a, CueAction::ShowMessage{id} if *id == m.id));
        ui.push_id(m.id, |ui| {
            if crate::text::contains_gujarati(&m.name) {
                crate::text::gu_label(ui, &m.name, crate::style::BODY);
            }
            ui.horizontal_wrapped(|ui| {
                changed |= ui.text_edit_singleline(&mut m.name).changed();
                if ui.button("Show").clicked() {
                    let _ =
                        runtime.apply_actions(&snapshot, &[CueAction::ShowMessage { id: m.id }]);
                }
                ui.menu_button(crate::style::icon('\u{e1fe}', 16.0), |ui| {
                    if ui
                        .add_enabled(!used, egui::Button::new("Delete message"))
                        .clicked()
                    {
                        remove = Some(i);
                        ui.close();
                    }
                });
            });
            if used {
                ui.small("Used by a slide or macro action; remove those references first.");
            }
            changed |= ui
                .add(
                    egui::TextEdit::multiline(&mut m.text)
                        .desired_rows(3)
                        .hint_text("Message text; insert {timer:<id>} for a timer"),
                )
                .changed();
            ui.menu_button("Insert timer", |ui| {
                for t in &project.timers {
                    if crate::text::gu_selectable(ui, false, &t.name, crate::style::BODY).clicked()
                    {
                        m.text.push_str(&format!("{{timer:{}}}", t.id));
                        changed = true;
                    }
                }
            });
        });
    }
    if let Some(i) = remove {
        project.messages.remove(i);
        changed = true;
    }
    if ui.button("Clear messages").clicked() {
        let _ = runtime.apply_actions(&snapshot, &[CueAction::ClearMessages]);
    }
    for id in project.messages.iter().map(|m| m.id) {
        if let Some(rendered) = runtime.render_message(project, id, now) {
            crate::text::gu_label(ui, &rendered, crate::style::CAPTION);
        }
    }
    changed
}

pub fn props(ui: &mut egui::Ui, project: &mut Project, runtime: &mut Runtime) -> bool {
    let mut changed = false;
    changed |= overlay_creator(ui, project);
    if runtime.output.deck.is_some() {
        ui.small("Overlays are unavailable while PowerPoint or Keynote is playing.");
    }
    let snapshot = project.clone();
    if project.props.is_empty() {
        ui.label("No overlays yet — add a lower third or logo.");
    }
    let mut remove = None;
    for (i, p) in project.props.iter_mut().enumerate() {
        let used = project
            .documents
            .iter()
            .flat_map(|d| &d.slides)
            .flat_map(|s| &s.actions)
            .chain(project.macros.iter().flat_map(|m| &m.actions))
            .any(|a| matches!(a, CueAction::TriggerProp{id} | CueAction::HideProp{id} if *id == p.id));
        ui.push_id(p.id, |ui| {
            if crate::text::contains_gujarati(&p.name) {
                crate::text::gu_label(ui, &p.name, crate::style::BODY);
            }
            ui.horizontal_wrapped(|ui| {
                changed |= ui.text_edit_singleline(&mut p.name).changed();
                let on = runtime.output.props.contains(&p.id);
                if ui.button(if on { "On" } else { "Off" }).clicked() {
                    let action = if on {
                        CueAction::HideProp { id: p.id }
                    } else {
                        CueAction::TriggerProp { id: p.id }
                    };
                    let _ = runtime.apply_actions(&snapshot, &[action]);
                }
                ui.menu_button(crate::style::icon('\u{e1fe}', 16.0), |ui| {
                    if ui
                        .add_enabled(!used, egui::Button::new("Delete overlay"))
                        .clicked()
                    {
                        remove = Some(i);
                        ui.close();
                    }
                });
            });
            if used {
                ui.small("Used by a slide or macro action; remove those references first.");
            }
            ui.collapsing("Edit", |ui| {
                changed |= crate::inspector::objects(ui, &mut p.objects)
            });
        });
    }
    if let Some(i) = remove {
        let id = project.props.remove(i).id;
        let _ = runtime.apply_actions(&snapshot, &[CueAction::HideProp { id }]);
        changed = true;
    }
    if ui.small_button("All overlays off").clicked() {
        let _ = runtime.apply_actions(&snapshot, &[CueAction::ClearProps]);
    }
    changed
}

fn overlay_creator(ui: &mut egui::Ui, project: &mut Project) -> bool {
    let mut changed = false;
    let name_id = ui.make_persistent_id("lower-third-name");
    let line_id = ui.make_persistent_id("lower-third-line");
    let mut name = ui
        .ctx()
        .data_mut(|d| d.get_temp::<String>(name_id).unwrap_or_default());
    let mut line = ui
        .ctx()
        .data_mut(|d| d.get_temp::<String>(line_id).unwrap_or_default());
    ui.horizontal_wrapped(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut name)
                .hint_text("Name")
                .desired_width(110.0),
        );
        ui.add(
            egui::TextEdit::singleline(&mut line)
                .hint_text("Second line")
                .desired_width(110.0),
        );
        if ui.button("+ Lower third").clicked() && !name.trim().is_empty() {
            project.props.push(lower_third(name.trim(), line.trim()));
            name.clear();
            line.clear();
            changed = true;
        }
    });
    ui.ctx().data_mut(|d| {
        d.insert_temp(name_id, name);
        d.insert_temp(line_id, line);
    });

    let corner_id = ui.make_persistent_id("logo-corner");
    let size_id = ui.make_persistent_id("logo-size");
    let mut corner = ui
        .ctx()
        .data_mut(|d| d.get_temp::<usize>(corner_id).unwrap_or(1));
    let mut size = ui
        .ctx()
        .data_mut(|d| d.get_temp::<usize>(size_id).unwrap_or(1));
    let notice_id = ui.make_persistent_id("logo-notice");
    let mut notice = ui
        .ctx()
        .data_mut(|d| d.get_temp::<String>(notice_id).unwrap_or_default());
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("logo-corner")
            .selected_text(["Top left", "Top right", "Bottom left", "Bottom right"][corner])
            .show_ui(ui, |ui| {
                for (i, label) in ["Top left", "Top right", "Bottom left", "Bottom right"]
                    .iter()
                    .enumerate()
                {
                    ui.selectable_value(&mut corner, i, *label);
                }
            });
        egui::ComboBox::from_id_salt("logo-size")
            .selected_text(["S · 8%", "M · 12%", "L · 18%"][size])
            .show_ui(ui, |ui| {
                for (i, label) in ["S · 8%", "M · 12%", "L · 18%"].iter().enumerate() {
                    ui.selectable_value(&mut size, i, *label);
                }
            });
        if ui.button("+ Logo…").clicked()
            && let Some(path) = rfd::FileDialog::new()
                .add_filter("Image", &["png", "jpg", "jpeg", "webp"])
                .pick_file()
        {
            match image::image_dimensions(&path) {
                Ok((w, h)) if w > 0 && h > 0 => {
                    if let Some(rect) = logo_rect(w, h, size, corner) {
                        project.props.push(Prop {
                            id: Uuid::new_v4(),
                            name: path
                                .file_stem()
                                .and_then(|n| n.to_str())
                                .unwrap_or("Logo")
                                .to_owned(),
                            objects: vec![SlideObject::Image {
                                id: Uuid::new_v4(),
                                rect,
                                path,
                            }],
                        });
                        changed = true;
                        notice.clear();
                    } else {
                        notice = "This logo is too tall at the selected size.".into();
                    }
                }
                _ => notice = "Couldn't read that image. Choose a PNG, JPG, or WebP file.".into(),
            }
        }
    });
    ui.ctx().data_mut(|d| {
        d.insert_temp(corner_id, corner);
        d.insert_temp(size_id, size);
        d.insert_temp(notice_id, notice.clone());
    });
    if !notice.is_empty() {
        ui.small(notice);
    }
    changed
}

fn lower_third(name: &str, second_line: &str) -> Prop {
    let plate = Rect {
        x: 0.05,
        y: 0.74,
        w: 0.42,
        h: 0.14,
    };
    Prop {
        id: Uuid::new_v4(),
        name: name.to_owned(),
        objects: vec![
            SlideObject::Shape {
                id: Uuid::new_v4(),
                rect: plate.clone(),
                color: crate::style::OVERLAY_PLATE.to_array(),
                corner_radius: 6.0 / 1080.0,
            },
            SlideObject::Shape {
                id: Uuid::new_v4(),
                rect: Rect {
                    x: 0.05,
                    y: 0.74,
                    w: 0.004,
                    h: 0.14,
                },
                color: [
                    crate::style::SAFFRON.r(),
                    crate::style::SAFFRON.g(),
                    crate::style::SAFFRON.b(),
                    255,
                ],
                corner_radius: 0.0,
            },
            SlideObject::Text {
                id: Uuid::new_v4(),
                rect: Rect {
                    x: 0.075,
                    y: 0.755,
                    w: 0.37,
                    h: 0.055,
                },
                text: format!("**{}**", lumen_core::markup::escape_markup(name)),
                style: TextStyle {
                    font_size: Some(44.0),
                    color: Some(crate::style::WHITE.to_array()),
                    ..Default::default()
                },
                align: TextAlign::Left,
            },
            SlideObject::Text {
                id: Uuid::new_v4(),
                rect: Rect {
                    x: 0.075,
                    y: 0.815,
                    w: 0.37,
                    h: 0.045,
                },
                text: lumen_core::markup::escape_markup(second_line),
                style: TextStyle {
                    font_size: Some(30.0),
                    color: Some(crate::style::WHITE.to_array()),
                    ..Default::default()
                },
                align: TextAlign::Left,
            },
        ],
    }
}

fn logo_rect(width: u32, height: u32, size: usize, corner: usize) -> Option<Rect> {
    if width == 0 || height == 0 {
        return None;
    }
    let frac = *[0.08, 0.12, 0.18].get(size)?;
    if corner > 3 {
        return None;
    }
    let h = frac * 16.0 / 9.0 * height as f32 / width as f32;
    if h > 0.94 {
        return None;
    }
    Some(Rect {
        x: if corner % 2 == 1 { 0.97 - frac } else { 0.03 },
        y: if corner >= 2 { 0.97 - h } else { 0.03 },
        w: frac,
        h,
    })
}

pub fn overlay_chips(ui: &mut egui::Ui, project: &Project, runtime: &mut Runtime) {
    if project.props.is_empty() && runtime.output.deck.is_none() {
        return;
    }
    crate::style::section(ui, "Overlays");
    let snapshot = project.clone();
    ui.horizontal_wrapped(|ui| {
        for overlay in &project.props {
            let on = runtime.output.props.contains(&overlay.id);
            let chip = format!("{} · {}", overlay.name, if on { "On" } else { "Off" });
            let width = (chip.chars().count() as f32 * crate::style::BODY * 0.8 + 16.0)
                .clamp(88.0, ui.available_width().clamp(88.0, 240.0));
            let response = ui
                .scope_builder(
                    egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                        ui.cursor().min,
                        egui::vec2(width, crate::style::CONTROL),
                    )),
                    |chip_ui| crate::text::gu_selectable(chip_ui, on, &chip, crate::style::BODY),
                )
                .inner;
            if response.clicked() {
                let action = if on {
                    CueAction::HideProp { id: overlay.id }
                } else {
                    CueAction::TriggerProp { id: overlay.id }
                };
                let _ = runtime.apply_actions(&snapshot, &[action]);
            }
        }
        if ui.small_button("All overlays off").clicked() {
            let _ = runtime.apply_actions(&snapshot, &[CueAction::ClearProps]);
        }
    });
    if runtime.output.deck.is_some() {
        ui.small("Overlays are unavailable while PowerPoint or Keynote is playing.");
    }
}

#[cfg(test)]
mod overlay_tests {
    use super::{logo_rect, lower_third};
    use lumen_core::{SlideObject, markup::strip_markup};

    #[test]
    fn lower_third_has_expected_geometry_and_literal_text() {
        let prop = lower_third("A **bold** & guest", "Second [line]");
        assert_eq!(prop.name, "A **bold** & guest");
        assert_eq!(prop.objects.len(), 4);
        let SlideObject::Shape {
            rect,
            color,
            corner_radius,
            ..
        } = &prop.objects[0]
        else {
            panic!("plate should be first")
        };
        assert_eq!((rect.x, rect.y, rect.w, rect.h), (0.05, 0.74, 0.42, 0.14));
        assert_eq!(*color, crate::style::OVERLAY_PLATE.to_array());
        assert_eq!(*corner_radius, 6.0 / 1080.0);
        let SlideObject::Shape { rect, color, .. } = &prop.objects[1] else {
            panic!("accent should be second")
        };
        assert_eq!((rect.x, rect.y, rect.w, rect.h), (0.05, 0.74, 0.004, 0.14));
        assert_eq!(
            &color[..3],
            &[
                crate::style::SAFFRON.r(),
                crate::style::SAFFRON.g(),
                crate::style::SAFFRON.b(),
            ]
        );
        let SlideObject::Text { text, style, .. } = &prop.objects[2] else {
            panic!("name should be third")
        };
        assert_eq!(strip_markup(text), "A **bold** & guest");
        assert_eq!(style.font_size, Some(44.0));
        let SlideObject::Text { text, .. } = &prop.objects[3] else {
            panic!("second line should be fourth")
        };
        assert_eq!(strip_markup(text), "Second [line]");
    }

    #[test]
    fn logo_rect_preserves_aspect_and_places_each_corner() {
        for size in 0..3 {
            let expected_width = [0.08, 0.12, 0.18][size];
            let expected_height = expected_width;
            for corner in 0..4 {
                let rect = logo_rect(1600, 900, size, corner).unwrap();
                assert!((rect.w / rect.h * 16.0 / 9.0 - 1600.0 / 900.0).abs() < 0.001);
                assert_eq!(rect.w, expected_width);
                assert!((rect.h - expected_height).abs() < 0.000001);
                assert_eq!(
                    rect.x,
                    if corner % 2 == 1 {
                        0.97 - expected_width
                    } else {
                        0.03
                    }
                );
                assert_eq!(
                    rect.y,
                    if corner >= 2 {
                        0.97 - expected_height
                    } else {
                        0.03
                    }
                );
            }
        }
        for (width, height) in [(3000, 1000), (1000, 3000)] {
            let rect = logo_rect(width, height, 0, 0).unwrap();
            assert!((rect.w / rect.h * 16.0 / 9.0 - width as f32 / height as f32).abs() < 0.001);
        }
        assert_eq!(logo_rect(0, 900, 1, 1), None);
        assert_eq!(logo_rect(100, 900, 2, 3), None);
        assert_eq!(logo_rect(1000, 1000, 3, 0), None);
        assert_eq!(logo_rect(1000, 1000, 1, 4), None);
    }
}

#[cfg(test)]
mod tests {
    use super::{TimerTone, timer_tone};
    #[test]
    fn timer_warning_and_zero_thresholds_are_distinct() {
        assert_eq!(timer_tone(31.0, Some(30.0)), TimerTone::Normal);
        assert_eq!(timer_tone(30.0, Some(30.0)), TimerTone::Warning);
        assert_eq!(timer_tone(0.0, Some(30.0)), TimerTone::Live);
        assert_eq!(timer_tone(-1.0, None), TimerTone::Live);
    }
}
