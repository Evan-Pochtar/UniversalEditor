use image::{DynamicImage, ImageEncoder};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExportFormat {
    Jpeg,
    Png,
    Webp,
    Bmp,
    Tiff,
    Ico,
}

impl ExportFormat {
    pub fn as_str(&self) -> &str {
        match self {
            ExportFormat::Jpeg => "JPEG",
            ExportFormat::Png => "PNG",
            ExportFormat::Webp => "WebP",
            ExportFormat::Bmp => "BMP",
            ExportFormat::Tiff => "TIFF",
            ExportFormat::Ico => "ICO",
        }
    }

    pub fn extension(&self) -> &str {
        match self {
            ExportFormat::Jpeg => "jpg",
            ExportFormat::Png => "png",
            ExportFormat::Webp => "webp",
            ExportFormat::Bmp => "bmp",
            ExportFormat::Tiff => "tiff",
            ExportFormat::Ico => "ico",
        }
    }

    pub fn all() -> Vec<ExportFormat> {
        vec![
            ExportFormat::Jpeg,
            ExportFormat::Png,
            ExportFormat::Webp,
            ExportFormat::Bmp,
            ExportFormat::Tiff,
            ExportFormat::Ico,
        ]
    }
}

pub fn export_image(
    img: &DynamicImage,
    path: &Path,
    format: ExportFormat,
    jpeg_quality: u8,
    png_compression: u8,
    _webp_quality: f32,
    auto_scale_ico: bool,
) -> Result<(), String> {
    let mut export_img = img.clone();
    
    if format == ExportFormat::Ico && auto_scale_ico {
        if export_img.width() > 256 || export_img.height() > 256 {
            let scale = 256.0 / export_img.width().max(export_img.height()) as f32;
            let new_width = (export_img.width() as f32 * scale) as u32;
            let new_height = (export_img.height() as f32 * scale) as u32;
            export_img = export_img.resize(new_width, new_height, image::imageops::FilterType::Lanczos3);
        }
    }

    match format {
        ExportFormat::Jpeg => {
            let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
                std::fs::File::create(path).map_err(|e| format!("Failed to create file: {}", e))?,
                jpeg_quality,
            );
            encoder.encode_image(&export_img)
                .map_err(|e| format!("Failed to encode JPEG: {}", e))?;
        }
        ExportFormat::Png => {
            let file = std::fs::File::create(path)
                .map_err(|e| format!("Failed to create file: {}", e))?;
            let compression = match png_compression {
                0..=3 => image::codecs::png::CompressionType::Fast,
                4..=6 => image::codecs::png::CompressionType::Default,
                _ => image::codecs::png::CompressionType::Best,
            };
            let encoder = image::codecs::png::PngEncoder::new_with_quality(
                file,
                compression,
                image::codecs::png::FilterType::Adaptive,
            );
            encoder.write_image(
                export_img.as_bytes(),
                export_img.width(),
                export_img.height(),
                export_img.color().into(),
            ).map_err(|e| format!("Failed to encode PNG: {}", e))?;
        }
        ExportFormat::Webp => {
            export_img.save_with_format(path, image::ImageFormat::WebP)
                .map_err(|e| format!("Failed to save WebP: {}", e))?;
        }
        ExportFormat::Bmp => {
            export_img.save_with_format(path, image::ImageFormat::Bmp)
                .map_err(|e| format!("Failed to save BMP: {}", e))?;
        }
        ExportFormat::Tiff => {
            export_img.save_with_format(path, image::ImageFormat::Tiff)
                .map_err(|e| format!("Failed to save TIFF: {}", e))?;
        }
        ExportFormat::Ico => {
            if export_img.width() > 256 || export_img.height() > 256 {
                return Err(format!(
                    "ICO format requires dimensions ≤256px. Image is {}x{}. Enable auto-scaling.",
                    export_img.width(), export_img.height()
                ));
            }
            export_img.save_with_format(path, image::ImageFormat::Ico)
                .map_err(|e| format!("Failed to save ICO: {}", e))?;
        }
    }

    Ok(())
}