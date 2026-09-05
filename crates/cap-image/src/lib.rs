//! Decode raster images into RGBA buffers for GPU upload.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::SystemTime;

use cap_core::MediaKind;
use image::codecs::jpeg::JpegDecoder;
use image::{AnimationDecoder, DynamicImage, ImageDecoder, ImageFormat, ImageReader};
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

/// One frame of an animated GIF (or similar).
#[derive(Debug, Clone)]
pub struct AnimFrame {
    pub image: DecodedImage,
    pub delay_ms: u32,
}

/// Decode GIF animation frames. Returns empty if not a multi-frame GIF.
pub fn decode_gif_animation(path: &Path) -> Result<Vec<AnimFrame>, ImageError> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase());
    if ext.as_deref() != Some("gif") {
        return Ok(Vec::new());
    }
    let file = File::open(path)?;
    let decoder = image::codecs::gif::GifDecoder::new(BufReader::new(file))?;
    let mut frames = Vec::new();
    let (native_w, native_h) = decoder.dimensions();
    for frame in decoder.into_frames() {
        let frame = frame?;
        let delay = frame.delay().numer_denom_ms();
        let delay_ms = if delay.1 == 0 {
            100
        } else {
            ((delay.0 as u64 * 1000) / delay.1 as u64).max(20) as u32
        };
        let rgba = frame.into_buffer();
        let (w, h) = rgba.dimensions();
        frames.push(AnimFrame {
            image: DecodedImage {
                width: w,
                height: h,
                rgba: rgba.into_raw(),
                native_width: native_w,
                native_height: native_h,
            },
            delay_ms,
        });
    }
    Ok(frames)
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

/// Lightweight file / EXIF-lite metadata for the info panel.
#[derive(Debug, Clone)]
pub struct ImageMeta {
    pub width: u32,
    pub height: u32,
    pub file_size: u64,
    pub modified: SystemTime,
    pub format: String,
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub datetime: Option<String>,
}

impl ImageMeta {
    pub fn file_size_label(&self) -> String {
        format_bytes(self.file_size)
    }

    pub fn megapixels(&self) -> f32 {
        (self.width as f64 * self.height as f64 / 1_000_000.0) as f32
    }
}

/// Read native dimensions, file stats, and optional EXIF camera fields.
pub fn read_image_meta(path: &Path) -> Result<ImageMeta, ImageError> {
    if cap_core::classify_extension(path) != Some(MediaKind::Image) {
        return Err(ImageError::UnsupportedFormat);
    }

    let (width, height) = probe_dimensions(path)?;
    let meta = std::fs::metadata(path)?;
    let file_size = meta.len();
    let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    let format = guess_format(path)
        .map(format_label)
        .or_else(|| {
            path.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_ascii_uppercase())
        })
        .unwrap_or_else(|| "IMAGE".to_string());

    let (camera_make, camera_model, datetime) = read_exif_lite(path);

    Ok(ImageMeta {
        width,
        height,
        file_size,
        modified,
        format,
        camera_make,
        camera_model,
        datetime,
    })
}

fn format_label(format: ImageFormat) -> String {
    match format {
        ImageFormat::Jpeg => "JPEG".into(),
        ImageFormat::Png => "PNG".into(),
        ImageFormat::Gif => "GIF".into(),
        ImageFormat::WebP => "WEBP".into(),
        ImageFormat::Bmp => "BMP".into(),
        ImageFormat::Tiff => "TIFF".into(),
        ImageFormat::Ico => "ICO".into(),
        ImageFormat::Avif => "AVIF".into(),
        _ => format!("{format:?}").to_ascii_uppercase(),
    }
}

fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let b = bytes as f64;
    if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.2} MB", b / MB)
    } else if b >= KB {
        format!("{:.1} KB", b / KB)
    } else {
        format!("{bytes} B")
    }
}

fn read_exif_lite(path: &Path) -> (Option<String>, Option<String>, Option<String>) {
    let Ok(file) = File::open(path) else {
        return (None, None, None);
    };
    let mut reader = BufReader::new(file);
    let Ok(exif) = exif::Reader::new().read_from_container(&mut reader) else {
        return (None, None, None);
    };

    let field = |tag: exif::Tag| {
        exif.get_field(tag, exif::In::PRIMARY)
            .map(|f| f.display_value().with_unit(&exif).to_string())
            .map(|s| s.trim().trim_matches('"').to_string())
            .filter(|s| !s.is_empty())
    };

    let datetime = field(exif::Tag::DateTimeOriginal).or_else(|| field(exif::Tag::DateTime));

    (field(exif::Tag::Make), field(exif::Tag::Model), datetime)
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

    #[test]
    fn reads_image_meta() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sample.png");
        sample_png(&path);
        let meta = read_image_meta(&path).unwrap();
        assert_eq!(meta.width, 4);
        assert_eq!(meta.height, 2);
        assert_eq!(meta.format, "PNG");
        assert!(meta.file_size > 0);
        assert!(meta.camera_make.is_none());
    }
}
