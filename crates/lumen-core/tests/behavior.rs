use lumen_core::{
    AudioCue, Background, CoreError, CueAction, Document, Macro, MessageTemplate, Project, Prop,
    Rect, Runtime, Slide, SlideObject, TextAlign, TextStyle, Theme, TimerDef, TimerKind,
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
        timers: vec![TimerDef {
            id: Uuid::new_v4(),
            name: "Countdown".into(),
            kind: TimerKind::Countdown {
                duration_secs: 300.0,
            },
            allow_overrun: false,
            warn_at_secs: None,
        }],
        props: vec![],
        messages: vec![],
        transition: lumen_core::Transition::Cut,
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
fn old_documents_without_deck_source_deserialize() {
    let mut encoded = serde_json::to_value(Document::new("Legacy")).unwrap();
    encoded.as_object_mut().unwrap().remove("source");
    let decoded: Document = serde_json::from_value(encoded).unwrap();
    assert_eq!(decoded.source, None);
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

fn repeated_order() -> Project {
    let theme = Theme::default();
    let tid = theme.id;
    let docs = vec![
        doc("A", tid, &["a"]),
        doc("B", tid, &["b"]),
        doc("C", tid, &["c"]),
    ];
    let ids: Vec<_> = docs.iter().map(|d| d.id).collect();
    let mut p = project_with_docs(docs, theme);
    p.playlist = [ids[0], ids[1], ids[0], ids[2]]
        .into_iter()
        .map(|document_id| lumen_core::PlaylistItem {
            id: Uuid::new_v4(),
            document_id,
        })
        .collect();
    p
}

#[test]
fn repeated_document_advances_through_every_occurrence() {
    let p = repeated_order();
    let mut r = Runtime::default();
    for (row, label) in ["a", "b", "a", "c"].into_iter().enumerate() {
        assert_eq!(
            r.peek_next(&p).unwrap().slide.label,
            label,
            "peek at step {row}"
        );
        assert_eq!(r.next_item_index(&p), Some(row));
        assert!(r.next(&p).unwrap());
        assert_eq!(r.output.live.as_ref().unwrap().slide.label, label);
        assert_eq!(r.live_item_index(&p), Some(row));
    }
    assert!(r.peek_next(&p).is_none());
    assert!(!r.next(&p).unwrap());
    assert_eq!(r.live_item_index(&p), Some(3));
    assert!(r.previous(&p).unwrap());
    assert_eq!(r.live_item_index(&p), Some(2));
    assert!(r.previous(&p).unwrap());
    assert!(r.previous(&p).unwrap());
    assert_eq!(r.live_item_index(&p), Some(0));
}

#[test]
fn triggering_a_repeated_document_keeps_the_live_occurrence() {
    let p = repeated_order();
    let (a, a_slide) = (p.documents[0].id, p.documents[0].slides[0].id);
    let mut r = Runtime::default();
    r.trigger(&p, a, a_slide).unwrap();
    assert_eq!(r.live_item_index(&p), Some(0));
    r.trigger_index(&p, 2).unwrap();
    assert_eq!(r.live_item_index(&p), Some(2));
    r.trigger(&p, a, a_slide).unwrap();
    assert_eq!(r.live_item_index(&p), Some(2));
    // From the last row, the next A match wraps around to the first row.
    r.trigger_index(&p, 3).unwrap();
    r.trigger(&p, a, a_slide).unwrap();
    assert_eq!(r.live_item_index(&p), Some(0));
    assert!(!r.trigger_index(&p, 9).unwrap());
}

#[test]
fn runtime_json_without_cursor_still_loads() {
    let mut value = serde_json::to_value(Runtime::default()).unwrap();
    value.as_object_mut().unwrap().remove("cursor");
    let r: Runtime = serde_json::from_value(value).unwrap();
    assert_eq!(r.cursor, None);
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
fn timers_countdown_pause_resume_reset_expire() {
    let mut p = Project::demo();
    let timer_id = p.timers[0].id;
    p.timers[0].kind = TimerKind::Countdown { duration_secs: 2.2 };
    let mut r = Runtime::default();
    r.sync_timers(&p);

    let display = r.timer_display(&p.timers[0], 0);
    assert_eq!(display, "00:03"); // ceil(2.2) = 3

    // Start
    r.apply_actions(&p, &[CueAction::StartTimer { id: timer_id }])
        .unwrap();
    assert!(r.timer_states[&timer_id].running);

    r.tick(&p, Duration::from_millis(700));
    let display = r.timer_display(&p.timers[0], 0);
    assert_eq!(display, "00:02");

    // Pause
    r.apply_actions(&p, &[CueAction::PauseTimer { id: timer_id }])
        .unwrap();
    r.tick(&p, Duration::from_secs(1));
    assert!((r.timer_states[&timer_id].value_secs - 1.5).abs() < 0.001);

    // Resume & expire
    r.apply_actions(&p, &[CueAction::StartTimer { id: timer_id }])
        .unwrap();
    r.tick(&p, Duration::from_secs(2));
    assert_eq!(r.timer_states[&timer_id].value_secs, 0.0);
    assert!(!r.timer_states[&timer_id].running);

    // Reset
    r.apply_actions(&p, &[CueAction::ResetTimer { id: timer_id }])
        .unwrap();
    assert!(!r.timer_states[&timer_id].running);
    assert!((r.timer_states[&timer_id].value_secs - 2.2).abs() < 0.001);

    // Hour display
    let mut p2 = Project::demo();
    p2.timers[0].kind = TimerKind::Countdown {
        duration_secs: 3600.0,
    };
    let mut r2 = Runtime::default();
    r2.sync_timers(&p2);
    assert_eq!(r2.timer_display(&p2.timers[0], 0), "01:00:00");
}

#[test]
fn timer_elapsed_counts_up() {
    let mut p = Project::demo();
    p.timers[0].kind = TimerKind::Elapsed;
    p.timers[0].name = "Elapsed".into();
    let timer_id = p.timers[0].id;
    let mut r = Runtime::default();
    r.sync_timers(&p);
    assert_eq!(r.timer_states[&timer_id].value_secs, 0.0);

    r.apply_actions(&p, &[CueAction::StartTimer { id: timer_id }])
        .unwrap();
    r.tick(&p, Duration::from_secs(5));
    assert!((r.timer_states[&timer_id].value_secs - 5.0).abs() < 0.001);
    assert_eq!(r.timer_display(&p.timers[0], 0), "00:05");

    // Reset elapsed -> 0
    r.apply_actions(&p, &[CueAction::ResetTimer { id: timer_id }])
        .unwrap();
    assert_eq!(r.timer_states[&timer_id].value_secs, 0.0);
    assert!(!r.timer_states[&timer_id].running);
}

#[test]
fn timer_countdown_overrun() {
    let mut p = Project::demo();
    p.timers[0].kind = TimerKind::Countdown { duration_secs: 1.0 };
    p.timers[0].allow_overrun = true;
    let timer_id = p.timers[0].id;
    let mut r = Runtime::default();
    r.sync_timers(&p);

    r.apply_actions(&p, &[CueAction::StartTimer { id: timer_id }])
        .unwrap();
    r.tick(&p, Duration::from_secs(3));
    // Should be negative (overrun)
    assert!(r.timer_states[&timer_id].value_secs < 0.0);
    assert!(r.timer_states[&timer_id].running); // still running with overrun

    let display = r.timer_display(&p.timers[0], 0);
    assert!(display.starts_with('-'));
}

#[test]
fn timer_countdown_to_across_target() {
    let p_timer = TimerDef {
        id: Uuid::new_v4(),
        name: "CountdownTo".into(),
        kind: TimerKind::CountdownTo {
            hour: 10,
            minute: 0,
            second: 0,
        },
        allow_overrun: false,
        warn_at_secs: None,
    };

    let mut p = Project::demo();
    p.timers = vec![p_timer];
    let mut r = Runtime::default();
    r.sync_timers(&p);

    // 09:59:00 -> target 10:00:00 = 60 seconds remaining
    let now = 9 * 3600 + 59 * 60; // 09:59:00
    let display = r.timer_display(&p.timers[0], now);
    assert_eq!(display, "01:00");

    // After target (10:01:00), no overrun -> shows 00:00
    let now_after = 10 * 3600 + 60; // 10:01:00
    let display = r.timer_display(&p.timers[0], now_after);
    assert_eq!(display, "00:00");

    // With overrun
    p.timers[0].allow_overrun = true;
    let display = r.timer_display(&p.timers[0], now_after);
    assert!(display.starts_with('-'));
}

#[test]
fn action_batches_are_ordered_bounded_and_invalid_batches_are_atomic() {
    let p = Project::demo();
    let timer = p.timers[0].id;
    let mut r = Runtime::default();
    r.apply_actions(
        &p,
        &[
            CueAction::Blackout(true),
            CueAction::Blackout(false),
            CueAction::StageMessage("Ready".into()),
            CueAction::StartTimer { id: timer },
            CueAction::PauseTimer { id: timer },
        ],
    )
    .unwrap();
    assert!(!r.output.blackout);
    assert_eq!(r.output.stage_message, "Ready");
    assert!(!r.timer_states[&timer].running);
    let before = r.clone();
    let missing = Uuid::new_v4();
    assert!(
        r.apply_actions(
            &p,
            &[
                CueAction::Blackout(true),
                CueAction::StartTimer { id: missing }
            ]
        )
        .is_err()
    );
    assert_eq!(r, before);
    let too_many = vec![CueAction::ClearAll; 65];
    assert!(r.apply_actions(&p, &too_many).is_err());
    let mut p2 = Project::demo();
    let id = Uuid::new_v4();
    p2.macros.push(Macro {
        id,
        name: "bad".into(),
        actions: vec![CueAction::ResetTimer { id: Uuid::new_v4() }],
    });
    assert!(r.run_macro(&p2, id).is_err());
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
    // Note: text is now escaped, so the content should survive strip_markup
    let stripped_0 = lumen_core::markup::strip_markup(&d.slides[0].text);
    assert_eq!(stripped_0, "  First\nline");
    let stripped_1 = lumen_core::markup::strip_markup(&d.slides[1].text);
    assert_eq!(stripped_1, "Second\nThird");
    assert!(
        document_from_text("empty", " \r\n \t\n", theme)
            .slides
            .is_empty()
    );
}

#[test]
fn text_import_escapes_markup_characters() {
    let theme = Uuid::new_v4();
    let d = document_from_text("Test", "Hello *world* and [bracket]", theme);
    // The slide text should be escaped
    let plain = lumen_core::markup::strip_markup(&d.slides[0].text);
    assert_eq!(plain, "Hello *world* and [bracket]");
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
    let p = Project::demo();
    let mut runtime = Runtime::default();
    runtime.sync_timers(&p);
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
    let timer_id = project.timers[0].id;
    runtime
        .apply_actions(&project, &[CueAction::StartTimer { id: timer_id }])
        .unwrap();

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
fn run_macro_is_ordered_and_clear_all_clears_props_and_message() {
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
    // ClearAll should also clear props and message
    assert!(runtime.output.props.is_empty());
    assert!(runtime.output.message.is_none());

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
    runtime
        .apply_actions(&project, &[CueAction::ClearAll])
        .unwrap();
    assert!(runtime.output.live.is_none());
    assert!(!runtime.output.clear_text);
    assert!(runtime.output.blackout);
    assert_eq!(runtime.output.stage_message, "Preserved");
}

// ---------------------------------------------------------------------------
// v2 Schema tests
// ---------------------------------------------------------------------------

#[test]
fn v1_fixture_loads_and_migrates() {
    let v1_json = r#"{
        "schema_version": 1,
        "name": "V1 Show",
        "documents": [],
        "playlist": [],
        "themes": [
            {
                "id": "00000000-0000-0000-0000-000000000001",
                "name": "Default",
                "font_size": 64.0,
                "text_color": [245, 246, 248, 255],
                "background_color": [18, 22, 30, 255],
                "align": "Center",
                "vertical_align": "Middle"
            }
        ],
        "macros": []
    }"#;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("v1.json");
    fs::write(&path, v1_json).unwrap();
    let project = load_project(&path).unwrap();
    assert_eq!(project.schema_version, 2);
    assert!(!project.timers.is_empty());
    assert_eq!(project.timers[0].name, "Countdown");
    assert!(project.props.is_empty());
    assert!(project.messages.is_empty());
}

#[test]
fn v2_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("v2.json");
    let mut p = Project::demo();
    // Add some v2 features
    let prop_id = Uuid::new_v4();
    p.props.push(Prop {
        id: prop_id,
        name: "Lower third".into(),
        objects: vec![SlideObject::Text {
            id: Uuid::new_v4(),
            rect: Rect::default(),
            text: "Speaker Name".into(),
            style: TextStyle::default(),
            align: TextAlign::Center,
        }],
    });
    let msg_id = Uuid::new_v4();
    p.messages.push(MessageTemplate {
        id: msg_id,
        name: "Timer msg".into(),
        text: format!("Time: {{timer:{}}}", p.timers[0].id),
    });
    save_project(&path, &p).unwrap();
    let loaded = load_project(&path).unwrap();
    assert_eq!(loaded, p);
}

#[test]
fn validation_rejects_invalid_rect() {
    let mut p = Project::demo();
    p.documents[0].slides[0].text_rect = Rect {
        x: -0.1,
        y: 0.0,
        w: 0.5,
        h: 0.5,
    };
    assert!(matches!(p.validate(), Err(CoreError::InvalidRect { .. })));
}

#[test]
fn validation_rejects_invalid_text_style_font_size() {
    let mut p = Project::demo();
    p.documents[0].slides[0].text_style.font_size = Some(5.0);
    assert!(matches!(
        p.validate(),
        Err(CoreError::InvalidTextStyleFontSize { .. })
    ));
}

#[test]
fn validation_rejects_invalid_audio_volume() {
    let mut p = Project::demo();
    p.documents[0].slides[0].audio = Some(AudioCue {
        path: "audio.mp3".into(),
        volume: 1.5,
    });
    assert!(matches!(p.validate(), Err(CoreError::InvalidVolume { .. })));
}

#[test]
fn validation_rejects_invalid_timer_kind() {
    let mut p = Project::demo();
    p.timers[0].kind = TimerKind::Countdown {
        duration_secs: -1.0,
    };
    assert!(matches!(
        p.validate(),
        Err(CoreError::InvalidTimerKind { .. })
    ));

    let mut p = Project::demo();
    p.timers[0].kind = TimerKind::CountdownTo {
        hour: 25,
        minute: 0,
        second: 0,
    };
    assert!(matches!(
        p.validate(),
        Err(CoreError::InvalidTimerKind { .. })
    ));
}

#[test]
fn validation_rejects_invalid_corner_radius() {
    let mut p = Project::demo();
    let obj_id = Uuid::new_v4();
    p.documents[0].slides[0].objects.push(SlideObject::Shape {
        id: obj_id,
        rect: Rect::default(),
        color: [255, 0, 0, 255],
        corner_radius: 0.6,
    });
    assert!(matches!(
        p.validate(),
        Err(CoreError::InvalidCornerRadius { .. })
    ));
}

#[test]
fn validation_rejects_too_many_objects() {
    let mut p = Project::demo();
    for _ in 0..33 {
        p.documents[0].slides[0].objects.push(SlideObject::Shape {
            id: Uuid::new_v4(),
            rect: Rect::default(),
            color: [0, 0, 0, 255],
            corner_radius: 0.0,
        });
    }
    assert!(matches!(
        p.validate(),
        Err(CoreError::TooManyObjects { .. })
    ));
}

#[test]
fn validation_rejects_missing_timer_prop_message_refs_in_cue() {
    // Missing timer ref
    let mut p = Project::demo();
    p.documents[0].slides[0].actions = vec![CueAction::StartTimer { id: Uuid::new_v4() }];
    assert!(matches!(p.validate(), Err(CoreError::MissingTimer(_))));

    // Missing prop ref
    let mut p = Project::demo();
    p.documents[0].slides[0].actions = vec![CueAction::TriggerProp { id: Uuid::new_v4() }];
    assert!(matches!(p.validate(), Err(CoreError::MissingProp(_))));

    // Missing message ref
    let mut p = Project::demo();
    p.documents[0].slides[0].actions = vec![CueAction::ShowMessage { id: Uuid::new_v4() }];
    assert!(matches!(p.validate(), Err(CoreError::MissingMessage(_))));
}

#[test]
fn validation_rejects_duplicate_ids_across_types() {
    let mut p = Project::demo();
    let dup = p.timers[0].id;
    p.props.push(Prop {
        id: dup,
        name: "dup".into(),
        objects: vec![],
    });
    assert!(matches!(p.validate(), Err(CoreError::DuplicateId(_))));
}

// ---------------------------------------------------------------------------
// Props / message actions
// ---------------------------------------------------------------------------

#[test]
fn props_trigger_clear_and_no_duplicates() {
    let mut p = Project::demo();
    let prop_id = Uuid::new_v4();
    p.props.push(Prop {
        id: prop_id,
        name: "Lower third".into(),
        objects: vec![],
    });
    let mut r = Runtime::default();

    r.apply_actions(&p, &[CueAction::TriggerProp { id: prop_id }])
        .unwrap();
    assert_eq!(r.output.props, vec![prop_id]);

    // Triggering same prop again: no duplicate
    r.apply_actions(&p, &[CueAction::TriggerProp { id: prop_id }])
        .unwrap();
    assert_eq!(r.output.props, vec![prop_id]);

    // Clear
    r.apply_actions(&p, &[CueAction::ClearProps]).unwrap();
    assert!(r.output.props.is_empty());
}

#[test]
fn message_show_clear() {
    let mut p = Project::demo();
    let msg_id = Uuid::new_v4();
    p.messages.push(MessageTemplate {
        id: msg_id,
        name: "Welcome".into(),
        text: "Welcome all!".into(),
    });
    let mut r = Runtime::default();

    r.apply_actions(&p, &[CueAction::ShowMessage { id: msg_id }])
        .unwrap();
    assert_eq!(r.output.message, Some(msg_id));

    r.apply_actions(&p, &[CueAction::ClearMessage]).unwrap();
    assert_eq!(r.output.message, None);
}

#[test]
fn render_message_replaces_timer_tokens() {
    let mut p = Project::demo();
    let timer_id = p.timers[0].id;
    let msg_id = Uuid::new_v4();
    p.messages.push(MessageTemplate {
        id: msg_id,
        name: "Timer".into(),
        text: format!("Time left: {{timer:{}}}", timer_id),
    });
    let mut r = Runtime::default();
    r.sync_timers(&p);
    let rendered = r.render_message(&p, msg_id, 0).unwrap();
    assert!(rendered.contains("05:00")); // 300 secs = 05:00
}

#[test]
fn render_message_unknown_token_left_literal() {
    let mut p = Project::demo();
    let msg_id = Uuid::new_v4();
    p.messages.push(MessageTemplate {
        id: msg_id,
        name: "Bad".into(),
        text: format!("Time: {{timer:{}}}", Uuid::new_v4()),
    });
    let r = Runtime::default();
    let rendered = r.render_message(&p, msg_id, 0).unwrap();
    assert!(rendered.contains("{timer:"));
}

#[test]
fn render_message_replaces_tokens_after_unknown_or_unclosed_ones() {
    let mut p = Project::demo();
    let timer_id = p.timers[0].id;
    let msg_id = Uuid::new_v4();
    p.messages.push(MessageTemplate {
        id: msg_id,
        name: "Mixed".into(),
        text: format!("{{timer:nope}} then {{timer:{timer_id}}} and {{timer:open"),
    });
    let mut r = Runtime::default();
    r.sync_timers(&p);
    let rendered = r.render_message(&p, msg_id, 0).unwrap();
    assert_eq!(rendered, "{timer:nope} then 05:00 and {timer:open");
}

#[test]
fn stop_audio_increments_serial() {
    let p = Project::demo();
    let mut r = Runtime::default();
    assert_eq!(r.output.stop_audio_serial, 0);
    r.apply_actions(&p, &[CueAction::StopAudio]).unwrap();
    assert_eq!(r.output.stop_audio_serial, 1);
    r.apply_actions(&p, &[CueAction::StopAudio]).unwrap();
    assert_eq!(r.output.stop_audio_serial, 2);
}

#[test]
fn clear_all_clears_props_and_message() {
    let mut p = Project::demo();
    let prop_id = Uuid::new_v4();
    p.props.push(Prop {
        id: prop_id,
        name: "test".into(),
        objects: vec![],
    });
    let msg_id = Uuid::new_v4();
    p.messages.push(MessageTemplate {
        id: msg_id,
        name: "test".into(),
        text: "test".into(),
    });
    let mut r = Runtime::default();
    r.apply_actions(
        &p,
        &[
            CueAction::TriggerProp { id: prop_id },
            CueAction::ShowMessage { id: msg_id },
        ],
    )
    .unwrap();
    assert!(!r.output.props.is_empty());
    assert!(r.output.message.is_some());

    r.apply_actions(&p, &[CueAction::ClearAll]).unwrap();
    assert!(r.output.props.is_empty());
    assert!(r.output.message.is_none());
}

// ---------------------------------------------------------------------------
// History tests
// ---------------------------------------------------------------------------

#[test]
fn history_capacity_undo_redo_branch_clear() {
    use lumen_core::history::History;

    let mut h = History::new(3);
    // Record 4 items (capacity 3, so oldest dropped)
    h.record(1);
    h.record(2);
    h.record(3);
    h.record(4); // 1 dropped

    assert!(h.can_undo());
    assert!(!h.can_redo());

    let v = h.undo(5).unwrap();
    assert_eq!(v, 4);
    let v = h.undo(4).unwrap();
    assert_eq!(v, 3);
    let v = h.undo(3).unwrap();
    assert_eq!(v, 2);
    assert!(h.undo(2).is_none()); // 1 was dropped

    // Redo
    let v = h.redo(2).unwrap();
    assert_eq!(v, 3);

    // Branch: record clears redo
    h.record(10);
    assert!(!h.can_redo());

    // Clear
    h.clear();
    assert!(!h.can_undo());
    assert!(!h.can_redo());
}

// ---------------------------------------------------------------------------
// Edit/live isolation for new fields
// ---------------------------------------------------------------------------

#[test]
fn edit_live_isolation_for_new_fields() {
    let mut p = Project::demo();
    let doc_id = p.documents[0].id;
    let slide_id = p.documents[0].slides[0].id;

    // Set some v2 fields
    p.documents[0].slides[0].text_rect = Rect {
        x: 0.1,
        y: 0.1,
        w: 0.8,
        h: 0.8,
    };
    p.documents[0].slides[0].objects.push(SlideObject::Shape {
        id: Uuid::new_v4(),
        rect: Rect::default(),
        color: [255, 0, 0, 255],
        corner_radius: 0.1,
    });

    let mut r = Runtime::default();
    r.trigger(&p, doc_id, slide_id).unwrap();

    let live_before = r.output.live.clone().unwrap();

    // Edit the project slide
    p.documents[0].slides[0].text_rect = Rect {
        x: 0.2,
        y: 0.2,
        w: 0.6,
        h: 0.6,
    };
    p.documents[0].slides[0].objects.clear();

    // Live output should be unchanged (cloned snapshot)
    assert_eq!(r.output.live.as_ref().unwrap(), &live_before);
    assert_eq!(
        r.output.live.as_ref().unwrap().slide.text_rect,
        Rect {
            x: 0.1,
            y: 0.1,
            w: 0.8,
            h: 0.8,
        }
    );
    assert_eq!(r.output.live.as_ref().unwrap().slide.objects.len(), 1);
}

// ---------------------------------------------------------------------------
// Markup tests (behavior-level)
// ---------------------------------------------------------------------------

#[test]
fn markup_parse_strip_escape_integration() {
    use lumen_core::markup::{escape_markup, parse_markup, strip_markup};

    // Nested bold+italic
    let spans = parse_markup("**bold *and italic* only bold**");
    assert_eq!(spans.len(), 3);
    assert!(spans[0].bold && !spans[0].italic);
    assert!(spans[1].bold && spans[1].italic);
    assert!(spans[2].bold && !spans[2].italic);

    // Colour
    let spans = parse_markup("[#FF0000]red[/]");
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].color, Some([255, 0, 0, 255]));
    assert_eq!(spans[0].text, "red");

    // Escapes
    assert_eq!(strip_markup("\\* \\_ \\[ \\\\"), "* _ [ \\");

    // Unclosed
    let spans = parse_markup("unclosed **bold");
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].text, "unclosed **bold");

    // Unicode
    let spans = parse_markup("**日本語**");
    assert_eq!(spans.len(), 1);
    assert!(spans[0].bold);
    assert_eq!(spans[0].text, "日本語");

    // Roundtrip property
    let test_strings = [
        "",
        "hello",
        "**bold**",
        "*italic*",
        "__underline__",
        "[#FF0000]red[/]",
        "a * b _ c [ d \\ e",
    ];
    for s in &test_strings {
        assert_eq!(
            strip_markup(&escape_markup(s)),
            *s,
            "roundtrip failed for {:?}",
            s
        );
    }
}

// ---------------------------------------------------------------------------
// Sync timers
// ---------------------------------------------------------------------------

#[test]
fn sync_timers_adds_new_and_drops_removed() {
    let mut p = Project::demo();
    let mut r = Runtime::default();
    r.sync_timers(&p);
    assert_eq!(r.timer_states.len(), 1);
    assert!(r.timer_states.contains_key(&p.timers[0].id));

    // Add a new timer
    let new_id = Uuid::new_v4();
    p.timers.push(TimerDef {
        id: new_id,
        name: "Elapsed".into(),
        kind: TimerKind::Elapsed,
        allow_overrun: false,
        warn_at_secs: None,
    });
    r.sync_timers(&p);
    assert_eq!(r.timer_states.len(), 2);
    assert!(r.timer_states.contains_key(&new_id));

    // Remove first timer
    let old_id = p.timers[0].id;
    p.timers.remove(0);
    r.sync_timers(&p);
    assert_eq!(r.timer_states.len(), 1);
    assert!(!r.timer_states.contains_key(&old_id));
}

// ---------------------------------------------------------------------------
// Schema version 0 rejected
// ---------------------------------------------------------------------------

#[test]
fn schema_version_0_rejected() {
    let v0_json = r#"{
        "schema_version": 0,
        "name": "Bad",
        "documents": [],
        "playlist": [],
        "themes": [{"id": "00000000-0000-0000-0000-000000000001", "name": "D", "font_size": 64.0, "text_color": [0,0,0,255], "background_color": [0,0,0,255], "align": "Center", "vertical_align": "Middle"}],
        "macros": []
    }"#;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("v0.json");
    fs::write(&path, v0_json).unwrap();
    assert!(matches!(
        load_project(&path),
        Err(CoreError::UnsupportedSchema { .. })
    ));
}
