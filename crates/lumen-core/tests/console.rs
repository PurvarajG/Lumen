use lumen_core::{CueAction, Project, PushDirection, Runtime, Transition};

#[test]
fn independent_clear_actions_and_retrigger_restore_only_their_layers() {
    let p = Project::demo();
    let mut r = Runtime::default();
    let d = &p.documents[0];
    r.trigger(&p, d.id, d.slides[0].id).unwrap();
    r.apply_actions(
        &p,
        &[
            CueAction::ClearSlide,
            CueAction::ClearMedia,
            CueAction::ClearAudio,
            CueAction::ClearMessages,
        ],
    )
    .unwrap();
    assert!(r.output.clear_text);
    assert!(r.output.media_cleared);
    assert_eq!(r.output.audio_clear_serial, 1);
    assert!(r.output.message.is_none());
    r.trigger(&p, d.id, d.slides[1].id).unwrap();
    assert!(!r.output.clear_text);
    assert!(!r.output.media_cleared);
    assert_eq!(r.output.audio_clear_serial, 1);
    r.apply_actions(&p, &[CueAction::ClearAll]).unwrap();
    assert!(r.output.live.is_none());
    assert!(r.output.media_cleared);
    assert_eq!(r.output.audio_clear_serial, 2);
}

#[test]
fn transitions_reject_overlong_durations_and_round_trip() {
    let mut p = Project::demo();
    for kind in [
        Transition::Fade { millis: 10001 },
        Transition::FadeThroughBlack { millis: 10001 },
        Transition::Push {
            millis: 10001,
            direction: PushDirection::Left,
        },
    ] {
        p.documents[0].slides[0].transition = kind;
        assert!(p.validate().is_err());
    }
    for kind in [
        Transition::Cut,
        Transition::Fade { millis: 0 },
        Transition::FadeThroughBlack { millis: 10000 },
        Transition::Push {
            millis: 800,
            direction: PushDirection::Down,
        },
    ] {
        p.documents[0].slides[0].transition = kind.clone();
        p.validate().unwrap();
        let encoded = serde_json::to_string(&p).unwrap();
        assert_eq!(
            serde_json::from_str::<Project>(&encoded).unwrap().documents[0].slides[0].transition,
            kind
        );
    }
}

#[test]
fn warning_threshold_is_backward_compatible_and_validated() {
    let mut p = Project::demo();
    let mut json = serde_json::to_value(&p).unwrap();
    json["timers"][0]
        .as_object_mut()
        .unwrap()
        .remove("warn_at_secs");
    assert_eq!(
        serde_json::from_value::<Project>(json).unwrap().timers[0].warn_at_secs,
        None
    );
    p.timers[0].warn_at_secs = Some(f64::NAN);
    assert!(p.validate().is_err());
    p.timers[0].warn_at_secs = Some(-1.0);
    assert!(p.validate().is_err());
    p.timers[0].warn_at_secs = Some(30.0);
    p.validate().unwrap();
}

#[test]
fn show_transition_defaults_to_fade_and_is_validated() {
    let mut json = serde_json::to_value(Project::demo()).unwrap();
    json.as_object_mut().unwrap().remove("transition");
    let mut p: Project = serde_json::from_value(json).unwrap();
    assert_eq!(p.transition, Transition::Fade { millis: 400 });
    p.validate().unwrap();
    p.transition = Transition::Fade { millis: 10001 };
    assert!(p.validate().is_err());
}
