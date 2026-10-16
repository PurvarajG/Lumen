use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::thread::JoinHandle;
use std::time::Duration;

use rosc::{OscMessage, OscPacket, OscType};

use crate::types::{ControlCommand, ControlError, ControlEvent, Source};

/// Maximum allowed OSC packet size (64 KiB = 65536 bytes).
pub const MAX_OSC_PACKET_SIZE: usize = 65536;

/// OSC UDP listener for remote control commands.
pub struct OscListener {
    local_addr: SocketAddr,
    running: Arc<AtomicBool>,
    dropped: Arc<AtomicU64>,
    thread: Option<JoinHandle<()>>,
}

impl OscListener {
    /// Starts an OSC listener bound to `bind` on a background UDP thread.
    pub fn start(bind: SocketAddr, tx: Sender<ControlEvent>) -> Result<Self, ControlError> {
        let socket = UdpSocket::bind(bind).map_err(|e| ControlError::Osc(e.to_string()))?;
        socket
            .set_read_timeout(Some(Duration::from_millis(50)))
            .map_err(|e| ControlError::Osc(e.to_string()))?;
        let local_addr = socket
            .local_addr()
            .map_err(|e| ControlError::Osc(e.to_string()))?;

        let running = Arc::new(AtomicBool::new(true));
        let dropped = Arc::new(AtomicU64::new(0));

        let run = Arc::clone(&running);
        let drop_counter = Arc::clone(&dropped);

        let thread = std::thread::spawn(move || {
            // Buffer sized to receive packets up to and slightly beyond 64 KiB
            let mut buf = vec![0u8; MAX_OSC_PACKET_SIZE + 1];
            while run.load(Ordering::SeqCst) {
                match socket.recv_from(&mut buf) {
                    Ok((len, _src)) => {
                        if !run.load(Ordering::SeqCst) {
                            break;
                        }
                        if len == 0 {
                            continue;
                        }
                        if len > MAX_OSC_PACKET_SIZE {
                            drop_counter.fetch_add(1, Ordering::SeqCst);
                            continue;
                        }

                        match rosc::decoder::decode_udp(&buf[..len]) {
                            Ok((_rem, packet)) => {
                                let mut messages = Vec::new();
                                flatten_packet(packet, &mut messages);
                                for msg in messages {
                                    if let Some(cmd) = parse_osc_message(msg) {
                                        let _ = tx.send(ControlEvent {
                                            source: Source::Osc,
                                            command: cmd,
                                        });
                                    } else {
                                        drop_counter.fetch_add(1, Ordering::SeqCst);
                                    }
                                }
                            }
                            Err(_) => {
                                drop_counter.fetch_add(1, Ordering::SeqCst);
                            }
                        }
                    }
                    Err(ref e)
                        if e.kind() == std::io::ErrorKind::WouldBlock
                            || e.kind() == std::io::ErrorKind::TimedOut =>
                    {
                        continue;
                    }
                    Err(_) => {
                        if !run.load(Ordering::SeqCst) {
                            break;
                        }
                    }
                }
            }
        });

        Ok(Self {
            local_addr,
            running,
            dropped,
            thread: Some(thread),
        })
    }

    /// Returns the local address this listener is bound to.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Total count of dropped unknown or malformed OSC packets.
    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::SeqCst)
    }

    /// Stops the listener thread and joins cleanly.
    pub fn shutdown(&mut self) {
        if self.running.swap(false, Ordering::SeqCst) {
            // Send empty wake-up packet
            if let Ok(dummy) = UdpSocket::bind("127.0.0.1:0") {
                let _ = dummy.send_to(&[], self.local_addr);
            }
            if let Some(handle) = self.thread.take() {
                let _ = handle.join();
            }
        }
    }
}

impl Drop for OscListener {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn flatten_packet(packet: OscPacket, messages: &mut Vec<OscMessage>) {
    match packet {
        OscPacket::Message(msg) => messages.push(msg),
        OscPacket::Bundle(bundle) => {
            for inner in bundle.content {
                flatten_packet(inner, messages);
            }
        }
    }
}

pub fn parse_osc_message(msg: OscMessage) -> Option<ControlCommand> {
    match msg.addr.as_str() {
        "/lumen/next" => {
            if msg.args.is_empty() {
                Some(ControlCommand::Next)
            } else {
                None
            }
        }
        "/lumen/previous" => {
            if msg.args.is_empty() {
                Some(ControlCommand::Previous)
            } else {
                None
            }
        }
        "/lumen/trigger" => {
            if msg.args.len() == 1 {
                match msg.args[0] {
                    OscType::Int(val) if val >= 0 => Some(ControlCommand::Trigger {
                        index: val as usize,
                    }),
                    _ => None,
                }
            } else {
                None
            }
        }
        "/lumen/blackout" => {
            if msg.args.len() == 1 {
                match msg.args[0] {
                    OscType::Int(0) => Some(ControlCommand::Blackout { on: false }),
                    OscType::Int(1) => Some(ControlCommand::Blackout { on: true }),
                    OscType::Bool(b) => Some(ControlCommand::Blackout { on: b }),
                    _ => None,
                }
            } else {
                None
            }
        }
        "/lumen/clear/text" => {
            if msg.args.is_empty() {
                Some(ControlCommand::ClearText)
            } else {
                None
            }
        }
        "/lumen/clear/all" => {
            if msg.args.is_empty() {
                Some(ControlCommand::ClearAll)
            } else {
                None
            }
        }
        "/lumen/macro" => {
            if msg.args.len() == 1 {
                match msg.args.into_iter().next() {
                    Some(OscType::String(name)) => Some(ControlCommand::RunMacro { name }),
                    _ => None,
                }
            } else {
                None
            }
        }
        "/lumen/timer/start" => {
            if msg.args.len() == 1 {
                match msg.args.into_iter().next() {
                    Some(OscType::String(name)) => Some(ControlCommand::TimerStart { name }),
                    _ => None,
                }
            } else {
                None
            }
        }
        "/lumen/timer/pause" => {
            if msg.args.len() == 1 {
                match msg.args.into_iter().next() {
                    Some(OscType::String(name)) => Some(ControlCommand::TimerPause { name }),
                    _ => None,
                }
            } else {
                None
            }
        }
        "/lumen/timer/reset" => {
            if msg.args.len() == 1 {
                match msg.args.into_iter().next() {
                    Some(OscType::String(name)) => Some(ControlCommand::TimerReset { name }),
                    _ => None,
                }
            } else {
                None
            }
        }
        "/lumen/message" => {
            if msg.args.len() == 1 {
                match msg.args.into_iter().next() {
                    Some(OscType::String(text)) if text.chars().count() <= 500 => {
                        Some(ControlCommand::StageMessage { text })
                    }
                    _ => None,
                }
            } else {
                None
            }
        }
        _ => None,
    }
}
