use crate::{MediaError, validate_file};
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, Source};
use std::{
    fs::File,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

#[derive(Clone, Copy, Debug)]
pub struct AudioOptions {
    pub volume: f32,
    pub looped: bool,
    pub fade_in_ms: u32,
}

impl Default for AudioOptions {
    fn default() -> Self {
        Self {
            volume: 1.0,
            looped: false,
            fade_in_ms: 0,
        }
    }
}

pub fn output_devices() -> Result<Vec<String>, MediaError> {
    use rodio::cpal::traits::{DeviceTrait, HostTrait};
    rodio::cpal::default_host()
        .output_devices()
        .map_err(|e| MediaError::Process(format!("could not enumerate audio devices: {e}")))?
        .map(|device| {
            device
                .name()
                .map_err(|e| MediaError::Process(format!("could not read audio device name: {e}")))
        })
        .collect()
}

pub struct AudioPlayer {
    _stream: OutputStream,
    sink: Option<Arc<Sink>>,
    fading: Option<Arc<Sink>>,
    fade_cancel: Option<Arc<AtomicBool>>,
    file_name: Option<String>,
    duration: Option<Duration>,
}

impl AudioPlayer {
    pub fn new(device: Option<&str>) -> Result<Self, MediaError> {
        use rodio::cpal::traits::{DeviceTrait, HostTrait};
        let stream = if let Some(name) = device {
            let device = rodio::cpal::default_host()
                .output_devices()
                .map_err(|e| {
                    MediaError::Process(format!("could not enumerate audio devices: {e}"))
                })?
                .find(|d| d.name().is_ok_and(|device_name| device_name == name))
                .ok_or_else(|| {
                    MediaError::InvalidOptions(format!("unknown audio output device: {name}"))
                })?;
            OutputStreamBuilder::from_device(device)
                .and_then(|builder| builder.open_stream())
                .map_err(|e| {
                    MediaError::Process(format!("could not open audio device {name}: {e}"))
                })?
        } else {
            OutputStreamBuilder::from_default_device()
                .and_then(|builder| builder.open_stream())
                .map_err(|e| {
                    MediaError::ToolUnavailable(format!("no usable audio output device: {e}"))
                })?
        };
        Ok(Self {
            _stream: stream,
            sink: None,
            fading: None,
            fade_cancel: None,
            file_name: None,
            duration: None,
        })
    }

    pub fn play(&mut self, path: &Path, options: AudioOptions) -> Result<(), MediaError> {
        if !options.volume.is_finite() || !(0.0..=1.0).contains(&options.volume) {
            return Err(MediaError::InvalidOptions(
                "audio volume must be between 0 and 1".into(),
            ));
        }
        validate_file(path)?;
        let file = File::open(path)
            .map_err(|e| MediaError::InvalidPath(format!("{}: {e}", path.display())))?;
        let source = Decoder::try_from(file)
            .map_err(|e| MediaError::InvalidMedia(format!("{}: {e}", path.display())))?;
        let duration = source.total_duration();
        self.cancel_fade();
        if let Some(old) = self.sink.take() {
            old.stop();
        }
        let sink = Arc::new(Sink::connect_new(self._stream.mixer()));
        sink.set_volume(if options.fade_in_ms == 0 {
            options.volume
        } else {
            0.0
        });
        if options.looped {
            sink.append(source.repeat_infinite());
        } else {
            sink.append(source);
        }
        if options.fade_in_ms > 0 {
            self.fade_cancel = Some(fade(
                Arc::clone(&sink),
                options.volume,
                options.fade_in_ms,
                false,
            ));
        }
        self.sink = Some(sink);
        self.duration = duration;
        self.file_name = path.file_name().map(|n| n.to_string_lossy().into_owned());
        Ok(())
    }

    pub fn stop(&mut self, fade_out_ms: u32) {
        if let Some(cancel) = self.fade_cancel.take() {
            cancel.store(true, Ordering::Release);
        }
        if let Some(fading) = self.fading.take() {
            fading.stop();
        }
        let Some(sink) = self.sink.take() else { return };
        if fade_out_ms == 0 {
            sink.stop();
        } else {
            self.fade_cancel = Some(fade(Arc::clone(&sink), 0.0, fade_out_ms, true));
            self.fading = Some(sink);
        }
    }

    pub fn set_volume(&mut self, volume: f32) -> Result<(), MediaError> {
        if !volume.is_finite() || !(0.0..=1.0).contains(&volume) {
            return Err(MediaError::InvalidOptions(
                "audio volume must be between 0 and 1".into(),
            ));
        }
        if let Some(sink) = &self.sink {
            sink.set_volume(volume);
        }
        Ok(())
    }

    pub fn pause(&self) {
        if let Some(sink) = &self.sink {
            sink.pause();
        }
    }

    pub fn resume(&self) {
        if let Some(sink) = &self.sink {
            sink.play();
        }
    }

    pub fn is_paused(&self) -> bool {
        self.sink.as_ref().is_some_and(|sink| sink.is_paused())
    }

    /// Length of the current file, when the decoder knows it.
    pub fn duration(&self) -> Option<Duration> {
        self.duration
    }

    pub fn file_name(&self) -> Option<&str> {
        self.file_name.as_deref()
    }

    pub fn is_playing(&self) -> bool {
        self.sink.as_ref().is_some_and(|sink| !sink.empty())
    }

    pub fn position(&self) -> Option<Duration> {
        self.sink
            .as_ref()
            .filter(|sink| !sink.empty())
            .map(|sink| sink.get_pos())
    }

    fn cancel_fade(&mut self) {
        if let Some(cancel) = self.fade_cancel.take() {
            cancel.store(true, Ordering::Release);
        }
        if let Some(fading) = self.fading.take() {
            fading.stop();
        }
    }
}

fn fade(sink: Arc<Sink>, target: f32, ms: u32, stop: bool) -> Arc<AtomicBool> {
    let start = sink.volume();
    let cancel = Arc::new(AtomicBool::new(false));
    let thread_cancel = Arc::clone(&cancel);
    let thread_sink = Arc::clone(&sink);
    let spawned = thread::Builder::new()
        .name("lumen-media-audio-fade".into())
        .spawn(move || {
            let steps = (ms / 10).clamp(1, 100) as u64;
            for step in 1..=steps {
                thread::sleep(Duration::from_millis((ms as u64 / steps).max(1)));
                if thread_cancel.load(Ordering::Acquire) {
                    return;
                }
                let fraction = step as f32 / steps as f32;
                thread_sink.set_volume(start + (target - start) * fraction);
            }
            if stop {
                thread_sink.stop();
            }
        });
    if spawned.is_err() {
        if stop {
            sink.stop();
        } else {
            sink.set_volume(target);
        }
    }
    cancel
}

impl Drop for AudioPlayer {
    fn drop(&mut self) {
        self.cancel_fade();
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }
    }
}

#[cfg(test)]
mod ogg_decode_tests {
    use super::*;
    use std::io::BufReader;
    use std::process::Command;

    #[test]
    fn decodes_generated_ogg_vorbis_when_ffmpeg_encoder_is_available() {
        let Ok(output) = Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:sample_rate=44100",
                "-t",
                "1",
                "-c:a",
                "vorbis",
                "-strict",
                "-2",
            ])
            .arg("-f")
            .arg("ogg")
            .arg("pipe:1")
            .output()
        else {
            eprintln!("SKIP: ffmpeg unavailable; cannot generate OGG fixture");
            return;
        };
        if !output.status.success() {
            eprintln!(
                "SKIP: ffmpeg has no Vorbis encoder: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        let decoder = Decoder::try_from(BufReader::new(std::io::Cursor::new(output.stdout)))
            .expect("generated OGG/Vorbis should decode");
        let samples = decoder.count();
        assert!(
            samples > 40_000,
            "expected about one second of audio, got {samples} samples"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        process::Command,
        time::{Duration, Instant},
    };

    fn wav_fixture() -> Option<(tempfile::TempDir, std::path::PathBuf)> {
        if !crate::tool_availability()
            .into_iter()
            .find(|(name, _)| name == "ffmpeg")
            .is_some_and(|(_, ok)| ok)
        {
            eprintln!("SKIP: ffmpeg unavailable; cannot generate audio fixture");
            return None;
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tone.wav");
        let output = Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=1",
                "-c:a",
                "pcm_s16le",
            ])
            .arg(&path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        Some((dir, path))
    }

    #[test]
    fn sine_plays_to_completion() {
        let Some((_dir, path)) = wav_fixture() else {
            return;
        };
        let mut player = match AudioPlayer::new(None) {
            Ok(player) => player,
            Err(error) => {
                eprintln!("SKIP: no audio output device: {error}");
                return;
            }
        };
        player.play(&path, AudioOptions::default()).unwrap();
        assert!(player.is_playing());
        let deadline = Instant::now() + Duration::from_secs(3);
        while player.is_playing() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(!player.is_playing(), "one second tone did not finish");
    }

    #[test]
    fn pause_freezes_position_and_resume_continues() {
        let Some((_dir, path)) = wav_fixture() else {
            return;
        };
        let mut player = match AudioPlayer::new(None) {
            Ok(player) => player,
            Err(error) => {
                eprintln!("SKIP: no audio output device: {error}");
                return;
            }
        };
        player.play(&path, AudioOptions::default()).unwrap();
        assert_eq!(player.file_name(), Some("tone.wav"));
        assert_eq!(player.duration(), Some(Duration::from_secs(1)));
        std::thread::sleep(Duration::from_millis(150));
        player.pause();
        assert!(player.is_paused() && player.is_playing());
        std::thread::sleep(Duration::from_millis(50));
        let frozen = player.position();
        std::thread::sleep(Duration::from_millis(200));
        assert_eq!(player.position(), frozen);
        player.resume();
        assert!(!player.is_paused());
    }

    #[test]
    fn volume_is_validated_and_fade_stop_returns_promptly() {
        let Some((_dir, path)) = wav_fixture() else {
            return;
        };
        let mut player = match AudioPlayer::new(None) {
            Ok(player) => player,
            Err(error) => {
                eprintln!("SKIP: no audio output device: {error}");
                return;
            }
        };
        assert!(
            player
                .play(
                    &path,
                    AudioOptions {
                        volume: f32::NAN,
                        ..AudioOptions::default()
                    }
                )
                .is_err()
        );
        player
            .play(
                &path,
                AudioOptions {
                    looped: true,
                    ..AudioOptions::default()
                },
            )
            .unwrap();
        let started = Instant::now();
        player.stop(300);
        assert!(started.elapsed() < Duration::from_millis(50));
        assert!(
            player
                .play(&path.with_file_name("missing.wav"), AudioOptions::default())
                .is_err()
        );
    }
}
