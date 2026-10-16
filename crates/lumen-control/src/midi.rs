use serde::{Deserialize, Serialize};

use crate::types::{ControlCommand, ControlError, ControlEvent, Source};

/// Mapping from a MIDI Note On message on a specific channel to a [`ControlCommand`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MidiMapping {
    /// MIDI channel (0-15).
    pub channel: u8,
    /// MIDI note number (0-127).
    pub note: u8,
    /// Command triggered by this note.
    pub command: ControlCommand,
}

/// Enumerates available MIDI input port names on the host system.
pub fn midi_ports() -> Result<Vec<String>, ControlError> {
    let midi_in =
        midir::MidiInput::new("lumen-control").map_err(|e| ControlError::Midi(e.to_string()))?;
    let ports = midi_in.ports();
    let mut names = Vec::with_capacity(ports.len());
    for port in ports {
        names.push(
            midi_in
                .port_name(&port)
                .map_err(|e| ControlError::Midi(e.to_string()))?,
        );
    }
    Ok(names)
}

/// Pure function to map raw MIDI bytes to a [`ControlCommand`].
///
/// Matches standard Note On events (`0x90..=0x9F`) where `velocity > 0`.
/// Note On with `velocity == 0` is treated as Note Off in MIDI 1.0 and ignored.
pub fn map_midi_message(msg: &[u8], mappings: &[MidiMapping]) -> Option<ControlCommand> {
    if msg.len() < 3 {
        return None;
    }

    let status = msg[0];
    // Note On status byte mask: 0x90 | channel
    if (status & 0xF0) != 0x90 {
        return None;
    }

    let channel = status & 0x0F;
    let note = msg[1];
    let velocity = msg[2];

    if velocity == 0 {
        return None;
    }

    for mapping in mappings {
        if mapping.channel == channel && mapping.note == note {
            return Some(mapping.command.clone());
        }
    }

    None
}

/// MIDI input listener attached to a hardware or virtual MIDI port.
pub struct MidiListener {
    _connection: midir::MidiInputConnection<()>,
}

impl MidiListener {
    /// Connects to a MIDI port by name and forwards mapped commands to `tx`.
    pub fn start(
        port_name: &str,
        map: Vec<MidiMapping>,
        tx: std::sync::mpsc::Sender<ControlEvent>,
    ) -> Result<Self, ControlError> {
        let midi_in = midir::MidiInput::new("lumen-control")
            .map_err(|e| ControlError::Midi(e.to_string()))?;

        let ports = midi_in.ports();
        let mut target_port = None;
        for p in ports {
            if midi_in.port_name(&p).is_ok_and(|name| name == port_name) {
                target_port = Some(p);
                break;
            }
        }

        let port = target_port.ok_or_else(|| ControlError::PortNotFound(port_name.to_string()))?;

        let connection = midi_in
            .connect(
                &port,
                "lumen-control-in",
                move |_timestamp, message, _| {
                    if let Some(cmd) = map_midi_message(message, &map) {
                        let _ = tx.send(ControlEvent {
                            source: Source::Midi,
                            command: cmd,
                        });
                    }
                },
                (),
            )
            .map_err(|e| ControlError::Midi(e.to_string()))?;

        Ok(Self {
            _connection: connection,
        })
    }
}
