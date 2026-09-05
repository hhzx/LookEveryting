//! Media Foundation audio decode (PCM float) for in-app playback.

use std::path::Path;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("unsupported platform")]
    UnsupportedPlatform,
    #[error("no audio stream")]
    NoAudio,
    #[error("audio error: {0}")]
    Message(String),
}

#[derive(Debug, Clone, Copy)]
pub struct AudioFormat {
    pub sample_rate: u32,
    pub channels: u16,
}

/// Decoded interleaved float samples with presentation time.
#[derive(Debug, Clone)]
pub struct AudioChunk {
    pub pts_secs: f32,
    pub samples: Vec<f32>,
}

/// Sidecar MF audio reader (separate from the video source reader).
pub struct AudioDecoder {
    #[cfg(windows)]
    inner: Option<mf_audio::MfAudio>,
    #[cfg(not(windows))]
    _unused: (),
}

#[derive(Debug, Clone)]
pub struct AudioTrackInfo {
    pub index: u32,
    pub sample_rate: u32,
    pub channels: u16,
    pub label: String,
}

impl AudioDecoder {
    pub fn open(path: &Path) -> Result<Self, AudioError> {
        Self::open_track(path, 0)
    }

    pub fn open_track(path: &Path, track_index: usize) -> Result<Self, AudioError> {
        #[cfg(windows)]
        {
            match mf_audio::MfAudio::open(path, track_index) {
                Ok(inner) => Ok(Self { inner: Some(inner) }),
                Err(AudioError::NoAudio) => Ok(Self { inner: None }),
                Err(e) => Err(e),
            }
        }
        #[cfg(not(windows))]
        {
            let _ = (path, track_index);
            Ok(Self { _unused: () })
        }
    }

    pub fn list_tracks(path: &Path) -> Vec<AudioTrackInfo> {
        #[cfg(windows)]
        {
            mf_audio::list_audio_tracks(path)
        }
        #[cfg(not(windows))]
        {
            let _ = path;
            Vec::new()
        }
    }

    pub fn format(&self) -> Option<AudioFormat> {
        #[cfg(windows)]
        {
            self.inner.as_ref().map(|a| a.format())
        }
        #[cfg(not(windows))]
        {
            None
        }
    }

    pub fn has_audio(&self) -> bool {
        self.format().is_some()
    }

    pub fn seek_secs(&mut self, secs: f32) -> Result<(), AudioError> {
        #[cfg(windows)]
        {
            if let Some(inner) = self.inner.as_mut() {
                inner.seek_secs(secs)?;
            }
            Ok(())
        }
        #[cfg(not(windows))]
        {
            let _ = secs;
            Ok(())
        }
    }

    /// Pull decoded PCM until the chunk PTS is at least `until_secs`, or EOF.
    pub fn pull_until(&mut self, until_secs: f32, out: &mut Vec<AudioChunk>) -> Result<(), AudioError> {
        #[cfg(windows)]
        {
            if let Some(inner) = self.inner.as_mut() {
                inner.pull_until(until_secs, out)?;
            }
            Ok(())
        }
        #[cfg(not(windows))]
        {
            let _ = (until_secs, out);
            Ok(())
        }
    }
}

#[cfg(windows)]
mod mf_audio {
    use std::path::Path;

    use super::{AudioChunk, AudioError, AudioFormat};
    use crate::mf_runtime;
    use windows::core::PCWSTR;
    use windows::Win32::Media::MediaFoundation::*;
    use windows_core::{GUID, PROPVARIANT};

    pub struct MfAudio {
        reader: IMFSourceReader,
        stream: u32,
        format: AudioFormat,
        ended: bool,
    }

    pub fn list_audio_tracks(path: &Path) -> Vec<super::AudioTrackInfo> {
        mf_runtime::ensure_initialized();
        let Ok(wide) = path_to_file_url(path) else {
            return Vec::new();
        };
        unsafe {
            let mut attrs = None;
            if MFCreateAttributes(&mut attrs, 1).is_err() {
                return Vec::new();
            }
            let Some(attrs) = attrs else {
                return Vec::new();
            };
            let Ok(reader) = MFCreateSourceReaderFromURL(PCWSTR(wide.as_ptr()), Some(&attrs))
            else {
                return Vec::new();
            };
            let mut tracks = Vec::new();
            for i in 0..64u32 {
                let Ok(mt) = reader.GetNativeMediaType(i, 0) else {
                    continue;
                };
                let Ok(major) = mt.GetGUID(&MF_MT_MAJOR_TYPE) else {
                    continue;
                };
                if major != MFMediaType_Audio {
                    continue;
                }
                let channels = mt.GetUINT32(&MF_MT_AUDIO_NUM_CHANNELS).unwrap_or(2) as u16;
                let sample_rate = mt
                    .GetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND)
                    .unwrap_or(48_000);
                let n = tracks.len() + 1;
                tracks.push(super::AudioTrackInfo {
                    index: i,
                    sample_rate,
                    channels,
                    label: format!("Track {n} ({channels}ch {sample_rate}Hz)"),
                });
            }
            tracks
        }
    }

    impl MfAudio {
        pub fn format(&self) -> AudioFormat {
            self.format
        }

        pub fn open(path: &Path, track_index: usize) -> Result<Self, AudioError> {
            mf_runtime::ensure_initialized();
            unsafe {
                let wide = path_to_file_url(path)?;
                let mut attrs = None;
                MFCreateAttributes(&mut attrs, 1)
                    .map_err(|e| AudioError::Message(e.to_string()))?;
                let attrs = attrs.ok_or_else(|| AudioError::Message("attrs null".into()))?;
                attrs
                    .SetUINT32(&MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS, 1)
                    .ok();

                let reader = MFCreateSourceReaderFromURL(PCWSTR(wide.as_ptr()), Some(&attrs))
                    .map_err(|e| AudioError::Message(format!("open audio: {e}")))?;

                let tracks = {
                    let mut t = Vec::new();
                    for i in 0..64u32 {
                        let Ok(mt) = reader.GetNativeMediaType(i, 0) else {
                            continue;
                        };
                        let Ok(major) = mt.GetGUID(&MF_MT_MAJOR_TYPE) else {
                            continue;
                        };
                        if major == MFMediaType_Audio {
                            t.push(i);
                        }
                    }
                    t
                };
                if tracks.is_empty() {
                    return Err(AudioError::NoAudio);
                }
                let stream = tracks[track_index.min(tracks.len() - 1)];

                let format = configure_pcm_float(&reader, stream)?;
                reader
                    .SetStreamSelection(MF_SOURCE_READER_ALL_STREAMS.0 as u32, false)
                    .ok();
                reader
                    .SetStreamSelection(stream, true)
                    .map_err(|e| AudioError::Message(e.to_string()))?;

                Ok(Self {
                    reader,
                    stream,
                    format,
                    ended: false,
                })
            }
        }

        pub fn seek_secs(&mut self, secs: f32) -> Result<(), AudioError> {
            self.ended = false;
            let pos = (secs.max(0.0) as f64 * 10_000_000.0) as i64;
            unsafe {
                let prop = PROPVARIANT::from(pos);
                self.reader
                    .SetCurrentPosition(&GUID::zeroed(), &prop)
                    .map_err(|e| AudioError::Message(e.to_string()))?;
            }
            Ok(())
        }

        pub fn pull_until(
            &mut self,
            until_secs: f32,
            out: &mut Vec<AudioChunk>,
        ) -> Result<(), AudioError> {
            if self.ended {
                return Ok(());
            }
            let stream = self.stream;
            let mut guard = 0u32;
            while guard < 64 {
                guard += 1;
                unsafe {
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
                        .map_err(|e| AudioError::Message(e.to_string()))?;

                    if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
                        self.ended = true;
                        break;
                    }
                    let Some(sample) = sample else {
                        continue;
                    };
                    let pts_secs = (timestamp.max(0) as f32) / 10_000_000.0;
                    let samples = sample_to_f32(&sample)?;
                    if !samples.is_empty() {
                        out.push(AudioChunk { pts_secs, samples });
                    }
                    if pts_secs >= until_secs {
                        break;
                    }
                }
            }
            Ok(())
        }
    }

    fn configure_pcm_float(
        reader: &IMFSourceReader,
        stream: u32,
    ) -> Result<AudioFormat, AudioError> {
        unsafe {
            let native = reader
                .GetNativeMediaType(stream, 0)
                .map_err(|_| AudioError::NoAudio)?;
            let channels = native
                .GetUINT32(&MF_MT_AUDIO_NUM_CHANNELS)
                .unwrap_or(2)
                .clamp(1, 2) as u16;
            let sample_rate = native
                .GetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND)
                .unwrap_or(48_000)
                .max(8_000);

            let mt = MFCreateMediaType().map_err(|e| AudioError::Message(e.to_string()))?;
            mt.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Audio)
                .map_err(|e| AudioError::Message(e.to_string()))?;
            mt.SetGUID(&MF_MT_SUBTYPE, &MFAudioFormat_Float)
                .map_err(|e| AudioError::Message(e.to_string()))?;
            mt.SetUINT32(&MF_MT_AUDIO_NUM_CHANNELS, channels as u32)
                .map_err(|e| AudioError::Message(e.to_string()))?;
            mt.SetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND, sample_rate)
                .map_err(|e| AudioError::Message(e.to_string()))?;
            mt.SetUINT32(&MF_MT_AUDIO_BITS_PER_SAMPLE, 32)
                .map_err(|e| AudioError::Message(e.to_string()))?;
            let block = channels as u32 * 4;
            mt.SetUINT32(&MF_MT_AUDIO_BLOCK_ALIGNMENT, block)
                .map_err(|e| AudioError::Message(e.to_string()))?;
            mt.SetUINT32(&MF_MT_AUDIO_AVG_BYTES_PER_SECOND, sample_rate * block)
                .map_err(|e| AudioError::Message(e.to_string()))?;

            reader
                .SetCurrentMediaType(stream, None, &mt)
                .map_err(|e| AudioError::Message(format!("audio media type: {e}")))?;

            Ok(AudioFormat {
                sample_rate,
                channels,
            })
        }
    }

    fn sample_to_f32(sample: &IMFSample) -> Result<Vec<f32>, AudioError> {
        unsafe {
            let buffer = sample
                .ConvertToContiguousBuffer()
                .map_err(|e| AudioError::Message(e.to_string()))?;
            let mut data: *mut u8 = std::ptr::null_mut();
            let mut max_len = 0u32;
            let mut cur_len = 0u32;
            buffer
                .Lock(&mut data, Some(&mut max_len), Some(&mut cur_len))
                .map_err(|e| AudioError::Message(e.to_string()))?;
            let bytes = std::slice::from_raw_parts(data, cur_len as usize);
            let mut out = Vec::with_capacity(bytes.len() / 4);
            for chunk in bytes.chunks_exact(4) {
                out.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
            }
            buffer.Unlock().ok();
            Ok(out)
        }
    }

    fn path_to_file_url(path: &Path) -> Result<Vec<u16>, AudioError> {
        let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
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
}
