use crate::settings::Settings;
use lumen_control::{
    ControlEvent, HttpConfig, HttpServer, MidiListener, OscListener, StatusHandle,
};
use std::sync::mpsc::{self, Receiver, Sender};

/// Owns the enabled remote listeners and their command queue.
pub struct RemoteManager {
    http: Option<HttpServer>,
    osc: Option<OscListener>,
    midi: Option<MidiListener>,
    tx: Sender<ControlEvent>,
    rx: Receiver<ControlEvent>,
}

impl RemoteManager {
    pub fn start(settings: &Settings, status: StatusHandle) -> Result<Self, String> {
        settings.validate()?;
        let (tx, rx) = mpsc::channel();
        let mut manager = Self {
            http: None,
            osc: None,
            midi: None,
            tx,
            rx,
        };
        manager.configure(settings, status)?;
        Ok(manager)
    }

    fn configure(&mut self, settings: &Settings, status: StatusHandle) -> Result<(), String> {
        if settings.http_enabled {
            let bind = settings
                .http_addr
                .parse()
                .map_err(|e| format!("Invalid HTTP bind address: {e}"))?;
            self.http = Some(
                HttpServer::start(
                    HttpConfig::new(bind, settings.http_token.clone()),
                    self.tx.clone(),
                    status.clone(),
                )
                .map_err(|e| format!("Couldn't start HTTP control: {e}"))?,
            );
        }
        if settings.osc_enabled {
            let bind = settings
                .osc_addr
                .parse()
                .map_err(|e| format!("Invalid OSC bind address: {e}"))?;
            self.osc = Some(
                OscListener::start(bind, self.tx.clone())
                    .map_err(|e| format!("Couldn't start OSC: {e}"))?,
            );
        }
        if let Some(port) = settings.midi_port.as_deref() {
            self.midi = Some(
                MidiListener::start(port, settings.midi_mappings.clone(), self.tx.clone())
                    .map_err(|e| format!("Couldn't connect MIDI input: {e}"))?,
            );
        }
        Ok(())
    }

    /// Replaces listeners using the updated settings. On failure, all newly requested
    /// listeners are stopped and the manager remains safely offline.
    pub fn restart(&mut self, settings: &Settings, status: StatusHandle) -> Result<(), String> {
        self.stop();
        settings.validate()?;
        if let Err(error) = self.configure(settings, status) {
            self.stop();
            Err(error)
        } else {
            Ok(())
        }
    }

    pub fn stop(&mut self) {
        self.midi.take();
        if let Some(mut server) = self.http.take() {
            server.shutdown();
        }
        if let Some(mut listener) = self.osc.take() {
            listener.shutdown();
        }
    }

    pub fn drain_commands(&self) -> Vec<ControlEvent> {
        self.rx.try_iter().collect()
    }

    pub fn is_running(&self) -> bool {
        self.http.is_some() || self.osc.is_some() || self.midi.is_some()
    }
}

impl Drop for RemoteManager {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lumen_control::{ControlCommand, Source};

    #[test]
    fn disabled_remote_defaults_start_offline() {
        let manager = RemoteManager::start(&Settings::default(), StatusHandle::default()).unwrap();
        assert!(!manager.is_running());
        assert!(manager.drain_commands().is_empty());
    }

    #[test]
    fn command_queue_drains_events_in_order() {
        let manager = RemoteManager::start(&Settings::default(), StatusHandle::default()).unwrap();
        manager
            .tx
            .send(ControlEvent {
                source: Source::Osc,
                command: ControlCommand::Next,
            })
            .unwrap();
        manager
            .tx
            .send(ControlEvent {
                source: Source::Midi,
                command: ControlCommand::Previous,
            })
            .unwrap();
        let events = manager.drain_commands();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].command, ControlCommand::Next);
        assert_eq!(events[1].command, ControlCommand::Previous);
    }
}
