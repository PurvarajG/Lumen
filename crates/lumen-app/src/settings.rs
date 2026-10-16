use crate::style;
use egui::{RichText, Ui};
use lumen_core::Project;
use serde::{Deserialize, Serialize};
use std::{fs, io, net::SocketAddr, path::Path};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct StageOptions {
    pub current: bool,
    pub next: bool,
    pub clock: bool,
    pub timer: Option<Uuid>,
    pub message: bool,
    pub notes: bool,
}

impl Default for StageOptions {
    fn default() -> Self {
        Self {
            current: true,
            next: true,
            clock: true,
            timer: None,
            message: true,
            notes: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub http_enabled: bool,
    pub http_addr: String,
    pub http_token: String,
    pub osc_enabled: bool,
    pub osc_addr: String,
    pub midi_port: Option<String>,
    pub midi_mappings: Vec<lumen_control::MidiMapping>,
    pub stage: StageOptions,
    #[serde(skip)]
    pub reveal_token: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            http_enabled: false,
            http_addr: "127.0.0.1:47800".into(),
            http_token: generate_token(),
            osc_enabled: false,
            osc_addr: "127.0.0.1:47801".into(),
            midi_port: None,
            midi_mappings: Vec::new(),
            stage: StageOptions::default(),
            reveal_token: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SettingsLoad {
    pub settings: Settings,
    pub notice: Option<String>,
}

impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        self.http_addr
            .parse::<SocketAddr>()
            .map_err(|e| format!("Invalid HTTP bind address: {e}"))?;
        self.osc_addr
            .parse::<SocketAddr>()
            .map_err(|e| format!("Invalid OSC bind address: {e}"))?;
        if self.http_token.chars().count() < 16 {
            return Err("HTTP token must contain at least 16 characters".into());
        }
        if self
            .midi_mappings
            .iter()
            .any(|m| m.channel > 15 || m.note > 127)
        {
            return Err("MIDI channels must be 0–15 and note numbers 0–127".into());
        }
        Ok(())
    }

    pub fn load(path: &Path) -> SettingsLoad {
        match fs::read(path) {
            Ok(bytes) => match serde_json::from_slice::<Settings>(&bytes) {
                Ok(settings) => match settings.validate() {
                    Ok(()) => SettingsLoad {
                        settings,
                        notice: None,
                    },
                    Err(error) => SettingsLoad {
                        settings: Self::default(),
                        notice: Some(format!("Settings were reset to defaults: {error}")),
                    },
                },
                Err(error) => SettingsLoad {
                    settings: Self::default(),
                    notice: Some(format!(
                        "Couldn't read settings; defaults are active ({error})"
                    )),
                },
            },
            Err(error) if error.kind() == io::ErrorKind::NotFound => SettingsLoad {
                settings: Self::default(),
                notice: None,
            },
            Err(error) => SettingsLoad {
                settings: Self::default(),
                notice: Some(format!(
                    "Couldn't open settings; defaults are active ({error})"
                )),
            },
        }
    }

    pub fn save_atomic(&self, path: &Path) -> io::Result<()> {
        self.validate()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
        let suffix = Uuid::new_v4();
        let temp = path.with_extension(format!(
            "{}.{}.tmp",
            path.extension().and_then(|v| v.to_str()).unwrap_or("json"),
            suffix
        ));
        let result = (|| {
            use std::io::Write;
            let mut file = private_temp_file(&temp)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            fs::rename(&temp, path)
        })();
        if result.is_err() {
            let _ = fs::remove_file(temp);
        }
        result
    }
}

#[cfg(unix)]
fn private_temp_file(path: &Path) -> io::Result<fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
}

#[cfg(not(unix))]
fn private_temp_file(path: &Path) -> io::Result<fs::File> {
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
}

pub fn settings_path() -> Option<std::path::PathBuf> {
    if let Some(root) = std::env::var_os("LUMEN_DATA_DIR") {
        return Some(std::path::PathBuf::from(root).join("settings.json"));
    }
    std::env::var_os("HOME").map(|home| {
        std::path::PathBuf::from(home)
            .join("Library/Application Support/Lumen Presenter/settings.json")
    })
}

pub fn generate_token() -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    // 24 base64url characters carry 144 bits, so use two UUIDs and consume the
    // first 144 bits. UUID v4 provides cryptographically random bytes.
    let mut bytes = [0_u8; 32];
    bytes[..16].copy_from_slice(Uuid::new_v4().as_bytes());
    bytes[16..].copy_from_slice(Uuid::new_v4().as_bytes());
    (0..24)
        .map(|i| {
            let bit = i * 6;
            let byte = bit / 8;
            let shift = bit % 8;
            let pair = (u16::from(bytes[byte]) << 8) | u16::from(bytes[byte + 1]);
            ALPHABET[((pair >> (10 - shift)) & 63) as usize] as char
        })
        .collect()
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SettingsAction {
    pub changed: bool,
    pub restart_requested: bool,
}

pub fn settings_panel(
    ui: &mut Ui,
    settings: &mut Settings,
    project: &Project,
    notice: Option<&str>,
) -> SettingsAction {
    let mut action = SettingsAction::default();
    if let Some(notice) = notice {
        ui.label(RichText::new(notice).color(style::LIVE));
        ui.add_space(8.0);
    }
    ui.heading("Settings");
    ui.add_space(8.0);
    ui.label(RichText::new("REMOTE CONTROL").color(style::MUTED).strong());
    action.changed |= ui
        .checkbox(&mut settings.http_enabled, "Enable HTTP control")
        .changed();
    ui.horizontal(|ui| {
        ui.label("Bind address");
        action.changed |= ui.text_edit_singleline(&mut settings.http_addr).changed();
    });
    ui.horizontal(|ui| {
        ui.label("Token");
        action.changed |= ui
            .add(
                egui::TextEdit::singleline(&mut settings.http_token)
                    .desired_width(220.0)
                    .password(!settings.reveal_token),
            )
            .changed();
        action.changed |= ui.checkbox(&mut settings.reveal_token, "Show").changed();
        if ui.button("Copy").clicked() {
            ui.ctx().copy_text(settings.http_token.clone());
        }
        if ui.button("Regenerate").clicked() {
            settings.http_token = generate_token();
            action.changed = true;
            action.restart_requested = true;
        }
    });
    action.changed |= ui
        .checkbox(&mut settings.osc_enabled, "Enable OSC")
        .changed();
    ui.horizontal(|ui| {
        ui.label("OSC address");
        action.changed |= ui.text_edit_singleline(&mut settings.osc_addr).changed();
    });
    if action.changed {
        action.restart_requested = true;
    }
    ui.add_space(8.0);
    ui.label(RichText::new("MIDI INPUT").color(style::MUTED).strong());
    let (ports, port_error) = match lumen_control::midi_ports() {
        Ok(ports) => (ports, None),
        Err(error) => (Vec::new(), Some(error.to_string())),
    };
    let selected = settings.midi_port.as_deref().unwrap_or("Disabled");
    egui::ComboBox::from_id_salt("midi-input-port")
        .selected_text(selected)
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(settings.midi_port.is_none(), "Disabled")
                .clicked()
            {
                settings.midi_port = None;
                action.changed = true;
                action.restart_requested = true;
            }
            for port in &ports {
                if ui
                    .selectable_label(settings.midi_port.as_deref() == Some(port.as_str()), port)
                    .clicked()
                {
                    settings.midi_port = Some(port.clone());
                    action.changed = true;
                    action.restart_requested = true;
                }
            }
        });
    if let Some(error) = port_error {
        ui.label(RichText::new(format!("Couldn't list MIDI ports: {error}")).color(style::LIVE));
    } else if ports.is_empty() {
        ui.label(RichText::new("No MIDI input ports found").color(style::MUTED));
    }
    if let Some(mac) = project.macros.first() {
        if ui.button("Add note mapping").clicked() {
            settings.midi_mappings.push(lumen_control::MidiMapping {
                channel: 0,
                note: 60,
                command: lumen_control::ControlCommand::RunMacro {
                    name: mac.name.clone(),
                },
            });
            action.changed = true;
            action.restart_requested = true;
        }
    } else {
        ui.label(RichText::new("Create a macro before assigning MIDI notes").color(style::MUTED));
    }
    let mut remove_mapping = None;
    for (index, mapping) in settings.midi_mappings.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.label("Ch");
            action.changed |= ui
                .add(egui::DragValue::new(&mut mapping.channel).range(0..=15))
                .changed();
            ui.label("Note");
            action.changed |= ui
                .add(egui::DragValue::new(&mut mapping.note).range(0..=127))
                .changed();
            let selected_macro = match &mapping.command {
                lumen_control::ControlCommand::RunMacro { name } => name.as_str(),
                _ => "Choose macro",
            };
            egui::ComboBox::from_id_salt(("midi-command", index)).selected_text(selected_macro)
                .show_ui(ui, |ui| {
                    for mac in &project.macros {
                        if ui.selectable_label(matches!(&mapping.command, lumen_control::ControlCommand::RunMacro { name } if name == &mac.name), &mac.name).clicked() {
                            mapping.command = lumen_control::ControlCommand::RunMacro { name: mac.name.clone() };
                            action.changed = true;
                            action.restart_requested = true;
                        }
                    }
                });
            if ui.small_button("Remove").clicked() {
                remove_mapping = Some(index);
            }
        });
    }
    if let Some(index) = remove_mapping {
        settings.midi_mappings.remove(index);
        action.changed = true;
        action.restart_requested = true;
    }
    if action.changed {
        action.restart_requested = true;
    }
    ui.add_space(16.0);
    action.changed |= stage_options(ui, &mut settings.stage, project);
    if let Err(error) = settings.validate() {
        ui.add_space(6.0);
        ui.label(RichText::new(error).color(style::LIVE));
    }
    action
}

pub fn stage_options(ui: &mut Ui, stage: &mut StageOptions, project: &Project) -> bool {
    let mut changed = false;
    ui.label(RichText::new("STAGE OUTPUT").color(style::MUTED).strong());
    changed |= ui.checkbox(&mut stage.current, "Current cue").changed();
    changed |= ui.checkbox(&mut stage.next, "Next cue").changed();
    changed |= ui.checkbox(&mut stage.clock, "Clock").changed();
    let selected = stage
        .timer
        .and_then(|id| project.timers.iter().find(|t| t.id == id));
    if stage.timer.is_some() && selected.is_none() {
        stage.timer = None;
        changed = true;
    }
    egui::ComboBox::from_id_salt("stage-timer")
        .selected_text(
            selected
                .map(|timer| timer.name.as_str())
                .unwrap_or("No timer"),
        )
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(stage.timer.is_none(), "No timer")
                .clicked()
            {
                stage.timer = None;
                changed = true;
            }
            for timer in &project.timers {
                if ui
                    .selectable_label(stage.timer == Some(timer.id), &timer.name)
                    .clicked()
                {
                    stage.timer = Some(timer.id);
                    changed = true;
                }
            }
        });
    changed |= ui.checkbox(&mut stage.message, "Stage message").changed();
    changed |= ui.checkbox(&mut stage.notes, "Notes").changed();
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_settings_are_valid_and_token_is_url_safe() {
        let settings = Settings::default();
        assert!(settings.validate().is_ok());
        assert_eq!(settings.http_addr, "127.0.0.1:47800");
        assert_eq!(settings.osc_addr, "127.0.0.1:47801");
        assert_eq!(settings.http_token.len(), 24);
        assert!(
            settings
                .http_token
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        );
    }
    #[test]
    fn invalid_addresses_and_midi_values_are_rejected() {
        let settings = Settings {
            http_addr: "bad".into(),
            ..Default::default()
        };
        assert!(settings.validate().is_err());
    }
    #[test]
    fn missing_and_malformed_settings_choose_defaults_with_notice_only_for_malformed() {
        let dir = std::env::temp_dir().join(format!("lumen-settings-{}", Uuid::new_v4()));
        let missing = Settings::load(&dir.join("missing.json"));
        assert!(missing.notice.is_none());
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("bad.json"), b"not json").unwrap();
        let broken = Settings::load(&dir.join("bad.json"));
        assert!(broken.notice.is_some());
        assert!(broken.settings.validate().is_ok());
        let _ = fs::remove_dir_all(dir);
    }
    #[test]
    fn atomic_settings_round_trip() {
        let dir = std::env::temp_dir().join(format!("lumen-settings-{}", Uuid::new_v4()));
        let path = dir.join("settings.json");
        let settings = Settings::default();
        settings.save_atomic(&path).unwrap();
        assert_eq!(Settings::load(&path).settings, settings);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        assert!(!fs::read_to_string(&path).unwrap().contains("reveal_token"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn token_regeneration_produces_distinct_url_safe_values() {
        let first = generate_token();
        let second = generate_token();
        assert_ne!(first, second);
        assert_eq!(first.len(), 24);
        assert!(
            first
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        );
    }
}
