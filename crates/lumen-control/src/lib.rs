//! Lumen Remote Control Seam (`lumen-control`)
//!
//! Provides remote control protocol listeners for Lumen Presenter:
//! - HTTP JSON REST API (`HttpServer`)
//! - OSC UDP packet listener (`OscListener`)
//! - MIDI input note listener (`MidiListener`)
//!
//! All transports emit [`ControlEvent`] messages containing normalized [`ControlCommand`]s
//! to a standard `std::sync::mpsc::Sender<ControlEvent>`.

pub mod http;
pub mod midi;
pub mod osc;
pub mod types;

pub use http::{DEFAULT_BIND_ADDR, HttpConfig, HttpServer, MAX_BODY_SIZE, constant_time_eq_slices};
pub use midi::{MidiListener, MidiMapping, map_midi_message, midi_ports};
pub use osc::{MAX_OSC_PACKET_SIZE, OscListener, parse_osc_message};
pub use types::{ControlCommand, ControlError, ControlEvent, Source, StatusHandle, StatusSnapshot};
