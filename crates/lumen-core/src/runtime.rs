use std::time::Duration;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{CoreError, CueAction, Project, Slide, Theme, model::validate_actions};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LiveSlide {
    pub document_id: Uuid,
    pub document_name: String,
    pub slide: Slide,
    pub theme: Theme,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct OutputState {
    pub live: Option<LiveSlide>,
    pub blackout: bool,
    pub clear_text: bool,
    pub stage_message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Timer {
    pub id: Uuid,
    pub name: String,
    pub duration_secs: f64,
    pub remaining_secs: f64,
    pub running: bool,
}

impl Timer {
    pub fn new(name: impl Into<String>, duration_secs: f64) -> Self {
        let duration_secs = if duration_secs.is_finite() && duration_secs >= 0.0 {
            duration_secs
        } else {
            0.0
        };
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            duration_secs,
            remaining_secs: duration_secs,
            running: false,
        }
    }
    pub fn start(&mut self) {
        if self.remaining_secs > 0.0 {
            self.running = true;
        }
    }
    pub fn pause(&mut self) {
        self.running = false;
    }
    pub fn reset(&mut self) {
        self.remaining_secs = self.duration_secs;
        self.running = false;
    }
    pub fn tick(&mut self, elapsed: Duration) {
        if self.running {
            self.remaining_secs = (self.remaining_secs - elapsed.as_secs_f64()).max(0.0);
            if self.remaining_secs == 0.0 {
                self.running = false;
            }
        }
    }
    pub fn display(&self) -> String {
        let seconds = self.remaining_secs.max(0.0).ceil().min(u64::MAX as f64) as u64;
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;
        let seconds = seconds % 60;
        if hours > 0 {
            format!("{hours:02}:{minutes:02}:{seconds:02}")
        } else {
            format!("{minutes:02}:{seconds:02}")
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Runtime {
    pub output: OutputState,
    pub timers: Vec<Timer>,
}

impl Default for Runtime {
    fn default() -> Self {
        Self {
            output: OutputState::default(),
            timers: vec![Timer::new("Countdown", 300.0)],
        }
    }
}

impl Runtime {
    pub fn trigger(
        &mut self,
        project: &Project,
        document_id: Uuid,
        slide_id: Uuid,
    ) -> Result<(), CoreError> {
        let document = project
            .document(document_id)
            .ok_or(CoreError::MissingDocumentById(document_id))?;
        let slide = document
            .slides
            .iter()
            .find(|slide| slide.id == slide_id)
            .ok_or(CoreError::MissingSlide {
                document_id,
                slide_id,
            })?;
        let theme = project
            .theme(slide.theme_id)
            .ok_or(CoreError::MissingTheme {
                slide_id,
                theme_id: slide.theme_id,
            })?;
        validate_actions(&slide.actions, format!("slide {}", slide.id))?;
        self.validate_action_references(&slide.actions)?;

        let mut candidate = self.clone();
        candidate.output.live = Some(LiveSlide {
            document_id,
            document_name: document.name.clone(),
            slide: slide.clone(),
            theme: theme.clone(),
        });
        candidate.output.clear_text = false;
        candidate.apply_actions_unchecked(&slide.actions);
        *self = candidate;
        Ok(())
    }

    pub fn next(&mut self, project: &Project) -> Result<bool, CoreError> {
        self.navigate(project, true)
    }
    pub fn previous(&mut self, project: &Project) -> Result<bool, CoreError> {
        self.navigate(project, false)
    }

    fn navigate(&mut self, project: &Project, forward: bool) -> Result<bool, CoreError> {
        let cues = playlist_cues(project);
        if cues.is_empty() {
            return Ok(false);
        }
        let current = self.output.live.as_ref().and_then(|live| {
            cues.iter()
                .position(|(doc, slide)| *doc == live.document_id && *slide == live.slide.id)
        });
        let target = match current {
            None => {
                if forward {
                    0
                } else {
                    cues.len() - 1
                }
            }
            Some(index) if forward && index + 1 < cues.len() => index + 1,
            Some(index) if !forward && index > 0 => index - 1,
            Some(_) => return Ok(false),
        };
        let (document_id, slide_id) = cues[target];
        self.trigger(project, document_id, slide_id)?;
        Ok(true)
    }

    pub fn peek_next(&self, project: &Project) -> Option<LiveSlide> {
        let cues = playlist_cues(project);
        if cues.is_empty() {
            return None;
        }
        let current = self.output.live.as_ref().and_then(|live| {
            cues.iter()
                .position(|(doc, slide)| *doc == live.document_id && *slide == live.slide.id)
        });
        let target = match current {
            None => 0,
            Some(index) if index + 1 < cues.len() => index + 1,
            _ => return None,
        };
        let (document_id, slide_id) = cues[target];
        let document = project.document(document_id)?;
        let slide = document
            .slides
            .iter()
            .find(|slide| slide.id == slide_id)?
            .clone();
        let theme = project.theme(slide.theme_id)?.clone();
        Some(LiveSlide {
            document_id,
            document_name: document.name.clone(),
            slide,
            theme,
        })
    }

    pub fn apply_actions(&mut self, actions: &[CueAction]) -> Result<(), CoreError> {
        validate_actions(actions, "action batch".into())?;
        self.validate_action_references(actions)?;
        let mut candidate = self.clone();
        candidate.apply_actions_unchecked(actions);
        *self = candidate;
        Ok(())
    }

    pub fn run_macro(&mut self, project: &Project, id: Uuid) -> Result<(), CoreError> {
        let mac = project
            .macros
            .iter()
            .find(|mac| mac.id == id)
            .ok_or(CoreError::MissingMacro(id))?;
        self.apply_actions(&mac.actions)
    }

    pub fn tick(&mut self, elapsed: Duration) {
        for timer in &mut self.timers {
            timer.tick(elapsed);
        }
    }

    fn validate_action_references(&self, actions: &[CueAction]) -> Result<(), CoreError> {
        for action in actions {
            let id = match action {
                CueAction::StartTimer { id }
                | CueAction::PauseTimer { id }
                | CueAction::ResetTimer { id } => Some(*id),
                _ => None,
            };
            if let Some(id) = id
                && !self.timers.iter().any(|timer| timer.id == id)
            {
                return Err(CoreError::MissingTimer(id));
            }
        }
        Ok(())
    }

    fn apply_actions_unchecked(&mut self, actions: &[CueAction]) {
        for action in actions {
            match action {
                CueAction::Blackout(value) => self.output.blackout = *value,
                CueAction::ClearText(value) => self.output.clear_text = *value,
                CueAction::ClearAll => {
                    self.output.live = None;
                    self.output.clear_text = false;
                }
                CueAction::StageMessage(message) => self.output.stage_message = message.clone(),
                CueAction::StartTimer { id } => {
                    if let Some(timer) = self.timers.iter_mut().find(|timer| timer.id == *id) {
                        timer.start()
                    }
                }
                CueAction::PauseTimer { id } => {
                    if let Some(timer) = self.timers.iter_mut().find(|timer| timer.id == *id) {
                        timer.pause()
                    }
                }
                CueAction::ResetTimer { id } => {
                    if let Some(timer) = self.timers.iter_mut().find(|timer| timer.id == *id) {
                        timer.reset()
                    }
                }
            }
        }
    }
}

// Playlist entries intentionally expand in order, including repeated documents.
// If live is repeated, navigation anchors to its first matching occurrence.
fn playlist_cues(project: &Project) -> Vec<(Uuid, Uuid)> {
    project
        .playlist
        .iter()
        .filter_map(|item| project.document(item.document_id))
        .flat_map(|doc| doc.slides.iter().map(move |slide| (doc.id, slide.id)))
        .collect()
}
