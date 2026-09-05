//! Dedicated video decode thread (MF/COM objects stay on one thread).

use std::path::PathBuf;
use std::thread::{self, JoinHandle};

use cap_video::{VideoFrame, VideoInfo, VideoPlayer};
use crossbeam_channel::{Receiver, Sender, TryRecvError, unbounded};

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
    Shutdown,
}

pub enum VideoEvent {
    Opened {
        info: VideoInfo,
        duration_secs: f32,
        width: u32,
        height: u32,
        first_frame: Option<VideoFrame>,
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

    pub fn seek_relative(&self, delta_secs: f32) {
        let _ = self.cmd_tx.send(VideoCommand::SeekRelative(delta_secs));
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

fn video_loop(cmd_rx: Receiver<VideoCommand>, evt_tx: Sender<VideoEvent>) {
    let mut player: Option<VideoPlayer> = None;

    while let Ok(cmd) = cmd_rx.recv() {
        match cmd {
            VideoCommand::Open {
                path,
                prefer_hw_decode,
            } => {
                player = None;
                match VideoInfo::from_path(&path) {
                    Ok(mut info) => match VideoPlayer::open_with_options(path, prefer_hw_decode) {
                        Ok(p) => {
                            let duration_secs = p.duration_secs();
                            let width = p.width();
                            let height = p.height();
                            info.duration_secs = duration_secs;
                            info.width = width;
                            info.height = height;
                            let first_frame = p.current_frame().cloned();
                            let _ = evt_tx.send(VideoEvent::Opened {
                                info,
                                duration_secs,
                                width,
                                height,
                                first_frame,
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
                    let _ = evt_tx.send(VideoEvent::Playing(true));
                }
            }
            VideoCommand::Pause => {
                if let Some(p) = player.as_mut() {
                    p.pause();
                    let _ = evt_tx.send(VideoEvent::Playing(false));
                }
            }
            VideoCommand::Toggle => {
                if let Some(p) = player.as_mut() {
                    p.toggle();
                    let _ = evt_tx.send(VideoEvent::Playing(p.is_playing()));
                }
            }
            VideoCommand::Tick => {
                if let Some(p) = player.as_mut() {
                    if let Some(frame) = p.tick() {
                        let _ = evt_tx.send(VideoEvent::Frame(frame));
                        emit_position(&evt_tx, p);
                    }
                }
            }
            VideoCommand::StepForward => {
                if let Some(p) = player.as_mut() {
                    if let Some(frame) = p.step_frame(true) {
                        let _ = evt_tx.send(VideoEvent::Frame(frame));
                        emit_position(&evt_tx, p);
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
                    let _ = evt_tx.send(VideoEvent::Playing(false));
                }
            }
            VideoCommand::Seek(fraction) => {
                if let Some(p) = player.as_mut() {
                    if let Some(frame) = p.seek_fraction(fraction) {
                        let _ = evt_tx.send(VideoEvent::Frame(frame));
                        emit_position(&evt_tx, p);
                    }
                    let _ = evt_tx.send(VideoEvent::Playing(false));
                }
            }
            VideoCommand::SeekRelative(delta) => {
                if let Some(p) = player.as_mut() {
                    if let Some(frame) = p.seek_by_secs(delta) {
                        let _ = evt_tx.send(VideoEvent::Frame(frame));
                        emit_position(&evt_tx, p);
                    }
                    let _ = evt_tx.send(VideoEvent::Playing(false));
                }
            }
            VideoCommand::Shutdown => break,
        }
    }
}
