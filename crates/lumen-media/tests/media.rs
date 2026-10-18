use lumen_media::{MediaError, MediaPlayer, PlaybackOptions, probe, tool_availability};
use std::{
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::{Duration, Instant},
};

fn tools_available() -> bool {
    tool_availability().into_iter().all(|(_, ok)| ok)
}
fn make_video(dir: &Path, name: &str, duration: f64, audio: bool) -> PathBuf {
    let path = dir.join(name);
    let mut cmd = Command::new("ffmpeg");
    cmd.args([
        "-hide_banner",
        "-loglevel",
        "error",
        "-y",
        "-f",
        "lavfi",
        "-i",
        "testsrc=size=96x54:rate=12",
    ]);
    if audio {
        cmd.args(["-f", "lavfi", "-i", "sine=frequency=440:sample_rate=44100"]);
    }
    cmd.args(["-t", &duration.to_string(), "-pix_fmt", "yuv420p"]);
    if audio {
        cmd.args(["-c:a", "aac", "-shortest"]);
    }
    let status = cmd
        .arg(&path)
        .status()
        .expect("ffmpeg fixture generator launches");
    assert!(status.success(), "fixture generation failed");
    path
}

fn options(looped: bool) -> PlaybackOptions {
    PlaybackOptions {
        width: 64,
        height: 64,
        fps: 12,
        looped,
        muted: true,
    }
}

#[test]
fn rejects_invalid_inputs_and_options() {
    if !tools_available() {
        return;
    }
    assert!(probe(Path::new("/definitely/missing/lumen-media.mov")).is_err());
    assert!(matches!(
        MediaPlayer::open(Path::new("/missing"), options(false)),
        Err(MediaError::InvalidPath(_))
    ));
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("garbage.mp4");
    std::fs::write(&file, b"not media").unwrap();
    assert!(probe(&file).is_err());
    assert!(MediaPlayer::open(&file, options(false)).is_err());
    let valid = make_video(dir.path(), "valid.mp4", 1.0, false);
    let mut bad = options(false);
    bad.width = 0;
    assert!(matches!(
        MediaPlayer::open(&valid, bad),
        Err(MediaError::InvalidOptions(_))
    ));
}

#[test]
fn probes_and_decodes_generated_media_at_fitted_size() {
    if !tools_available() {
        eprintln!("SKIP: required FFmpeg tools unavailable");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let path = make_video(dir.path(), "video with spaces.mp4", 1.5, false);
    let info = probe(&path).unwrap();
    assert_eq!((info.width, info.height), (96, 54));
    assert!(info.duration_secs.unwrap() > 1.0);
    assert!(!info.has_audio);
    assert_eq!(info.fps, Some(12.0));
    let player = MediaPlayer::open(&path, options(false)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    let frame = loop {
        if let Some(frame) = player.take_frame() {
            break frame;
        }
        assert!(
            player.error().is_none(),
            "decode error: {:?}",
            player.error()
        );
        assert!(Instant::now() < deadline, "first frame timed out");
        thread::sleep(Duration::from_millis(10));
    };
    // 96x54 fitted into 64x64 keeps its aspect ratio; no letterbox bars.
    assert_eq!((frame.width, frame.height), (64, 36));
    assert_eq!(frame.rgba.len(), 64 * 36 * 4);
    assert!(frame.sequence > 0);
    assert!(
        player
            .take_frame()
            .is_none_or(|next| next.sequence > frame.sequence),
        "a taken frame must not be handed out twice"
    );
}

#[test]
fn non_looping_playback_finishes_and_stop_is_idempotent() {
    if !tools_available() {
        eprintln!("SKIP: required FFmpeg tools unavailable");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let path = make_video(dir.path(), "short.mp4", 0.6, false);
    let mut player = MediaPlayer::open(&path, options(false)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(4);
    while !player.is_finished() {
        assert!(Instant::now() < deadline, "playback did not finish");
        thread::sleep(Duration::from_millis(20));
    }
    assert!(
        player.error().is_none(),
        "playback error: {:?}",
        player.error()
    );
    assert!(player.take_frame().is_some());
    player.stop();
    player.stop();
}

#[test]
fn pause_and_drop_clean_up_owned_children() {
    if !tools_available() {
        eprintln!("SKIP: required FFmpeg tools unavailable");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let path = make_video(dir.path(), "long.mp4", 8.0, false);
    let mut player = MediaPlayer::open(&path, options(true)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while player.take_frame().is_none() {
        assert!(Instant::now() < deadline, "first frame timed out");
        thread::sleep(Duration::from_millis(10));
    }
    player.set_paused(true).unwrap();
    assert!(player.is_paused());
    thread::sleep(Duration::from_millis(100)); // let an in-flight frame land
    let _ = player.take_frame();
    thread::sleep(Duration::from_millis(180));
    assert!(player.take_frame().is_none(), "frames arrived while paused");
    player.set_paused(false).unwrap();
    assert!(!player.is_paused());
    let deadline = Instant::now() + Duration::from_secs(2);
    while player.take_frame().is_none() {
        assert!(Instant::now() < deadline, "decoder did not resume");
        thread::sleep(Duration::from_millis(10));
    }
    player.stop();
    assert!(player.is_finished());
    drop(player);
}

#[test]
fn path_with_shell_metacharacters_is_passed_as_one_argument() {
    if !tools_available() {
        eprintln!("SKIP: required FFmpeg tools unavailable");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let path = make_video(dir.path(), "x; touch SHOULD_NOT_EXIST.mp4", 0.5, false);
    assert_eq!(probe(&path).unwrap().width, 96);
    assert!(!dir.path().join("SHOULD_NOT_EXIST.mp4").exists());
}
