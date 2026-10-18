//! Now-playing strip under the transport row: one row per source that is making sound.

use crate::{app::LumenApp, audio::NowPlaying, style};
use egui::{Align, Layout, RichText, Ui, WidgetInfo, WidgetType};
use std::time::Duration;

// Phosphor Regular codepoints (the font is embedded by `style::install`).
const PLAY: char = '\u{e3d0}';
const PAUSE: char = '\u{e39e}';
const STOP: char = '\u{e46c}';

/// "1:23 / -2:41"; with an unknown total just "1:23".
pub fn format_clock(elapsed: Duration, total: Option<Duration>) -> String {
    match total {
        Some(total) => format!(
            "{} / -{}",
            clock(elapsed.min(total)),
            clock(total.saturating_sub(elapsed))
        ),
        None => clock(elapsed),
    }
}

fn clock(d: Duration) -> String {
    let s = d.as_secs();
    match s {
        0..3600 => format!("{}:{:02}", s / 60, s % 60),
        _ => format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60),
    }
}

/// Shows the strip, and nothing at all when no audio cue or unmuted video is playing.
pub fn now_playing(app: &mut LumenApp, ui: &mut Ui) {
    let audio = app.audio.now_playing();
    let video = app
        .media
        .player
        .as_ref()
        .filter(|p| p.has_sound() && !p.is_finished())
        .map(|p| NowPlaying {
            name: app
                .media
                .video_path
                .as_ref()
                .and_then(|path| path.file_name())
                .map_or_else(|| "Video".into(), |n| n.to_string_lossy().into_owned()),
            elapsed: Duration::from_secs_f64(p.position_secs().max(0.0)),
            total: p.duration_secs().map(Duration::from_secs_f64),
            paused: p.is_paused(),
            volume: 0.0,
        });
    if audio.is_none() && video.is_none() {
        return;
    }
    ui.add_space(style::GAP / 2.0);
    if let Some(np) = &audio {
        let (toggle, stop, volume) = row(ui, "now-playing-audio", np, true);
        if toggle {
            app.audio.toggle_pause();
        }
        if stop {
            app.audio.clear();
        }
        if let Some(volume) = volume {
            app.audio.set_live_volume(volume);
        }
    }
    if let Some(np) = &video {
        let (toggle, stop, _) = row(ui, "now-playing-video", np, false);
        if toggle
            && let Some(player) = app.media.player.as_mut()
            && let Err(e) = player.set_paused(!player.is_paused())
        {
            app.error = e.to_string();
        }
        if stop {
            app.clear_layer(3);
        }
    }
    // Seconds clock: 4 Hz is plenty, and a frozen strip needs no repaints.
    if [&audio, &video].into_iter().flatten().any(|np| !np.paused) {
        ui.ctx().request_repaint_after(Duration::from_millis(250));
    }
}

/// Returns (pause toggled, stop pressed, new volume).
fn row(ui: &mut Ui, id: &str, np: &NowPlaying, volume: bool) -> (bool, bool, Option<f32>) {
    let (mut toggle, mut stop, mut new_volume) = (false, false, None);
    ui.push_id(id, |ui| {
        ui.horizontal(|ui| {
            let (glyph, tip) = if np.paused {
                (PLAY, "Resume")
            } else {
                (PAUSE, "Pause")
            };
            toggle = icon_button(ui, glyph, tip);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                stop = icon_button(ui, STOP, "Stop");
                let text = RichText::new(format_clock(np.elapsed, np.total))
                    .monospace()
                    .size(style::BODY)
                    .color(style::TEXT_DIM);
                ui.label(text);
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.add(egui::Label::new(RichText::new(&np.name)).truncate())
                        .on_hover_text(&np.name);
                });
            });
        });
        if let Some(total) = np.total.filter(|t| !t.is_zero()) {
            let fraction = (np.elapsed.as_secs_f32() / total.as_secs_f32()).clamp(0.0, 1.0);
            let fill = if np.paused {
                style::TEXT_FAINT
            } else {
                style::OK
            };
            ui.add(
                egui::ProgressBar::new(fraction)
                    .desired_width(ui.available_width())
                    .desired_height(4.0)
                    .corner_radius(2)
                    .fill(fill),
            );
        }
        if volume {
            ui.horizontal(|ui| {
                ui.spacing_mut().slider_width = (ui.available_width() - 72.0).max(60.0);
                let mut v = np.volume;
                if ui
                    .add(
                        egui::Slider::new(&mut v, 0.0..=1.0)
                            .show_value(false)
                            .text("Volume"),
                    )
                    .changed()
                {
                    new_volume = Some(v);
                }
            });
        }
    });
    (toggle, stop, new_volume)
}

fn icon_button(ui: &mut Ui, glyph: char, label: &str) -> bool {
    let response = ui.add_sized(
        [style::CONTROL, style::CONTROL],
        egui::Button::new(style::icon(glyph, 16.0)),
    );
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, label));
    response.on_hover_text(label).clicked()
}

#[cfg(test)]
mod tests {
    use super::format_clock;
    use std::time::Duration;

    fn secs(s: u64) -> Duration {
        Duration::from_secs(s)
    }

    #[test]
    fn clock_shows_elapsed_and_remaining() {
        assert_eq!(format_clock(secs(0), Some(secs(0))), "0:00 / -0:00");
        assert_eq!(format_clock(secs(83), Some(secs(244))), "1:23 / -2:41");
        assert_eq!(format_clock(secs(3599), Some(secs(3599))), "59:59 / -0:00");
    }

    #[test]
    fn clock_switches_to_hours() {
        assert_eq!(
            format_clock(secs(3723), Some(secs(7200))),
            "1:02:03 / -57:57"
        );
    }

    #[test]
    fn unknown_total_shows_elapsed_only() {
        assert_eq!(format_clock(secs(83), None), "1:23");
        assert_eq!(format_clock(secs(3600), None), "1:00:00");
    }

    #[test]
    fn elapsed_past_the_total_never_goes_negative() {
        assert_eq!(format_clock(secs(70), Some(secs(60))), "1:00 / -0:00");
    }
}
