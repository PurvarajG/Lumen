use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    CoreError, CueAction, Project, Slide, Theme, TimerDef, TimerKind, model::validate_actions,
};

// ---------------------------------------------------------------------------
// LiveSlide
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LiveSlide {
    pub document_id: Uuid,
    pub document_name: String,
    pub slide: Slide,
    pub theme: Theme,
}

// ---------------------------------------------------------------------------
// OutputState
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct OutputState {
    pub live: Option<LiveSlide>,
    pub blackout: bool,
    pub clear_text: bool,
    pub stage_message: String,
    #[serde(default)]
    pub props: Vec<Uuid>,
    #[serde(default)]
    pub message: Option<Uuid>,
    #[serde(default)]
    pub stop_audio_serial: u64,
}

// ---------------------------------------------------------------------------
// TimerState (runtime state per TimerDef)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TimerState {
    pub running: bool,
    pub value_secs: f64,
}

impl Default for TimerState {
    fn default() -> Self {
        Self {
            running: false,
            value_secs: 0.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Runtime
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Runtime {
    pub output: OutputState,
    pub timer_states: HashMap<Uuid, TimerState>,
}

impl Runtime {
    /// Synchronize timer runtime states with project timer definitions.
    /// Adds state for new defs (with initial values), drops removed ones.
    pub fn sync_timers(&mut self, project: &Project) {
        // Remove states for timers that no longer exist in the project
        let timer_ids: std::collections::HashSet<Uuid> =
            project.timers.iter().map(|t| t.id).collect();
        self.timer_states.retain(|id, _| timer_ids.contains(id));

        // Add states for new timers
        for def in &project.timers {
            self.timer_states.entry(def.id).or_insert_with(|| {
                let value = match &def.kind {
                    TimerKind::Countdown { duration_secs } => *duration_secs,
                    TimerKind::CountdownTo { .. } => 0.0, // derived from wall clock
                    TimerKind::Elapsed => 0.0,
                };
                TimerState {
                    running: false,
                    value_secs: value,
                }
            });
        }
    }

    /// Advance running timers by `elapsed`.
    pub fn tick(&mut self, project: &Project, elapsed: Duration) {
        self.sync_timers(project);
        let dt = elapsed.as_secs_f64();
        for def in &project.timers {
            if let Some(state) = self.timer_states.get_mut(&def.id) {
                if !state.running {
                    continue;
                }
                match &def.kind {
                    TimerKind::Countdown { .. } => {
                        state.value_secs -= dt;
                        if state.value_secs <= 0.0 && !def.allow_overrun {
                            state.value_secs = 0.0;
                            state.running = false;
                        }
                    }
                    TimerKind::CountdownTo { .. } => {
                        // CountdownTo is derived from wall clock, not tick
                        // value_secs is updated by timer_display, tick is a no-op
                    }
                    TimerKind::Elapsed => {
                        state.value_secs += dt;
                    }
                }
            }
        }
    }

    /// Display a timer as `MM:SS` or `HH:MM:SS`, prefixed `-` when overrun.
    pub fn timer_display(&self, def: &TimerDef, now_secs_of_day: u32) -> String {
        match &def.kind {
            TimerKind::Countdown { .. } => {
                let value = self
                    .timer_states
                    .get(&def.id)
                    .map(|s| s.value_secs)
                    .unwrap_or(0.0);
                format_timer_value(value)
            }
            TimerKind::CountdownTo {
                hour,
                minute,
                second,
            } => {
                let target = *hour as i64 * 3600 + *minute as i64 * 60 + *second as i64;
                let now = now_secs_of_day as i64;
                let mut remaining = target - now;
                if remaining < 0 {
                    if def.allow_overrun {
                        // negative (overrun)
                    } else {
                        remaining = 0;
                    }
                }
                format_timer_value(remaining as f64)
            }
            TimerKind::Elapsed => {
                let value = self
                    .timer_states
                    .get(&def.id)
                    .map(|s| s.value_secs)
                    .unwrap_or(0.0);
                format_timer_value(value)
            }
        }
    }

    /// Render a message template, replacing `{timer:<uuid>}` tokens with display values.
    pub fn render_message(
        &self,
        project: &Project,
        id: Uuid,
        now_secs_of_day: u32,
    ) -> Option<String> {
        let msg = project.message(id)?;
        let mut result = String::with_capacity(msg.text.len());
        let mut rest = msg.text.as_str();

        // Replace {timer:<uuid>} tokens; unknown or malformed tokens stay literal.
        while let Some(start) = rest.find("{timer:") {
            result.push_str(&rest[..start]);
            let token = &rest[start..];
            let Some(end) = token.find('}') else {
                rest = token;
                break;
            };
            let display = Uuid::parse_str(&token[7..end])
                .ok()
                .and_then(|id| project.timer_def(id))
                .map(|def| self.timer_display(def, now_secs_of_day));
            result.push_str(display.as_deref().unwrap_or(&token[..=end]));
            rest = &token[end + 1..];
        }
        result.push_str(rest);

        Some(result)
    }

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
        self.validate_action_references(project, &slide.actions)?;

        let mut candidate = self.clone();
        candidate.sync_timers(project);
        candidate.output.live = Some(LiveSlide {
            document_id,
            document_name: document.name.clone(),
            slide: slide.clone(),
            theme: theme.clone(),
        });
        candidate.output.clear_text = false;
        candidate.apply_actions_unchecked(project, &slide.actions);
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

    pub fn apply_actions(
        &mut self,
        project: &Project,
        actions: &[CueAction],
    ) -> Result<(), CoreError> {
        validate_actions(actions, "action batch".into())?;
        self.validate_action_references(project, actions)?;
        let mut candidate = self.clone();
        candidate.sync_timers(project);
        candidate.apply_actions_unchecked(project, actions);
        *self = candidate;
        Ok(())
    }

    pub fn run_macro(&mut self, project: &Project, id: Uuid) -> Result<(), CoreError> {
        let mac = project
            .macros
            .iter()
            .find(|mac| mac.id == id)
            .ok_or(CoreError::MissingMacro(id))?;
        let actions = mac.actions.clone();
        self.apply_actions(project, &actions)
    }

    fn validate_action_references(
        &self,
        project: &Project,
        actions: &[CueAction],
    ) -> Result<(), CoreError> {
        for action in actions {
            match action {
                CueAction::StartTimer { id }
                | CueAction::PauseTimer { id }
                | CueAction::ResetTimer { id } => {
                    if project.timer_def(*id).is_none() {
                        return Err(CoreError::MissingTimer(*id));
                    }
                }
                CueAction::TriggerProp { id } => {
                    if project.prop(*id).is_none() {
                        return Err(CoreError::MissingProp(*id));
                    }
                }
                CueAction::ShowMessage { id } if project.message(*id).is_none() => {
                    return Err(CoreError::MissingMessage(*id));
                }
                CueAction::ShowMessage { .. } => {}
                _ => {}
            }
        }
        Ok(())
    }

    fn apply_actions_unchecked(&mut self, project: &Project, actions: &[CueAction]) {
        for action in actions {
            match action {
                CueAction::Blackout(value) => self.output.blackout = *value,
                CueAction::ClearText(value) => self.output.clear_text = *value,
                CueAction::ClearAll => {
                    self.output.live = None;
                    self.output.clear_text = false;
                    self.output.props.clear();
                    self.output.message = None;
                }
                CueAction::StageMessage(message) => self.output.stage_message = message.clone(),
                CueAction::StartTimer { id } => {
                    self.sync_timers(project);
                    if let Some(state) = self.timer_states.get_mut(id) {
                        // For Countdown: only start if value > 0
                        if let Some(def) = project.timer_def(*id) {
                            match &def.kind {
                                TimerKind::Countdown { .. } => {
                                    if state.value_secs > 0.0 || def.allow_overrun {
                                        state.running = true;
                                    }
                                }
                                _ => {
                                    state.running = true;
                                }
                            }
                        }
                    }
                }
                CueAction::PauseTimer { id } => {
                    if let Some(state) = self.timer_states.get_mut(id) {
                        state.running = false;
                    }
                }
                CueAction::ResetTimer { id } => {
                    if let Some(def) = project.timer_def(*id)
                        && let Some(state) = self.timer_states.get_mut(id)
                    {
                        state.running = false;
                        match &def.kind {
                            TimerKind::Countdown { duration_secs } => {
                                state.value_secs = *duration_secs;
                            }
                            TimerKind::Elapsed => {
                                state.value_secs = 0.0;
                            }
                            TimerKind::CountdownTo { .. } => {
                                // CountdownTo resets to wall-clock derived
                                state.value_secs = 0.0;
                            }
                        }
                    }
                }
                CueAction::TriggerProp { id } => {
                    if !self.output.props.contains(id) {
                        self.output.props.push(*id);
                    }
                }
                CueAction::ClearProps => {
                    self.output.props.clear();
                }
                CueAction::ShowMessage { id } => {
                    self.output.message = Some(*id);
                }
                CueAction::ClearMessage => {
                    self.output.message = None;
                }
                CueAction::StopAudio => {
                    self.output.stop_audio_serial += 1;
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Timer display helper
// ---------------------------------------------------------------------------

fn format_timer_value(value: f64) -> String {
    let negative = value < 0.0;
    let abs = value.abs();
    let total_secs = abs.ceil().min(u64::MAX as f64) as u64;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    let prefix = if negative { "-" } else { "" };
    if hours > 0 {
        format!("{prefix}{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{prefix}{minutes:02}:{seconds:02}")
    }
}

// ---------------------------------------------------------------------------
// Playlist helpers
// ---------------------------------------------------------------------------

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
