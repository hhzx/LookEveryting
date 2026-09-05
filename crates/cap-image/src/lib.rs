//! Decode raster images into RGBA buffers for GPU upload.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use cap_core::MediaKind;
use image::codecs::jpeg::JpegDecoder;
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ImageError {
    #[error("unsupported image format")]
    UnsupportedFormat,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("decode error: {0}")]
    Decode(#[from] image::ImageError),
}

/// Decoded image ready for texture upload.
#[derive(Debug, Clone)]
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    /// Original file pixel width (may exceed `width` when downscaled).
    pub native_width: u32,
    /// Original file pixel height (may exceed `height` when downscaled).
    pub native_height: u32,
}

impl DecodedImage {
    pub fn from_path(path: &Path) -> Result<Self, ImageError> {
        if cap_core::classify_extension(path) != Some(MediaKind::Image) {
            return Err(ImageError::UnsupportedFormat);
        }

        let reader = ImageReader::open(path)?;
        let format = reader
            .format()
            .or_else(|| guess_format(path))
            .ok_or(ImageError::UnsupportedFormat)?;
        let image = reader.with_guessed_format()?.decode()?;
        Ok(Self::from_dynamic(image, format))
    }

    pub fn from_dynamic(image: DynamicImage, _format: ImageFormat) -> Self {
        let rgba = image.to_rgba8();
        let (width, height) = rgba.dimensions();
        Self {
            width,
            height,
            rgba: rgba.into_raw(),
            native_width: width,
            native_height: height,
        }
    }

    pub fn with_native(mut self, native_width: u32, native_height: u32) -> Self {
        self.native_width = native_width;
        self.native_height = native_height;
        self
    }

    pub fn is_capped(&self) -> bool {
        self.width != self.native_width || self.height != self.native_height
    }

    pub fn aspect_ratio(&self) -> f32 {
        if self.height == 0 {
            1.0
        } else {
            self.width as f32 / self.height as f32
        }
    }

    pub fn megapixels(&self) -> f32 {
        (self.native_width as f64 * self.native_height as f64 / 1_000_000.0) as f32
    }
}

/// Max edge for on-screen viewing (avoids decoding 24MP+ into memory).
pub const MAX_VIEW_EDGE: u32 = 4096;

/// Fast first-paint preview edge.
pub const PREVIEW_EDGE: u32 = 512;

/// Prefetch cache edge for neighbor files.
pub const PREFETCH_EDGE: u32 = 2048;

/// Decode once, optionally emit a low-res preview and a view-sized image.
pub fn decode_staged(
    path: &Path,
    preview_edge: u32,
    view_edge: u32,
) -> Result<(Option<DecodedImage>, DecodedImage), ImageError> {
    if cap_core::classify_extension(path) != Some(MediaKind::Image) {
        return Err(ImageError::UnsupportedFormat);
    }

    let (native_w, native_h) = probe_dimensions(path)?;
    let max_dim = native_w.max(native_h);

    let reader = ImageReader::open(path)?;
    let format = reader
        .format()
        .or_else(|| guess_format(path))
        .ok_or(ImageError::UnsupportedFormat)?;
    let image = reader.with_guessed_format()?.decode()?;

    let preview = if max_dim > preview_edge {
        Some(
            DecodedImage::from_dynamic(image.thumbnail(preview_edge, preview_edge), format)
                .with_native(native_w, native_h),
        )
    } else {
        None
    };

    let view = if max_dim > view_edge {
        DecodedImage::from_dynamic(image.thumbnail(view_edge, view_edge), format)
            .with_native(native_w, native_h)
    } else {
        DecodedImage::from_dynamic(image, format)
    };

    Ok((preview, view))
}

/// Lightweight decode for prefetch cache (single decode, capped resolution).
pub fn decode_prefetch(path: &Path, max_edge: u32) -> Result<DecodedImage, ImageError> {
    if cap_core::classify_extension(path) != Some(MediaKind::Image) {
        return Err(ImageError::UnsupportedFormat);
    }
    let (native_w, native_h) = probe_dimensions(path)?;
    if native_w.max(native_h) <= max_edge {
        return DecodedImage::from_path(path);
    }
    let reader = ImageReader::open(path)?;
    let format = reader
        .format()
        .or_else(|| guess_format(path))
        .ok_or(ImageError::UnsupportedFormat)?;
    let image = reader.with_guessed_format()?.decode()?;
    Ok(
        DecodedImage::from_dynamic(image.thumbnail(max_edge, max_edge), format)
            .with_native(native_w, native_h),
    )
}

/// Decode a downscaled thumbnail for the filmstrip UI.
pub fn decode_thumbnail(path: &Path, max_size: u32) -> Result<DecodedImage, ImageError> {
    if cap_core::classify_extension(path) != Some(MediaKind::Image) {
        return Err(ImageError::UnsupportedFormat);
    }
    let (w, h) = probe_dimensions(path)?;
    if w.max(h) <= max_size {
        DecodedImage::from_path(path)
    } else {
        let reader = ImageReader::open(path)?;
        let format = reader
            .format()
            .or_else(|| guess_format(path))
            .ok_or(ImageError::UnsupportedFormat)?;
        let image = reader.with_guessed_format()?.decode()?;
        Ok(
            DecodedImage::from_dynamic(image.thumbnail(max_size, max_size), format)
                .with_native(w, h),
        )
    }
}

/// Full-resolution decode (on demand for 1:1 viewing).
pub fn decode_full(path: &Path) -> Result<DecodedImage, ImageError> {
    DecodedImage::from_path(path)
}

/// Load image dimensions without full decode.
pub fn probe_dimensions(path: &Path) -> Result<(u32, u32), ImageError> {
    if is_jpeg(path) {
        if let Ok(dims) = jpeg_probe_dimensions(path) {
            return Ok(dims);
        }
    }
    let reader = ImageReader::open(path)?.with_guessed_format()?;
    Ok(reader.into_dimensions()?)
}

fn is_jpeg(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase()),
        Some(ext) if ext == "jpg" || ext == "jpeg"
    )
}

fn jpeg_probe_dimensions(path: &Path) -> Result<(u32, u32), ImageError> {
    let file = File::open(path)?;
    let decoder = JpegDecoder::new(BufReader::new(file))?;
    Ok(decoder.dimensions())
}

fn guess_format(path: &Path) -> Option<ImageFormat> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "jpg" | "jpeg" => Some(ImageFormat::Jpeg),
        "png" => Some(ImageFormat::Png),
        "gif" => Some(ImageFormat::Gif),
        "webp" => Some(ImageFormat::WebP),
        "bmp" => Some(ImageFormat::Bmp),
        "tif" | "tiff" => Some(ImageFormat::Tiff),
        "ico" => Some(ImageFormat::Ico),
        "avif" => Some(ImageFormat::Avif),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};

    fn sample_png(path: &Path) {
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_fn(4, 2, |x, y| Rgba([x as u8 * 40, y as u8 * 80, 200, 255]));
        img.save(path).unwrap();
    }

    #[test]
    fn decodes_png() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sample.png");
        sample_png(&path);
        let decoded = DecodedImage::from_path(&path).unwrap();
        assert_eq!(decoded.width, 4);
        assert_eq!(decoded.height, 2);
        assert_eq!(decoded.native_width, 4);
        assert_eq!(decoded.rgba.len(), 4 * 2 * 4);
    }

    #[test]
    fn rejects_non_images() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("video.mp4");
        std::fs::write(&path, b"fake").unwrap();
        let err = DecodedImage::from_path(&path).unwrap_err();
        assert!(matches!(err, ImageError::UnsupportedFormat));
    }

    #[test]
    fn decode_staged_scales_large_image() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.png");
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_fn(800, 600, |x, y| Rgba([x as u8, y as u8, 128, 255]));
        img.save(&path).unwrap();
        let (preview, view) = decode_staged(&path, 512, 4096).unwrap();
        assert!(preview.is_some());
        assert_eq!(preview.unwrap().width, 512);
        assert_eq!(view.width, 800);
        assert_eq!(view.native_width, 800);
    }

    #[test]
    fn capped_flag() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.png");
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_fn(800, 600, |_, _| Rgba([128, 128, 128, 255]));
        img.save(&path).unwrap();
        let (_, view) = decode_staged(&path, 512, 256).unwrap();
        assert!(view.is_capped());
        assert_eq!(view.native_width, 800);
    }
}
