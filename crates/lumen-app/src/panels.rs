use crate::thumbnails::PreviewState;
use crate::{app::LumenApp, render, style};
use egui::{self, RichText, Vec2};
use lumen_core::{
    Background, CueAction, Document, Macro, MediaFit, PlaylistItem, Project, Slide, Transition,
};
use uuid::Uuid;

/// The running-order pane: search, the three tabs, the document rows.
pub fn running_order(app: &mut LumenApp, ui: &mut egui::Ui) {
    // The rail is a dense list; 4 pt between its controls gives the running order the room.
    ui.spacing_mut().item_spacing.y = style::GAP / 2.0;
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
    let order = app.library_tab == 0;
    if order {
        add_header(app, ui, !docs.is_empty());
    }
    // The list fills the pane; Prepare keeps one control row below it for "New document".
    let reserve = if app.present {
        0.0
    } else {
        style::CONTROL + style::GAP
    };
    if order && docs.is_empty() {
        empty_running_order(app, ui);
    } else {
        document_rows(app, ui, &docs, order, reserve);
    }
    // Present is for running the show; creating documents is editing.
    if !app.present && ui.button("New document").clicked() {
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
}

/// The macros pane: one button per macro, with its action editor in Prepare.
pub fn macros(app: &mut LumenApp, ui: &mut egui::Ui) {
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
const ROW_HEIGHT: f32 = 56.0;
const ROW_GAP: f32 = 1.0;

fn add_header(app: &mut LumenApp, ui: &mut egui::Ui, has_items: bool) {
    if has_items
        && ui
            .add_sized(
                [ui.available_width(), style::CONTROL],
                egui::Button::new("Add files…"),
            )
            .clicked()
    {
        app.start_add(None);
    }
    if let Some(detail) = &app.add_detail {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(detail);
        });
    } else if let Some((done, total)) = app.add_progress {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(format!("Adding {done} of {total}…"));
        });
    } else if let Some(notice) = app.add_notice.clone() {
        ui.horizontal_wrapped(|ui| {
            let color = if notice.contains("skipped") || notice.contains("isn't installed") {
                style::WARN
            } else {
                style::TEXT_DIM
            };
            ui.colored_label(color, &notice);
            if ui.small_button("Dismiss").clicked() {
                app.add_notice = None;
            }
        });
    }
}

fn empty_running_order(app: &mut LumenApp, ui: &mut egui::Ui) {
    let height = (ui.available_height() - 210.0).clamp(180.0, 320.0);
    let hovering = ui.ctx().input(|i| !i.raw.hovered_files.is_empty());
    let stroke = if hovering {
        egui::Stroke::new(2.0_f32, style::SAFFRON)
    } else {
        egui::Stroke::new(1.0_f32, style::LINE)
    };
    egui::Frame::new()
        .stroke(stroke)
        .corner_radius(style::RADIUS_SURFACE)
        .inner_margin(16.0)
        .show(ui, |ui| {
            ui.set_min_size(Vec2::new(ui.available_width(), height - 32.0));
            ui.vertical_centered(|ui| {
                ui.add_space(height * 0.15);
                ui.label(style::icon('\u{e4c0}', 32.0).color(style::TEXT_DIM));
                ui.label("Drop videos, pictures, PowerPoints or PDFs here");
                if ui.button("Add files…").clicked() {
                    app.start_add(None);
                }
            });
        });
}

fn document_rows(
    app: &mut LumenApp,
    ui: &mut egui::Ui,
    docs: &[Uuid],
    reorder: bool,
    reserve: f32,
) {
    let next = app.runtime.peek_next(&app.project);
    let next_row = app.runtime.next_item_index(&app.project);
    let live_row = app.runtime.live_item_index(&app.project);
    // The armed row is the one after live, or live itself at the end of the order.
    let armed_row = next_row.or(live_row);
    let armed_is_selected = armed_row
        .and_then(|row| app.project.playlist.get(row))
        .is_some_and(|item| item.document_id == app.selected_doc)
        && app.selected_slide
            == next
                .as_ref()
                .or(app.runtime.output.live.as_ref())
                .map(|l| l.slide.id);
    let scroll_to_armed = app.scroll_to_armed;
    let images = app.thumbnails.images();
    egui::ScrollArea::vertical()
        .id_salt("library-list")
        // Fill the rail down to `reserve`, so Macros sits at the bottom with no dead gap.
        .auto_shrink([false, false])
        .max_height((ui.available_height() - reserve).max(100.0))
        .show(ui, |ui| {
            // Rows are 56 pt tall; a 1 pt gap rather than 8 fits one more on screen.
            ui.spacing_mut().item_spacing.y = ROW_GAP;
            let mut rects = Vec::new();
            let mut dragged: Option<(usize, bool)> = None;
            for (index, id) in docs.iter().enumerate() {
                let Some(doc) = app.project.document(*id) else {
                    continue;
                };
                let (name, count, kind) = (
                    doc.name.clone(),
                    doc.slides.len(),
                    crate::add::doc_kind(doc),
                );
                ui.push_id(index, |ui| {
                    // The running order marks one row; the library lists each document once.
                    let on_deck = if reorder {
                        next_row == Some(index)
                    } else {
                        next.as_ref().is_some_and(|n| n.document_id == *id)
                    };
                    let live = if reorder {
                        live_row == Some(index)
                    } else {
                        app.runtime
                            .output
                            .live
                            .as_ref()
                            .is_some_and(|n| n.document_id == *id)
                    };
                    // A repeated document marks only the armed copy when it is the selected one.
                    let selected = app.selected_doc == *id
                        && !crate::books::has_preview(app)
                        && !(reorder && armed_is_selected && armed_row != Some(index));
                    let (rect, r) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), ROW_HEIGHT),
                        if reorder {
                            egui::Sense::click_and_drag()
                        } else {
                            egui::Sense::click()
                        },
                    );
                    rects.push(rect);
                    if scroll_to_armed && selected && (!reorder || armed_row == Some(index)) {
                        r.scroll_to_me(Some(egui::Align::Center));
                    }
                    // Hover stays weaker than selection so the next cue is never ambiguous.
                    if selected {
                        ui.painter()
                            .rect_filled(rect, style::RADIUS_SURFACE, style::RAISED);
                    } else if r.hovered() || r.dragged() {
                        ui.painter().rect_filled(
                            rect,
                            style::RADIUS_SURFACE,
                            style::RAISED.gamma_multiply(0.5),
                        );
                    }
                    if r.has_focus() {
                        ui.painter().rect_stroke(
                            rect,
                            style::RADIUS_SURFACE,
                            egui::Stroke::new(2.0_f32, style::SAFFRON),
                            egui::StrokeKind::Inside,
                        );
                    }
                    let mut x = rect.left() + 6.0;
                    if reorder {
                        ui.painter().text(
                            egui::pos2(x + 6.0, rect.center().y),
                            egui::Align2::CENTER_CENTER,
                            '\u{eae2}',
                            egui::FontId::new(16.0, egui::FontFamily::Name("Phosphor".into())),
                            style::TEXT_DIM,
                        );
                        x += 16.0;
                        if r.dragged() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                        } else if r.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                        }
                    }
                    let thumb = egui::Rect::from_min_size(
                        egui::pos2(x, rect.top() + 6.0),
                        Vec2::new(44.0 * 16.0 / 9.0, 44.0),
                    );
                    if ui.is_rect_visible(rect) {
                        let live_slide = app
                            .project
                            .document(*id)
                            .and_then(|d| d.slides.first())
                            .and_then(|slide| {
                                app.project.theme(slide.theme_id).map(|theme| {
                                    lumen_core::LiveSlide {
                                        document_id: *id,
                                        document_name: name.clone(),
                                        slide: slide.clone(),
                                        theme: theme.clone(),
                                    }
                                })
                            });
                        let texture = live_slide
                            .as_ref()
                            .and_then(|l| app.thumbnail(&l.slide.background));
                        render::paint_preview(
                            ui,
                            thumb,
                            live_slide.as_ref(),
                            texture.as_ref(),
                            &images,
                        );
                        if let Some(live) = &live_slide {
                            thumbnail_placeholder(app, ui, thumb, &live.slide.background);
                        }
                        ui.painter().rect_stroke(
                            thumb,
                            0.0,
                            egui::Stroke::new(1.0_f32, style::LINE),
                            egui::StrokeKind::Inside,
                        );
                    }
                    let text_x = thumb.right() + 8.0;
                    let label = crate::books::document_label(&name);
                    ui.scope_builder(
                        egui::UiBuilder::new().max_rect(egui::Rect::from_min_max(
                            egui::pos2(text_x, rect.top() + 2.0),
                            egui::pos2(rect.right() - 6.0, rect.top() + 30.0),
                        )),
                        |ui| {
                            if crate::text::contains_gujarati(label) {
                                crate::text::gu_label(ui, label, style::BODY);
                            } else {
                                ui.add(
                                    egui::Label::new(RichText::new(label).size(style::BODY))
                                        .truncate(),
                                );
                            }
                        },
                    );
                    let line = rect.top() + 43.0;
                    ui.painter().text(
                        egui::pos2(text_x + 8.0, line),
                        egui::Align2::CENTER_CENTER,
                        kind.icon(),
                        egui::FontId::new(14.0, egui::FontFamily::Name("Phosphor".into())),
                        style::TEXT_DIM,
                    );
                    if live {
                        let pill = egui::Rect::from_min_size(
                            egui::pos2(text_x + 22.0, line - 8.0),
                            Vec2::new(38.0, 16.0),
                        );
                        ui.painter()
                            .rect_filled(pill, style::RADIUS_SURFACE, style::LIVE);
                        ui.painter().text(
                            pill.center(),
                            egui::Align2::CENTER_CENTER,
                            "LIVE",
                            egui::FontId::proportional(style::CAPTION),
                            style::WHITE,
                        );
                    } else {
                        ui.painter().text(
                            egui::pos2(text_x + 22.0, line),
                            egui::Align2::LEFT_CENTER,
                            format!("{count} slide{}", if count == 1 { "" } else { "s" }),
                            egui::FontId::proportional(style::CAPTION),
                            style::TEXT_DIM,
                        );
                    }
                    if on_deck {
                        ui.painter().rect_stroke(
                            rect.expand(2.0),
                            3.0,
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
                    if reorder && (r.dragged() || r.drag_stopped()) {
                        dragged = Some((index, r.drag_stopped()));
                    }
                    if r.clicked() {
                        crate::books::close_preview(app);
                        app.select_document(*id);
                        if let Some(slide) = app.selected_slide {
                            crate::books::remember_cue(app, *id, slide);
                        }
                    }
                    r.clone().on_hover_text(label);
                    cue_menu(app, r, *id, None);
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
                            } else if !app.present && ui.small_button("Remove from order").clicked()
                            {
                                app.remove_item(index);
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
            if let Some((from, released)) = dragged
                && rects.len() == docs.len()
                && let Some(pointer) = ui.ctx().pointer_latest_pos()
            {
                let slot = crate::add::list_slot(&rects, pointer);
                let y = match slot {
                    0 => rects[0].top() - ROW_GAP / 2.0,
                    s if s == rects.len() => rects[s - 1].bottom() + ROW_GAP / 2.0,
                    s => (rects[s - 1].bottom() + rects[s].top()) / 2.0,
                };
                ui.painter().line_segment(
                    [
                        egui::pos2(rects[0].left(), y),
                        egui::pos2(rects[0].right(), y),
                    ],
                    egui::Stroke::new(2.0_f32, style::SAFFRON),
                );
                if released {
                    app.move_running_order(from, crate::add::slot_to_index(from, slot));
                }
            }
            if docs.is_empty() {
                ui.label("No documents yet — create or import a presentation.");
            }
        });
}
/// Covers a media cue's preview with a loading or error placeholder while it has no thumbnail,
/// and returns the tooltip for a failed one.
fn thumbnail_placeholder(
    app: &LumenApp,
    ui: &egui::Ui,
    rect: egui::Rect,
    background: &Background,
) -> Option<String> {
    let (Background::Image { path, .. } | Background::Video { path, .. }) = background else {
        return None;
    };
    let state = app.thumbnails.state(path)?;
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    render::media_placeholder(ui, rect, &name, &state);
    match state {
        PreviewState::Loading => None,
        PreviewState::Failed(reason) => Some(reason),
        PreviewState::Missing => Some(format!("File not found: {}", path.display())),
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

/// The big Preview of the selected cue, its label and notes.
pub fn preview(app: &mut LumenApp, ui: &mut egui::Ui) {
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
    let preview = app.selected_live();
    let next = app.runtime.peek_next(&app.project);
    // "Up next" tells the operator that what they see is what the primary button sends.
    let up_next = app.runtime.output.live.is_some()
        && preview
            .as_ref()
            .zip(next.as_ref())
            .is_some_and(|(p, n)| (p.document_id, p.slide.id) == (n.document_id, n.slide.id));
    if up_next {
        ui.colored_label(
            style::SAFFRON,
            RichText::new("Up next").size(style::CAPTION),
        );
    }
    let height = preview_pane_height(ui.available_width(), ui.available_height());
    // One snapshot per call; cloning the map per tile was O(tiles x thumbnails).
    let images = app.thumbnails.images();
    let small = preview
        .as_ref()
        .and_then(|p| app.thumbnail(&p.slide.background));
    // The grid thumbnail shows at once; the sharper one replaces it when it arrives.
    let still = preview
        .as_ref()
        .and_then(|p| app.thumbnail_large(&p.slide.background))
        .or(small);
    // Only the live background has live frames; any other cue shows its still.
    let on_air = preview.as_ref().is_some_and(|p| {
        !app.runtime.output.media_cleared
            && !app.media.pending
            && app.media.matches_background(Some(&p.slide.background))
    });
    let texture = preview_texture(on_air, app.media.image_texture.clone(), still);
    let frame = render::preview_frame_with_images(
        ui,
        preview.as_ref(),
        false,
        false,
        texture.as_ref(),
        &images,
        height,
    );
    if texture.is_none()
        && let Some(p) = &preview
        && let Some(reason) = thumbnail_placeholder(app, ui, frame, &p.slide.background)
    {
        ui.interact(
            frame,
            ui.id().with("preview-placeholder"),
            egui::Sense::hover(),
        )
        .on_hover_text(reason);
    }
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
    if app.present
        && let Some(notes) = app
            .runtime
            .output
            .live
            .as_ref()
            .map(|live| live.slide.notes.as_str())
            .filter(|notes| !notes.trim().is_empty())
    {
        crate::text::gu_label(ui, notes, style::CAPTION);
    }
    ui.small("Edits stay in preview until triggered.");
}

/// The document heading, its edit buttons (Prepare) and the grid of cue thumbnails.
pub fn cues(app: &mut LumenApp, ui: &mut egui::Ui) {
    if crate::books::has_preview(app) {
        ui.label(
            RichText::new(
                "A book passage is being previewed. Add it to the running order to see its cues.",
            )
            .color(style::TEXT_DIM),
        );
        return;
    }
    let Some(index) = app
        .project
        .documents
        .iter()
        .position(|d| d.id == app.selected_doc)
    else {
        return;
    };
    if app.project.documents[index].slides.is_empty() {
        return;
    }
    let preview = app.selected_live();
    let next = app.runtime.peek_next(&app.project);
    let images = app.thumbnails.images();
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
    let scroll_to_armed = app.scroll_to_armed;
    egui::ScrollArea::vertical()
        .id_salt("cue-grid")
        .show(ui, |ui| {
            let mut tiles = Vec::new();
            let mut dragged: Option<(usize, bool)> = None;
            for (row, chunk) in slides.chunks(columns).enumerate() {
                ui.horizontal_top(|ui| {
                    for (column, slide) in chunk.iter().enumerate() {
                        ui.vertical(|ui| {
                            ui.set_width(width);
                            let (rect, r) = ui.allocate_exact_size(
                                Vec2::new(width, width * 9.0 / 16.0),
                                if app.present {
                                    egui::Sense::click()
                                } else {
                                    egui::Sense::click_and_drag()
                                },
                            );
                            tiles.push(rect);
                            if scroll_to_armed && app.selected_slide == Some(slide.id) {
                                r.scroll_to_me(Some(egui::Align::Center));
                            }
                            if !app.present && (r.dragged() || r.drag_stopped()) {
                                dragged = Some((row * columns + column, r.drag_stopped()));
                            }
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
                                &images,
                            );
                            if texture.is_none()
                                && let Some(reason) =
                                    thumbnail_placeholder(app, ui, rect, &slide.background)
                            {
                                r.clone().on_hover_text(reason);
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
                            cue_menu(app, r.clone(), app.selected_doc, Some(slide.id));
                            ui.horizontal(|ui| {
                                ui.add(egui::Label::new(&slide.label).truncate());
                                if live {
                                    ui.label(
                                        RichText::new("LIVE")
                                            .color(style::WHITE)
                                            .background_color(style::LIVE),
                                    );
                                } else if on_deck {
                                    ui.colored_label(style::SAFFRON, "Up next");
                                }
                            });
                        });
                    }
                });
                ui.add_space(style::GAP);
            }
            if let Some((from, released)) = dragged
                && let Some(pointer) = ui.ctx().pointer_latest_pos()
                && let Some((nearest, after)) = crate::add::grid_target(&tiles, pointer)
            {
                let tile = tiles[nearest];
                let x =
                    if after { tile.right() } else { tile.left() } + if after { 4.0 } else { -4.0 };
                ui.painter().line_segment(
                    [egui::pos2(x, tile.top()), egui::pos2(x, tile.bottom())],
                    egui::Stroke::new(3.0_f32, style::SAFFRON),
                );
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                if released {
                    let to = crate::add::slot_to_index(from, nearest + usize::from(after));
                    let moved = app.project.documents[index].slides.get(from).map(|s| s.id);
                    if crate::add::move_item(&mut app.project.documents[index].slides, from, to) {
                        app.selected_slide = moved;
                        app.mark();
                    }
                }
            }
        });
}

/// The live frame when the cue's background is on air, otherwise the still thumbnail.
fn preview_texture<T>(on_air: bool, live: Option<T>, still: Option<T>) -> Option<T> {
    if on_air { live.or(still) } else { still }
}

/// Right-click menu of a cue tile (`slide` set) or a running-order row (`slide` None): Play in
/// PowerPoint/Keynote for a deck, Scale for photos and videos.
fn cue_menu(app: &mut LumenApp, response: egui::Response, document_id: Uuid, slide: Option<Uuid>) {
    let source = app
        .project
        .document(document_id)
        .and_then(|document| document.source.clone());
    // A row speaks for every photo or video in its document; its check shows the first one's.
    let fits: Vec<MediaFit> = app
        .project
        .document(document_id)
        .into_iter()
        .flat_map(|document| &document.slides)
        .filter(|s| slide.is_none_or(|id| s.id == id))
        .filter_map(|s| s.background.fit())
        .collect();
    if source.is_none() && fits.is_empty() {
        return;
    }
    let live =
        app.runtime.output.live.as_ref().is_some_and(|l| {
            l.document_id == document_id && slide.is_none_or(|id| l.slide.id == id)
        });
    response.context_menu(|ui| {
        if let Some(source) = &source {
            let powerpoint = !matches!(
                source
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(str::to_ascii_lowercase)
                    .as_deref(),
                Some("key")
            );
            if powerpoint {
                if ui.button("Play inside Lumen").clicked() {
                    app.apply_actions(&[CueAction::PlayDeckInLumen {
                        path: source.clone(),
                    }]);
                    ui.close();
                }
                if ui.button("Play in PowerPoint (fullscreen)").clicked() {
                    app.apply_actions(&[CueAction::PlayDeck {
                        path: source.clone(),
                    }]);
                    ui.close();
                }
                ui.label("Keeps animations");
            } else {
                if ui.button("Play in Keynote instead").clicked() {
                    app.apply_actions(&[CueAction::PlayDeck {
                        path: source.clone(),
                    }]);
                    ui.close();
                }
                ui.label("Keeps animations; overlays hidden");
            }
        }
        if let Some(current) = fits.first() {
            // Not a submenu: the arrow glyph is missing from the bundled fonts.
            ui.label(
                RichText::new("Scale")
                    .size(style::CAPTION)
                    .color(style::TEXT_DIM),
            );
            for choice in crate::inspector::FIT_CHOICES {
                let label = crate::inspector::fit_label(choice);
                if ui.selectable_label(*current == choice, label).clicked() {
                    set_fit(app, document_id, slide, choice);
                    ui.close();
                }
            }
            if live {
                ui.label(
                    RichText::new("Applies the next time it goes live.")
                        .size(style::CAPTION)
                        .color(style::TEXT_DIM),
                );
            }
        }
    });
}

/// Applies a scale to one slide or to every photo and video of a document; one undo step.
fn set_fit(app: &mut LumenApp, document_id: Uuid, slide: Option<Uuid>, fit: MediaFit) {
    let Some(document) = app
        .project
        .documents
        .iter_mut()
        .find(|d| d.id == document_id)
    else {
        return;
    };
    let mut changed = false;
    for s in document
        .slides
        .iter_mut()
        .filter(|s| slide.is_none_or(|id| s.id == id))
    {
        changed |= s.background.set_fit(fit);
    }
    if changed {
        app.mark();
    }
}

/// The LIVE monitor, the transport row, the transition picker and the now-playing strip.
pub fn live(app: &mut LumenApp, ui: &mut egui::Ui) {
    let pane_height = ui.available_height();
    egui::ScrollArea::vertical()
        .id_salt("live")
        .auto_shrink([false, false])
        .show(ui, |ui| live_body(app, ui, pane_height));
}

fn live_body(app: &mut LumenApp, ui: &mut egui::Ui, pane_height: f32) {
    egui::Frame::new()
        .fill(style::LIVE)
        .corner_radius(style::RADIUS_SURFACE)
        .inner_margin(egui::Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.label(RichText::new("LIVE").strong().color(style::WHITE));
        });
    let width = ui.available_width();
    // A wide pane must not push the transport row out of sight.
    let height = (width * 9.0 / 16.0).min(pane_height * 0.55);
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
    let book_preview = crate::books::has_preview(app);
    let ids = |l: &lumen_core::LiveSlide| (l.document_id, l.slide.id);
    let armed = app.selected_live();
    let next = app.runtime.peek_next(&app.project);
    let label = if book_preview {
        "Add and go live".to_owned()
    } else {
        primary_label(
            armed.as_ref().map(|a| (ids(a), a.slide.label.as_str())),
            next.as_ref().map(ids),
            app.runtime.output.live.is_some(),
            app.runtime.output.deck.is_some(),
        )
    };
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
                style::primary(&label).truncate(),
            )
            .on_hover_text(&label)
            .clicked()
        {
            if book_preview {
                crate::books::add_and_go_live(app);
            } else {
                app.send_armed();
            }
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.menu_button("Clear", |ui| {
            for (n, name) in [
                (1, "All"),
                (2, "Slide"),
                (3, "Media"),
                (4, "Overlays"),
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
            app.apply_actions(&[CueAction::Blackout(!blackout)]);
        }
        if transition_picker(ui, &mut app.project.transition) {
            app.mark();
        }
        if app.runtime.output.blackout {
            ui.colored_label(style::LIVE, "Output is blacked out");
        }
    });
    crate::transport::now_playing(app, ui);
}

/// Present: the tool tabs. Prepare: the slide inspector and Overlays.
pub fn tools(app: &mut LumenApp, ui: &mut egui::Ui) {
    if !app.present {
        let mut selected = app.selected_live();
        let mut changed = false;
        egui::ScrollArea::vertical()
            .id_salt("slide-inspector")
            .show(ui, |ui| {
                if let Some(selected) = selected.as_mut() {
                    ui.horizontal(|ui| {
                        if ui.button("Choose image…").clicked() {
                            app.choose_media(selected.slide.id, false);
                        }
                        if ui.button("Choose video…").clicked() {
                            app.choose_media(selected.slide.id, true);
                        }
                    });
                    changed |= crate::inspector::slide(ui, &mut selected.slide, &mut app.project);
                }
                ui.collapsing("Overlays", |ui| {
                    changed |= crate::tools::props(ui, &mut app.project, &mut app.runtime);
                });
            });
        if let Some(selected) = selected
            && let Some(d) = app
                .project
                .documents
                .iter_mut()
                .find(|d| d.id == selected.document_id)
            && let Some(s) = d.slides.iter_mut().find(|s| s.id == selected.slide.id)
        {
            *s = selected.slide;
        }
        if changed {
            app.mark();
        }
        return;
    }
    tool_tabs(ui, &mut app.tool_tab);
    if app.tool_tab != 2 {
        crate::tools::overlay_chips(ui, &app.project, &mut app.runtime);
    }
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
                    if crate::tools::props(ui, &mut app.project, &mut app.runtime) {
                        app.mark();
                    }
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
        for (i, name) in ["Timers", "Messages", "Overlays", "Audio", "Stage"]
            .iter()
            .enumerate()
        {
            ui.selectable_value(tab, i, *name);
        }
    })
}

/// What the primary button sends: "Next" when the selected cue is the one after live,
/// otherwise the selected cue by name.
fn primary_label(
    armed: Option<((Uuid, Uuid), &str)>,
    next: Option<(Uuid, Uuid)>,
    live: bool,
    deck: bool,
) -> String {
    match armed {
        _ if deck => "Next".into(),
        Some((ids, _)) if live && Some(ids) == next => "Next".into(),
        // shortcut: egui cannot shape Gujarati inside a button, so a Gujarati name falls back to
        // the plain label (the Preview shows the shaped name); draw it with lumen-text if asked.
        Some((_, name)) if live && !crate::text::contains_gujarati(name) => {
            format!("Go live: {name}")
        }
        _ => "Go live".into(),
    }
}

fn grid_layout(available: f32, gap: f32) -> (usize, f32) {
    let columns = ((available + gap) / (180.0 + gap)).floor().max(1.0) as usize;
    (
        columns,
        (available - gap * (columns - 1) as f32).max(1.0) / columns as f32,
    )
}

/// The Preview pane has the label, notes and hint below the picture, and nothing else.
fn preview_pane_height(width: f32, height: f32) -> f32 {
    (width * 9.0 / 16.0).min((height - 72.0).max(80.0))
}

// Reserve one thumbnail row, its label and document controls below the preview.
pub fn preview_height(width: f32, height: f32) -> f32 {
    let (_, thumb_width) = grid_layout(width, style::GAP);
    (width * 9.0 / 16.0).min((height - thumb_width * 9.0 / 16.0 - 122.0).max(80.0))
}

const TRANSITION_KINDS: [&str; 4] = ["Cut", "Fade", "Fade through black", "Push left"];
const DEFAULT_SHOW_FADE_MS: u32 = 400;

fn transition_parts(transition: &Transition) -> (usize, u32) {
    match transition {
        Transition::Cut => (0, DEFAULT_SHOW_FADE_MS),
        Transition::Fade { millis } => (1, *millis),
        Transition::FadeThroughBlack { millis } => (2, *millis),
        Transition::Push { millis, .. } => (3, *millis),
    }
}

fn transition_from_parts(kind: usize, millis: u32) -> Transition {
    match kind {
        0 => Transition::Cut,
        1 => Transition::Fade { millis },
        2 => Transition::FadeThroughBlack { millis },
        _ => Transition::Push {
            millis,
            direction: lumen_core::PushDirection::Left,
        },
    }
}

/// Show-wide transition picker for Present. Returns true when the value changed.
fn transition_picker(ui: &mut egui::Ui, transition: &mut Transition) -> bool {
    let before = transition_parts(transition);
    let (mut kind, mut millis) = before;
    let summary = if kind == 0 {
        "Transition: Cut".to_string()
    } else {
        format!(
            "Transition: {} {} s",
            TRANSITION_KINDS[kind],
            millis as f32 / 1000.0
        )
    };
    egui::ComboBox::from_id_salt("show-transition")
        .selected_text(summary)
        .show_ui(ui, |ui| {
            for (i, name) in TRANSITION_KINDS.iter().enumerate() {
                ui.selectable_value(&mut kind, i, *name);
            }
            ui.separator();
            ui.add_enabled_ui(kind != 0, |ui| {
                ui.horizontal(|ui| {
                    for ms in [250, 500, 1000] {
                        ui.selectable_value(&mut millis, ms, format!("{} s", ms as f32 / 1000.0));
                    }
                });
            });
        });
    if (kind, millis) == before {
        return false;
    }
    *transition = transition_from_parts(kind, millis);
    true
}

#[cfg(test)]
mod transition_picker_tests {
    use super::*;

    #[test]
    fn picker_parts_round_trip_and_cut_keeps_a_default_duration() {
        for t in [
            Transition::Fade { millis: 250 },
            Transition::FadeThroughBlack { millis: 1000 },
            Transition::Push {
                millis: 500,
                direction: lumen_core::PushDirection::Left,
            },
        ] {
            let (kind, millis) = transition_parts(&t);
            assert_eq!(transition_from_parts(kind, millis), t);
        }
        assert_eq!(transition_parts(&Transition::Cut), (0, 400));
        assert_eq!(transition_from_parts(0, 400), Transition::Cut);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn primary_label_names_what_the_button_sends() {
        let (a, b) = (
            (Uuid::new_v4(), Uuid::new_v4()),
            (Uuid::new_v4(), Uuid::new_v4()),
        );
        assert_eq!(
            primary_label(Some((a, "A")), Some(a), false, false),
            "Go live"
        );
        assert_eq!(primary_label(None, Some(a), true, false), "Go live");
        assert_eq!(primary_label(Some((a, "A")), Some(a), true, false), "Next");
        assert_eq!(
            primary_label(Some((b, "photo.png")), Some(a), true, false),
            "Go live: photo.png"
        );
        // End of the order: nothing follows, so the selected live cue is re-sent by name.
        assert_eq!(
            primary_label(Some((a, "A")), None, true, false),
            "Go live: A"
        );
        assert_eq!(primary_label(Some((b, "B")), Some(a), true, true), "Next");
        assert_eq!(
            primary_label(Some((b, "\u{0a97}\u{0ac1}")), Some(a), true, false),
            "Go live"
        );
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
    fn preview_pane_fills_its_width_and_leaves_room_for_its_label() {
        assert!((preview_pane_height(800.0, 900.0) * 16.0 / 9.0 - 800.0).abs() < 0.1);
        assert!(preview_pane_height(800.0, 400.0) + 72.0 <= 400.0 + 0.1);
        // A very short pane still shows a usable picture.
        assert_eq!(preview_pane_height(800.0, 20.0), 80.0);
    }
    #[test]
    fn book_preview_keeps_one_grid_row_free() {
        let width = 1100.0 - 300.0 - 360.0 - 48.0;
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
    fn preview_uses_live_frames_only_for_the_on_air_background() {
        assert_eq!(
            preview_texture(true, Some("live"), Some("still")),
            Some("live")
        );
        // No frame yet: fall back to the still instead of showing nothing.
        assert_eq!(preview_texture(true, None, Some("still")), Some("still"));
        assert_eq!(
            preview_texture(false, Some("live"), Some("still")),
            Some("still")
        );
        assert_eq!(preview_texture::<&str>(false, Some("live"), None), None);
    }
    #[test]
    fn missing_document_cannot_enter_playlist() {
        let mut p = Project::demo();
        assert!(!append_playlist_item(&mut p, Uuid::nil()));
    }
}
