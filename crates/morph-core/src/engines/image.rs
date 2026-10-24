use crate::error::{MorphError, Result};
use crate::format::Format;
use crate::output::{OutputManager, OutputOptions};
use image::{DynamicImage, ImageFormat};
use pdf_writer::{Content, Finish, Name, Pdf, Rect, Ref};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct ImageEngine;

impl ImageEngine {
    /// Convert an image file from one format to another (PNG, JPG, WebP, BMP, TIFF, GIF, ICO)
    pub fn convert_image(
        input_path: &Path,
        target_format: &Format,
        quality: Option<u8>,
        output_options: &OutputOptions,
    ) -> Result<PathBuf> {
        let destination = OutputManager::resolve_destination(
            input_path,
            target_format.extension(),
            output_options,
        )?;

        let img = image::open(input_path).map_err(|e| {
            MorphError::ImageError(format!(
                "Failed to decode image '{}': {}",
                input_path.display(),
                e
            ))
        })?;

        let staging = OutputManager::create_staging_file(
            destination.parent().unwrap_or_else(|| Path::new(".")),
        )?;

        let img_format = match target_format {
            Format::Png => ImageFormat::Png,
            Format::Jpeg => ImageFormat::Jpeg,
            Format::Webp => ImageFormat::WebP,
            Format::Bmp => ImageFormat::Bmp,
            Format::Tiff => ImageFormat::Tiff,
            Format::Gif => ImageFormat::Gif,
            Format::Ico => ImageFormat::Ico,
            other => {
                return Err(MorphError::UnsupportedFormat(format!(
                    "Cannot convert image to format '{}'",
                    other
                )));
            }
        };

        let file = staging.as_file();
        let mut writer = std::io::BufWriter::new(file);

        // Handle alpha channel when target format does not support transparency (e.g. JPEG)
        let processed_img = if target_format == &Format::Jpeg && img.color().has_alpha() {
            // Flatten onto white background
            let rgba = img.to_rgba8();
            let mut rgb = image::RgbImage::new(img.width(), img.height());
            for (x, y, pixel) in rgba.enumerate_pixels() {
                let alpha = pixel[3] as f32 / 255.0;
                let r = (pixel[0] as f32 * alpha + 255.0 * (1.0 - alpha)).round() as u8;
                let g = (pixel[1] as f32 * alpha + 255.0 * (1.0 - alpha)).round() as u8;
                let b = (pixel[2] as f32 * alpha + 255.0 * (1.0 - alpha)).round() as u8;
                rgb.put_pixel(x, y, image::Rgb([r, g, b]));
            }
            DynamicImage::ImageRgb8(rgb)
        } else if target_format == &Format::Ico
            && (img.width() > 256 || img.height() > 256)
        {
            img.resize(256, 256, image::imageops::FilterType::Lanczos3)
        } else {
            img
        };

        if target_format == &Format::Jpeg {
            let q = quality.unwrap_or(85);
            let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut writer, q);
            encoder
                .encode(
                    processed_img.as_bytes(),
                    processed_img.width(),
                    processed_img.height(),
                    processed_img.color().into(),
                )
                .map_err(|e| MorphError::ImageError(format!("Failed to encode JPEG: {}", e)))?;
        } else {
            processed_img
                .write_to(&mut writer, img_format)
                .map_err(|e| MorphError::ImageError(format!("Failed to encode image: {}", e)))?;
        }

        writer.flush()?;
        drop(writer);

        OutputManager::finalize_atomic(staging, &destination)
    }

    /// Convert a single image or multiple images into a multi-page PDF document.
    pub fn images_to_pdf(
        image_paths: &[PathBuf],
        output_options: &OutputOptions,
    ) -> Result<PathBuf> {
        if image_paths.is_empty() {
            return Err(MorphError::InvalidInput(
                "No images provided for PDF creation".into(),
            ));
        }

        let first_image = &image_paths[0];
        let destination = OutputManager::resolve_destination(first_image, "pdf", output_options)?;

        let staging = OutputManager::create_staging_file(
            destination.parent().unwrap_or_else(|| Path::new(".")),
        )?;

        let mut pdf = Pdf::new();
        let catalog_id = Ref::new(1);
        let pages_id = Ref::new(2);

        let num_pages = image_paths.len();
        let mut page_ids = Vec::with_capacity(num_pages);
        let mut next_ref_val = 3;

        // Collect page descriptors
        struct PageData {
            page_id: Ref,
            image_id: Ref,
            mask_id: Option<Ref>,
            content_id: Ref,
            width: u32,
            height: u32,
            raw_rgb: Vec<u8>,
            mask_bytes: Option<Vec<u8>>,
            is_jpeg: bool,
            jpeg_bytes: Option<Vec<u8>>,
        }

        let mut pages = Vec::with_capacity(num_pages);

        for path in image_paths {
            let page_id = Ref::new(next_ref_val);
            next_ref_val += 1;
            let image_id = Ref::new(next_ref_val);
            next_ref_val += 1;
            let content_id = Ref::new(next_ref_val);
            next_ref_val += 1;

            page_ids.push(page_id);

            // Fast path: If the file is already a JPEG, we can embed it directly without decoding/recompressing!
            let file_bytes = fs::read(path)?;
            let is_jpeg = file_bytes.len() >= 3 && &file_bytes[0..3] == b"\xFF\xD8\xFF";

            if is_jpeg {
                // Use image crate to get dimensions only
                let reader = image::ImageReader::open(path)
                    .map_err(|e| MorphError::ImageError(e.to_string()))?
                    .into_dimensions()
                    .map_err(|e| MorphError::ImageError(e.to_string()))?;
                let (width, height) = reader;

                pages.push(PageData {
                    page_id,
                    image_id,
                    mask_id: None,
                    content_id,
                    width,
                    height,
                    raw_rgb: Vec::new(),
                    mask_bytes: None,
                    is_jpeg: true,
                    jpeg_bytes: Some(file_bytes),
                });
            } else {
                let img = image::open(path).map_err(|e| {
                    MorphError::ImageError(format!(
                        "Failed to open image '{}': {}",
                        path.display(),
                        e
                    ))
                })?;

                let width = img.width();
                let height = img.height();

                if img.color().has_alpha() {
                    let rgba = img.to_rgba8();
                    let mut rgb_bytes = Vec::with_capacity((width * height * 3) as usize);
                    let mut mask_bytes = Vec::with_capacity((width * height) as usize);

                    for pixel in rgba.pixels() {
                        rgb_bytes.push(pixel[0]);
                        rgb_bytes.push(pixel[1]);
                        rgb_bytes.push(pixel[2]);
                        mask_bytes.push(pixel[3]);
                    }

                    let mask_id = Ref::new(next_ref_val);
                    next_ref_val += 1;

                    pages.push(PageData {
                        page_id,
                        image_id,
                        mask_id: Some(mask_id),
                        content_id,
                        width,
                        height,
                        raw_rgb: rgb_bytes,
                        mask_bytes: Some(mask_bytes),
                        is_jpeg: false,
                        jpeg_bytes: None,
                    });
                } else {
                    let rgb = img.to_rgb8();
                    pages.push(PageData {
                        page_id,
                        image_id,
                        mask_id: None,
                        content_id,
                        width,
                        height,
                        raw_rgb: rgb.into_raw(),
                        mask_bytes: None,
                        is_jpeg: false,
                        jpeg_bytes: None,
                    });
                }
            }
        }

        // 1. Write Catalog
        pdf.catalog(catalog_id).pages(pages_id);

        // 2. Write Pages dictionary
        pdf.pages(pages_id)
            .kids(page_ids.iter().copied())
            .count(num_pages as i32);

        // 3. Write each Page, Image XObject, and Content stream
        for p in pages {
            let pw = p.width as f32;
            let ph = p.height as f32;

            // Page
            let mut page = pdf.page(p.page_id);
            page.parent(pages_id);
            page.media_box(Rect::new(0.0, 0.0, pw, ph));
            page.contents(p.content_id);
            let mut resources = page.resources();
            let mut xobjs = resources.x_objects();
            xobjs.pair(Name(b"Im0"), p.image_id);
            xobjs.finish();
            resources.finish();
            page.finish();

            // Content stream: scale image to page size
            let mut content = Content::new();
            content.save_state();
            content.transform([pw, 0.0, 0.0, ph, 0.0, 0.0]);
            content.x_object(Name(b"Im0"));
            content.restore_state();
            pdf.stream(p.content_id, &content.finish());

            // Image XObject
            if p.is_jpeg {
                let bytes = p.jpeg_bytes.unwrap();
                let mut image_xobj = pdf.image_xobject(p.image_id, &bytes);
                image_xobj.width(p.width as i32);
                image_xobj.height(p.height as i32);
                image_xobj.color_space().device_rgb();
                image_xobj.bits_per_component(8);
                image_xobj.filter(pdf_writer::Filter::DctDecode);
                image_xobj.finish();
            } else {
                // Compress raw RGB with flate/miniz
                let compressed_rgb = miniz_oxide::deflate::compress_to_vec_zlib(&p.raw_rgb, 6);
                let mut image_xobj = pdf.image_xobject(p.image_id, &compressed_rgb);
                image_xobj.width(p.width as i32);
                image_xobj.height(p.height as i32);
                image_xobj.color_space().device_rgb();
                image_xobj.bits_per_component(8);
                image_xobj.filter(pdf_writer::Filter::FlateDecode);

                if let (Some(m_id), Some(m_bytes)) = (p.mask_id, p.mask_bytes) {
                    image_xobj.s_mask(m_id);
                    image_xobj.finish();

                    // Mask XObject
                    let compressed_mask = miniz_oxide::deflate::compress_to_vec_zlib(&m_bytes, 6);
                    let mut mask_xobj = pdf.image_xobject(m_id, &compressed_mask);
                    mask_xobj.width(p.width as i32);
                    mask_xobj.height(p.height as i32);
                    mask_xobj.color_space().device_gray();
                    mask_xobj.bits_per_component(8);
                    mask_xobj.filter(pdf_writer::Filter::FlateDecode);
                    mask_xobj.finish();
                } else {
                    image_xobj.finish();
                }
            }
        }

        let pdf_data = pdf.finish();
        let mut file = staging.as_file();
        file.write_all(&pdf_data)?;
        file.flush()?;

        OutputManager::finalize_atomic(staging, &destination)
    }
}
