use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub const SCHEMA_VERSION: u32 = 1;
pub(crate) const MAX_ACTIONS: usize = 64;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Project {
    pub schema_version: u32,
    pub name: String,
    pub documents: Vec<Document>,
    pub playlist: Vec<PlaylistItem>,
    pub themes: Vec<Theme>,
    pub macros: Vec<Macro>,
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
}

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
}

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
            .chain(self.themes.iter().map(|theme| theme.id))
            .chain(self.playlist.iter().map(|item| item.id))
            .chain(self.macros.iter().map(|mac| mac.id))
        {
            if !ids.insert(id) {
                return Err(CoreError::DuplicateId(id));
            }
        }
        for theme in &self.themes {
            if !theme.font_size.is_finite() || !(8.0..=400.0).contains(&theme.font_size) {
                return Err(CoreError::InvalidFontSize {
                    theme_id: theme.id,
                    font_size: theme.font_size,
                });
            }
        }
        for doc in &self.documents {
            for slide in &doc.slides {
                if self.theme(slide.theme_id).is_none() {
                    return Err(CoreError::MissingTheme {
                        slide_id: slide.id,
                        theme_id: slide.theme_id,
                    });
                }
                validate_actions(&slide.actions, format!("slide {}", slide.id))?;
            }
        }
        for item in &self.playlist {
            if self.document(item.document_id).is_none() {
                return Err(CoreError::MissingDocument {
                    item_id: item.id,
                    document_id: item.document_id,
                });
            }
        }
        for mac in &self.macros {
            validate_actions(&mac.actions, format!("macro {}", mac.id))?;
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
        }
    }
}

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
