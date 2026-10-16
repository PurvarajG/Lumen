use crate::{app::LumenApp, render, style};
use egui::{self, RichText, Vec2};
use lumen_core::{CueAction, Document, Macro, PlaylistItem, Project, Slide};
use uuid::Uuid;

pub fn library(app: &mut LumenApp, ui: &mut egui::Ui) {
    if crate::books::search_field(app, ui) {
        return;
    }
    ui.horizontal(|ui| {
        ui.spacing_mut().button_padding.x = 5.0;
        ui.selectable_value(&mut app.library_tab, 0, "Running order");
        ui.selectable_value(&mut app.library_tab, 1, "Library");
        ui.selectable_value(&mut app.library_tab, 2, "Books");
    });
    if app.library_tab == 2 {
        crate::books::catalog_panel(app, ui);
        return;
    }
    let docs: Vec<_> = if app.library_tab == 1 {
        app.project.documents.iter().map(|d| d.id).collect()
    } else {
        app.project.playlist.iter().map(|p| p.document_id).collect()
    };
    let next = app.runtime.peek_next(&app.project);
    egui::ScrollArea::vertical()
        .id_salt("library-list")
        .max_height((ui.available_height() - 210.0).max(100.0))
        .show(ui, |ui| {
            let mut shown = 0;
            for (index, id) in docs.iter().enumerate() {
                let Some(doc) = app.project.document(*id).cloned() else {
                    continue;
                };
                shown += 1;
                ui.push_id(index, |ui| {
                    let on_deck = next.as_ref().is_some_and(|n| n.document_id == *id);
                    let live = app
                        .runtime
                        .output
                        .live
                        .as_ref()
                        .is_some_and(|n| n.document_id == *id);
                    let r = ui
                        .horizontal(|ui| {
                            let state_width = 52.0;
                            let width =
                                (ui.available_width() - state_width - ui.spacing().item_spacing.x)
                                    .max(40.0);
                            let row = ui
                                .allocate_ui_with_layout(
                                    egui::vec2(width, style::CONTROL),
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        crate::text::gu_selectable(
                                            ui,
                                            app.selected_doc == *id
                                                && !crate::books::has_preview(app),
                                            crate::books::document_label(&doc.name),
                                            style::BODY,
                                        )
                                    },
                                )
                                .inner;
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if live {
                                        ui.colored_label(style::LIVE, "LIVE");
                                    } else {
                                        ui.label(format!("{} cues", doc.slides.len()));
                                    }
                                },
                            );
                            row
                        })
                        .inner;
                    if on_deck {
                        ui.painter().rect_stroke(
                            r.rect.expand(2.0),
                            3.0,
                            egui::Stroke::new(4.0_f32, style::SAFFRON.gamma_multiply(0.14)),
                            egui::StrokeKind::Outside,
                        );
                        ui.painter().rect_filled(
                            egui::Rect::from_min_size(
                                r.rect.min,
                                Vec2::new(style::ON_DECK_RAIL, r.rect.height()),
                            ),
                            0.0,
                            style::SAFFRON,
                        );
                    }
                    if r.clicked() {
                        crate::books::close_preview(app);
                        app.selected_doc = *id;
                        app.selected_slide = doc.slides.first().map(|s| s.id);
                        if let Some(slide) = app.selected_slide {
                            crate::books::remember_cue(app, *id, slide);
                        }
                    }
                    if app.selected_doc == *id && (!app.present || app.library_tab == 1) {
                        ui.horizontal(|ui| {
                            if app.library_tab == 1 {
                                if ui.small_button("Add to order").clicked()
                                    && append_playlist_item(&mut app.project, *id)
                                {
                                    app.mark();
                                }
                                if !app.present && ui.small_button("Delete").clicked() {
                                    app.confirm_delete = Some(*id);
                                }
                            } else {
                                if !app.present && ui.small_button("Up").clicked() && index > 0 {
                                    app.project.playlist.swap(index, index - 1);
                                    app.mark();
                                }
                                if !app.present
                                    && ui.small_button("Down").clicked()
                                    && index + 1 < app.project.playlist.len()
                                {
                                    app.project.playlist.swap(index, index + 1);
                                    app.mark();
                                }
                                if !app.present && ui.small_button("Remove").clicked() {
                                    app.project.playlist.remove(index);
                                    app.mark();
                                }
                            }
                        });
                        if !app.present
                            && let Some(d) = app.project.documents.iter_mut().find(|d| d.id == *id)
                        {
                            ui.text_edit_singleline(&mut d.name);
                        }
                    }
                });
            }
            if shown == 0 {
                if docs.is_empty() {
                    ui.label(if app.library_tab == 1 {
                        "No documents yet — create or import a presentation."
                    } else {
                        "Running order is empty — add a document from Library."
                    });
                } else {
                    ui.label("No matching documents. Clear search to see the full list.");
                }
            }
        });
    if ui.button("New document").clicked() {
        let mut doc = Document::new("New document");
        if let Some(t) = app.project.themes.first() {
            doc.slides.push(Slide::new(t.id));
        }
        app.selected_doc = doc.id;
        app.selected_slide = doc.slides.first().map(|s| s.id);
        let id = doc.id;
        app.project.documents.push(doc);
        append_playlist_item(&mut app.project, id);
        app.mark();
    }
    ui.separator();
    style::section(ui, "Macros");
    egui::ScrollArea::vertical()
        .id_salt("macros")
        .show(ui, |ui| {
            for mac in app.project.macros.clone() {
                ui.push_id(mac.id, |ui| {
                    if ui
                        .add_sized(
                            [ui.available_width(), style::CONTROL],
                            egui::Button::new(&mac.name),
                        )
                        .clicked()
                    {
                        app.run_macro(mac.id);
                    }
                    if !app.present {
                        ui.collapsing("Edit actions", |ui| {
                            let mut edited = mac.clone();
                            ui.text_edit_singleline(&mut edited.name);
                            crate::inspector::actions(ui, &mut edited.actions, &app.project);
                            if let Some(target) =
                                app.project.macros.iter_mut().find(|m| m.id == mac.id)
                            {
                                *target = edited;
                            }
                        });
                    }
                });
            }
            if !app.present && ui.button("Add macro").clicked() {
                app.project.macros.push(Macro {
                    id: Uuid::new_v4(),
                    name: "New macro".into(),
                    actions: Vec::new(),
                });
            }
        });
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
    if crate::books::preview_panel(app, ui) {
        return;
    }
    let Some(index) = app
        .project
        .documents
        .iter()
        .position(|d| d.id == app.selected_doc)
    else {
        ui.heading("No document selected");
        ui.label("Create a document or import a presentation to begin.");
        return;
    };
    if app.project.documents[index].slides.is_empty() {
        ui.heading("Empty document");
        ui.label("This document has no slides yet — switch to Prepare and add a slide.");
        if !app.present
            && ui.button("Add slide").clicked()
            && let Some(theme) = app.project.themes.first()
        {
            let slide = Slide::new(theme.id);
            app.selected_slide = Some(slide.id);
            app.project.documents[index].slides.push(slide);
            app.mark();
        }
        return;
    }
    ui.heading("Preview");
    let preview = app.selected_live();
    let height = preview_height(ui.available_width(), ui.available_height());
    let texture = preview
        .as_ref()
        .and_then(|p| app.thumbnail(&p.slide.background));
    render::preview_frame_with_images(
        ui,
        preview.as_ref(),
        false,
        false,
        texture.as_ref(),
        &app.thumbnails.images(),
        height,
    );
    if let Some(p) = &preview {
        crate::text::gu_label(
            ui,
            &format!(
                "{} / {}",
                crate::books::document_label(&p.document_name),
                p.slide.label
            ),
            style::BODY,
        );
    }
    ui.small("Edits stay in preview until triggered.");
    ui.horizontal_wrapped(|ui| {
        crate::text::gu_label(
            ui,
            crate::books::document_label(&app.project.documents[index].name),
            style::HEADING,
        );
        if !app.present {
            if ui.button("Add slide").clicked()
                && let Some(t) = app.project.themes.first()
            {
                let s = Slide::new(t.id);
                app.selected_slide = Some(s.id);
                app.project.documents[index].slides.push(s);
            }
            if ui.button("Duplicate").clicked()
                && let Some(mut s) = preview.as_ref().map(|p| p.slide.clone())
            {
                s.id = Uuid::new_v4();
                for o in &mut s.objects {
                    match o {
                        lumen_core::SlideObject::Text { id, .. }
                        | lumen_core::SlideObject::Shape { id, .. }
                        | lumen_core::SlideObject::Image { id, .. } => *id = Uuid::new_v4(),
                    }
                }
                app.selected_slide = Some(s.id);
                app.project.documents[index].slides.push(s);
            }
            if ui.button("Delete slide").clicked() {
                app.project.documents[index]
                    .slides
                    .retain(|s| Some(s.id) != app.selected_slide);
                app.selected_slide = app.project.documents[index].slides.first().map(|s| s.id);
            }
            for (name, step) in [("Move up", -1_isize), ("Move down", 1)] {
                if ui.button(name).clicked()
                    && let Some(i) = app.project.documents[index]
                        .slides
                        .iter()
                        .position(|s| Some(s.id) == app.selected_slide)
                {
                    let target = i as isize + step;
                    if (0..app.project.documents[index].slides.len() as isize).contains(&target) {
                        app.project.documents[index].slides.swap(i, target as usize);
                    }
                }
            }
        }
    });
    ui.separator();
    let (columns, width) = grid_layout(ui.available_width(), ui.spacing().item_spacing.x);
    let slides = app.project.documents[index].slides.clone();
    let next = app.runtime.peek_next(&app.project);
    egui::ScrollArea::vertical()
        .id_salt("cue-grid")
        .show(ui, |ui| {
            for chunk in slides.chunks(columns) {
                ui.horizontal_top(|ui| {
                    for slide in chunk {
                        ui.vertical(|ui| {
                            ui.set_width(width);
                            let (rect, r) = ui.allocate_exact_size(
                                Vec2::new(width, width * 9.0 / 16.0),
                                egui::Sense::click(),
                            );
                            let cue =
                                app.project
                                    .theme(slide.theme_id)
                                    .map(|t| lumen_core::LiveSlide {
                                        document_id: app.selected_doc,
                                        document_name: app.project.documents[index].name.clone(),
                                        slide: slide.clone(),
                                        theme: t.clone(),
                                    });
                            let texture = app.thumbnail(&slide.background);
                            render::paint_preview(
                                ui,
                                rect,
                                cue.as_ref(),
                                texture.as_ref(),
                                &app.thumbnails.images(),
                            );
                            if let lumen_core::Background::Image { path }
                            | lumen_core::Background::Video { path, .. } = &slide.background
                                && texture.is_none()
                            {
                                let status =
                                    app.thumbnails.status(path).unwrap_or("Loading media…");
                                ui.painter().text(
                                    rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "Media unavailable / loading",
                                    egui::FontId::proportional(style::CAPTION),
                                    style::TEXT_DIM,
                                );
                                r.clone().on_hover_text(status);
                            }
                            let live = app
                                .runtime
                                .output
                                .live
                                .as_ref()
                                .is_some_and(|l| l.slide.id == slide.id);
                            let on_deck = next.as_ref().is_some_and(|n| n.slide.id == slide.id);
                            let selected = app.selected_slide == Some(slide.id);
                            let stroke = egui::Stroke::new(
                                if live { 2.0_f32 } else { 1.0_f32 },
                                if live {
                                    style::LIVE
                                } else if selected {
                                    style::SAFFRON
                                } else {
                                    style::LINE
                                },
                            );
                            ui.painter()
                                .rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Inside);
                            if on_deck {
                                ui.painter().rect_stroke(
                                    rect.expand(2.0),
                                    2.0,
                                    egui::Stroke::new(4.0_f32, style::SAFFRON.gamma_multiply(0.14)),
                                    egui::StrokeKind::Outside,
                                );
                                ui.painter().rect_filled(
                                    egui::Rect::from_min_size(
                                        rect.min,
                                        Vec2::new(style::ON_DECK_RAIL, rect.height()),
                                    ),
                                    0.0,
                                    style::SAFFRON,
                                );
                            }
                            if r.clicked() {
                                app.selected_slide = Some(slide.id);
                                crate::books::remember_cue(app, app.selected_doc, slide.id);
                            }
                            if r.double_clicked() {
                                app.trigger(app.selected_doc, slide.id);
                            }
                            ui.horizontal(|ui| {
                                ui.add(egui::Label::new(&slide.label).truncate());
                                if live {
                                    ui.label(
                                        RichText::new("LIVE")
                                            .color(style::WHITE)
                                            .background_color(style::LIVE),
                                    );
                                } else if on_deck {
                                    ui.colored_label(style::SAFFRON, "On deck");
                                }
                            });
                        });
                    }
                });
                ui.add_space(style::GAP);
            }
        });
}

pub fn inspector(app: &mut LumenApp, ui: &mut egui::Ui) {
    ui.colored_label(style::LIVE, RichText::new("LIVE").strong());
    let width = ui.available_width();
    let height = width * 9.0 / 16.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::hover());
    if app.runtime.output.deck.is_some() {
        ui.put(
            rect,
            egui::Label::new(format!(
                "Playing in {} — use Next/Previous",
                app.deck.app_name()
            )),
        );
        if ui.button("Stop deck").clicked() {
            app.apply_actions(&[CueAction::StopDeck]);
        }
    } else {
        app.paint_live(ui, rect);
    }
    let live_in_document = app
        .runtime
        .output
        .live
        .as_ref()
        .is_some_and(|live| live.document_id == app.selected_doc)
        || app.runtime.output.deck.is_some();
    let book_preview = crate::books::has_preview(app);
    ui.horizontal(|ui| {
        if ui
            .add_sized(
                [ui.available_width() * 0.43, style::PRIMARY],
                egui::Button::new("Previous"),
            )
            .clicked()
        {
            app.navigate(false);
        }
        if ui
            .add_sized(
                [ui.available_width(), style::PRIMARY],
                style::primary(if book_preview {
                    "Add and go live"
                } else {
                    transport_primary_label(live_in_document)
                }),
            )
            .clicked()
        {
            if book_preview {
                crate::books::add_and_go_live(app);
            } else if live_in_document {
                app.navigate(true);
            } else if let Some(l) = app.selected_live() {
                app.trigger(l.document_id, l.slide.id);
            }
        }
    });
    ui.horizontal(|ui| {
        ui.menu_button("Clear", |ui| {
            for (n, name) in [
                (1, "All"),
                (2, "Slide"),
                (3, "Media"),
                (4, "Props"),
                (5, "Audio"),
                (6, "Messages"),
            ] {
                if ui.button(format!("{name}    F{n}")).clicked() {
                    app.clear_layer(n);
                    ui.close();
                }
            }
        });
        let blackout = app.runtime.output.blackout;
        let blackout_button = egui::Button::new(if blackout {
            "Blackout active · B"
        } else {
            "Blackout · B"
        })
        .fill(if blackout { style::LIVE } else { style::RAISED })
        .stroke(egui::Stroke::new(
            1.0_f32,
            if blackout { style::LIVE } else { style::LINE },
        ));
        if ui.add(blackout_button).clicked() {
            app.runtime.output.blackout = !app.runtime.output.blackout;
        }
        if app.runtime.output.blackout {
            ui.colored_label(style::LIVE, "Output is blacked out");
        }
    });
    ui.separator();
    if !app.present {
        if let Some(mut selected) = app.selected_live() {
            egui::ScrollArea::vertical()
                .id_salt("slide-inspector")
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("Choose image…").clicked() {
                            app.choose_media(selected.slide.id, false);
                        }
                        if ui.button("Choose video…").clicked() {
                            app.choose_media(selected.slide.id, true);
                        }
                    });
                    crate::inspector::slide(ui, &mut selected.slide, &app.project);
                });
            if let Some(d) = app
                .project
                .documents
                .iter_mut()
                .find(|d| d.id == selected.document_id)
                && let Some(s) = d.slides.iter_mut().find(|s| s.id == selected.slide.id)
            {
                *s = selected.slide;
            }
        }
        return;
    }
    tool_tabs(ui, &mut app.tool_tab);
    egui::ScrollArea::vertical()
        .id_salt("tools")
        .show(ui, |ui| {
            let now = app.seconds_of_day();
            match app.tool_tab {
                0 => {
                    crate::tools::timers(ui, &mut app.project, &mut app.runtime, now);
                }
                1 => {
                    crate::tools::messages(ui, &mut app.project, &mut app.runtime, now);
                }
                2 => {
                    crate::tools::props(ui, &mut app.project, &mut app.runtime);
                }
                3 => {
                    app.audio.ui(ui);
                }
                _ => {
                    if crate::settings::stage_options(ui, &mut app.settings.stage, &app.project) {
                        app.persist_settings();
                    }
                    ui.label("Stage message");
                    ui.text_edit_singleline(&mut app.stage_draft);
                    if ui.button("Show on stage").clicked() {
                        app.apply_actions(&[CueAction::StageMessage(app.stage_draft.clone())]);
                    }
                    if ui.button("Clear stage message").clicked() {
                        app.apply_actions(&[CueAction::StageMessage(String::new())]);
                    }
                }
            }
        });
}

fn tool_tabs(ui: &mut egui::Ui, tab: &mut usize) -> egui::InnerResponse<()> {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 3.0;
        ui.spacing_mut().button_padding = egui::vec2(5.0, 4.0);
        for (i, name) in ["Timers", "Messages", "Props", "Audio", "Stage"]
            .iter()
            .enumerate()
        {
            ui.selectable_value(tab, i, *name);
        }
    })
}

fn transport_primary_label(live_in_document: bool) -> &'static str {
    if live_in_document { "Next" } else { "Go live" }
}

fn grid_layout(available: f32, gap: f32) -> (usize, f32) {
    let columns = ((available + gap) / (180.0 + gap)).floor().max(1.0) as usize;
    (
        columns,
        (available - gap * (columns - 1) as f32).max(1.0) / columns as f32,
    )
}

// Reserve one thumbnail row, its label and document controls below the preview.
pub fn preview_height(width: f32, height: f32) -> f32 {
    let (_, thumb_width) = grid_layout(width, style::GAP);
    (width * 9.0 / 16.0).min((height - thumb_width * 9.0 / 16.0 - 122.0).max(80.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transport_keeps_one_contextual_primary_label() {
        assert_eq!(transport_primary_label(false), "Go live");
        assert_eq!(transport_primary_label(true), "Next");
    }
    #[test]
    fn all_five_tool_tabs_fit_one_row_at_minimum_column_width() {
        let ctx = egui::Context::default();
        style::install(&ctx);
        for width in [306.0, 336.0] {
            let mut rect = egui::Rect::NOTHING;
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, 200.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default()
                        .frame(egui::Frame::NONE)
                        .show(ctx, |ui| {
                            rect = tool_tabs(ui, &mut 0).response.rect;
                        });
                },
            );
            assert!(rect.width() <= width, "tabs overflow {width}: {rect:?}");
            assert!(rect.height() <= style::CONTROL + 1.0, "tabs wrap: {rect:?}");
        }
    }
    #[test]
    fn preview_fills_small_console_and_reserves_one_grid_row() {
        let width = 1100.0 - style::LEFT_WIDTH - style::RIGHT_WIDTH - 48.0;
        let height = 700.0 - 130.0;
        assert!((preview_height(width, height) * 16.0 / 9.0 - width).abs() < 0.1);
        for width in [width, 700.0, 1100.0] {
            let (_, thumb) = grid_layout(width, style::GAP);
            assert!(preview_height(width, height) + thumb * 9.0 / 16.0 + 122.0 <= height + 0.1);
        }
    }
    #[test]
    fn grid_fits_width_without_overflow() {
        for available in [100.0, 280.0, 600.0, 1200.0] {
            let (n, w) = grid_layout(available, 8.0);
            assert!(n >= 1);
            assert!((w * n as f32 + (n - 1) as f32 * 8.0 - available).abs() < 0.01);
        }
    }
    #[test]
    fn missing_document_cannot_enter_playlist() {
        let mut p = Project::demo();
        assert!(!append_playlist_item(&mut p, Uuid::nil()));
    }
}
