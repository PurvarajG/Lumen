use lumen_core::{Background, CueAction, PlaylistItem, Project, Prop, Runtime};
use uuid::Uuid;

fn caption_project() -> Project {
    let mut project = Project::demo();
    project.documents.truncate(1);
    let document = &mut project.documents[0];
    let mut video = document.slides[0].clone();
    video.actions.clear();
    video.background = Background::Video {
        path: "service.mp4".into(),
        looped: true,
        muted: true,
        fit: Default::default(),
    };
    document.slides = vec![video.clone()];
    for _ in 0..3 {
        let mut caption = video.clone();
        caption.id = Uuid::new_v4();
        caption.background = Background::Solid([0, 0, 0, 0]);
        document.slides.push(caption);
    }
    project.playlist = vec![PlaylistItem {
        id: Uuid::new_v4(),
        document_id: document.id,
    }];
    project
}

#[test]
fn captions_preserve_video_across_trigger_next_previous_and_detach_from_edits() {
    let mut p = caption_project();
    let mut r = Runtime::default();
    assert!(r.next(&p).unwrap());
    let video = r.output.live.as_ref().unwrap().slide.background.clone();
    for _ in 0..3 {
        assert!(r.next(&p).unwrap());
        assert_eq!(r.output.effective_background(), Some(&video));
        assert_eq!(r.output.underlay.as_ref(), Some(&video));
    }
    p.documents[0].slides[0].background = Background::Solid([255; 4]);
    assert!(r.previous(&p).unwrap());
    assert_eq!(r.output.effective_background(), Some(&video));
    let last = &p.documents[0].slides[3];
    r.trigger(&p, p.documents[0].id, last.id).unwrap();
    assert_eq!(r.output.effective_background(), Some(&video));
    r.trigger(&p, p.documents[0].id, p.documents[0].slides[0].id)
        .unwrap();
    assert!(r.output.underlay.is_none());
    assert_eq!(
        r.output.effective_background(),
        Some(&Background::Solid([255; 4]))
    );
}

#[test]
fn clear_slide_keeps_the_video_under_a_caption() {
    let p = caption_project();
    let mut r = Runtime::default();
    r.next(&p).unwrap();
    r.next(&p).unwrap();
    let video = r.output.underlay.clone();
    assert!(video.is_some());
    r.apply_actions(&p, &[CueAction::ClearSlide]).unwrap();
    assert!(r.output.clear_text);
    assert_eq!(r.output.effective_background(), video.as_ref());
}

#[test]
fn clearing_all_or_media_prevents_underlay_resurrection() {
    let p = caption_project();
    for action in [CueAction::ClearAll, CueAction::ClearMedia] {
        let mut r = Runtime::default();
        r.next(&p).unwrap();
        r.next(&p).unwrap();
        r.apply_actions(&p, std::slice::from_ref(&action)).unwrap();
        if !matches!(action, CueAction::ClearMedia) {
            assert!(r.output.underlay.is_none());
        }
        if matches!(action, CueAction::ClearMedia) {
            assert!(r.output.media_cleared);
        } else {
            assert!(r.output.effective_background().is_none());
        }
        r.trigger(&p, p.documents[0].id, p.documents[0].slides[2].id)
            .unwrap();
        assert!(r.output.underlay.is_none());
        assert!(r.output.effective_background().is_none());
    }
    let mut r = Runtime::default();
    r.trigger(&p, p.documents[0].id, p.documents[0].slides[1].id)
        .unwrap();
    assert!(r.output.effective_background().is_none());
}

#[test]
fn hide_prop_removes_only_its_snapshot_and_is_validated_atomically() {
    let mut p = caption_project();
    p.props = (0..3)
        .map(|i| Prop {
            id: Uuid::new_v4(),
            name: format!("Overlay {i}"),
            objects: vec![],
        })
        .collect();
    let mut r = Runtime::default();
    for prop in &p.props {
        r.apply_actions(&p, &[CueAction::TriggerProp { id: prop.id }])
            .unwrap();
    }
    let other_themes = vec![
        r.output.prop_theme_snapshots[0].clone(),
        r.output.prop_theme_snapshots[2].clone(),
    ];
    r.apply_actions(&p, &[CueAction::HideProp { id: p.props[1].id }])
        .unwrap();
    assert_eq!(r.output.props, vec![p.props[0].id, p.props[2].id]);
    assert_eq!(
        r.output.prop_snapshots,
        vec![p.props[0].clone(), p.props[2].clone()]
    );
    assert_eq!(r.output.prop_theme_snapshots, other_themes);
    r.apply_actions(&p, &[CueAction::HideProp { id: p.props[1].id }])
        .unwrap();
    let before = r.clone();
    let missing = CueAction::HideProp { id: Uuid::new_v4() };
    assert!(
        r.apply_actions(&p, &[CueAction::ClearAll, missing.clone()])
            .is_err()
    );
    assert_eq!(r, before);
    p.documents[0].slides[0].actions = vec![missing];
    assert!(p.validate().is_err());
}

#[test]
fn old_json_defaults_and_new_fields_round_trip_without_schema_change() {
    let p = caption_project();
    let mut old = serde_json::to_value(&p).unwrap();
    for theme in old["themes"].as_array_mut().unwrap() {
        theme.as_object_mut().unwrap().remove("backdrop");
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old.json");
    std::fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
    let mut loaded = lumen_core::load_project(&path).unwrap();
    assert_eq!(loaded.themes[0].backdrop, lumen_core::Backdrop::None);
    loaded.themes[0].backdrop = lumen_core::Backdrop::BottomGradient;
    loaded.documents[0].slides[0].background = Background::Solid([0, 0, 0, 0]);
    lumen_core::save_project(&path, &loaded).unwrap();
    assert_eq!(lumen_core::load_project(&path).unwrap(), loaded);
    assert_eq!(loaded.schema_version, p.schema_version);
    let mut value = serde_json::to_value(Runtime::default()).unwrap();
    value["output"].as_object_mut().unwrap().remove("underlay");
    let r: Runtime = serde_json::from_value(value).unwrap();
    assert!(r.output.underlay.is_none());
}

#[test]
fn caption_chunks_bound_long_words_without_splitting_gujarati_clusters() {
    let word = "ક્ષ".repeat(60);
    let chunks = lumen_core::caption_chunks(&word, 20);
    assert_eq!(chunks.len(), 4);
    assert!(
        chunks
            .iter()
            .flat_map(|chunk| chunk.lines())
            .all(|line| line == "ક્ષ".repeat(6))
    );
    assert_eq!(chunks.join("").replace('\n', ""), word);
    let chunks = lumen_core::caption_chunks(&"a".repeat(181), 60);
    assert_eq!(chunks.len(), 2);
    assert!(chunks.iter().all(|chunk| chunk.lines().count() <= 3));
    assert!(
        chunks
            .iter()
            .flat_map(|c| c.lines())
            .all(|line| line.len() <= 60)
    );
    assert_eq!(chunks.join("").replace('\n', ""), "a".repeat(181));
}
