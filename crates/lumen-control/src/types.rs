use std::sync::{Arc, Mutex};

use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ControlError {
    #[error("HTTP auth token must be at least 16 characters (got {0})")]
    TokenTooShort(usize),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("OSC error: {0}")]
    Osc(String),

    #[error("MIDI error: {0}")]
    Midi(String),

    #[error("MIDI port not found: {0}")]
    PortNotFound(String),

    #[error("Malformed command: {0}")]
    MalformedCommand(String),

    #[error("Payload too large: {0} bytes (max 4096 bytes)")]
    PayloadTooLarge(usize),
}

/// Commands accepted by the Lumen remote control seam.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "cmd")]
pub enum ControlCommand {
    Next,
    Previous,
    Trigger {
        index: usize,
    },
    Blackout {
        on: bool,
    },
    ClearText,
    ClearAll,
    RunMacro {
        name: String,
    },
    TimerStart {
        name: String,
    },
    TimerPause {
        name: String,
    },
    TimerReset {
        name: String,
    },
    StageMessage {
        #[serde(deserialize_with = "deserialize_stage_message_text")]
        text: String,
    },
}

fn deserialize_stage_message_text<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if s.chars().count() > 500 {
        return Err(serde::de::Error::custom(
            "stage message text exceeds maximum length of 500 characters",
        ));
    }
    Ok(s)
}

impl ControlCommand {
    /// Validates internal constraints of the command (e.g. stage message text length <= 500 chars).
    pub fn validate(&self) -> Result<(), ControlError> {
        match self {
            ControlCommand::StageMessage { text } if text.chars().count() > 500 => {
                Err(ControlError::MalformedCommand(
                    "stage message text exceeds 500 characters".to_string(),
                ))
            }
            _ => Ok(()),
        }
    }
}

/// Snapshot of the live output state published to remote control clients.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusSnapshot {
    pub project: String,
    pub live_index: Option<usize>,
    pub live_text: String,
    pub next_text: String,
    pub blackout: bool,
    pub timers: Vec<(String, String)>,
}

/// Thread-safe handle for publishing and reading the live `StatusSnapshot`.
#[derive(Clone, Debug, Default)]
pub struct StatusHandle {
    inner: Arc<Mutex<StatusSnapshot>>,
}

impl StatusHandle {
    pub fn new(snapshot: StatusSnapshot) -> Self {
        Self {
            inner: Arc::new(Mutex::new(snapshot)),
        }
    }

    pub fn set(&self, snapshot: StatusSnapshot) {
        match self.inner.lock() {
            Ok(mut guard) => *guard = snapshot,
            Err(poisoned) => *poisoned.into_inner() = snapshot,
        }
    }

    pub fn get(&self) -> StatusSnapshot {
        match self.inner.lock() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }
}

impl From<StatusSnapshot> for StatusHandle {
    fn from(snapshot: StatusSnapshot) -> Self {
        Self::new(snapshot)
    }
}

/// Source protocol of an incoming remote control event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Source {
    Http,
    Osc,
    Midi,
}

/// Event emitted when a control command is received from any transport.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlEvent {
    pub source: Source,
    pub command: ControlCommand,
}
