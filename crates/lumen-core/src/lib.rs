mod captions;
pub mod history;
mod import;
pub mod markup;
mod model;
mod runtime;
mod storage;

pub use captions::caption_chunks;
pub use import::document_from_text;
pub use model::{
    AudioCue, Backdrop, Background, CoreError, CueAction, Document, Macro, MessageTemplate,
    PlaylistItem, Project, Prop, PushDirection, Rect, SCHEMA_VERSION, Slide, SlideObject,
    TextAlign, TextStyle, Theme, TimerDef, TimerKind, Transition, VerticalAlign,
};
pub use runtime::{LiveSlide, OutputState, Runtime, TimerState};
pub use storage::{load_project, save_project};
