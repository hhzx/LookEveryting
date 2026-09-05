//! YUV → RGBA conversion for MF decoded frames.

/// Convert NV12 (Y + interleaved UV) to RGBA8888.
pub fn nv12_to_rgba(
    y_plane: &[u8],
    uv_plane: &[u8],
    width: u32,
    height: u32,
    stride_y: usize,
    stride_uv: usize,
    out: &mut [u8],
) {
    let w = width as usize;
    let h = height as usize;
    for row in 0..h {
        for col in 0..w {
            let y = y_plane[row * stride_y + col] as i32;
            let uv_row = (row / 2) * stride_uv + (col & !1);
            let u = uv_plane.get(uv_row).copied().unwrap_or(128) as i32 - 128;
            let v = uv_plane.get(uv_row + 1).copied().unwrap_or(128) as i32 - 128;
            let r = (y + ((1436 * v) >> 10)).clamp(0, 255) as u8;
            let g = (y - ((352 * u + 731 * v) >> 10)).clamp(0, 255) as u8;
            let b = (y + ((1814 * u) >> 10)).clamp(0, 255) as u8;
            let i = (row * w + col) * 4;
            if i + 3 < out.len() {
                out[i] = r;
                out[i + 1] = g;
                out[i + 2] = b;
                out[i + 3] = 255;
            }
        }
    }
}

/// Convert YUY2 packed 4:2:2 to RGBA8888.
pub fn yuy2_to_rgba(data: &[u8], width: u32, height: u32, stride: usize, out: &mut [u8]) {
    let w = width as usize;
    let h = height as usize;
    for row in 0..h {
        let row_start = row * stride;
        for col in (0..w).step_by(2) {
            let base = row_start + col * 2;
            if base + 3 >= data.len() {
                continue;
            }
            let y0 = data[base] as i32;
            let u = data[base + 1] as i32 - 128;
            let y1 = data[base + 2] as i32;
            let v = data[base + 3] as i32 - 128;
            for (y, px) in [(y0, col), (y1, col + 1)] {
                if px >= w {
                    continue;
                }
                let r = (y + ((1436 * v) >> 10)).clamp(0, 255) as u8;
                let g = (y - ((352 * u + 731 * v) >> 10)).clamp(0, 255) as u8;
                let b = (y + ((1814 * u) >> 10)).clamp(0, 255) as u8;
                let i = (row * w + px) * 4;
                if i + 3 < out.len() {
                    out[i] = r;
                    out[i + 1] = g;
                    out[i + 2] = b;
                    out[i + 3] = 255;
                }
            }
        }
    }
}
