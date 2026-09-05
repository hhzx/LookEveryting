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
    /// Playback rate multiplier (0.25–2.0). Affects frame advance timing.
    rate: f32,
}

impl VideoPlayer {
    pub fn open(path: PathBuf) -> Result<Self, PlayerError> {
        Self::open_with_options(path, true)
    }

    pub fn open_with_options(path: PathBuf, prefer_hw_decode: bool) -> Result<Self, PlayerError> {
        #[cfg(windows)]
        {
            let inner = mf::MfPlayer::open(&path, prefer_hw_decode)?;
            let fps = inner.fps().max(24.0);
            Ok(Self {
                path,
                inner,
                playing: false,
                last_tick: Instant::now(),
                fps,
                rate: 1.0,
            })
        }
        #[cfg(not(windows))]
        {
            let _ = (path, prefer_hw_decode);
            Err(PlayerError::UnsupportedPlatform)
        }
    }

    pub fn set_rate(&mut self, rate: f32) {
        self.rate = rate.clamp(0.25, 2.0);
    }

    pub fn rate(&self) -> f32 {
        self.rate
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    pub fn fps(&self) -> f32 {
        self.fps
    }

    pub fn duration_secs(&self) -> f32 {
        #[cfg(windows)]
        {
            return self.inner.duration_secs();
        }
        #[cfg(not(windows))]
        {
            0.0
        }
    }

    pub fn position_secs(&self) -> f32 {
        #[cfg(windows)]
        {
            return self.inner.position_secs();
        }
        #[cfg(not(windows))]
        {
            0.0
        }
    }

    pub fn position_fraction(&self) -> f32 {
        #[cfg(windows)]
        {
            return self.inner.position_fraction();
        }
        #[cfg(not(windows))]
        {
            0.0
        }
    }

    pub fn width(&self) -> u32 {
        #[cfg(windows)]
        {
            return self.inner.width();
        }
        #[cfg(not(windows))]
        {
            0
        }
    }

    pub fn height(&self) -> u32 {
        #[cfg(windows)]
        {
            return self.inner.height();
        }
        #[cfg(not(windows))]
        {
            0
        }
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
                let frame_dt = (1.0 / self.fps) / self.rate.max(0.25);
                if elapsed >= frame_dt {
                    self.last_tick = Instant::now();
                    if self.inner.next_frame().is_none() {
                        self.playing = false;
                    }
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

    /// Seek to a fraction of total duration [0, 1].
    pub fn seek_fraction(&mut self, fraction: f32) -> Option<VideoFrame> {
        #[cfg(windows)]
        {
            self.playing = false;
            self.inner.seek_fraction(fraction).ok()?;
            self.inner.current_frame().cloned()
        }
        #[cfg(not(windows))]
        {
            let _ = fraction;
            None
        }
    }

    /// Seek by a relative time delta in seconds (clamped to [0, duration]).
    pub fn seek_by_secs(&mut self, delta: f32) -> Option<VideoFrame> {
        let duration = self.duration_secs();
        if duration <= 0.0 {
            return None;
        }
        let target = (self.position_secs() + delta).clamp(0.0, duration);
        self.seek_fraction(target / duration)
    }

    /// Step one frame forward or backward (pauses playback).
    pub fn step_frame(&mut self, forward: bool) -> Option<VideoFrame> {
        #[cfg(windows)]
        {
            self.playing = false;
            if forward {
                self.inner.next_frame();
            } else {
                self.inner.prev_frame().ok();
            }
            self.inner.current_frame().cloned()
        }
        #[cfg(not(windows))]
        {
            let _ = forward;
            None
        }
    }
}

#[cfg(windows)]
mod mf {
    use std::path::Path;

    use super::{PlayerError, VideoFrame};
    use crate::mf_runtime;
    use crate::yuv::{nv12_to_rgba, yuy2_to_rgba};
    use windows::core::PCWSTR;
    use windows::Win32::Media::MediaFoundation::*;
    use windows_core::{GUID, Interface, PROPVARIANT};

    enum PixelFormat {
        Rgb32,
        Nv12,
        Yuy2,
    }

    pub struct MfPlayer {
        reader: IMFSourceReader,
        width: u32,
        height: u32,
        pixel_format: PixelFormat,
        current: Option<VideoFrame>,
        rgba_buf: Vec<u8>,
        fps: f32,
        frame_duration_100ns: i64,
        position_100ns: i64,
        duration_100ns: i64,
    }

    impl MfPlayer {
        pub fn width(&self) -> u32 {
            self.width
        }

        pub fn height(&self) -> u32 {
            self.height
        }

        pub fn duration_secs(&self) -> f32 {
            if self.duration_100ns > 0 {
                self.duration_100ns as f32 / 10_000_000.0
            } else {
                0.0
            }
        }

        pub fn position_secs(&self) -> f32 {
            (self.position_100ns.max(0) as f32) / 10_000_000.0
        }

        pub fn position_fraction(&self) -> f32 {
            if self.duration_100ns <= 0 {
                return 0.0;
            }
            (self.position_100ns.max(0) as f32 / self.duration_100ns as f32).clamp(0.0, 1.0)
        }

        pub fn seek_fraction(&mut self, fraction: f32) -> Result<(), PlayerError> {
            if self.duration_100ns <= 0 {
                return Err(PlayerError::Message("unknown duration".into()));
            }
            let target = (self.duration_100ns as f64 * fraction.clamp(0.0, 1.0) as f64) as i64;
            self.seek_to_100ns(target)
        }

        pub fn open(path: &Path, prefer_hw_decode: bool) -> Result<Self, PlayerError> {
            mf_runtime::ensure_initialized();

            unsafe {
                let wide = path_to_file_url(path)?;
                let mut attrs = None;
                MFCreateAttributes(&mut attrs, 2)
                    .map_err(|e| PlayerError::Message(e.to_string()))?;
                let attrs = attrs.ok_or_else(|| {
                    PlayerError::Message("MFCreateAttributes returned null".into())
                })?;
                attrs
                    .SetUINT32(&MF_SOURCE_READER_ENABLE_VIDEO_PROCESSING, 1)
                    .map_err(|e| PlayerError::Message(e.to_string()))?;
                if prefer_hw_decode {
                    attrs
                        .SetUINT32(&MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS, 1)
                        .map_err(|e| PlayerError::Message(e.to_string()))?;
                }

                let reader = MFCreateSourceReaderFromURL(PCWSTR(wide.as_ptr()), Some(&attrs))
                    .map_err(|e| PlayerError::Message(format!("open video: {e}")))?;

                let native = reader
                    .GetNativeMediaType(MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32, 0)
                    .map_err(|e| PlayerError::Message(e.to_string()))?;
                let (w, h) = frame_size_from_type(&native);
                let fps = fps_from_type(&native);
                let frame_duration_100ns = if fps > 0.0 {
                    (10_000_000.0 / fps) as i64
                } else {
                    333_667
                };

                let pixel_format = configure_output_format(&reader)?;

                let duration_100ns = duration_from_source(&reader);

                let mut player = Self {
                    reader,
                    width: w,
                    height: h,
                    pixel_format,
                    current: None,
                    rgba_buf: vec![0u8; (w * h * 4) as usize],
                    fps,
                    frame_duration_100ns,
                    position_100ns: 0,
                    duration_100ns,
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

        pub fn prev_frame(&mut self) -> Result<(), PlayerError> {
            let target = (self.position_100ns - self.frame_duration_100ns).max(0);
            self.seek_to_100ns(target)
        }

        pub fn seek_start(&mut self) -> Result<(), PlayerError> {
            self.seek_to_100ns(0)
        }

        fn seek_to_100ns(&mut self, position: i64) -> Result<(), PlayerError> {
            unsafe {
                let prop = PROPVARIANT::from(position);
                self.reader
                    .SetCurrentPosition(&GUID::zeroed(), &prop)
                    .map_err(|e| PlayerError::Message(e.to_string()))?;
                self.position_100ns = position;
                self.read_frame_into_current()?;
                Ok(())
            }
        }

        fn read_frame_into_current(&mut self) -> Result<(), PlayerError> {
            unsafe {
                const MAX_ATTEMPTS: u32 = 32;
                let stream = MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32;

                for _ in 0..MAX_ATTEMPTS {
                    let mut flags = 0u32;
                    let mut sample = None;
                    let mut timestamp = 0i64;
                    self.reader
                        .ReadSample(
                            stream,
                            0,
                            None,
                            Some(&mut flags),
                            Some(&mut timestamp),
                            Some(&mut sample),
                        )
                        .map_err(|e| PlayerError::Message(e.to_string()))?;

                    if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
                        return Err(PlayerError::Message("end of stream".into()));
                    }

                    if flags & MF_SOURCE_READERF_CURRENTMEDIATYPECHANGED.0 as u32 != 0 {
                        if let Ok(media_type) = self.reader.GetCurrentMediaType(stream) {
                            let (w, h) = frame_size_from_type(&media_type);
                            if w > 0 && h > 0 {
                                self.width = w;
                                self.height = h;
                                self.rgba_buf.resize((w * h * 4) as usize, 0);
                            }
                        }
                        continue;
                    }

                    let Some(sample) = sample else {
                        continue;
                    };

                    if timestamp >= 0 {
                        self.position_100ns = timestamp;
                    }
                    return self.decode_sample(&sample);
                }

                Err(PlayerError::Message("no video frame after read loop".into()))
            }
        }

        fn decode_sample(&mut self, sample: &IMFSample) -> Result<(), PlayerError> {
            unsafe {
                let buffer = sample
                    .ConvertToContiguousBuffer()
                    .map_err(|e| PlayerError::Message(e.to_string()))?;

                let w = self.width.max(1);
                let h = self.height.max(1);
                let needed = (w * h * 4) as usize;
                if self.rgba_buf.len() != needed {
                    self.rgba_buf.resize(needed, 0);
                }

                match self.pixel_format {
                    PixelFormat::Rgb32 => self.decode_rgb32(&buffer, w, h)?,
                    PixelFormat::Nv12 => self.decode_nv12(&buffer, w, h)?,
                    PixelFormat::Yuy2 => self.decode_yuy2(&buffer, w, h)?,
                }

                self.current = Some(VideoFrame {
                    width: w,
                    height: h,
                    rgba: self.rgba_buf.clone(),
                });
                Ok(())
            }
        }

        fn decode_rgb32(
            &mut self,
            buffer: &IMFMediaBuffer,
            w: u32,
            h: u32,
        ) -> Result<(), PlayerError> {
            unsafe {
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
                        std::ptr::copy_nonoverlapping(
                            src,
                            self.rgba_buf.as_mut_ptr().add(dst_start),
                            row_bytes,
                        );
                    }
                    buf2d.Unlock2D().ok();
                } else {
                    let mut data: *mut u8 = std::ptr::null_mut();
                    buffer
                        .Lock(&mut data, None, None)
                        .map_err(|e| PlayerError::Message(e.to_string()))?;
                    let len = buffer
                        .GetCurrentLength()
                        .map_err(|e| PlayerError::Message(e.to_string()))?
                        as usize;
                    let slice = std::slice::from_raw_parts(data, len);
                    let stride = MFGetStrideForBitmapInfoHeader(32, w)
                        .map(|s| s.unsigned_abs() as usize)
                        .unwrap_or((w * 4) as usize);
                    let row_bytes = (w * 4) as usize;
                    for row in 0..h as usize {
                        let src_start = row * stride;
                        let dst_start = row * row_bytes;
                        if src_start + row_bytes <= slice.len()
                            && dst_start + row_bytes <= self.rgba_buf.len()
                        {
                            self.rgba_buf[dst_start..dst_start + row_bytes]
                                .copy_from_slice(&slice[src_start..src_start + row_bytes]);
                        }
                    }
                    buffer.Unlock().ok();
                }

                for px in self.rgba_buf.chunks_exact_mut(4) {
                    px.swap(0, 2);
                }
                Ok(())
            }
        }

        fn decode_nv12(
            &mut self,
            buffer: &IMFMediaBuffer,
            w: u32,
            h: u32,
        ) -> Result<(), PlayerError> {
            unsafe {
                let mut data: *mut u8 = std::ptr::null_mut();
                buffer
                    .Lock(&mut data, None, None)
                    .map_err(|e| PlayerError::Message(e.to_string()))?;
                let len = buffer
                    .GetCurrentLength()
                    .map_err(|e| PlayerError::Message(e.to_string()))?
                    as usize;
                let slice = std::slice::from_raw_parts(data, len);
                let y_size = (w * h) as usize;
                if slice.len() < y_size {
                    buffer.Unlock().ok();
                    return Err(PlayerError::Message("NV12 buffer too small".into()));
                }
                let stride_y = w as usize;
                let stride_uv = w as usize;
                nv12_to_rgba(
                    &slice[..y_size],
                    &slice[y_size..],
                    w,
                    h,
                    stride_y,
                    stride_uv,
                    &mut self.rgba_buf,
                );
                buffer.Unlock().ok();
                Ok(())
            }
        }

        fn decode_yuy2(
            &mut self,
            buffer: &IMFMediaBuffer,
            w: u32,
            h: u32,
        ) -> Result<(), PlayerError> {
            unsafe {
                let mut data: *mut u8 = std::ptr::null_mut();
                buffer
                    .Lock(&mut data, None, None)
                    .map_err(|e| PlayerError::Message(e.to_string()))?;
                let len = buffer
                    .GetCurrentLength()
                    .map_err(|e| PlayerError::Message(e.to_string()))?
                    as usize;
                let slice = std::slice::from_raw_parts(data, len);
                let stride = (w * 2) as usize;
                yuy2_to_rgba(slice, w, h, stride, &mut self.rgba_buf);
                buffer.Unlock().ok();
                Ok(())
            }
        }
    }

    fn configure_output_format(reader: &IMFSourceReader) -> Result<PixelFormat, PlayerError> {
        unsafe {
            let stream = MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32;
            for (fmt, guid) in [
                (PixelFormat::Rgb32, MFVideoFormat_RGB32),
                (PixelFormat::Nv12, MFVideoFormat_NV12),
                (PixelFormat::Yuy2, MFVideoFormat_YUY2),
                (PixelFormat::Nv12, MFVideoFormat_I420),
            ] {
                let out_type =
                    MFCreateMediaType().map_err(|e| PlayerError::Message(e.to_string()))?;
                out_type
                    .SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)
                    .map_err(|e| PlayerError::Message(e.to_string()))?;
                out_type
                    .SetGUID(&MF_MT_SUBTYPE, &guid)
                    .map_err(|e| PlayerError::Message(e.to_string()))?;
                if reader.SetCurrentMediaType(stream, None, &out_type).is_ok() {
                    return Ok(fmt);
                }
            }
            Err(PlayerError::Message(
                "no supported video output format (install H.264/H.265 codecs)".into(),
            ))
        }
    }

    fn path_to_file_url(path: &Path) -> Result<Vec<u16>, PlayerError> {
        let path = path
            .canonicalize()
            .unwrap_or_else(|_| path.to_path_buf());
        let mut text = path.to_string_lossy().to_string();
        if let Some(stripped) = text.strip_prefix(r"\\?\") {
            text = stripped.to_string();
        }
        let url = if text.starts_with("//") {
            format!("file:{text}")
        } else {
            format!("file:///{}", text.replace('\\', "/"))
        };
        Ok(url.encode_utf16().chain(std::iter::once(0)).collect())
    }

    fn frame_size_from_type(media_type: &IMFMediaType) -> (u32, u32) {
        let frame_size = unsafe { media_type.GetUINT64(&MF_MT_FRAME_SIZE).unwrap_or(0) };
        let width = (frame_size & 0xFFFF_FFFF) as u32;
        let height = (frame_size >> 32) as u32;
        if width > 0 && height > 0 {
            (width, height)
        } else {
            (1280, 720)
        }
    }

    fn fps_from_type(media_type: &IMFMediaType) -> f32 {
        unsafe {
            media_type
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
                .unwrap_or(30.0)
        }
    }

    fn duration_from_source(reader: &IMFSourceReader) -> i64 {
        unsafe {
            reader
                .GetPresentationAttribute(MF_SOURCE_READER_MEDIASOURCE.0 as u32, &MF_PD_DURATION)
                .ok()
                .and_then(|prop| u64::try_from(&prop).ok())
                .map(|v| v as i64)
                .unwrap_or(0)
        }
    }
}
