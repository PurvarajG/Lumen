use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    thread,
};

use egui::Ui;
use lumen_core::AudioCue;
use lumen_media::{AudioOptions, AudioPlayer};

const CLEAR_FADE_MS: u32 = 300;

enum PickerResult {
    Files(Vec<PathBuf>),
    Devices(Result<Vec<String>, String>),
}

pub struct AudioState {
    player: Option<AudioPlayer>,
    pub error: String,
    bin: Vec<AudioCue>,
    volume: f32,
    devices: Vec<String>,
    selected_device: Option<String>,
    pending: Option<Receiver<PickerResult>>,
    device_scan_started: bool,
    current_path: Option<PathBuf>,
}
impl Default for AudioState {
    fn default() -> Self {
        Self {
            player: None,
            error: String::new(),
            bin: Vec::new(),
            volume: 1.0,
            devices: Vec::new(),
            selected_device: None,
            pending: None,
            device_scan_started: false,
            current_path: None,
        }
    }
}
impl AudioState {
    pub fn play_cue(&mut self, cue: &AudioCue) {
        if self.player.is_none() {
            match AudioPlayer::new(self.selected_device.as_deref()) {
                Ok(player) => self.player = Some(player),
                Err(error) => {
                    self.error = error.to_string();
                    return;
                }
            }
        }
        let Some(player) = self.player.as_mut() else {
            self.error = "Audio output could not be initialized.".into();
            return;
        };
        match player.play(
            &cue.path,
            AudioOptions {
                volume: cue.volume.clamp(0.0, 1.0),
                ..AudioOptions::default()
            },
        ) {
            Ok(()) => {
                self.error.clear();
                self.current_path = Some(cue.path.clone());
            }
            Err(error) => self.error = error.to_string(),
        }
    }
    pub fn clear(&mut self) {
        if let Some(player) = self.player.as_mut() {
            player.stop(CLEAR_FADE_MS);
        }
    }
    pub fn stop(&mut self) {
        if let Some(player) = self.player.as_mut() {
            player.stop(0);
        }
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        self.poll();
        ui.heading("Audio");
        ui.horizontal_wrapped(|ui| {
            if ui.button("Add audio files").clicked() && self.pending.is_none() {
                self.start_file_picker();
            }
            ui.add(egui::Slider::new(&mut self.volume, 0.0..=1.0).text("New file volume"));
            if ui.button("Stop").clicked() {
                self.stop();
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("Output device");
            let selected = self.selected_device.as_deref().unwrap_or("System default");
            egui::ComboBox::from_id_salt("audio-output-device")
                .selected_text(selected)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(self.selected_device.is_none(), "System default")
                        .clicked()
                    {
                        self.set_device(None);
                    }
                    for device in self.devices.clone() {
                        if ui
                            .selectable_label(
                                self.selected_device.as_deref() == Some(&device),
                                &device,
                            )
                            .clicked()
                        {
                            self.set_device(Some(device));
                        }
                    }
                });
            if ui.button("Refresh").clicked() && self.pending.is_none() {
                self.start_device_scan();
            }
        });
        if self.pending.is_some() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(100));
            ui.horizontal_wrapped(|ui| {
                ui.add(egui::Spinner::new());
                ui.label("Loading audio files or devices…");
            });
        }
        if self.devices.is_empty() && self.device_scan_started && self.pending.is_none() {
            ui.label("No audio output device found. Connect a device, then Refresh.");
        }
        if self.bin.is_empty() {
            ui.label("No audio files added yet — Add audio files.");
        }
        let mut remove = None;
        let mut play = None;
        let mut volume_change = None;
        for (i, cue) in self.bin.iter_mut().enumerate() {
            ui.push_id(i, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        cue.path
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or("Audio file"),
                    );
                    if ui.button("Play").clicked() {
                        play = Some(i);
                    }
                    if ui
                        .add(egui::Slider::new(&mut cue.volume, 0.0..=1.0).text("Volume"))
                        .changed()
                        && self.current_path.as_ref() == Some(&cue.path)
                    {
                        volume_change = Some(cue.volume);
                    }
                    if ui.small_button("Remove").clicked() {
                        remove = Some(i);
                    }
                });
                ui.small(cue.path.display().to_string());
            });
        }
        if let Some(i) = remove {
            self.bin.remove(i);
        } else if let Some(i) = play
            && let Some(cue) = self.bin.get(i).cloned()
        {
            self.play_cue(&cue);
        }
        if let Some(volume) = volume_change
            && let Some(player) = self.player.as_mut()
            && let Err(error) = player.set_volume(volume)
        {
            self.error = error.to_string();
        }
        if !self.device_scan_started && self.pending.is_none() {
            self.start_device_scan();
        }
        if !self.error.is_empty() {
            ui.colored_label(crate::style::WARN, &self.error);
        }
    }

    fn set_device(&mut self, device: Option<String>) {
        if self.selected_device != device {
            self.selected_device = device;
            self.player = None;
            self.current_path = None;
        }
    }
    fn poll(&mut self) {
        let Some(receiver) = self.pending.as_ref() else {
            return;
        };
        match receiver.try_recv() {
            Ok(PickerResult::Files(files)) => {
                self.pending = None;
                for path in files {
                    self.bin.push(AudioCue {
                        path,
                        volume: self.volume,
                    });
                }
            }
            Ok(PickerResult::Devices(result)) => {
                self.pending = None;
                match result {
                    Ok(devices) => {
                        self.devices = devices;
                        self.error.clear();
                    }
                    Err(error) => self.error = error,
                }
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.pending = None;
                self.error = "Audio picker worker stopped unexpectedly.".into();
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
    }
    fn start_file_picker(&mut self) {
        let (tx, rx) = mpsc::channel();
        self.pending = Some(rx);
        thread::Builder::new()
            .name("lumen-audio-picker".into())
            .spawn(move || {
                let files = rfd::FileDialog::new()
                    .add_filter("Audio", &["mp3", "wav", "flac", "ogg", "m4a", "aac"])
                    .pick_files()
                    .unwrap_or_default();
                let _ = tx.send(PickerResult::Files(files));
            })
            .map_err(|e| self.error = format!("Could not start audio picker: {e}"))
            .ok();
    }
    fn start_device_scan(&mut self) {
        let (tx, rx) = mpsc::channel();
        self.pending = Some(rx);
        self.device_scan_started = true;
        if let Err(e) = thread::Builder::new()
            .name("lumen-audio-devices".into())
            .spawn(move || {
                let result = lumen_media::output_devices().map_err(|e| e.to_string());
                let _ = tx.send(PickerResult::Devices(result));
            })
        {
            self.pending = None;
            self.error = format!("Could not scan audio devices: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CLEAR_FADE_MS;
    #[test]
    fn clear_uses_short_fade_duration() {
        assert_eq!(CLEAR_FADE_MS, 300);
    }
}
