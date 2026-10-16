use lumen_core::{
    Background, CoreError, CueAction, Document, Macro, Project, Runtime, Slide, Theme,
    document_from_text, load_project, save_project,
};
use std::{fs, time::Duration};
use uuid::Uuid;

fn project_with_docs(docs: Vec<Document>, theme: Theme) -> Project {
    Project {
        schema_version: lumen_core::SCHEMA_VERSION,
        name: "Test".into(),
        documents: docs,
        playlist: vec![],
        themes: vec![theme],
        macros: vec![],
    }
}
fn doc(name: &str, theme: Uuid, labels: &[&str]) -> Document {
    let mut d = Document::new(name);
    d.slides = labels
        .iter()
        .map(|label| {
            let mut s = Slide::new(theme);
            s.label = (*label).into();
            s
        })
        .collect();
    d
}

#[test]
fn live_trigger_is_a_detached_snapshot_and_actions_apply_in_order() {
    let mut p = Project::demo();
    let d_id = p.documents[0].id;
    let slide_id = p.documents[0].slides[0].id;
    p.documents[0].slides[0].actions = vec![
        CueAction::Blackout(true),
        CueAction::ClearText(true),
        CueAction::StageMessage("Hold".into()),
    ];
    let mut r = Runtime::default();
    r.trigger(&p, d_id, slide_id).unwrap();
    assert!(r.output.blackout);
    assert!(r.output.clear_text);
    assert_eq!(r.output.stage_message, "Hold");
    let original = r.output.live.clone().unwrap();
    p.documents[0].slides[0].text = "edited".into();
    p.themes[0].font_size = 123.0;
    assert_eq!(r.output.live.unwrap(), original);
}

#[test]
fn navigation_crosses_documents_skips_empty_and_honors_boundaries() {
    let theme = Theme::default();
    let tid = theme.id;
    let mut a = doc("a", tid, &["a1", "a2"]);
    let mut empty = doc("empty", tid, &[]);
    let b = doc("b", tid, &["b1"]);
    let playlist = [a.id, empty.id, b.id]
        .into_iter()
        .map(|document_id| lumen_core::PlaylistItem {
            id: Uuid::new_v4(),
            document_id,
        })
        .collect();
    let mut p = project_with_docs(vec![a.clone(), empty.clone(), b.clone()], theme.clone());
    p.playlist = playlist;
    let mut r = Runtime::default();
    assert!(r.next(&p).unwrap());
    assert_eq!(r.output.live.as_ref().unwrap().slide.label, "a1");
    assert!(r.next(&p).unwrap());
    assert_eq!(r.output.live.as_ref().unwrap().slide.label, "a2");
    assert!(r.next(&p).unwrap());
    assert_eq!(r.output.live.as_ref().unwrap().slide.label, "b1");
    assert!(!r.next(&p).unwrap());
    assert_eq!(r.output.live.as_ref().unwrap().slide.label, "b1");
    assert!(r.previous(&p).unwrap());
    assert_eq!(r.output.live.as_ref().unwrap().slide.label, "a2");
    assert!(r.previous(&p).unwrap());
    assert_eq!(r.output.live.as_ref().unwrap().slide.label, "a1");
    assert!(!r.previous(&p).unwrap());
    a.slides.clear();
    empty.slides.clear();
    let p2 = project_with_docs(vec![a, empty], theme);
    let mut r2 = Runtime::default();
    assert!(!r2.next(&p2).unwrap());
    assert!(!r2.previous(&p2).unwrap());
}

#[test]
fn navigation_from_no_live_uses_playlist_edges_and_peek_is_pure() {
    let mut p = Project::demo();
    let mut r = Runtime::default();
    let next = r.peek_next(&p).unwrap();
    assert_eq!(next.slide.label, p.documents[0].slides[0].label);
    assert!(r.output.live.is_none());
    assert!(r.previous(&p).unwrap());
    assert_eq!(
        r.output.live.as_ref().unwrap().slide.label,
        p.documents.last().unwrap().slides.last().unwrap().label
    );
    p.playlist.clear();
    let mut r = Runtime::default();
    assert!(r.peek_next(&p).is_none());
    assert!(!r.next(&p).unwrap());
}

#[test]
fn timers_pause_resume_reset_expire_and_ceil_fractional_display() {
    let mut t = lumen_core::Timer::new("short", 2.2);
    assert_eq!(t.display(), "00:03");
    t.start();
    t.tick(Duration::from_millis(700));
    assert_eq!(t.display(), "00:02");
    t.pause();
    t.tick(Duration::from_secs(1));
    assert!((t.remaining_secs - 1.5).abs() < 0.001);
    t.start();
    t.tick(Duration::from_secs(2));
    assert_eq!(t.remaining_secs, 0.0);
    assert!(!t.running);
    t.reset();
    assert!(!t.running);
    assert!((t.remaining_secs - 2.2).abs() < 0.001);
    let hour = lumen_core::Timer::new("hour", 3600.0);
    assert_eq!(hour.display(), "01:00:00");
}

#[test]
fn action_batches_are_ordered_bounded_and_invalid_batches_are_atomic() {
    let mut r = Runtime::default();
    let timer = r.timers[0].id;
    r.apply_actions(&[
        CueAction::Blackout(true),
        CueAction::Blackout(false),
        CueAction::StageMessage("Ready".into()),
        CueAction::StartTimer { id: timer },
        CueAction::PauseTimer { id: timer },
    ])
    .unwrap();
    assert!(!r.output.blackout);
    assert_eq!(r.output.stage_message, "Ready");
    assert!(!r.timers[0].running);
    let before = r.clone();
    let missing = Uuid::new_v4();
    assert!(
        r.apply_actions(&[
            CueAction::Blackout(true),
            CueAction::StartTimer { id: missing }
        ])
        .is_err()
    );
    assert_eq!(r, before);
    let too_many = vec![CueAction::ClearAll; 65];
    assert!(r.apply_actions(&too_many).is_err());
    let mut p = Project::demo();
    let id = Uuid::new_v4();
    p.macros.push(Macro {
        id,
        name: "bad".into(),
        actions: vec![CueAction::ResetTimer { id: Uuid::new_v4() }],
    });
    assert!(r.run_macro(&p, id).is_err());
    assert_eq!(r, before);
}

#[test]
fn validation_rejects_duplicate_dangling_schema_font_and_action_errors() {
    let mut p = Project::demo();
    assert!(p.validate().is_ok());
    p.schema_version = 99;
    assert!(p.validate().is_err());
    p.schema_version = lumen_core::SCHEMA_VERSION;
    let duplicate = p.documents[0].slides[0].id;
    p.documents[0].slides[1].id = duplicate;
    assert!(p.validate().is_err());
    let mut p = Project::demo();
    p.documents[0].slides[0].theme_id = Uuid::new_v4();
    assert!(p.validate().is_err());
    let mut p = Project::demo();
    p.themes[0].font_size = f32::INFINITY;
    assert!(p.validate().is_err());
    let mut p = Project::demo();
    p.themes[0].font_size = 7.99;
    assert!(p.validate().is_err());
    let mut p = Project::demo();
    p.documents[0].slides[0].actions = vec![CueAction::ClearAll; 65];
    assert!(p.validate().is_err());
    let mut p = Project::demo();
    p.playlist[0].document_id = Uuid::new_v4();
    assert!(p.validate().is_err());
    let mut p = Project::demo();
    p.themes[0].id = p.documents[0].id;
    assert!(matches!(p.validate(), Err(CoreError::DuplicateId(_))));
}

#[test]
fn storage_roundtrips_and_rejects_invalid_or_oversized_files_without_clobbering() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("show.json");
    let p = Project::demo();
    save_project(&path, &p).unwrap();
    assert_eq!(load_project(&path).unwrap(), p);
    let mut replacement = p.clone();
    replacement.name = "Replacement show".into();
    save_project(&path, &replacement).unwrap();
    assert_eq!(load_project(&path).unwrap(), replacement);
    let old = fs::read(&path).unwrap();
    let mut invalid = p.clone();
    invalid.schema_version = 999;
    assert!(save_project(&path, &invalid).is_err());
    assert_eq!(fs::read(&path).unwrap(), old);
    fs::write(&path, b"{bad json").unwrap();
    assert!(load_project(&path).is_err());
    fs::write(&path, vec![b' '; 16 * 1024 * 1024 + 1]).unwrap();
    assert!(matches!(
        load_project(&path),
        Err(CoreError::ProjectTooLarge { .. })
    ));
}

#[test]
fn text_import_normalizes_lines_and_splits_only_blank_paragraphs() {
    let theme = Uuid::new_v4();
    let d = document_from_text(
        "Imported",
        "\r\n  First  \r\nline  \r\n \t\r\nSecond\rThird \t\r\n",
        theme,
    );
    assert_eq!(d.slides.len(), 2);
    assert_eq!(d.slides[0].label, "Slide 1");
    assert_eq!(d.slides[0].text, "  First\nline");
    assert_eq!(d.slides[1].text, "Second\nThird");
    assert!(
        document_from_text("empty", " \r\n \t\n", theme)
            .slides
            .is_empty()
    );
}

#[test]
fn serde_background_uses_external_tags() {
    let value = serde_json::to_value(Background::Video {
        path: "movie.mov".into(),
        looped: true,
        muted: false,
    })
    .unwrap();
    assert!(value.get("Video").is_some());
}

#[test]
fn runtime_serde_roundtrips_runtime_state() {
    let runtime = Runtime::default();
    let encoded = serde_json::to_string(&runtime).unwrap();
    let decoded: Runtime = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, runtime);
}

#[test]
fn failed_slide_trigger_preserves_prior_runtime_for_missing_timers_and_action_limit() {
    let mut project = Project::demo();
    let doc_id = project.documents[0].id;
    let target_id = project.documents[0].slides[0].id;
    let mut runtime = Runtime::default();
    runtime.trigger(&project, doc_id, target_id).unwrap();
    runtime.output.blackout = true;
    runtime.output.stage_message = "Prior state".into();
    runtime.timers[0].start();

    project.documents[0].slides[1].actions = vec![CueAction::StartTimer { id: Uuid::new_v4() }];
    let before = runtime.clone();
    assert!(matches!(
        runtime.trigger(&project, doc_id, project.documents[0].slides[1].id),
        Err(CoreError::MissingTimer(_))
    ));
    assert_eq!(runtime, before);

    project.documents[0].slides[1].actions = vec![CueAction::ClearAll; 65];
    assert!(matches!(
        runtime.trigger(&project, doc_id, project.documents[0].slides[1].id),
        Err(CoreError::TooManyActions { .. })
    ));
    assert_eq!(runtime, before);
}

#[test]
fn run_macro_is_ordered_and_clear_all_preserves_blackout_and_stage_message() {
    let mut project = Project::demo();
    let doc_id = project.documents[0].id;
    let slide_id = project.documents[0].slides[0].id;
    let macro_id = Uuid::new_v4();
    project.macros.push(Macro {
        id: macro_id,
        name: "Ordered restore".into(),
        actions: vec![
            CueAction::Blackout(true),
            CueAction::StageMessage("Hold".into()),
            CueAction::ClearText(true),
            CueAction::ClearAll,
            CueAction::Blackout(false),
            CueAction::StageMessage("Ready".into()),
        ],
    });
    let mut runtime = Runtime::default();
    runtime.trigger(&project, doc_id, slide_id).unwrap();
    runtime.output.blackout = true;
    runtime.output.clear_text = true;
    runtime.output.stage_message = "Before".into();
    runtime.run_macro(&project, macro_id).unwrap();
    assert!(runtime.output.live.is_none());
    assert!(!runtime.output.clear_text);
    assert!(!runtime.output.blackout);
    assert_eq!(runtime.output.stage_message, "Ready");

    let before = runtime.clone();
    assert!(matches!(
        runtime.run_macro(&project, Uuid::new_v4()),
        Err(CoreError::MissingMacro(_))
    ));
    assert_eq!(runtime, before);

    runtime.output.live = Some(runtime.peek_next(&project).unwrap());
    runtime.output.blackout = true;
    runtime.output.clear_text = true;
    runtime.output.stage_message = "Preserved".into();
    runtime.apply_actions(&[CueAction::ClearAll]).unwrap();
    assert!(runtime.output.live.is_none());
    assert!(!runtime.output.clear_text);
    assert!(runtime.output.blackout);
    assert_eq!(runtime.output.stage_message, "Preserved");
}
