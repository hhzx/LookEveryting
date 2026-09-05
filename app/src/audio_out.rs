//! WASAPI (via cpal) output for decoded PCM float.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};

/// Shared playback state between video worker and cpal callback.
pub struct AudioShared {
    pub queue: Mutex<VecDeque<f32>>,
    pub volume_bits: AtomicU32,
    pub playing: AtomicBool,
    pub channels: u16,
    pub sample_rate: u32,
}

impl AudioShared {
    pub fn new(sample_rate: u32, channels: u16) -> Self {
        Self {
            queue: Mutex::new(VecDeque::with_capacity(sample_rate as usize * channels as usize)),
            volume_bits: AtomicU32::new(1.0_f32.to_bits()),
            playing: AtomicBool::new(false),
            channels,
            sample_rate,
        }
    }

    pub fn set_volume(&self, volume: f32) {
        self.volume_bits
            .store(volume.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    }

    pub fn set_playing(&self, playing: bool) {
        self.playing.store(playing, Ordering::Relaxed);
        if !playing {
            self.clear();
        }
    }

    pub fn clear(&self) {
        if let Ok(mut q) = self.queue.lock() {
            q.clear();
        }
    }

    pub fn push_samples(&self, samples: &[f32]) {
        let Ok(mut q) = self.queue.lock() else {
            return;
        };
        q.extend(samples.iter().copied());
        // Cap ~1.0s of backlog to avoid lag after seeks.
        let cap = self.sample_rate as usize * self.channels as usize;
        while q.len() > cap {
            q.pop_front();
        }
    }

    pub fn queued_secs(&self) -> f32 {
        let Ok(q) = self.queue.lock() else {
            return 0.0;
        };
        let frames = q.len() / self.channels.max(1) as usize;
        frames as f32 / self.sample_rate.max(1) as f32
    }
}

pub struct AudioOut {
    pub shared: Arc<AudioShared>,
    _stream: Stream,
}

impl AudioOut {
    pub fn try_start(sample_rate: u32, channels: u16) -> Option<Self> {
        let host = cpal::default_host();
        let device = host.default_output_device()?;
        let config = device.default_output_config().ok()?;
        let sample_format = config.sample_format();

        let mut stream_config: StreamConfig = config.into();
        // Prefer source rate when device allows; otherwise let cpal use default and we still push floats.
        if stream_config.sample_rate.0 == 0 {
            stream_config.sample_rate = cpal::SampleRate(sample_rate);
        }
        // Keep device channel count if stereo/mono mismatch — we'll expand/collapse in callback.
        let device_channels = stream_config.channels;
        let shared = Arc::new(AudioShared::new(sample_rate, channels));
        let shared_cb = Arc::clone(&shared);

        let err_fn = |e| eprintln!("audio stream error: {e}");

        let stream = match sample_format {
            SampleFormat::F32 => device
                .build_output_stream(
                    &stream_config,
                    move |data: &mut [f32], _| {
                        fill_f32(data, &shared_cb, device_channels, channels);
                    },
                    err_fn,
                    None,
                )
                .ok()?,
            SampleFormat::I16 => device
                .build_output_stream(
                    &stream_config,
                    move |data: &mut [i16], _| {
                        let mut tmp = vec![0.0_f32; data.len()];
                        fill_f32(&mut tmp, &shared_cb, device_channels, channels);
                        for (d, s) in data.iter_mut().zip(tmp.iter()) {
                            *d = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                        }
                    },
                    err_fn,
                    None,
                )
                .ok()?,
            _ => return None,
        };

        stream.play().ok()?;
        Some(Self {
            shared,
            _stream: stream,
        })
    }
}

fn fill_f32(data: &mut [f32], shared: &AudioShared, device_ch: u16, source_ch: u16) {
    let volume = f32::from_bits(shared.volume_bits.load(Ordering::Relaxed));
    let playing = shared.playing.load(Ordering::Relaxed);
    data.fill(0.0);
    if !playing || volume <= 0.0001 {
        if let Ok(mut q) = shared.queue.lock() {
            // Drain slowly when muted/paused to avoid huge backlog on resume.
            let drop_n = (data.len() / 4).min(q.len());
            for _ in 0..drop_n {
                q.pop_front();
            }
        }
        return;
    }

    let Ok(mut q) = shared.queue.lock() else {
        return;
    };

    let frames = data.len() / device_ch.max(1) as usize;
    for frame in 0..frames {
        let mut src = [0.0_f32; 2];
        for c in 0..source_ch.min(2) as usize {
            src[c] = q.pop_front().unwrap_or(0.0) * volume;
        }
        if source_ch == 1 {
            src[1] = src[0];
        }
        for c in 0..device_ch as usize {
            let sample = if c < 2 { src[c] } else { 0.0 };
            data[frame * device_ch as usize + c] = sample;
        }
    }
}
