use lumen_core::{Background, CueAction, Project, Runtime, load_project, save_project};
use std::path::PathBuf;

#[test]
fn pdf_and_deck_cues_round_trip_without_schema_bump() {
    let mut project = Project::demo();
    let slide = &mut project.documents[0].slides[0];
    slide.background = Background::Pdf {
        path: "/media/vectors.pdf".into(),
        page: 2,
    };
    slide.actions = vec![CueAction::PlayDeck {
        path: "/media/animated.key".into(),
    }];
    project.validate().unwrap();
    assert_eq!(project.schema_version, 2);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("show.json");
    save_project(&path, &project).unwrap();
    assert_eq!(load_project(&path).unwrap(), project);
}

#[test]
fn pdf_page_zero_and_empty_deck_paths_are_rejected_atomically() {
    let mut project = Project::demo();
    project.documents[0].slides[0].background = Background::Pdf {
        path: "x.pdf".into(),
        page: 0,
    };
    assert!(project.validate().is_err());
    project.documents[0].slides[0].background = Background::Pdf {
        path: "x.pdf".into(),
        page: 1,
    };
    let mut runtime = Runtime::default();
    let before = runtime.clone();
    assert!(
        runtime
            .apply_actions(
                &project,
                &[CueAction::PlayDeck {
                    path: PathBuf::new()
                }]
            )
            .is_err()
    );
    assert_eq!(runtime, before);
    project.macros[0].actions = vec![CueAction::PlayDeck {
        path: PathBuf::new(),
    }];
    assert!(project.validate().is_err());
}

#[test]
fn deck_is_a_live_snapshot_and_clear_actions_stop_it() {
    let mut project = Project::demo();
    let doc = project.documents[0].id;
    let slide = &mut project.documents[0].slides[0];
    let id = slide.id;
    slide.actions = vec![CueAction::PlayDeck {
        path: "original.pptx".into(),
    }];
    for clear in [
        CueAction::StopDeck,
        CueAction::ClearSlide,
        CueAction::ClearAll,
    ] {
        let mut runtime = Runtime::default();
        runtime.trigger(&project, doc, id).unwrap();
        assert_eq!(runtime.output.deck, Some("original.pptx".into()));
        runtime.apply_actions(&project, &[clear]).unwrap();
        assert!(runtime.output.deck.is_none());
    }
    let mut runtime = Runtime::default();
    runtime.trigger(&project, doc, id).unwrap();
    project.documents[0].slides[0].actions.clear();
    assert_eq!(runtime.output.deck, Some("original.pptx".into()));
}
