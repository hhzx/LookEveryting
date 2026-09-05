//! Dedicated video decode thread (MF/COM objects stay on one thread).

use std::path::PathBuf;
use std::thread::{self, JoinHandle};

use cap_video::{AudioDecoder, VideoFrame, VideoInfo, VideoPlayer};
use crossbeam_channel::{unbounded, Receiver, Sender, TryRecvError};

use crate::audio_out::AudioOut;

pub enum VideoCommand {
    Open {
        path: PathBuf,
        prefer_hw_decode: bool,
    },
    Play,
    Pause,
    Toggle,
    Tick,
    StepForward,
    StepBackward,
    Seek(f32),
    SeekRelative(f32),
    /// Seek then optionally resume playback (used by A-B loop).
    SeekResume(f32),
    SetVolume(f32),
    SetRate(f32),
    Shutdown,
}

pub enum VideoEvent {
    Opened {
        info: VideoInfo,
        duration_secs: f32,
        width: u32,
        height: u32,
        first_frame: Option<VideoFrame>,
        has_audio: bool,
    },
    Frame(VideoFrame),
    Position {
        fraction: f32,
        secs: f32,
    },
    Playing(bool),
    Error(String),
}

pub struct VideoThread {
    cmd_tx: Sender<VideoCommand>,
    evt_rx: Receiver<VideoEvent>,
    _worker: JoinHandle<()>,
}

impl VideoThread {
    pub fn spawn() -> Self {
        let (cmd_tx, cmd_rx) = unbounded();
        let (evt_tx, evt_rx) = unbounded();

        let worker = thread::Builder::new()
            .name("lookeveryting-video".into())
            .spawn(move || video_loop(cmd_rx, evt_tx))
            .expect("spawn video thread");

        Self {
            cmd_tx,
            evt_rx,
            _worker: worker,
        }
    }

    pub fn open(&self, path: PathBuf, prefer_hw_decode: bool) {
        let _ = self.cmd_tx.send(VideoCommand::Open {
            path,
            prefer_hw_decode,
        });
    }

    pub fn play(&self) {
        let _ = self.cmd_tx.send(VideoCommand::Play);
    }

    pub fn pause(&self) {
        let _ = self.cmd_tx.send(VideoCommand::Pause);
    }

    pub fn toggle(&self) {
        let _ = self.cmd_tx.send(VideoCommand::Toggle);
    }

    pub fn tick(&self) {
        let _ = self.cmd_tx.send(VideoCommand::Tick);
    }

    pub fn step_forward(&self) {
        let _ = self.cmd_tx.send(VideoCommand::StepForward);
    }

    pub fn step_backward(&self) {
        let _ = self.cmd_tx.send(VideoCommand::StepBackward);
    }

    pub fn seek(&self, fraction: f32) {
        let _ = self.cmd_tx.send(VideoCommand::Seek(fraction));
    }

    pub fn seek_resume(&self, fraction: f32) {
        let _ = self.cmd_tx.send(VideoCommand::SeekResume(fraction));
    }

    pub fn seek_relative(&self, delta_secs: f32) {
        let _ = self.cmd_tx.send(VideoCommand::SeekRelative(delta_secs));
    }

    pub fn set_volume(&self, volume: f32) {
        let _ = self.cmd_tx.send(VideoCommand::SetVolume(volume));
    }

    pub fn set_rate(&self, rate: f32) {
        let _ = self.cmd_tx.send(VideoCommand::SetRate(rate));
    }

    pub fn poll(&self) -> Vec<VideoEvent> {
        let mut out = Vec::new();
        loop {
            match self.evt_rx.try_recv() {
                Ok(evt) => out.push(evt),
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => break,
            }
        }
        out
    }
}

impl Drop for VideoThread {
    fn drop(&mut self) {
        let _ = self.cmd_tx.send(VideoCommand::Shutdown);
    }
}

fn emit_position(evt_tx: &Sender<VideoEvent>, player: &VideoPlayer) {
    let _ = evt_tx.send(VideoEvent::Position {
        fraction: player.position_fraction(),
        secs: player.position_secs(),
    });
}

fn sync_audio(
    audio: &mut Option<AudioDecoder>,
    out: &Option<AudioOut>,
    position_secs: f32,
    playing: bool,
) {
    let Some(decoder) = audio.as_mut() else {
        return;
    };
    let Some(out) = out.as_ref() else {
        return;
    };
    out.shared.set_playing(playing);
    if !playing {
        return;
    }
    // Keep ~200–350ms buffered ahead of the video clock.
    if out.shared.queued_secs() > 0.35 {
        return;
    }
    let until = position_secs + 0.35;
    let mut chunks = Vec::new();
    if decoder.pull_until(until, &mut chunks).is_ok() {
        for chunk in chunks {
            out.shared.push_samples(&chunk.samples);
        }
    }
}

fn video_loop(cmd_rx: Receiver<VideoCommand>, evt_tx: Sender<VideoEvent>) {
    let mut player: Option<VideoPlayer> = None;
    let mut audio: Option<AudioDecoder> = None;
    let mut audio_out: Option<AudioOut> = None;
    let mut volume = 1.0_f32;
    let mut rate = 1.0_f32;

    while let Ok(cmd) = cmd_rx.recv() {
        match cmd {
            VideoCommand::Open {
                path,
                prefer_hw_decode,
            } => {
                player = None;
                audio = None;
                if let Some(out) = audio_out.take() {
                    out.shared.set_playing(false);
                    out.shared.clear();
                }
                match VideoInfo::from_path(&path) {
                    Ok(mut info) => match VideoPlayer::open_with_options(path.clone(), prefer_hw_decode)
                    {
                        Ok(mut p) => {
                            p.set_rate(rate);
                            let duration_secs = p.duration_secs();
                            let width = p.width();
                            let height = p.height();
                            info.duration_secs = duration_secs;
                            info.width = width;
                            info.height = height;
                            let first_frame = p.current_frame().cloned();

                            let mut has_audio = false;
                            if let Ok(decoder) = AudioDecoder::open(&path) {
                                if let Some(fmt) = decoder.format() {
                                    has_audio = true;
                                    audio_out = AudioOut::try_start(fmt.sample_rate, fmt.channels);
                                    if let Some(out) = audio_out.as_ref() {
                                        out.shared.set_volume(volume);
                                    }
                                    audio = Some(decoder);
                                }
                            }

                            let _ = evt_tx.send(VideoEvent::Opened {
                                info,
                                duration_secs,
                                width,
                                height,
                                first_frame,
                                has_audio,
                            });
                            emit_position(&evt_tx, &p);
                            player = Some(p);
                        }
                        Err(err) => {
                            let _ = evt_tx.send(VideoEvent::Error(err.to_string()));
                        }
                    },
                    Err(err) => {
                        let _ = evt_tx.send(VideoEvent::Error(err.to_string()));
                    }
                }
            }
            VideoCommand::Play => {
                if let Some(p) = player.as_mut() {
                    p.play();
                    sync_audio(&mut audio, &audio_out, p.position_secs(), true);
                    let _ = evt_tx.send(VideoEvent::Playing(true));
                }
            }
            VideoCommand::Pause => {
                if let Some(p) = player.as_mut() {
                    p.pause();
                    if let Some(out) = audio_out.as_ref() {
                        out.shared.set_playing(false);
                    }
                    let _ = evt_tx.send(VideoEvent::Playing(false));
                }
            }
            VideoCommand::Toggle => {
                if let Some(p) = player.as_mut() {
                    p.toggle();
                    let playing = p.is_playing();
                    if playing {
                        sync_audio(&mut audio, &audio_out, p.position_secs(), true);
                    } else if let Some(out) = audio_out.as_ref() {
                        out.shared.set_playing(false);
                    }
                    let _ = evt_tx.send(VideoEvent::Playing(playing));
                }
            }
            VideoCommand::Tick => {
                if let Some(p) = player.as_mut() {
                    let playing = p.is_playing();
                    if let Some(frame) = p.tick() {
                        let _ = evt_tx.send(VideoEvent::Frame(frame));
                        emit_position(&evt_tx, p);
                    }
                    sync_audio(&mut audio, &audio_out, p.position_secs(), playing);
                    if !playing {
                        let _ = evt_tx.send(VideoEvent::Playing(false));
                    }
                }
            }
            VideoCommand::StepForward => {
                if let Some(p) = player.as_mut() {
                    if let Some(frame) = p.step_frame(true) {
                        let _ = evt_tx.send(VideoEvent::Frame(frame));
                        emit_position(&evt_tx, p);
                    }
                    if let Some(out) = audio_out.as_ref() {
                        out.shared.set_playing(false);
                    }
                    if let Some(a) = audio.as_mut() {
                        let _ = a.seek_secs(p.position_secs());
                    }
                    let _ = evt_tx.send(VideoEvent::Playing(false));
                }
            }
            VideoCommand::StepBackward => {
                if let Some(p) = player.as_mut() {
                    if let Some(frame) = p.step_frame(false) {
                        let _ = evt_tx.send(VideoEvent::Frame(frame));
                        emit_position(&evt_tx, p);
                    }
                    if let Some(out) = audio_out.as_ref() {
                        out.shared.set_playing(false);
                    }
                    if let Some(a) = audio.as_mut() {
                        let _ = a.seek_secs(p.position_secs());
                    }
                    let _ = evt_tx.send(VideoEvent::Playing(false));
                }
            }
            VideoCommand::Seek(fraction) => {
                if let Some(p) = player.as_mut() {
                    if let Some(frame) = p.seek_fraction(fraction) {
                        let _ = evt_tx.send(VideoEvent::Frame(frame));
                        emit_position(&evt_tx, p);
                    }
                    if let Some(out) = audio_out.as_ref() {
                        out.shared.set_playing(false);
                        out.shared.clear();
                    }
                    if let Some(a) = audio.as_mut() {
                        let _ = a.seek_secs(p.position_secs());
                    }
                    let _ = evt_tx.send(VideoEvent::Playing(false));
                }
            }
            VideoCommand::SeekResume(fraction) => {
                if let Some(p) = player.as_mut() {
                    if let Some(frame) = p.seek_fraction(fraction) {
                        let _ = evt_tx.send(VideoEvent::Frame(frame));
                        emit_position(&evt_tx, p);
                    }
                    if let Some(out) = audio_out.as_ref() {
                        out.shared.clear();
                    }
                    if let Some(a) = audio.as_mut() {
                        let _ = a.seek_secs(p.position_secs());
                    }
                    p.play();
                    sync_audio(&mut audio, &audio_out, p.position_secs(), true);
                    let _ = evt_tx.send(VideoEvent::Playing(true));
                }
            }
            VideoCommand::SeekRelative(delta) => {
                if let Some(p) = player.as_mut() {
                    if let Some(frame) = p.seek_by_secs(delta) {
                        let _ = evt_tx.send(VideoEvent::Frame(frame));
                        emit_position(&evt_tx, p);
                    }
                    if let Some(out) = audio_out.as_ref() {
                        out.shared.set_playing(false);
                        out.shared.clear();
                    }
                    if let Some(a) = audio.as_mut() {
                        let _ = a.seek_secs(p.position_secs());
                    }
                    let _ = evt_tx.send(VideoEvent::Playing(false));
                }
            }
            VideoCommand::SetVolume(v) => {
                volume = v.clamp(0.0, 1.0);
                if let Some(out) = audio_out.as_ref() {
                    out.shared.set_volume(volume);
                }
            }
            VideoCommand::SetRate(r) => {
                rate = r.clamp(0.25, 2.0);
                if let Some(p) = player.as_mut() {
                    p.set_rate(rate);
                }
            }
            VideoCommand::Shutdown => break,
        }
    }
}
