//! Decode raster images into RGBA buffers for GPU upload.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use cap_core::MediaKind;
use image::{DynamicImage, GenericImageView, ImageFormat, ImageReader};
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
        }
    }

    pub fn aspect_ratio(&self) -> f32 {
        if self.height == 0 {
            1.0
        } else {
            self.width as f32 / self.height as f32
        }
    }

    pub fn megapixels(&self) -> f32 {
        (self.width as f64 * self.height as f64 / 1_000_000.0) as f32
    }
}

/// Load image dimensions without full decode when possible.
pub fn probe_dimensions(path: &Path) -> Result<(u32, u32), ImageError> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let image = ImageReader::new(reader).with_guessed_format()?.decode()?;
    Ok(image.dimensions())
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
}
