//! Display selection state and the Outputs panel.
//!
//! eframe exposes the active window's monitor bounds, but not the system's
//! monitor list. The platform layer supplies `DisplayInfo` values to this UI.

use crate::settings::Settings;
use egui::Ui;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DisplayKey {
    pub name: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisplayInfo {
    pub key: DisplayKey,
    /// Origin in the coordinate system used by native window placement.
    pub x: i32,
    pub y: i32,
    pub primary: bool,
}

impl DisplayInfo {
    pub fn label(&self) -> String {
        format!(
            "{}, {} × {}",
            self.key.name, self.key.width, self.key.height
        )
    }
}

/// Match a persisted display identity to the currently attached displays, most exact first:
/// same name and resolution; same name at any resolution (an LED processor or projector that
/// renegotiated its mode); otherwise the only non-primary display, since a rented or swapped
/// projector arrives under a new name. `None` when none applies.
/// shortcut: the saved display's primary-ness is not stored, so a saved built-in screen with the
/// lid closed and two externals attached can match the wrong external; fine until displays keep ids.
pub fn match_display<'a>(
    saved: Option<&DisplayKey>,
    available: &'a [DisplayInfo],
) -> Option<&'a DisplayInfo> {
    let Some(saved) = saved else {
        return available
            .iter()
            .find(|display| display.primary)
            .or_else(|| available.first());
    };
    available
        .iter()
        .find(|display| display.key == *saved)
        .or_else(|| available.iter().find(|d| d.key.name == saved.name))
        .or_else(|| {
            let mut others = available.iter().filter(|display| !display.primary);
            others.next().filter(|_| others.next().is_none())
        })
}

/// Where an output window actually goes: its matched display, else the primary one.
pub fn target_display<'a>(
    saved: Option<&DisplayKey>,
    available: &'a [DisplayInfo],
) -> Option<&'a DisplayInfo> {
    match_display(saved, available).or_else(|| match_display(None, available))
}

/// An open, placed output window must be put back on its display (windowed, then fullscreen)
/// when the display it should be on changed: it dropped out, came back, moved or changed mode.
pub fn needs_replace(placed: bool, last: Option<&DisplayInfo>, now: Option<&DisplayInfo>) -> bool {
    placed && last != now
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OutputsAction {
    pub changed: bool,
    /// Start temporary numbered windows on all available displays.
    pub identify: bool,
}

pub fn outputs_panel(
    ui: &mut Ui,
    settings: &mut Settings,
    displays: &[DisplayInfo],
) -> OutputsAction {
    let mut action = OutputsAction::default();
    ui.heading("Displays");
    if displays.is_empty() {
        ui.label("No displays were reported by the platform.");
        return action;
    }
    if displays.len() == 1 {
        ui.label(format!("{} (primary)", displays[0].label()));
    }

    for (name, saved, fullscreen) in [
        (
            "Audience",
            &mut settings.audience_display,
            &mut settings.audience_fullscreen,
        ),
        (
            "Stage",
            &mut settings.stage_display,
            &mut settings.stage_fullscreen,
        ),
    ] {
        ui.separator();
        ui.label(name);
        let matched = match_display(saved.as_ref(), displays);
        if saved.is_some() && matched.is_none() {
            ui.colored_label(
                crate::style::WARN,
                "Saved display is unavailable; using the primary display until it returns.",
            );
        }
        let selected_index = matched
            .or_else(|| match_display(None, displays))
            .map(|display| {
                displays
                    .iter()
                    .position(|item| item == display)
                    .unwrap_or(0)
            })
            .unwrap_or(0);
        for (index, display) in displays.iter().enumerate() {
            if ui.radio(index == selected_index, display.label()).clicked()
                && saved.as_ref() != Some(&display.key)
            {
                *saved = Some(display.key.clone());
                action.changed = true;
            }
        }
        if ui.checkbox(fullscreen, "Fullscreen").changed() {
            action.changed = true;
        }
        ui.horizontal(|ui| {
            if ui.button("Identify").clicked() {
                action.identify = true;
            }
            ui.label("Shows a number on every display for 3 seconds.");
        });
    }
    action
}

/// Build an output viewport positioned over the selected display. eframe does
/// not expose monitor handles; placing the window at the monitor origin lets
/// the native backend select that monitor for fullscreen.
pub fn output_viewport_builder(
    title: &str,
    size: [f32; 2],
    saved: Option<&DisplayKey>,
    displays: &[DisplayInfo],
    fullscreen: bool,
) -> (egui::ViewportBuilder, bool) {
    let selected = match_display(saved, displays);
    let missing = saved.is_some() && selected.is_none();
    let selected = selected.or_else(|| match_display(None, displays));
    let mut builder = egui::ViewportBuilder::default()
        .with_title(title.to_owned())
        .with_inner_size(size);
    if let Some(display) = selected {
        builder = builder.with_position([display.x as f32, display.y as f32]);
    }
    (builder.with_fullscreen(fullscreen && !missing), missing)
}

/// Open one temporary, always-on-top numbered viewport per display. Call from
/// the root update loop while an Identify deadline is active; the viewports
/// close themselves after the deadline.
pub fn show_identify(ctx: &egui::Context, displays: &[DisplayInfo], deadline: Instant) {
    for (index, display) in displays.iter().enumerate() {
        let id = egui::ViewportId::from_hash_of(("identify-display", index));
        let builder = egui::ViewportBuilder::default()
            .with_title(format!("Identify display {}", index + 1))
            .with_inner_size([360.0, 240.0])
            .with_position([display.x as f32, display.y as f32])
            .with_decorations(false)
            .with_always_on_top();
        ctx.show_viewport_deferred(id, builder, move |identify_ctx, _| {
            if Instant::now() >= deadline {
                identify_ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(crate::style::CANVAS))
                .show(identify_ctx, |ui| {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            egui::RichText::new((index + 1).to_string())
                                .size(112.0)
                                .strong()
                                .color(crate::style::WHITE),
                        );
                    });
                });
            identify_ctx.request_repaint_after(std::time::Duration::from_millis(50));
        });
    }
}

/// Enumerate active macOS displays via CoreGraphics. Other platforms should
/// provide their platform's monitor list to the same UI/model API.
#[cfg(target_os = "macos")]
pub fn enumerate_displays() -> Vec<DisplayInfo> {
    macos::enumerate()
}

#[cfg(not(target_os = "macos"))]
pub fn enumerate_displays() -> Vec<DisplayInfo> {
    Vec::new()
}

#[cfg(target_os = "macos")]
mod macos {
    use super::{DisplayInfo, DisplayKey};
    use objc2::{class, msg_send, runtime::AnyObject};
    use std::{collections::HashMap, ffi::CStr};
    type DisplayId = u32;
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Point {
        x: f64,
        y: f64,
    }
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Size {
        width: f64,
        height: f64,
    }
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Rect {
        origin: Point,
        size: Size,
    }

    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGGetActiveDisplayList(max: u32, displays: *mut DisplayId, count: *mut u32) -> i32;
        fn CGMainDisplayID() -> DisplayId;
        fn CGDisplayBounds(display: DisplayId) -> Rect;
        fn CGDisplayPixelsWide(display: DisplayId) -> usize;
        fn CGDisplayPixelsHigh(display: DisplayId) -> usize;
    }

    pub(super) fn enumerate() -> Vec<DisplayInfo> {
        let mut count = 0;
        // First call asks CoreGraphics for the active display count.
        if unsafe { CGGetActiveDisplayList(0, std::ptr::null_mut(), &mut count) } != 0 {
            return Vec::new();
        }
        let mut ids = vec![0; count as usize];
        if unsafe { CGGetActiveDisplayList(count, ids.as_mut_ptr(), &mut count) } != 0 {
            return Vec::new();
        }
        let primary = unsafe { CGMainDisplayID() };
        let names = screen_names();
        ids.into_iter()
            .take(count as usize)
            .map(|id| {
                let bounds = unsafe { CGDisplayBounds(id) };
                let name = names
                    .get(&id)
                    .cloned()
                    .unwrap_or_else(|| format!("Display {id}"));
                DisplayInfo {
                    key: DisplayKey {
                        name,
                        width: unsafe { CGDisplayPixelsWide(id) } as u32,
                        height: unsafe { CGDisplayPixelsHigh(id) } as u32,
                    },
                    x: bounds.origin.x.round() as i32,
                    y: bounds.origin.y.round() as i32,
                    primary: id == primary,
                }
            })
            .collect()
    }

    fn screen_names() -> HashMap<DisplayId, String> {
        let mut names = HashMap::new();
        unsafe {
            let screens: *mut AnyObject = msg_send![class!(NSScreen), screens];
            if screens.is_null() {
                return names;
            }
            let count: usize = msg_send![screens, count];
            let key = b"NSScreenNumber\0";
            let key_string: *mut AnyObject = msg_send![class!(NSString), stringWithUTF8String: key.as_ptr().cast::<std::ffi::c_char>()];
            for index in 0..count {
                let screen: *mut AnyObject = msg_send![screens, objectAtIndex: index];
                let name: *mut AnyObject = msg_send![screen, localizedName];
                let description: *mut AnyObject = msg_send![screen, deviceDescription];
                let number: *mut AnyObject = msg_send![description, objectForKey: key_string];
                if name.is_null() || number.is_null() {
                    continue;
                }
                let display_id: u32 = msg_send![number, unsignedIntValue];
                let utf8: *const std::ffi::c_char = msg_send![name, UTF8String];
                if !utf8.is_null() {
                    let label = CStr::from_ptr(utf8).to_string_lossy().into_owned();
                    if !label.is_empty() {
                        names.insert(display_id, label);
                    }
                }
            }
        }
        names
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn display(name: &str, width: u32, height: u32, x: i32) -> DisplayInfo {
        DisplayInfo {
            key: DisplayKey {
                name: name.into(),
                width,
                height,
            },
            x,
            y: 0,
            primary: x == 0,
        }
    }

    #[test]
    fn saved_display_requires_name_and_resolution_match() {
        let displays = [
            display("Built-in", 3024, 1964, 0),
            display("Projector", 1920, 1080, 3024),
        ];
        assert_eq!(
            match_display(Some(&displays[1].key), &displays),
            Some(&displays[1])
        );
        assert_eq!(
            match_display(Some(&displays[0].key), &displays),
            Some(&displays[0])
        );
    }

    #[test]
    fn saved_display_survives_a_resolution_change_or_a_rename() {
        let displays = [
            display("Built-in", 3024, 1964, 0),
            display("Projector", 1920, 1080, 3024),
        ];
        let changed_resolution = DisplayKey {
            name: "Projector".into(),
            width: 1280,
            height: 720,
        };
        assert_eq!(
            match_display(Some(&changed_resolution), &displays),
            Some(&displays[1])
        );
        let renamed = DisplayKey {
            name: "LED processor".into(),
            width: 3840,
            height: 2160,
        };
        assert_eq!(match_display(Some(&renamed), &displays), Some(&displays[1]));
    }

    #[test]
    fn a_missing_display_is_not_guessed_when_several_others_are_attached() {
        let displays = [
            display("Built-in", 3024, 1964, 0),
            display("Projector", 1920, 1080, 3024),
            display("Stage TV", 1920, 1080, 4944),
        ];
        let gone = DisplayKey {
            name: "LED processor".into(),
            width: 3840,
            height: 2160,
        };
        assert_eq!(match_display(Some(&gone), &displays), None);
        assert_eq!(match_display(Some(&gone), &displays[..1]), None);
        assert_eq!(target_display(Some(&gone), &displays), Some(&displays[0]));
    }

    #[test]
    fn replaces_a_placed_window_only_when_its_display_changed() {
        let a = display("Projector", 1920, 1080, 3024);
        let b = display("Projector", 1280, 720, 3024);
        assert!(!needs_replace(true, Some(&a), Some(&a)));
        assert!(needs_replace(true, Some(&a), Some(&b)));
        assert!(needs_replace(true, Some(&a), None));
        assert!(needs_replace(true, None, Some(&a)));
        assert!(!needs_replace(false, Some(&a), Some(&b)));
    }

    #[test]
    fn missing_or_unset_selection_falls_back_to_primary() {
        let displays = [
            display("Built-in", 3024, 1964, 0),
            display("Projector", 1920, 1080, 3024),
        ];
        assert_eq!(match_display(None, &displays), Some(&displays[0]));
        let missing = DisplayKey {
            name: "Gone".into(),
            width: 800,
            height: 600,
        };
        // Only the laptop is attached, so nothing else can stand in for the missing display.
        assert_eq!(match_display(Some(&missing), &displays[..1]), None);
        assert_eq!(
            target_display(Some(&missing), &displays[..1]),
            Some(&displays[0])
        );
    }
}
