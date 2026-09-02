//! In-app video playback (Windows Media Foundation).

use std::path::{Path, PathBuf};
use std::time::Instant;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PlayerError {
    #[error("unsupported platform")]
    UnsupportedPlatform,
    #[error("playback error: {0}")]
    Message(String),
}

/// One decoded RGBA video frame.
#[derive(Clone)]
pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// In-app video player.
pub struct VideoPlayer {
    path: PathBuf,
    #[cfg(windows)]
    inner: mf::MfPlayer,
    playing: bool,
    last_tick: Instant,
    fps: f32,
}

impl VideoPlayer {
    pub fn open(path: PathBuf) -> Result<Self, PlayerError> {
        #[cfg(windows)]
        {
            let inner = mf::MfPlayer::open(&path)?;
            let fps = inner.fps().max(24.0);
            Ok(Self {
                path,
                inner,
                playing: false,
                last_tick: Instant::now(),
                fps,
            })
        }
        #[cfg(not(windows))]
        {
            let _ = path;
            Err(PlayerError::UnsupportedPlatform)
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    pub fn play(&mut self) {
        self.playing = true;
        self.last_tick = Instant::now();
    }

    pub fn pause(&mut self) {
        self.playing = false;
    }

    pub fn toggle(&mut self) {
        if self.playing {
            self.pause();
        } else {
            self.play();
        }
    }

    pub fn current_frame(&self) -> Option<&VideoFrame> {
        #[cfg(windows)]
        {
            return self.inner.current_frame();
        }
        #[cfg(not(windows))]
        {
            None
        }
    }

    /// Advance playback and return the current frame when playing.
    pub fn tick(&mut self) -> Option<VideoFrame> {
        #[cfg(windows)]
        {
            if self.playing {
                let elapsed = self.last_tick.elapsed().as_secs_f32();
                let frame_dt = 1.0 / self.fps;
                if elapsed >= frame_dt {
                    self.last_tick = Instant::now();
                    if let Some(frame) = self.inner.next_frame() {
                        return Some(frame);
                    }
                    self.playing = false;
                }
            }
            return self.inner.current_frame().cloned();
        }
        #[cfg(not(windows))]
        {
            let _ = self;
            None
        }
    }

    pub fn seek_start(&mut self) -> Option<VideoFrame> {
        #[cfg(windows)]
        {
            self.inner.seek_start().ok();
            self.inner.current_frame().cloned()
        }
        #[cfg(not(windows))]
        {
            None
        }
    }
}

#[cfg(windows)]
mod mf {
    use std::path::Path;

    use super::{PlayerError, VideoFrame};
    use windows::core::PCWSTR;
    use windows::Win32::Media::MediaFoundation::*;
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
    use windows_core::{GUID, Interface, PROPVARIANT};

    pub struct MfPlayer {
        reader: IMFSourceReader,
        width: u32,
        height: u32,
        current: Option<VideoFrame>,
        fps: f32,
    }

    impl MfPlayer {
        pub fn open(path: &Path) -> Result<Self, PlayerError> {
            unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
                MFStartup(MF_VERSION, MFSTARTUP_LITE)
                    .map_err(|e| PlayerError::Message(e.to_string()))?;

                let wide: Vec<u16> = path
                    .canonicalize()
                    .unwrap_or_else(|_| path.to_path_buf())
                    .to_string_lossy()
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .collect();

                let reader = MFCreateSourceReaderFromURL(PCWSTR(wide.as_ptr()), None)
                    .map_err(|e| PlayerError::Message(e.to_string()))?;

                let out_type = MFCreateMediaType().map_err(|e| PlayerError::Message(e.to_string()))?;
                out_type
                    .SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)
                    .map_err(|e| PlayerError::Message(e.to_string()))?;
                out_type
                    .SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_RGB32)
                    .map_err(|e| PlayerError::Message(e.to_string()))?;
                reader
                    .SetCurrentMediaType(
                        MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32,
                        None,
                        &out_type,
                    )
                    .map_err(|e| PlayerError::Message(e.to_string()))?;

                let native = reader
                    .GetNativeMediaType(MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32, 0)
                    .map_err(|e| PlayerError::Message(e.to_string()))?;
                let frame_size = native.GetUINT64(&MF_MT_FRAME_SIZE).unwrap_or(0);
                let width = (frame_size & 0xFFFF_FFFF) as u32;
                let height = (frame_size >> 32) as u32;
                let fps = native
                    .GetUINT64(&MF_MT_FRAME_RATE)
                    .map(|v| {
                        let num = (v >> 32) as f32;
                        let den = (v & 0xFFFF_FFFF) as f32;
                        if den > 0.0 {
                            (num / den).max(24.0)
                        } else {
                            30.0
                        }
                    })
                    .unwrap_or(30.0);

                let (w, h) = if width > 0 && height > 0 {
                    (width, height)
                } else {
                    (1280, 720)
                };

                let mut player = Self {
                    reader,
                    width: w,
                    height: h,
                    current: None,
                    fps,
                };
                player.read_frame_into_current()?;
                Ok(player)
            }
        }

        pub fn fps(&self) -> f32 {
            self.fps
        }

        pub fn current_frame(&self) -> Option<&VideoFrame> {
            self.current.as_ref()
        }

        pub fn next_frame(&mut self) -> Option<VideoFrame> {
            self.read_frame_into_current().ok()?;
            self.current.clone()
        }

        pub fn seek_start(&mut self) -> Result<(), PlayerError> {
            unsafe {
                let prop = PROPVARIANT::from(0i64);
                self.reader
                    .SetCurrentPosition(&GUID::zeroed(), &prop)
                    .map_err(|e| PlayerError::Message(e.to_string()))?;
                self.read_frame_into_current()?;
                Ok(())
            }
        }

        fn read_frame_into_current(&mut self) -> Result<(), PlayerError> {
            unsafe {
                let mut flags = 0u32;
                let mut sample = None;
                self.reader
                    .ReadSample(
                        MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32,
                        0,
                        None,
                        Some(&mut flags),
                        None,
                        Some(&mut sample),
                    )
                    .map_err(|e| PlayerError::Message(e.to_string()))?;

                if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
                    return Err(PlayerError::Message("end of stream".into()));
                }

                let Some(sample) = sample else {
                    return Err(PlayerError::Message("no video sample".into()));
                };

                let buffer = sample
                    .ConvertToContiguousBuffer()
                    .map_err(|e| PlayerError::Message(e.to_string()))?;

                let w = self.width;
                let h = self.height;
                let mut rgba = vec![0u8; (w * h * 4) as usize];

                if let Ok(buf2d) = buffer.cast::<IMF2DBuffer>() {
                    let mut scan0: *mut u8 = std::ptr::null_mut();
                    let mut pitch = 0i32;
                    buf2d
                        .Lock2D(&mut scan0, &mut pitch)
                        .map_err(|e| PlayerError::Message(e.to_string()))?;
                    let pitch = pitch.unsigned_abs() as usize;
                    let row_bytes = (w * 4) as usize;
                    for row in 0..h as usize {
                        let src = scan0.add(row * pitch);
                        let dst_start = row * row_bytes;
                        std::ptr::copy_nonoverlapping(src, rgba.as_mut_ptr().add(dst_start), row_bytes);
                    }
                    buf2d.Unlock2D().ok();
                } else {
                    let mut data: *mut u8 = std::ptr::null_mut();
                    buffer
                        .Lock(&mut data, None, None)
                        .map_err(|e| PlayerError::Message(e.to_string()))?;
                    let len = buffer.GetCurrentLength().map_err(|e| PlayerError::Message(e.to_string()))?
                        as usize;
                    let slice = std::slice::from_raw_parts(data, len);
                    let stride = MFGetStrideForBitmapInfoHeader(32, w)
                        .map(|s| s.unsigned_abs() as usize)
                        .unwrap_or((w * 4) as usize);
                    let row_bytes = (w * 4) as usize;
                    for row in 0..h as usize {
                        let src_start = row * stride;
                        let dst_start = row * row_bytes;
                        if src_start + row_bytes <= slice.len() && dst_start + row_bytes <= rgba.len() {
                            rgba[dst_start..dst_start + row_bytes]
                                .copy_from_slice(&slice[src_start..src_start + row_bytes]);
                        }
                    }
                    buffer.Unlock().ok();
                }

                // BGRA -> RGBA
                for px in rgba.chunks_exact_mut(4) {
                    px.swap(0, 2);
                }

                self.current = Some(VideoFrame {
                    width: w,
                    height: h,
                    rgba,
                });
                Ok(())
            }
        }
    }
}
