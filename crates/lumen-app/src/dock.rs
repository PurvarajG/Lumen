//! Movable panels: the six panes, their default arrangement per mode, and the saved layouts.
use crate::{app::LumenApp, settings::Layouts, style};
use egui::{Color32, CornerRadius, Margin, Stroke};
use egui_dock::{
    AllowedSplits, DockArea, DockState, Node, NodeIndex, OverlayType, Style, TabInteractionStyle,
    TabViewer,
};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// A saved layout is written at most this often while it keeps changing.
const SAVE_EVERY: Duration = Duration::from_secs(5);
/// Six panes fit in 63 nodes at any depth; more means the file is not one of ours.
const MAX_NODES: usize = 255;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum Pane {
    RunningOrder,
    Macros,
    Preview,
    Cues,
    Live,
    Tools,
}

const ALL: [Pane; 6] = [
    Pane::RunningOrder,
    Pane::Macros,
    Pane::Preview,
    Pane::Cues,
    Pane::Live,
    Pane::Tools,
];

/// Today's arrangement in Present; in Prepare a big running order and a small Preview.
pub fn default_layout(present: bool) -> DockState<Pane> {
    if present {
        let mut state = DockState::new(vec![Pane::Preview]);
        let tree = state.main_surface_mut();
        // A fraction is the share of the left or top node: the old one for `below` and `right`,
        // the new one for `above` and `left`.
        tree.split_below(NodeIndex::root(), 0.58, vec![Pane::Cues]);
        let [centre, _] = tree.split_left(
            NodeIndex::root(),
            0.22,
            vec![Pane::RunningOrder, Pane::Macros],
        );
        let [_, live] = tree.split_right(centre, 0.64, vec![Pane::Live]);
        tree.split_below(live, 0.45, vec![Pane::Tools]);
        state
    } else {
        let mut state = DockState::new(vec![Pane::Cues]);
        let tree = state.main_surface_mut();
        tree.split_above(NodeIndex::root(), 0.30, vec![Pane::Preview]);
        let [centre, left] = tree.split_left(NodeIndex::root(), 0.34, vec![Pane::RunningOrder]);
        tree.split_below(left, 0.75, vec![Pane::Macros]);
        tree.split_right(centre, 0.515, vec![Pane::Tools, Pane::Live]);
        state
    }
}

/// True when the state is one the app can show: only the main surface, a well-formed tree and
/// every pane exactly once where a split reaches it. Saved files are untrusted input.
pub fn is_valid(state: &DockState<Pane>) -> bool {
    if state.surfaces_count() != 1 {
        return false;
    }
    let nodes: Vec<&Node<Pane>> = state.main_surface().iter().collect();
    if nodes.is_empty() || nodes.len() > MAX_NODES {
        return false;
    }
    let mut seen = Vec::new();
    if !collect(&nodes, 0, &mut seen) || seen.len() != ALL.len() {
        return false;
    }
    ALL.iter()
        .all(|pane| seen.iter().filter(|p| *p == pane).count() == 1)
}

fn collect(nodes: &[&Node<Pane>], index: usize, seen: &mut Vec<Pane>) -> bool {
    match nodes.get(index) {
        Some(Node::Leaf(leaf)) => {
            seen.extend(leaf.tabs.iter().copied());
            !leaf.tabs.is_empty() && leaf.active.0 < leaf.tabs.len()
        }
        Some(Node::Horizontal(split) | Node::Vertical(split)) => {
            (0.0..=1.0).contains(&split.fraction)
                && collect(nodes, 2 * index + 1, seen)
                && collect(nodes, 2 * index + 2, seen)
        }
        _ => false,
    }
}

/// The layouts of both modes and what settings last held of them.
pub struct Docks {
    present: Option<DockState<Pane>>,
    prepare: Option<DockState<Pane>>,
    /// Serialized form last written (or loaded): `[prepare, present]`.
    saved: [String; 2],
    last_save: Instant,
    /// The mode on screen at the last frame, to save at a switch.
    shown: Option<bool>,
}

impl Docks {
    /// Loads the saved layouts. A missing one is the default; a bad one is the default and is
    /// named in the returned notice.
    pub fn restore(layouts: &mut Layouts) -> (Self, Option<String>) {
        let mut reset = false;
        let mut load = |saved: &mut Option<serde_json::Value>, present| match saved.take() {
            None => default_layout(present),
            Some(value) => match serde_json::from_value::<DockState<Pane>>(value) {
                Ok(state) if is_valid(&state) => state,
                _ => {
                    reset = true;
                    default_layout(present)
                }
            },
        };
        let present = load(&mut layouts.present, true);
        let prepare = load(&mut layouts.prepare, false);
        let docks = Self {
            saved: [form(&prepare), form(&present)],
            present: Some(present),
            prepare: Some(prepare),
            last_save: Instant::now(),
            shown: None,
        };
        (docks, reset.then(|| "Panel layout was reset".to_owned()))
    }

    fn slot(&mut self, present: bool) -> &mut Option<DockState<Pane>> {
        if present {
            &mut self.present
        } else {
            &mut self.prepare
        }
    }

    /// Takes the layout out so the viewer can borrow the app; give it back with `put`.
    fn take(&mut self, present: bool) -> DockState<Pane> {
        self.slot(present)
            .take()
            .unwrap_or_else(|| default_layout(present))
    }

    fn put(&mut self, present: bool, state: DockState<Pane>) {
        *self.slot(present) = Some(state);
    }

    pub fn reset(&mut self, present: bool) {
        *self.slot(present) = Some(default_layout(present));
    }

    fn forms(&self) -> [String; 2] {
        let form = |state: &Option<DockState<Pane>>| state.as_ref().map(form).unwrap_or_default();
        [form(&self.prepare), form(&self.present)]
    }

    fn layouts(&self) -> Layouts {
        let value = |state: &Option<DockState<Pane>>| {
            state
                .as_ref()
                .and_then(|s| serde_json::to_value(sanitized(s)).ok())
        };
        Layouts {
            present: value(&self.present),
            prepare: value(&self.prepare),
        }
    }
}

/// The serialized layout as a comparable string; the rectangles egui_dock measures each frame
/// are zeroed (an unmeasured one is infinite, which JSON cannot hold).
fn form(state: &DockState<Pane>) -> String {
    serde_json::to_string(&sanitized(state)).unwrap_or_default()
}

fn sanitized(state: &DockState<Pane>) -> DockState<Pane> {
    let mut copy = state.clone();
    for node in copy.main_surface_mut().iter_mut() {
        match node {
            Node::Leaf(leaf) => {
                leaf.rect = egui::Rect::ZERO;
                leaf.viewport = egui::Rect::ZERO;
            }
            Node::Horizontal(split) | Node::Vertical(split) => split.rect = egui::Rect::ZERO,
            Node::Empty => {}
        }
    }
    copy
}

impl LumenApp {
    /// Shows the panes of the current mode in the area the toolbar and status bar leave.
    pub(super) fn show_docks(&mut self, ctx: &egui::Context) {
        let present = self.present;
        let mut state = self.docks.take(present);
        // The tab that is showing in each stack, for its underline.
        let active: Vec<Pane> = state
            .main_surface()
            .iter()
            .filter_map(|node| node.get_leaf())
            .filter_map(|leaf| leaf.tabs.get(leaf.active.0).copied())
            .collect();
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                let mut viewer = Viewer { app: self, active };
                DockArea::new(&mut state)
                    .id(egui::Id::new(("lumen-dock", present)))
                    .style(dock_style(ui.style()))
                    .show_close_buttons(false)
                    .show_add_buttons(false)
                    .show_leaf_close_all_buttons(false)
                    .show_leaf_collapse_buttons(false)
                    .tab_context_menus(false)
                    .allowed_splits(AllowedSplits::All)
                    .show_inside(ui, &mut viewer);
            });
        self.docks.put(present, state);
        let switched = self
            .docks
            .shown
            .replace(present)
            .is_some_and(|before| before != present);
        let forms = self.docks.forms();
        if forms != self.docks.saved {
            let wait = SAVE_EVERY.saturating_sub(self.docks.last_save.elapsed());
            if switched || wait.is_zero() {
                self.save_layouts(forms);
            } else {
                // A drop that ends a drag may be the last repaint for a while.
                ctx.request_repaint_after(wait);
            }
        }
    }

    /// Writes the layouts to settings now if they changed (leaving the app).
    pub(super) fn save_layouts_now(&mut self) {
        let forms = self.docks.forms();
        if forms != self.docks.saved {
            self.save_layouts(forms);
        }
    }

    fn save_layouts(&mut self, forms: [String; 2]) {
        self.docks.saved = forms;
        self.docks.last_save = Instant::now();
        self.settings.layouts = self.docks.layouts();
        self.persist_settings();
    }
}

struct Viewer<'a> {
    app: &'a mut LumenApp,
    active: Vec<Pane>,
}

impl TabViewer for Viewer<'_> {
    type Tab = Pane;

    fn title(&mut self, pane: &mut Pane) -> egui::WidgetText {
        match pane {
            Pane::RunningOrder => "Running order",
            Pane::Macros => "Macros",
            Pane::Preview => "Preview",
            Pane::Cues => "Cues",
            Pane::Live => "Live",
            Pane::Tools if self.app.present => "Tools",
            Pane::Tools => "Inspector",
        }
        .into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, pane: &mut Pane) {
        let fill = match pane {
            Pane::Preview | Pane::Cues => style::CANVAS,
            _ => style::PANEL,
        };
        ui.painter().rect_filled(ui.max_rect(), 0.0, fill);
        egui::Frame::new()
            .inner_margin(Margin::same(style::PADDING))
            .show(ui, |ui| match pane {
                Pane::RunningOrder => crate::panels::running_order(self.app, ui),
                Pane::Macros => crate::panels::macros(self.app, ui),
                Pane::Preview => crate::panels::preview(self.app, ui),
                Pane::Cues => crate::panels::cues(self.app, ui),
                Pane::Live => crate::panels::live(self.app, ui),
                Pane::Tools => self.app.tools_pane(ui),
            });
    }

    /// A panel can be moved, never lost.
    fn closeable(&mut self, _: &mut Pane) -> bool {
        false
    }

    fn allowed_in_windows(&self, _: &mut Pane) -> bool {
        false
    }

    /// Each pane brings its own fill and scroll areas.
    fn clear_background(&self, _: &Pane) -> bool {
        false
    }

    fn scroll_bars(&self, _: &Pane) -> [bool; 2] {
        [false, false]
    }

    /// The signature saffron underline of the tab that is showing.
    fn on_tab_button(&mut self, pane: &mut Pane, response: &egui::Response) {
        if self.active.contains(pane) {
            let rect = response.rect;
            response.ctx.layer_painter(response.layer_id).line_segment(
                [rect.left_bottom(), rect.right_bottom()],
                Stroke::new(2.0_f32, style::SAFFRON),
            );
        }
    }
}

/// egui_dock's look from the style tokens only.
fn dock_style(base: &egui::Style) -> Style {
    let mut s = Style::from_egui(base);
    let tab = |fill: Color32, text: Color32| TabInteractionStyle {
        outline_color: Color32::TRANSPARENT,
        corner_radius: CornerRadius::same(3),
        bg_fill: fill,
        text_color: text,
    };
    s.dock_area_padding = None;
    s.main_surface_border_stroke = Stroke::NONE;
    s.tab_bar.bg_fill = style::PANEL;
    s.tab_bar.hline_color = style::LINE;
    s.tab_bar.height = style::CONTROL;
    s.tab_bar.corner_radius = CornerRadius::ZERO;
    s.tab.active = tab(style::RAISED, style::TEXT);
    s.tab.focused = tab(style::RAISED, style::TEXT);
    s.tab.active_with_kb_focus = tab(style::RAISED, style::TEXT);
    s.tab.focused_with_kb_focus = tab(style::RAISED, style::TEXT);
    s.tab.inactive = tab(Color32::TRANSPARENT, style::TEXT_DIM);
    s.tab.inactive_with_kb_focus = tab(Color32::TRANSPARENT, style::TEXT_DIM);
    s.tab.hovered = tab(style::RAISED, style::TEXT);
    s.tab.tab_body.bg_fill = Color32::TRANSPARENT;
    s.tab.tab_body.stroke = Stroke::NONE;
    s.tab.tab_body.inner_margin = Margin::ZERO;
    s.tab.tab_body.corner_radius = CornerRadius::ZERO;
    s.tab.hline_below_active_tab_name = false;
    s.separator.width = 1.0;
    s.separator.extra_interact_width = 3.0;
    s.separator.color_idle = style::LINE;
    s.separator.color_hovered = style::TEXT_DIM;
    s.separator.color_dragged = style::SAFFRON;
    s.overlay.selection_color = style::SAFFRON.gamma_multiply(0.25);
    // Drop on a node's edge to split it, in its middle to stack.
    s.overlay.overlay_type = OverlayType::HighlightedAreas;
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panes(state: &DockState<Pane>) -> Vec<Pane> {
        let mut all: Vec<Pane> = state.iter_all_tabs().map(|(_, pane)| *pane).collect();
        all.sort_by_key(|p| ALL.iter().position(|a| a == p));
        all
    }

    #[test]
    fn both_default_layouts_hold_every_pane_once() {
        for present in [true, false] {
            let state = default_layout(present);
            assert_eq!(panes(&state), ALL.to_vec(), "present: {present}");
            assert!(is_valid(&state), "present: {present}");
        }
    }

    #[test]
    fn a_missing_or_repeated_pane_is_not_valid() {
        let mut missing = default_layout(true);
        missing.retain_tabs(|pane| *pane != Pane::Macros);
        assert!(!is_valid(&missing));
        let mut twice = default_layout(true);
        twice.push_to_first_leaf(Pane::Cues);
        assert!(!is_valid(&twice));
    }

    #[test]
    fn a_layout_survives_the_round_trip_through_json() {
        for present in [true, false] {
            let text = form(&default_layout(present));
            let back: DockState<Pane> = serde_json::from_str(&text).unwrap();
            assert!(is_valid(&back));
            assert_eq!(form(&back), text);
        }
    }

    #[test]
    fn restore_uses_good_layouts_and_replaces_bad_ones_with_a_notice() {
        // Complete layouts, one of them moved from the default.
        let mut moved = default_layout(false);
        let from = moved.find_tab(&Pane::Tools).unwrap();
        let (surface, node, _) = moved.find_tab(&Pane::Cues).unwrap();
        moved.move_tab(from, (surface, node, egui_dock::TabInsert::Append));
        let mut layouts = Layouts {
            present: serde_json::to_value(sanitized(&default_layout(true))).ok(),
            prepare: serde_json::to_value(sanitized(&moved)).ok(),
        };
        let (docks, notice) = Docks::restore(&mut layouts);
        assert_eq!(notice, None);
        assert_eq!(docks.saved[1], form(&default_layout(true)));
        assert_ne!(docks.saved[0], form(&default_layout(false)));

        // Unparsable JSON, a state missing a pane, and a tree whose children do not exist.
        let mut without = default_layout(true);
        without.retain_tabs(|pane| *pane != Pane::Live);
        for bad in [
            serde_json::json!("garbage"),
            serde_json::json!({"surfaces": 7}),
            serde_json::to_value(sanitized(&without)).unwrap(),
        ] {
            let mut layouts = Layouts {
                present: Some(bad),
                prepare: None,
            };
            let (docks, notice) = Docks::restore(&mut layouts);
            assert_eq!(notice.as_deref(), Some("Panel layout was reset"));
            assert_eq!(docks.saved[1], form(&default_layout(true)));
            assert_eq!(docks.saved[0], form(&default_layout(false)));
        }
    }

    #[test]
    fn a_tree_with_missing_children_is_not_valid() {
        let mut json = serde_json::to_value(sanitized(&default_layout(true))).unwrap();
        // Cut the node list down to the root split; its children are gone.
        let nodes = json
            .pointer_mut("/surfaces/0/Main/nodes")
            .and_then(|v| v.as_array_mut())
            .expect("the serialized shape changed; update this test");
        nodes.truncate(1);
        let state: Result<DockState<Pane>, _> = serde_json::from_value(json);
        assert!(state.is_ok_and(|s| !is_valid(&s)));
    }
}
