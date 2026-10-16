use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub const SCHEMA_VERSION: u32 = 2;
pub(crate) const MAX_ACTIONS: usize = 64;
pub(crate) const MAX_OBJECTS: usize = 32;

// ---------------------------------------------------------------------------
// Rect
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Default for Rect {
    fn default() -> Self {
        Self {
            x: 0.05,
            y: 0.05,
            w: 0.9,
            h: 0.9,
        }
    }
}

impl Rect {
    pub fn is_valid(&self) -> bool {
        self.x.is_finite()
            && self.y.is_finite()
            && self.w.is_finite()
            && self.h.is_finite()
            && self.x >= 0.0
            && self.y >= 0.0
            && self.w > 0.0
            && self.h > 0.0
            && self.x + self.w <= 1.0001
            && self.y + self.h <= 1.0001
    }
}

// ---------------------------------------------------------------------------
// TextStyle
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct TextStyle {
    #[serde(default)]
    pub font_size: Option<f32>,
    #[serde(default)]
    pub color: Option<[u8; 4]>,
    #[serde(default)]
    pub outline: Option<(f32, [u8; 4])>,
    #[serde(default)]
    pub shadow: bool,
}

// ---------------------------------------------------------------------------
// SlideObject
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SlideObject {
    Text {
        id: Uuid,
        rect: Rect,
        text: String,
        style: TextStyle,
        align: TextAlign,
    },
    Image {
        id: Uuid,
        rect: Rect,
        path: PathBuf,
    },
    Shape {
        id: Uuid,
        rect: Rect,
        color: [u8; 4],
        corner_radius: f32,
    },
}

impl SlideObject {
    pub fn id(&self) -> Uuid {
        match self {
            SlideObject::Text { id, .. }
            | SlideObject::Image { id, .. }
            | SlideObject::Shape { id, .. } => *id,
        }
    }

    pub fn rect(&self) -> &Rect {
        match self {
            SlideObject::Text { rect, .. }
            | SlideObject::Image { rect, .. }
            | SlideObject::Shape { rect, .. } => rect,
        }
    }
}

// ---------------------------------------------------------------------------
// Timer types
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum TimerKind {
    Countdown { duration_secs: f64 },
    CountdownTo { hour: u8, minute: u8, second: u8 },
    Elapsed,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TimerDef {
    pub id: Uuid,
    pub name: String,
    pub kind: TimerKind,
    pub allow_overrun: bool,
}

// ---------------------------------------------------------------------------
// Prop / MessageTemplate / AudioCue
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Prop {
    pub id: Uuid,
    pub name: String,
    pub objects: Vec<SlideObject>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MessageTemplate {
    pub id: Uuid,
    pub name: String,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AudioCue {
    pub path: PathBuf,
    pub volume: f32,
}

// ---------------------------------------------------------------------------
// Existing core types (with v2 extensions)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Project {
    pub schema_version: u32,
    pub name: String,
    pub documents: Vec<Document>,
    pub playlist: Vec<PlaylistItem>,
    pub themes: Vec<Theme>,
    pub macros: Vec<Macro>,
    #[serde(default)]
    pub timers: Vec<TimerDef>,
    #[serde(default)]
    pub props: Vec<Prop>,
    #[serde(default)]
    pub messages: Vec<MessageTemplate>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Document {
    pub id: Uuid,
    pub name: String,
    pub slides: Vec<Slide>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PlaylistItem {
    pub id: Uuid,
    pub document_id: Uuid,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Slide {
    pub id: Uuid,
    pub label: String,
    pub text: String,
    pub notes: String,
    pub theme_id: Uuid,
    pub background: Background,
    pub transition: Transition,
    pub actions: Vec<CueAction>,
    #[serde(default)]
    pub text_rect: Rect,
    #[serde(default)]
    pub text_style: TextStyle,
    #[serde(default)]
    pub objects: Vec<SlideObject>,
    #[serde(default)]
    pub audio: Option<AudioCue>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Background {
    Solid([u8; 4]),
    Image {
        path: PathBuf,
    },
    Video {
        path: PathBuf,
        looped: bool,
        muted: bool,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Transition {
    Cut,
    Fade { millis: u32 },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Theme {
    pub id: Uuid,
    pub name: String,
    pub font_size: f32,
    pub text_color: [u8; 4],
    pub background_color: [u8; 4],
    pub align: TextAlign,
    pub vertical_align: VerticalAlign,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum VerticalAlign {
    Top,
    Middle,
    Bottom,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Macro {
    pub id: Uuid,
    pub name: String,
    pub actions: Vec<CueAction>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum CueAction {
    Blackout(bool),
    ClearText(bool),
    ClearAll,
    StageMessage(String),
    StartTimer { id: Uuid },
    PauseTimer { id: Uuid },
    ResetTimer { id: Uuid },
    TriggerProp { id: Uuid },
    ClearProps,
    ShowMessage { id: Uuid },
    ClearMessage,
    StopAudio,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("unsupported project schema version {found}; expected {expected}")]
    UnsupportedSchema { found: u32, expected: u32 },
    #[error("project must contain at least one theme")]
    NoThemes,
    #[error("duplicate identifier {0}")]
    DuplicateId(Uuid),
    #[error("slide {slide_id} references missing theme {theme_id}")]
    MissingTheme { slide_id: Uuid, theme_id: Uuid },
    #[error("playlist item {item_id} references missing document {document_id}")]
    MissingDocument { item_id: Uuid, document_id: Uuid },
    #[error(
        "theme {theme_id} has invalid font size {font_size}; expected a finite value from 8 through 400"
    )]
    InvalidFontSize { theme_id: Uuid, font_size: f32 },
    #[error("{owner} has {actual} actions; maximum is {maximum}")]
    TooManyActions {
        owner: String,
        actual: usize,
        maximum: usize,
    },
    #[error("timer {0} does not exist")]
    MissingTimer(Uuid),
    #[error("macro {0} does not exist")]
    MissingMacro(Uuid),
    #[error("document {0} does not exist")]
    MissingDocumentById(Uuid),
    #[error("slide {slide_id} does not exist in document {document_id}")]
    MissingSlide { document_id: Uuid, slide_id: Uuid },
    #[error("project file is {actual} bytes; maximum is {maximum} bytes")]
    ProjectTooLarge { actual: u64, maximum: u64 },
    #[error("I/O error while {operation}: {source}")]
    Io {
        operation: String,
        #[source]
        source: std::io::Error,
    },
    #[error("project JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid rect in {owner}: x={x}, y={y}, w={w}, h={h}")]
    InvalidRect {
        owner: String,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    },
    #[error("invalid text style font size {font_size} in {owner}; expected 8..=400 or None")]
    InvalidTextStyleFontSize { owner: String, font_size: f32 },
    #[error("invalid audio volume {volume} in {owner}; expected finite 0..=1")]
    InvalidVolume { owner: String, volume: f32 },
    #[error("invalid timer kind for timer {id}: {reason}")]
    InvalidTimerKind { id: Uuid, reason: String },
    #[error("invalid corner radius {radius} in shape {id}; expected finite 0..=0.5")]
    InvalidCornerRadius { id: Uuid, radius: f32 },
    #[error("{owner} has {actual} objects; maximum is {maximum}")]
    TooManyObjects {
        owner: String,
        actual: usize,
        maximum: usize,
    },
    #[error("prop {0} does not exist")]
    MissingProp(Uuid),
    #[error("message {0} does not exist")]
    MissingMessage(Uuid),
}

// ---------------------------------------------------------------------------
// Defaults / constructors
// ---------------------------------------------------------------------------

impl Default for Theme {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Default".into(),
            font_size: 64.0,
            text_color: [245, 246, 248, 255],
            background_color: [18, 22, 30, 255],
            align: TextAlign::Center,
            vertical_align: VerticalAlign::Middle,
        }
    }
}

impl Slide {
    pub fn new(theme_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            label: "New slide".into(),
            text: String::new(),
            notes: String::new(),
            theme_id,
            background: Background::Solid([18, 22, 30, 255]),
            transition: Transition::Cut,
            actions: Vec::new(),
            text_rect: Rect::default(),
            text_style: TextStyle::default(),
            objects: Vec::new(),
            audio: None,
        }
    }
}

impl Document {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            slides: Vec::new(),
        }
    }
}

impl Project {
    pub fn document(&self, id: Uuid) -> Option<&Document> {
        self.documents.iter().find(|doc| doc.id == id)
    }
    pub fn theme(&self, id: Uuid) -> Option<&Theme> {
        self.themes.iter().find(|theme| theme.id == id)
    }
    pub fn timer_def(&self, id: Uuid) -> Option<&TimerDef> {
        self.timers.iter().find(|t| t.id == id)
    }
    pub fn prop(&self, id: Uuid) -> Option<&Prop> {
        self.props.iter().find(|p| p.id == id)
    }
    pub fn message(&self, id: Uuid) -> Option<&MessageTemplate> {
        self.messages.iter().find(|m| m.id == id)
    }

    pub fn validate(&self) -> Result<(), CoreError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(CoreError::UnsupportedSchema {
                found: self.schema_version,
                expected: SCHEMA_VERSION,
            });
        }
        if self.themes.is_empty() {
            return Err(CoreError::NoThemes);
        }

        // Collect ALL ids for duplicate check
        let mut ids = std::collections::HashSet::new();
        for id in self
            .documents
            .iter()
            .map(|doc| doc.id)
            .chain(
                self.documents
                    .iter()
                    .flat_map(|doc| doc.slides.iter().map(|slide| slide.id)),
            )
            .chain(self.documents.iter().flat_map(|doc| {
                doc.slides
                    .iter()
                    .flat_map(|slide| slide.objects.iter().map(|obj| obj.id()))
            }))
            .chain(self.themes.iter().map(|theme| theme.id))
            .chain(self.playlist.iter().map(|item| item.id))
            .chain(self.macros.iter().map(|mac| mac.id))
            .chain(self.timers.iter().map(|t| t.id))
            .chain(self.props.iter().map(|p| p.id))
            .chain(
                self.props
                    .iter()
                    .flat_map(|p| p.objects.iter().map(|obj| obj.id())),
            )
            .chain(self.messages.iter().map(|m| m.id))
        {
            if !ids.insert(id) {
                return Err(CoreError::DuplicateId(id));
            }
        }

        // Theme font sizes
        for theme in &self.themes {
            if !theme.font_size.is_finite() || !(8.0..=400.0).contains(&theme.font_size) {
                return Err(CoreError::InvalidFontSize {
                    theme_id: theme.id,
                    font_size: theme.font_size,
                });
            }
        }

        // Documents / slides
        for doc in &self.documents {
            for slide in &doc.slides {
                if self.theme(slide.theme_id).is_none() {
                    return Err(CoreError::MissingTheme {
                        slide_id: slide.id,
                        theme_id: slide.theme_id,
                    });
                }
                validate_actions(&slide.actions, format!("slide {}", slide.id))?;
                self.validate_action_refs(&slide.actions)?;

                // text_rect
                if !slide.text_rect.is_valid() {
                    return Err(CoreError::InvalidRect {
                        owner: format!("slide {}", slide.id),
                        x: slide.text_rect.x,
                        y: slide.text_rect.y,
                        w: slide.text_rect.w,
                        h: slide.text_rect.h,
                    });
                }

                // text_style
                validate_text_style(&slide.text_style, format!("slide {}", slide.id))?;

                // objects count
                if slide.objects.len() > MAX_OBJECTS {
                    return Err(CoreError::TooManyObjects {
                        owner: format!("slide {}", slide.id),
                        actual: slide.objects.len(),
                        maximum: MAX_OBJECTS,
                    });
                }

                // objects validity
                for obj in &slide.objects {
                    validate_object(obj)?;
                }

                // audio
                if let Some(audio) = &slide.audio {
                    validate_audio(audio, format!("slide {}", slide.id))?;
                }
            }
        }

        // Playlist
        for item in &self.playlist {
            if self.document(item.document_id).is_none() {
                return Err(CoreError::MissingDocument {
                    item_id: item.id,
                    document_id: item.document_id,
                });
            }
        }

        // Macros
        for mac in &self.macros {
            validate_actions(&mac.actions, format!("macro {}", mac.id))?;
            self.validate_action_refs(&mac.actions)?;
        }

        // Timers
        for timer in &self.timers {
            validate_timer_kind(&timer.kind, timer.id)?;
        }

        // Props
        for prop in &self.props {
            if prop.objects.len() > MAX_OBJECTS {
                return Err(CoreError::TooManyObjects {
                    owner: format!("prop {}", prop.id),
                    actual: prop.objects.len(),
                    maximum: MAX_OBJECTS,
                });
            }
            for obj in &prop.objects {
                validate_object(obj)?;
            }
        }

        Ok(())
    }

    /// Validate that action references (timer/prop/message ids) exist in the project.
    fn validate_action_refs(&self, actions: &[CueAction]) -> Result<(), CoreError> {
        for action in actions {
            match action {
                CueAction::StartTimer { id }
                | CueAction::PauseTimer { id }
                | CueAction::ResetTimer { id } => {
                    if self.timer_def(*id).is_none() {
                        return Err(CoreError::MissingTimer(*id));
                    }
                }
                CueAction::TriggerProp { id } => {
                    if self.prop(*id).is_none() {
                        return Err(CoreError::MissingProp(*id));
                    }
                }
                CueAction::ShowMessage { id } if self.message(*id).is_none() => {
                    return Err(CoreError::MissingMessage(*id));
                }
                CueAction::ShowMessage { .. } => {}
                _ => {}
            }
        }
        Ok(())
    }

    pub fn demo() -> Self {
        let welcome = Theme {
            name: "Welcome".into(),
            ..Theme::default()
        };
        let mut song = Theme {
            name: "Song".into(),
            font_size: 58.0,
            background_color: [12, 15, 22, 255],
            ..Theme::default()
        };
        let mut notice = Theme {
            name: "Announcements".into(),
            font_size: 52.0,
            align: TextAlign::Left,
            vertical_align: VerticalAlign::Top,
            ..Theme::default()
        };
        // Separate generated identifiers even when using struct update syntax.
        if song.id == welcome.id {
            song.id = Uuid::new_v4();
        }
        if notice.id == welcome.id || notice.id == song.id {
            notice.id = Uuid::new_v4();
        }
        let mut welcome_doc = Document::new("Welcome");
        welcome_doc.slides = [
            (
                "Welcome",
                "Welcome\nWe're glad you're here.",
                "Stand by for the opening.",
            ),
            (
                "Gather",
                "Take a moment to settle in.",
                "House lights may lower.",
            ),
            ("Begin", "Let's begin.", "Trigger when the room is ready."),
        ]
        .into_iter()
        .map(|(label, text, notes)| {
            let mut slide = Slide::new(welcome.id);
            slide.label = label.into();
            slide.text = text.into();
            slide.notes = notes.into();
            slide
        })
        .collect();
        let mut song_doc = Document::new("Song");
        song_doc.slides = [
            (
                "Verse",
                "We gather in the morning light,\nWith open hands and hearts made new.",
                "Check the band is ready.",
            ),
            (
                "Chorus",
                "Hope rises up; we sing as one.",
                "Keep this cue through the refrain.",
            ),
            (
                "Bridge",
                "In every season, still we stand.",
                "Watch the director for the return.",
            ),
        ]
        .into_iter()
        .map(|(label, text, notes)| {
            let mut slide = Slide::new(song.id);
            slide.label = label.into();
            slide.text = text.into();
            slide.notes = notes.into();
            slide
        })
        .collect();
        let mut announcement_doc = Document::new("Announcements");
        announcement_doc.slides = [
            (
                "Community",
                "Community lunch\nSunday after the gathering",
                "Mention dietary options.",
            ),
            (
                "Next week",
                "Next week\nA new series begins",
                "Invite people to bring a friend.",
            ),
            (
                "Thanks",
                "Thank you for being here.",
                "Close with the house lights.",
            ),
        ]
        .into_iter()
        .map(|(label, text, notes)| {
            let mut slide = Slide::new(notice.id);
            slide.label = label.into();
            slide.text = text.into();
            slide.notes = notes.into();
            slide
        })
        .collect();
        let documents = vec![welcome_doc, song_doc, announcement_doc];
        let playlist = documents
            .iter()
            .map(|doc| PlaylistItem {
                id: Uuid::new_v4(),
                document_id: doc.id,
            })
            .collect();
        let mut restore = vec![CueAction::Blackout(false), CueAction::ClearText(false)];
        restore.push(CueAction::StageMessage("Welcome".into()));
        let default_timer = TimerDef {
            id: Uuid::new_v4(),
            name: "Countdown".into(),
            kind: TimerKind::Countdown {
                duration_secs: 300.0,
            },
            allow_overrun: false,
        };
        Self {
            schema_version: SCHEMA_VERSION,
            name: "Sunday Gathering".into(),
            documents,
            playlist,
            themes: vec![welcome, song, notice],
            macros: vec![
                Macro {
                    id: Uuid::new_v4(),
                    name: "Blackout".into(),
                    actions: vec![CueAction::Blackout(true)],
                },
                Macro {
                    id: Uuid::new_v4(),
                    name: "Restore output".into(),
                    actions: restore,
                },
                Macro {
                    id: Uuid::new_v4(),
                    name: "Quiet stage".into(),
                    actions: vec![CueAction::StageMessage("Please hold".into())],
                },
            ],
            timers: vec![default_timer],
            props: Vec::new(),
            messages: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

pub(crate) fn validate_actions(actions: &[CueAction], owner: String) -> Result<(), CoreError> {
    if actions.len() > MAX_ACTIONS {
        Err(CoreError::TooManyActions {
            owner,
            actual: actions.len(),
            maximum: MAX_ACTIONS,
        })
    } else {
        Ok(())
    }
}

fn validate_text_style(style: &TextStyle, owner: String) -> Result<(), CoreError> {
    if let Some(fs) = style.font_size
        && (!fs.is_finite() || !(8.0..=400.0).contains(&fs))
    {
        return Err(CoreError::InvalidTextStyleFontSize {
            owner,
            font_size: fs,
        });
    }
    Ok(())
}

fn validate_object(obj: &SlideObject) -> Result<(), CoreError> {
    if !obj.rect().is_valid() {
        let r = obj.rect();
        return Err(CoreError::InvalidRect {
            owner: format!("object {}", obj.id()),
            x: r.x,
            y: r.y,
            w: r.w,
            h: r.h,
        });
    }
    match obj {
        SlideObject::Text { style, .. } => {
            validate_text_style(style, format!("object {}", obj.id()))?;
        }
        SlideObject::Shape {
            id, corner_radius, ..
        } => {
            if !corner_radius.is_finite() || !(0.0..=0.5).contains(corner_radius) {
                return Err(CoreError::InvalidCornerRadius {
                    id: *id,
                    radius: *corner_radius,
                });
            }
        }
        SlideObject::Image { .. } => {}
    }
    Ok(())
}

fn validate_timer_kind(kind: &TimerKind, id: Uuid) -> Result<(), CoreError> {
    match kind {
        TimerKind::Countdown { duration_secs } => {
            if !duration_secs.is_finite() || *duration_secs < 0.0 || *duration_secs > 360_000.0 {
                return Err(CoreError::InvalidTimerKind {
                    id,
                    reason: format!(
                        "countdown duration {} must be finite, >= 0 and <= 360000",
                        duration_secs
                    ),
                });
            }
        }
        TimerKind::CountdownTo {
            hour,
            minute,
            second,
        } => {
            if *hour >= 24 || *minute >= 60 || *second >= 60 {
                return Err(CoreError::InvalidTimerKind {
                    id,
                    reason: format!(
                        "countdown-to time {}:{:02}:{:02} is out of range",
                        hour, minute, second
                    ),
                });
            }
        }
        TimerKind::Elapsed => {}
    }
    Ok(())
}

fn validate_audio(audio: &AudioCue, owner: String) -> Result<(), CoreError> {
    if !audio.volume.is_finite() || !(0.0..=1.0).contains(&audio.volume) {
        return Err(CoreError::InvalidVolume {
            owner,
            volume: audio.volume,
        });
    }
    Ok(())
}
