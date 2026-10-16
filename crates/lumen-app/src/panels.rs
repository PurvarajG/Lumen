use crate::{app::LumenApp, render};
use eframe::egui::{self, Color32, Vec2};
use lumen_core::{
    Background, CueAction, Document, PlaylistItem, Project, Slide, TextAlign, TimerKind,
    VerticalAlign,
};
use uuid::Uuid;

pub fn library(app: &mut LumenApp, ui: &mut egui::Ui) {
    crate::style::section(ui, "Playlist");
    ui.small("Your running order");
    if ui
        .add_enabled(
            app.project.document(app.selected_doc).is_some(),
            egui::Button::new("Add selected document"),
        )
        .clicked()
        && append_playlist_item(&mut app.project, app.selected_doc)
    {
        app.mark();
    }
    egui::ScrollArea::vertical()
        .id_salt("playlist-cues")
        .max_height(260.0)
        .show(ui, |ui| {
            for (index, item) in app.project.playlist.clone().iter().enumerate() {
                let Some(doc) = app.project.document(item.document_id) else {
                    continue;
                };
                let id = doc.id;
                let first = doc.slides.first().map(|s| s.id);
                let selected = app.selected_doc == id;
                let live = app
                    .runtime
                    .output
                    .live
                    .as_ref()
                    .is_some_and(|l| l.document_id == id);
                let title = format!("{}   {}", doc.name, if live { "LIVE" } else { "" });
                let button = egui::Button::new(egui::RichText::new(title).color(if live {
                    crate::style::LIVE
                } else {
                    crate::style::TEXT
                }))
                .selected(selected)
                .min_size(egui::vec2(ui.available_width(), 36.0));
                if ui.add(button).clicked() {
                    app.selected_doc = id;
                    app.selected_slide = first;
                }
                if selected {
                    ui.horizontal(|ui| {
                        if ui.small_button("Up").clicked() && index > 0 {
                            app.project.playlist.swap(index, index - 1);
                            app.mark();
                        }
                        if ui.small_button("Down").clicked()
                            && index + 1 < app.project.playlist.len()
                        {
                            app.project.playlist.swap(index, index + 1);
                            app.mark();
                        }
                        if ui.small_button("Remove").clicked() {
                            app.project.playlist.remove(index);
                            app.mark();
                        }
                    });
                }
            }
        });
    ui.add_space(12.0);
    ui.separator();
    crate::style::section(ui, "Library");
    if ui.button("New document").clicked() {
        let mut doc = Document::new("New document");
        if let Some(theme) = app.project.themes.first() {
            doc.slides.push(Slide::new(theme.id));
        }
        app.selected_doc = doc.id;
        app.selected_slide = doc.slides.first().map(|s| s.id);
        app.project.documents.push(doc);
        app.mark();
    }
    egui::ScrollArea::vertical()
        .id_salt("document-library")
        .max_height(240.0)
        .show(ui, |ui| {
            for doc in app.project.documents.clone() {
                let selected = app.selected_doc == doc.id;
                if ui
                    .selectable_label(selected, format!("{}    {}", doc.name, doc.slides.len()))
                    .clicked()
                {
                    app.selected_doc = doc.id;
                    app.selected_slide = doc.slides.first().map(|s| s.id);
                }
                if selected {
                    let renamed = app
                        .project
                        .documents
                        .iter_mut()
                        .find(|d| d.id == doc.id)
                        .is_some_and(|d| {
                            ui.add(
                                egui::TextEdit::singleline(&mut d.name)
                                    .hint_text("Document name")
                                    .desired_width(ui.available_width()),
                            )
                            .changed()
                        });
                    if renamed {
                        app.mark();
                    }
                    if ui.small_button("Delete document").clicked() {
                        app.confirm_delete = Some(doc.id);
                    }
                }
            }
        });
    ui.add_space(12.0);
    ui.separator();
    crate::style::section(ui, "Macros");
    for mac in app.project.macros.clone() {
        if ui
            .add_sized([ui.available_width(), 30.0], egui::Button::new(&mac.name))
            .clicked()
        {
            if let Err(e) = app.runtime.run_macro(&app.project, mac.id) {
                app.error = e.to_string();
            } else {
                app.status = format!("Macro executed: {}", mac.name);
            }
        }
    }
}

fn append_playlist_item(project: &mut Project, document_id: Uuid) -> bool {
    if project.document(document_id).is_none() {
        return false;
    }
    project.playlist.push(PlaylistItem {
        id: Uuid::new_v4(),
        document_id,
    });
    true
}

pub fn slides(app: &mut LumenApp, ui: &mut egui::Ui) {
    let Some(doc_index) = app
        .project
        .documents
        .iter()
        .position(|d| d.id == app.selected_doc)
    else {
        ui.vertical_centered(|ui| {
            ui.add_space(90.0);
            ui.heading("Your show is ready");
            ui.label("Create a document, then add slides to build your first cues.");
            if ui.button("Create document").clicked() {
                let mut doc = Document::new("New document");
                if let Some(theme) = app.project.themes.first() {
                    doc.slides.push(Slide::new(theme.id));
                }
                app.selected_doc = doc.id;
                app.selected_slide = doc.slides.first().map(|slide| slide.id);
                app.project.documents.push(doc);
                app.mark();
            }
        });
        return;
    };
    crate::style::section(ui, "Preview");
    let preview = app.selected_live();
    let texture = preview
        .as_ref()
        .filter(|p| {
            app.runtime
                .output
                .live
                .as_ref()
                .is_some_and(|l| l.slide.background == p.slide.background)
        })
        .and(app.media.image_texture.as_ref());
    // Reserve a complete thumbnail row and controls before sizing preview.
    let preview_height = (ui.available_width() * 9.0 / 16.0)
        .min((ui.available_height() - 320.0).clamp(160.0, 360.0));
    ui.vertical_centered(|ui| {
        render::preview_frame(ui, preview.as_ref(), false, false, texture, preview_height);
    });
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Edits stay in preview until you go live.")
                .small()
                .color(crate::style::MUTED),
        );
        if ui.add(crate::style::primary("Go Live")).clicked()
            && let Some(cue) = app.selected_live()
        {
            app.trigger(cue.document_id, cue.slide.id);
        }
    });
    ui.add_space(12.0);
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        ui.heading(&app.project.documents[doc_index].name);
        let count = app.project.documents[doc_index].slides.len();
        ui.label(format!(
            "{count} {}",
            if count == 1 { "slide" } else { "slides" }
        ));
        if ui.button("Add slide").clicked()
            && let Some(theme_id) = app.project.themes.first().map(|theme| theme.id)
        {
            let slide = Slide::new(theme_id);
            app.project.documents[doc_index].slides.push(slide.clone());
            app.selected_slide = Some(slide.id);
            changed = true;
        }
        if ui.button("Duplicate").clicked()
            && let Some(index) = app.project.documents[doc_index]
                .slides
                .iter()
                .position(|s| Some(s.id) == app.selected_slide)
        {
            let mut slide = app.project.documents[doc_index].slides[index].clone();
            slide.id = Uuid::new_v4();
            slide.label = format!("{} copy", slide.label);
            app.project.documents[doc_index]
                .slides
                .insert(index + 1, slide.clone());
            app.selected_slide = Some(slide.id);
            changed = true;
        }
        if ui.button("Delete").clicked() {
            let slides = &mut app.project.documents[doc_index].slides;
            if let Some(index) = slides.iter().position(|s| Some(s.id) == app.selected_slide) {
                slides.remove(index);
                app.selected_slide = slides
                    .get(index.min(slides.len().saturating_sub(1)))
                    .map(|s| s.id);
                changed = true;
            }
        }
        for (label, direction) in [("Move up", -1isize), ("Move down", 1)] {
            if ui.button(label).clicked() {
                let slides = &mut app.project.documents[doc_index].slides;
                if let Some(index) = slides.iter().position(|s| Some(s.id) == app.selected_slide) {
                    let target = index as isize + direction;
                    if (0..slides.len() as isize).contains(&target) {
                        slides.swap(index, target as usize);
                        changed = true;
                    }
                }
            }
        }
    });
    if changed {
        app.mark();
    }
    ui.separator();
    let columns = ((ui.available_width() / 220.0).floor() as usize).clamp(1, 4);
    let slides = app.project.documents[doc_index].slides.clone();
    let column_width = (ui.available_width() - (columns - 1) as f32 * ui.spacing().item_spacing.x)
        / columns as f32;
    egui::ScrollArea::vertical()
        .id_salt("slide-grid")
        .show(ui, |ui| {
            for (row, chunk) in slides.chunks(columns).enumerate() {
                ui.horizontal_top(|ui| {
                    for (col, slide) in chunk.iter().enumerate() {
                        let selected = Some(slide.id) == app.selected_slide;
                        let live = app
                            .runtime
                            .output
                            .live
                            .as_ref()
                            .is_some_and(|l| l.slide.id == slide.id);
                        ui.vertical(|ui| {
                            ui.set_width(column_width);
                            let (rect, response) = ui.allocate_exact_size(
                                Vec2::new(column_width, column_width * 9.0 / 16.0),
                                egui::Sense::click(),
                            );
                            let live_slide = app
                                .project
                                .document(app.selected_doc)
                                .and_then(|d| d.slides.iter().find(|s| s.id == slide.id))
                                .and_then(|s| {
                                    app.project.theme(s.theme_id).map(|theme| {
                                        lumen_core::LiveSlide {
                                            document_id: app.selected_doc,
                                            document_name: app.project.documents[doc_index]
                                                .name
                                                .clone(),
                                            slide: s.clone(),
                                            theme: theme.clone(),
                                        }
                                    })
                                });
                            render::paint(ui, rect, live_slide.as_ref(), false, false, None);
                            let (width, color) = if live {
                                (3.0_f32, crate::style::LIVE)
                            } else if selected {
                                (2.0_f32, crate::style::ACCENT)
                            } else {
                                (
                                    1.0_f32,
                                    if response.hovered() {
                                        crate::style::MUTED
                                    } else {
                                        crate::style::LINE
                                    },
                                )
                            };
                            ui.painter().rect_stroke(
                                rect,
                                2.0,
                                egui::Stroke::new(width, color),
                                egui::StrokeKind::Inside,
                            );
                            if response.clicked() {
                                app.selected_slide = Some(slide.id);
                            }
                            if response.double_clicked() {
                                app.trigger(app.selected_doc, slide.id);
                            }
                            ui.horizontal(|ui| {
                                ui.label(format!(
                                    "{:02}  {}",
                                    row * columns + col + 1,
                                    slide.label
                                ));
                                if live {
                                    ui.colored_label(Color32::from_rgb(244, 118, 117), "LIVE");
                                }
                            });
                            if ui.add(egui::Button::new("Go Live").small()).clicked() {
                                app.trigger(app.selected_doc, slide.id);
                            }
                        });
                    }
                });
                ui.add_space(10.0);
            }
        });
}

pub fn inspector(app: &mut LumenApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        crate::style::section(ui, "Live output");
        if app.runtime.output.live.is_some() {
            ui.colored_label(crate::style::LIVE, "LIVE");
        }
    });
    let live = app.runtime.output.live.clone();
    render::preview_frame(
        ui,
        live.as_ref(),
        app.runtime.output.blackout,
        app.runtime.output.clear_text,
        app.media.image_texture.as_ref(),
        190.0,
    );
    ui.label(
        live.as_ref()
            .map(|l| format!("{} | {}", l.document_name, l.slide.label))
            .unwrap_or_else(|| "No cue live".into()),
    );
    ui.horizontal(|ui| {
        if ui.button("Previous").clicked() {
            if let Err(error) = app.runtime.previous(&app.project) {
                app.error = error.to_string();
            } else {
                app.sync_media();
            }
        }
        if ui.button("Next").clicked() {
            if let Err(error) = app.runtime.next(&app.project) {
                app.error = error.to_string();
            } else {
                app.sync_media();
            }
        }
    });
    ui.horizontal(|ui| {
        if ui
            .button(if app.runtime.output.blackout {
                "Restore output"
            } else {
                "Blackout"
            })
            .clicked()
        {
            app.runtime.output.blackout = !app.runtime.output.blackout;
        }
        if ui.button("Clear Text").clicked() {
            app.runtime.output.clear_text = true;
        }
        if ui.button("Clear All").clicked() {
            let _ = app
                .runtime
                .apply_actions(&app.project, &[CueAction::ClearAll]);
            app.media.stop();
        }
    });
    if app.media.player.is_some() {
        let mut stop_video = false;
        if let Some(player) = app.media.player.as_mut() {
            ui.horizontal(|ui| {
                let label = if player.is_paused() {
                    "Resume video"
                } else {
                    "Pause video"
                };
                if ui.button(label).clicked()
                    && let Err(error) = player.set_paused(!player.is_paused())
                {
                    app.error = error.to_string();
                }
                stop_video = ui.button("Stop video").clicked();
            });
        }
        if stop_video {
            let background = app
                .runtime
                .output
                .live
                .as_ref()
                .map(|live| live.slide.background.clone());
            app.media.stop();
            if let Some(background @ Background::Video { .. }) = background {
                app.media.remember_background(background);
            }
        }
    }
    ui.separator();
    ui.horizontal(|ui| {
        ui.selectable_value(&mut app.stage_tools, false, "Edit slide");
        ui.selectable_value(&mut app.stage_tools, true, "Stage tools");
    });
    ui.add_space(6.0);
    if app.stage_tools {
        egui::ScrollArea::vertical()
            .id_salt("stage-tools")
            .show(ui, |ui| runtime_controls(app, ui));
        return;
    }
    crate::style::section(ui, "Selected slide");
    let Some(document_index) = app
        .project
        .documents
        .iter()
        .position(|d| d.id == app.selected_doc)
    else {
        return;
    };
    let Some(slide_index) = app.project.documents[document_index]
        .slides
        .iter()
        .position(|s| Some(s.id) == app.selected_slide)
    else {
        if ui.button("Add first slide").clicked()
            && let Some(theme_id) = app.project.themes.first().map(|t| t.id)
        {
            let slide = Slide::new(theme_id);
            app.project.documents[document_index]
                .slides
                .push(slide.clone());
            app.selected_slide = Some(slide.id);
            app.mark();
        }
        return;
    };
    let mut dirty = false;
    let mut pick_video = None;
    egui::ScrollArea::vertical()
        .id_salt("slide-inspector")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let slide = &mut app.project.documents[document_index].slides[slide_index];
            ui.horizontal(|ui| {
                ui.label("Label");
                dirty |= ui.text_edit_singleline(&mut slide.label).changed();
            });
            ui.label("Slide text");
            dirty |= ui
                .add(
                    egui::TextEdit::multiline(&mut slide.text)
                        .desired_width(f32::INFINITY)
                        .desired_rows(4)
                        .hint_text("Type the words for this cue"),
                )
                .changed();
            ui.label("Operator notes");
            dirty |= ui
                .add(
                    egui::TextEdit::multiline(&mut slide.notes)
                        .desired_width(f32::INFINITY)
                        .desired_rows(2),
                )
                .changed();
            ui.label("Transition: Cut");
            let theme_index = app
                .project
                .themes
                .iter()
                .position(|t| t.id == slide.theme_id)
                .unwrap_or(0);
            let mut selected_theme = theme_index;
            egui::ComboBox::from_label("Theme")
                .selected_text(
                    app.project
                        .themes
                        .get(theme_index)
                        .map(|t| t.name.as_str())
                        .unwrap_or("Missing theme"),
                )
                .show_ui(ui, |ui| {
                    for (index, theme) in app.project.themes.iter().enumerate() {
                        ui.selectable_value(&mut selected_theme, index, &theme.name);
                    }
                });
            if selected_theme != theme_index
                && let Some(theme) = app.project.themes.get(selected_theme)
            {
                slide.theme_id = theme.id;
                dirty = true;
            }
            if let Some(theme) = app
                .project
                .themes
                .iter_mut()
                .find(|t| t.id == slide.theme_id)
            {
                ui.horizontal(|ui| {
                    ui.label("Font size");
                    let mut size = theme.font_size;
                    if ui
                        .add(egui::Slider::new(&mut size, 8.0..=180.0).suffix(" px"))
                        .changed()
                    {
                        theme.font_size = size.clamp(8.0, 400.0);
                        dirty = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Color");
                    let mut color = Color32::from_rgba_unmultiplied(
                        theme.text_color[0],
                        theme.text_color[1],
                        theme.text_color[2],
                        theme.text_color[3],
                    );
                    if ui.color_edit_button_srgba(&mut color).changed() {
                        theme.text_color = [color.r(), color.g(), color.b(), color.a()];
                        dirty = true;
                    }
                });
                ui.horizontal(|ui| {
                    for (name, align) in [
                        ("Left", TextAlign::Left),
                        ("Center", TextAlign::Center),
                        ("Right", TextAlign::Right),
                    ] {
                        if ui.selectable_label(theme.align == align, name).clicked() {
                            theme.align = align;
                            dirty = true;
                        }
                    }
                });
                ui.horizontal(|ui| {
                    for (name, align) in [
                        ("Top", VerticalAlign::Top),
                        ("Middle", VerticalAlign::Middle),
                        ("Bottom", VerticalAlign::Bottom),
                    ] {
                        if ui
                            .selectable_label(theme.vertical_align == align, name)
                            .clicked()
                        {
                            theme.vertical_align = align;
                            dirty = true;
                        }
                    }
                });
            }
            ui.separator();
            ui.label("Background");
            let kind = match &slide.background {
                Background::Solid(_) => "Solid",
                Background::Image { .. } => "Image",
                Background::Video { .. } => "Video",
            };
            egui::ComboBox::from_id_salt("background-kind")
                .selected_text(kind)
                .show_ui(ui, |ui| {
                    if ui.selectable_label(kind == "Solid", "Solid").clicked() {
                        slide.background = Background::Solid(
                            app.project
                                .themes
                                .first()
                                .map(|t| t.background_color)
                                .unwrap_or([18, 22, 30, 255]),
                        );
                        dirty = true;
                    }
                    if ui.selectable_label(kind == "Image", "Image").clicked() {
                        pick_video = Some(false);
                    }
                    if ui.selectable_label(kind == "Video", "Video").clicked() {
                        pick_video = Some(true);
                    }
                });
            if let Background::Solid(rgba) = &mut slide.background {
                let mut color = Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3]);
                if ui.color_edit_button_srgba(&mut color).changed() {
                    *rgba = [color.r(), color.g(), color.b(), color.a()];
                    dirty = true;
                }
            }
            if let Background::Image { path } = &slide.background {
                ui.label(path.file_name().unwrap_or_default().to_string_lossy());
                if ui.button("Choose image...").clicked() {
                    pick_video = Some(false);
                }
            }
            if let Background::Video {
                path,
                looped,
                muted,
            } = &mut slide.background
            {
                ui.label(path.file_name().unwrap_or_default().to_string_lossy());
                dirty |= ui.checkbox(looped, "Loop").changed();
                dirty |= ui.checkbox(muted, "Mute audio").changed();
                if ui.button("Choose video...").clicked() {
                    pick_video = Some(true);
                }
            }
        });
    if dirty {
        app.mark();
    }
    if let Some(video) = pick_video {
        app.choose_media(app.selected_slide.unwrap_or_default(), video);
    }
}

fn runtime_controls(app: &mut LumenApp, ui: &mut egui::Ui) {
    crate::style::section(ui, "Countdown clocks");
    app.runtime.sync_timers(&app.project);
    let mut remove_timers = Vec::new();
    let mut actions = Vec::new();
    let mut changed = false;
    for timer in &mut app.project.timers {
        let running = app
            .runtime
            .timer_states
            .get(&timer.id)
            .is_some_and(|state| state.running);
        changed |= ui
            .horizontal(|ui| {
                let name_changed = ui
                    .add(egui::TextEdit::singleline(&mut timer.name).desired_width(90.0))
                    .changed();
                ui.strong(app.runtime.timer_display(timer, 0));
                name_changed
            })
            .inner;
        ui.horizontal(|ui| {
            if let TimerKind::Countdown { duration_secs } = &mut timer.kind {
                let mut duration = *duration_secs;
                let duration_changed = ui
                    .add(
                        egui::DragValue::new(&mut duration)
                            .range(0.0_f64..=86400.0_f64)
                            .suffix(" sec"),
                    )
                    .changed();
                if duration_changed && !running && duration.is_finite() {
                    *duration_secs = duration;
                    actions.push(CueAction::ResetTimer { id: timer.id });
                    changed = true;
                }
            }
            if ui
                .small_button(if running { "Pause" } else { "Start" })
                .clicked()
            {
                if running {
                    actions.push(CueAction::PauseTimer { id: timer.id });
                } else {
                    actions.push(CueAction::StartTimer { id: timer.id });
                }
            }
            if ui.small_button("Reset").clicked() {
                actions.push(CueAction::ResetTimer { id: timer.id });
            }
            if ui.small_button("Delete").clicked() {
                remove_timers.push(timer.id);
            }
        });
    }
    if !remove_timers.is_empty() {
        app.project
            .timers
            .retain(|timer| !remove_timers.contains(&timer.id));
        changed = true;
    }
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut app.timer_name_draft)
                .desired_width(100.0)
                .hint_text("Timer name"),
        );
        ui.add(
            egui::TextEdit::singleline(&mut app.timer_duration_draft)
                .desired_width(62.0)
                .hint_text("MM:SS"),
        );
        if ui.button("Add timer").clicked() {
            match parse_duration(&app.timer_duration_draft) {
                Some(seconds) if !app.timer_name_draft.trim().is_empty() => {
                    app.project.timers.push(lumen_core::TimerDef {
                        id: Uuid::new_v4(),
                        name: app.timer_name_draft.trim().into(),
                        kind: TimerKind::Countdown {
                            duration_secs: seconds,
                        },
                        allow_overrun: false,
                    });
                    changed = true;
                    app.timer_error.clear();
                }
                _ => app.timer_error = "Enter a name and a duration from 0 to 24 hours.".into(),
            }
        }
    });
    if changed {
        app.mark();
    }
    if !actions.is_empty()
        && let Err(error) = app.runtime.apply_actions(&app.project, &actions)
    {
        app.error = error.to_string();
    }
    app.runtime.sync_timers(&app.project);
    if !app.timer_error.is_empty() {
        ui.colored_label(Color32::from_rgb(238, 184, 92), &app.timer_error);
    }
    ui.separator();
    crate::style::section(ui, "Stage message");
    let mut draft = app.stage_draft.clone();
    ui.add(egui::TextEdit::singleline(&mut draft).hint_text("Message for stage"));
    app.stage_draft = draft;
    if ui.button("Send to stage").clicked() {
        app.runtime.output.stage_message = app.stage_draft.clone();
    }
    if ui.button("Clear message").clicked() {
        app.runtime.output.stage_message.clear();
    }
    ui.separator();
    ui.label("Media status");
    if !app.media.status.is_empty() {
        ui.label(&app.media.status);
    }
}

fn parse_duration(value: &str) -> Option<f64> {
    let parts: Vec<&str> = value.trim().split(':').collect();
    let seconds = match parts.as_slice() {
        [seconds] => seconds.parse::<f64>().ok()?,
        [minutes, seconds] => {
            let minutes = minutes.parse::<u64>().ok()?;
            let seconds = seconds.parse::<f64>().ok()?;
            if !(0.0..60.0).contains(&seconds) {
                return None;
            }
            minutes as f64 * 60.0 + seconds
        }
        [hours, minutes, seconds] => {
            let hours = hours.parse::<u64>().ok()?;
            let minutes = minutes.parse::<u64>().ok()?;
            let seconds = seconds.parse::<f64>().ok()?;
            if minutes >= 60 || !(0.0..60.0).contains(&seconds) {
                return None;
            }
            hours as f64 * 3600.0 + minutes as f64 * 60.0 + seconds
        }
        _ => return None,
    };
    (seconds.is_finite() && (0.0..=86400.0).contains(&seconds)).then_some(seconds)
}

#[cfg(test)]
mod tests {
    use super::{append_playlist_item, parse_duration};
    use lumen_core::Project;
    use uuid::Uuid;

    #[test]
    fn duration_parser_rejects_invalid_and_unbounded_values() {
        assert_eq!(parse_duration("1:30"), Some(90.0));
        assert_eq!(parse_duration("1:02:03"), Some(3723.0));
        assert_eq!(parse_duration("NaN"), None);
        assert_eq!(parse_duration("1:75"), None);
        assert_eq!(parse_duration("1:-1"), None);
        assert_eq!(parse_duration("1:00:-1"), None);
        assert_eq!(parse_duration("24:00:01"), None);
        assert_eq!(parse_duration("-2"), None);
    }

    #[test]
    fn empty_library_cannot_add_dangling_playlist_entries() {
        let mut project = Project::demo();
        project.documents.clear();
        project.playlist.clear();
        assert!(!append_playlist_item(&mut project, Uuid::new_v4()));
        assert!(project.playlist.is_empty());
    }
}
