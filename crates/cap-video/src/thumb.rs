//! Lightweight first-frame thumbnail extraction for filmstrip.

use std::path::Path;

use crate::{VideoFrame, VideoPlayer};

/// Decode a downscaled first frame for thumbnail use.
pub fn decode_thumbnail(path: &Path, max_edge: u32) -> Option<VideoFrame> {
    let player = VideoPlayer::open_with_options(path.to_path_buf(), true).ok()?;
    let frame = player.current_frame()?.clone();
    Some(scale_frame(frame, max_edge.max(32)))
}

fn scale_frame(frame: VideoFrame, max_edge: u32) -> VideoFrame {
    let w = frame.width.max(1);
    let h = frame.height.max(1);
    let scale = (max_edge as f32 / w.max(h) as f32).min(1.0);
    if (scale - 1.0).abs() < f32::EPSILON {
        return frame;
    }
    let nw = ((w as f32 * scale).round() as u32).max(1);
    let nh = ((h as f32 * scale).round() as u32).max(1);
    let mut rgba = vec![0u8; (nw * nh * 4) as usize];
    for y in 0..nh {
        let sy = (y as f32 / nh as f32 * h as f32) as u32;
        for x in 0..nw {
            let sx = (x as f32 / nw as f32 * w as f32) as u32;
            let si = ((sy * w + sx) * 4) as usize;
            let di = ((y * nw + x) * 4) as usize;
            if si + 3 < frame.rgba.len() && di + 3 < rgba.len() {
                rgba[di..di + 4].copy_from_slice(&frame.rgba[si..si + 4]);
            }
        }
    }
    VideoFrame {
        width: nw,
        height: nh,
        rgba,
    }
}
