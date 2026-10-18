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
    #[serde(default)]
    pub deck: Option<std::path::PathBuf>,
    pub live: Option<LiveSlide>,
    #[serde(default)]
    pub underlay: Option<crate::Background>,
    pub blackout: bool,
    pub clear_text: bool,
    pub stage_message: String,
    #[serde(default)]
    pub props: Vec<Uuid>,
    /// Trigger-time copies of prop definitions. `props` remains the stable ID API.
    #[serde(default)]
    pub prop_snapshots: Vec<crate::Prop>,
    /// Trigger-time styles aligned with `props` and `prop_snapshots`.
    #[serde(default)]
    pub prop_theme_snapshots: Vec<Theme>,
    #[serde(default)]
    pub message: Option<Uuid>,
    /// Trigger-time message template; timer tokens are resolved when rendered.
    #[serde(default)]
    pub message_snapshot: Option<String>,
    #[serde(default)]
    pub stop_audio_serial: u64,
    #[serde(default)]
    pub audio_clear_serial: u64,
    #[serde(default)]
    pub media_cleared: bool,
}

impl OutputState {
    pub fn effective_background(&self) -> Option<&crate::Background> {
        let live = self.live.as_ref()?;
        if live.slide.background.is_transparent() {
            self.underlay.as_ref()
        } else {
            Some(&live.slide.background)
        }
    }
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
    /// Index into the playlist cues of the live cue, so a repeated document stays distinct.
    #[serde(default)]
    pub cursor: Option<usize>,
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
        Some(self.render_message_template(project, &msg.text, now_secs_of_day))
    }

    /// Render the currently triggered message snapshot, resolving timer tokens live.
    pub fn render_output_message(&self, project: &Project, now_secs_of_day: u32) -> Option<String> {
        let template = self.output.message_snapshot.as_deref()?;
        Some(self.render_message_template(project, template, now_secs_of_day))
    }

    fn render_message_template(
        &self,
        project: &Project,
        template: &str,
        now_secs_of_day: u32,
    ) -> String {
        let mut result = String::with_capacity(template.len());
        let mut rest = template;

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

        result
    }

    /// Trigger a cue by document and slide. A repeated document resolves to the first
    /// occurrence at or after the live cue (wrapping), so the live copy keeps its place.
    pub fn trigger(
        &mut self,
        project: &Project,
        document_id: Uuid,
        slide_id: Uuid,
    ) -> Result<(), CoreError> {
        let cues = playlist_cues(project);
        let start = self.current_index(&cues).unwrap_or(0);
        let cursor = (start..cues.len())
            .chain(0..start)
            .find(|&i| cues[i] == (document_id, slide_id));
        self.trigger_cue(project, document_id, slide_id, cursor)
    }

    /// Trigger the exact playlist cue at `index`. Returns `Ok(false)` when out of range.
    pub fn trigger_index(&mut self, project: &Project, index: usize) -> Result<bool, CoreError> {
        let Some(&(document_id, slide_id)) = playlist_cues(project).get(index) else {
            return Ok(false);
        };
        self.trigger_cue(project, document_id, slide_id, Some(index))?;
        Ok(true)
    }

    /// Index of the live cue among all running-order cues (the exact occurrence).
    pub fn live_cue_index(&self, project: &Project) -> Option<usize> {
        self.current_index(&playlist_cues(project))
    }

    /// Playlist row (item index) holding the live cue.
    pub fn live_item_index(&self, project: &Project) -> Option<usize> {
        let cues = playlist_cues(project);
        cue_row(project, self.current_index(&cues)?)
    }

    /// Playlist row holding the cue `peek_next` would return.
    pub fn next_item_index(&self, project: &Project) -> Option<usize> {
        let cues = playlist_cues(project);
        cue_row(project, self.next_index(&cues)?)
    }

    fn current_index(&self, cues: &[(Uuid, Uuid)]) -> Option<usize> {
        let live = self.output.live.as_ref()?;
        let live = (live.document_id, live.slide.id);
        match self.cursor {
            Some(i) if cues.get(i) == Some(&live) => Some(i),
            _ => cues.iter().position(|cue| *cue == live),
        }
    }

    fn next_index(&self, cues: &[(Uuid, Uuid)]) -> Option<usize> {
        match self.current_index(cues) {
            None => (!cues.is_empty()).then_some(0),
            Some(i) => (i + 1 < cues.len()).then_some(i + 1),
        }
    }

    fn trigger_cue(
        &mut self,
        project: &Project,
        document_id: Uuid,
        slide_id: Uuid,
        cursor: Option<usize>,
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
        candidate.output.underlay =
            if slide.background.is_transparent() && !self.output.media_cleared {
                self.output.effective_background().cloned()
            } else {
                None
            };
        candidate.output.live = Some(LiveSlide {
            document_id,
            document_name: document.name.clone(),
            slide: slide.clone(),
            theme: theme.clone(),
        });
        candidate.output.clear_text = false;
        candidate.output.media_cleared = false;
        candidate.cursor = cursor;
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
        let target = match self.current_index(&cues) {
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
        self.trigger_cue(project, document_id, slide_id, Some(target))?;
        Ok(true)
    }

    pub fn peek_next(&self, project: &Project) -> Option<LiveSlide> {
        let cues = playlist_cues(project);
        let (document_id, slide_id) = cues[self.next_index(&cues)?];
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

    fn normalize_prop_snapshots(&mut self, project: &Project) {
        let old_props = self.output.prop_snapshots.clone();
        let old_themes = self.output.prop_theme_snapshots.clone();
        let fallback_theme = self
            .output
            .live
            .as_ref()
            .map(|live| live.theme.clone())
            .or_else(|| project.themes.first().cloned())
            .unwrap_or_default();
        self.output.prop_snapshots = self
            .output
            .props
            .iter()
            .map(|id| {
                old_props
                    .iter()
                    .find(|snapshot| snapshot.id == *id)
                    .cloned()
                    .or_else(|| project.prop(*id).cloned())
                    .unwrap_or_else(|| crate::Prop {
                        id: *id,
                        name: String::new(),
                        objects: Vec::new(),
                    })
            })
            .collect();
        self.output.prop_theme_snapshots = self
            .output
            .prop_snapshots
            .iter()
            .map(|snapshot| {
                old_props
                    .iter()
                    .position(|old| old.id == snapshot.id)
                    .and_then(|index| old_themes.get(index).cloned())
                    .unwrap_or_else(|| fallback_theme.clone())
            })
            .collect();
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
                CueAction::TriggerProp { id } | CueAction::HideProp { id } => {
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
                CueAction::PlayDeck { path } => self.output.deck = Some(path.clone()),
                CueAction::StopDeck => self.output.deck = None,
                CueAction::ClearSlide => {
                    // Like an opaque slide's own background, a caption's underlay stays visible.
                    self.output.deck = None;
                    self.output.clear_text = true;
                }
                CueAction::ClearMedia => self.output.media_cleared = true,
                CueAction::ClearAll => {
                    self.output.deck = None;
                    self.output.live = None;
                    self.output.underlay = None;
                    self.output.clear_text = false;
                    self.output.props.clear();
                    self.output.prop_snapshots.clear();
                    self.output.prop_theme_snapshots.clear();
                    self.output.message = None;
                    self.output.message_snapshot = None;
                    self.output.media_cleared = true;
                    self.output.audio_clear_serial = self.output.audio_clear_serial.wrapping_add(1);
                    self.output.stop_audio_serial = self.output.stop_audio_serial.wrapping_add(1);
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
                CueAction::HideProp { id } => {
                    self.normalize_prop_snapshots(project);
                    if let Some(index) = self.output.props.iter().position(|active| active == id) {
                        self.output.props.remove(index);
                        self.output.prop_snapshots.remove(index);
                        self.output.prop_theme_snapshots.remove(index);
                    }
                }
                CueAction::TriggerProp { id } => {
                    if let Some(prop) = project.prop(*id) {
                        self.normalize_prop_snapshots(project);
                        if let Some(index) =
                            self.output.props.iter().position(|active| active == id)
                        {
                            // Retrigger refreshes an already-live prop in place.
                            self.output.prop_snapshots[index] = prop.clone();
                            let theme = self
                                .output
                                .live
                                .as_ref()
                                .map(|live| live.theme.clone())
                                .or_else(|| project.themes.first().cloned())
                                .unwrap_or_default();
                            self.output.prop_theme_snapshots[index] = theme;
                        } else {
                            self.output.props.push(*id);
                            self.output.prop_snapshots.push(prop.clone());
                            self.output.prop_theme_snapshots.push(
                                self.output
                                    .live
                                    .as_ref()
                                    .map(|live| live.theme.clone())
                                    .or_else(|| project.themes.first().cloned())
                                    .unwrap_or_default(),
                            );
                        }
                    }
                }
                CueAction::ClearProps => {
                    self.output.props.clear();
                    self.output.prop_snapshots.clear();
                    self.output.prop_theme_snapshots.clear();
                }
                CueAction::ShowMessage { id } => {
                    self.output.message = Some(*id);
                    self.output.message_snapshot =
                        project.message(*id).map(|message| message.text.clone());
                }
                CueAction::ClearMessage | CueAction::ClearMessages => {
                    self.output.message = None;
                    self.output.message_snapshot = None;
                }
                CueAction::StopAudio | CueAction::ClearAudio => {
                    self.output.stop_audio_serial = self.output.stop_audio_serial.wrapping_add(1);
                    self.output.audio_clear_serial = self.output.audio_clear_serial.wrapping_add(1);
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
// `Runtime::cursor` says which occurrence is live.
fn playlist_cues(project: &Project) -> Vec<(Uuid, Uuid)> {
    project
        .playlist
        .iter()
        .filter_map(|item| project.document(item.document_id))
        .flat_map(|doc| doc.slides.iter().map(move |slide| (doc.id, slide.id)))
        .collect()
}

/// Playlist row (item index) that holds the cue at `cue`.
fn cue_row(project: &Project, mut cue: usize) -> Option<usize> {
    for (row, item) in project.playlist.iter().enumerate() {
        let slides = project
            .document(item.document_id)
            .map_or(0, |doc| doc.slides.len());
        if cue < slides {
            return Some(row);
        }
        cue -= slides;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CueAction, Macro, MessageTemplate, Prop, Rect, SlideObject, TextAlign, TextStyle};

    fn fixture() -> (Project, Uuid, Uuid, Uuid) {
        let mut project = Project::demo();
        let prop_id = Uuid::new_v4();
        let message_id = Uuid::new_v4();
        let timer_id = project.timers[0].id;
        project.props.push(Prop {
            id: prop_id,
            name: "Original prop".into(),
            objects: vec![SlideObject::Text {
                id: Uuid::new_v4(),
                rect: Rect::default(),
                text: "ORIGINAL".into(),
                style: TextStyle::default(),
                align: TextAlign::Center,
            }],
        });
        project.messages.push(MessageTemplate {
            id: message_id,
            name: "Message".into(),
            text: format!("Time {{timer:{timer_id}}}"),
        });
        (project, prop_id, message_id, timer_id)
    }

    #[test]
    fn snapshots_freeze_prop_and_message_until_retrigger_and_clear() {
        let (mut project, prop_id, message_id, _) = fixture();
        let mut runtime = Runtime::default();
        runtime
            .apply_actions(
                &project,
                &[
                    CueAction::TriggerProp { id: prop_id },
                    CueAction::ShowMessage { id: message_id },
                ],
            )
            .unwrap();
        project.props[0].name = "Edited prop".into();
        project.props[0].objects.clear();
        project.messages[0].text = "Edited message".into();
        assert_eq!(runtime.output.props, vec![prop_id]);
        assert_eq!(runtime.output.prop_snapshots[0].name, "Original prop");
        assert_eq!(runtime.output.prop_snapshots[0].objects.len(), 1);
        assert!(
            runtime
                .output
                .message_snapshot
                .as_deref()
                .unwrap()
                .starts_with("Time {timer:")
        );

        // Triggering an already shown prop refreshes its snapshot without duplicating its ID.
        runtime
            .apply_actions(&project, &[CueAction::TriggerProp { id: prop_id }])
            .unwrap();
        assert_eq!(runtime.output.props, vec![prop_id]);
        assert_eq!(runtime.output.prop_snapshots[0].name, "Edited prop");
        let macro_id = Uuid::new_v4();
        project.macros.push(Macro {
            id: macro_id,
            name: "Clear outputs".into(),
            actions: vec![CueAction::ClearProps, CueAction::ClearMessage],
        });
        runtime.run_macro(&project, macro_id).unwrap();
        assert!(runtime.output.props.is_empty());
        assert!(runtime.output.prop_snapshots.is_empty());
        assert_eq!(runtime.output.message, None);
        assert_eq!(runtime.output.message_snapshot, None);
    }

    #[test]
    fn message_snapshot_survives_template_edit_while_timer_values_stay_live() {
        let (mut project, _, message_id, timer_id) = fixture();
        let mut runtime = Runtime::default();
        runtime
            .apply_actions(&project, &[CueAction::ShowMessage { id: message_id }])
            .unwrap();
        let timer_id = project.timer_def(timer_id).unwrap().id;
        let timer_text = runtime.render_output_message(&project, 0).unwrap();
        assert_eq!(timer_text, "Time 05:00");
        runtime.timer_states.get_mut(&timer_id).unwrap().value_secs = 61.0;
        assert_eq!(
            runtime.render_output_message(&project, 0).unwrap(),
            "Time 01:01"
        );
        project.messages.clear();
        runtime.timer_states.get_mut(&timer_id).unwrap().running = true;
        runtime.tick(&project, Duration::from_secs(1));
        assert_eq!(
            runtime.render_output_message(&project, 0).unwrap(),
            "Time 01:00"
        );
        runtime
            .apply_actions(&project, &[CueAction::ClearMessages])
            .unwrap();
        assert_eq!(runtime.render_output_message(&project, 0), None);
    }

    #[test]
    fn prop_uses_trigger_time_live_theme_even_after_project_theme_edit() {
        let (mut project, prop_id, _, _) = fixture();
        let mut runtime = Runtime::default();
        let theme = project.themes[0].clone();
        runtime.output.live = Some(LiveSlide {
            document_id: project.documents[0].id,
            document_name: project.documents[0].name.clone(),
            slide: project.documents[0].slides[0].clone(),
            theme: theme.clone(),
        });
        runtime
            .apply_actions(&project, &[CueAction::TriggerProp { id: prop_id }])
            .unwrap();
        project.themes[0].font_size = 177.0;
        assert_eq!(runtime.output.prop_theme_snapshots, vec![theme]);
    }
}
