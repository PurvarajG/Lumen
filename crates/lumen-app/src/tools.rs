use egui::{self, RichText};
use lumen_core::{CueAction, MessageTemplate, Project, Prop, Runtime, TimerDef, TimerKind};
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
    let mut remove = None;
    for (i, t) in project.timers.iter_mut().enumerate() {
        let used = project.documents.iter().flat_map(|d| &d.slides).flat_map(|s| &s.actions).chain(project.macros.iter().flat_map(|m| &m.actions)).any(|a| matches!(a, CueAction::StartTimer{id} | CueAction::PauseTimer{id} | CueAction::ResetTimer{id} if *id == t.id));
        let original_kind = t.kind.clone();
        ui.push_id(t.id, |ui| {
            ui.horizontal(|ui| {
                changed |= ui.text_edit_singleline(&mut t.name).changed();
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
                if ui.add_enabled(!used, egui::Button::new("Delete")).clicked() {
                    remove = Some(i);
                }
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
                    ui.horizontal(|ui| {
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
            ui.horizontal(|ui| {
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
            ui.horizontal(|ui| {
                changed |= ui.text_edit_singleline(&mut m.name).changed();
                if ui.button("Show").clicked() {
                    let _ =
                        runtime.apply_actions(&snapshot, &[CueAction::ShowMessage { id: m.id }]);
                }
                if ui.add_enabled(!used, egui::Button::new("Delete")).clicked() {
                    remove = Some(i);
                }
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
                    if ui.button(&t.name).clicked() {
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
            ui.small(rendered);
        }
    }
    changed
}

pub fn props(ui: &mut egui::Ui, project: &mut Project, runtime: &mut Runtime) -> bool {
    let mut changed = false;
    let snapshot = project.clone();
    ui.heading("Props");
    if ui.button("Add prop").clicked() {
        project.props.push(Prop {
            id: Uuid::new_v4(),
            name: "New prop".into(),
            objects: Vec::new(),
        });
        changed = true;
    }
    let mut remove = None;
    for (i, p) in project.props.iter_mut().enumerate() {
        let used = project
            .documents
            .iter()
            .flat_map(|d| &d.slides)
            .flat_map(|s| &s.actions)
            .chain(project.macros.iter().flat_map(|m| &m.actions))
            .any(|a| matches!(a, CueAction::TriggerProp{id} if *id == p.id));
        ui.push_id(p.id, |ui| {
            ui.horizontal(|ui| {
                changed |= ui.text_edit_singleline(&mut p.name).changed();
                if ui.button("Trigger").clicked() {
                    let _ =
                        runtime.apply_actions(&snapshot, &[CueAction::TriggerProp { id: p.id }]);
                }
                if ui.add_enabled(!used, egui::Button::new("Delete")).clicked() {
                    remove = Some(i);
                }
            });
            if used {
                ui.small("Used by a slide or macro action; remove those references first.");
            }
            changed |= crate::inspector::objects(ui, &mut p.objects);
        });
    }
    if let Some(i) = remove {
        let id = project.props.remove(i).id;
        runtime.output.props.retain(|p| *p != id);
        changed = true;
    }
    if ui.button("Clear props").clicked() {
        let _ = runtime.apply_actions(&snapshot, &[CueAction::ClearProps]);
    }
    changed
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
