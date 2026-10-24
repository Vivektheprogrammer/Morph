use image::{Rgb, RgbImage, Rgba, RgbaImage};
use morph_core::engines::image::ImageEngine;
use morph_core::engines::pdf::PdfEngine;
use morph_core::format::Format;
use morph_core::output::OutputOptions;
use morph_core::{detect_format_from_path, MorphError};
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

fn create_sample_png(path: &PathBuf, width: u32, height: u32) {
    let mut img = RgbaImage::new(width, height);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        if (x + y) % 2 == 0 {
            *pixel = Rgba([255, 0, 0, 255]); // Solid red
        } else {
            *pixel = Rgba([0, 0, 255, 128]); // Semi-transparent blue
        }
    }
    img.save(path).unwrap();
}

fn create_sample_jpg(path: &PathBuf, width: u32, height: u32) {
    let mut img = RgbImage::new(width, height);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        *pixel = Rgb([((x * 255) / width) as u8, ((y * 255) / height) as u8, 128]);
    }
    img.save(path).unwrap();
}

#[test]
fn test_image_to_pdf_single() {
    let temp = tempdir().unwrap();
    let img_path = temp.path().join("document.png");
    create_sample_png(&img_path, 200, 100);

    let options = OutputOptions::default();
    let pdf_path = ImageEngine::images_to_pdf(&[img_path.clone()], &options).unwrap();

    assert!(pdf_path.exists());
    assert_eq!(pdf_path, temp.path().join("document.pdf"));

    let format = detect_format_from_path(&pdf_path).unwrap();
    assert_eq!(format, Format::Pdf);

    let pages = PdfEngine::page_count(&pdf_path).unwrap();
    assert_eq!(pages, 1);
}

#[test]
fn test_image_to_pdf_multiple() {
    let temp = tempdir().unwrap();
    let img1 = temp.path().join("page1.png");
    let img2 = temp.path().join("page2.jpg");
    let img3 = temp.path().join("page3.png");

    create_sample_png(&img1, 150, 150);
    create_sample_jpg(&img2, 200, 300);
    create_sample_png(&img3, 100, 100);

    let options = OutputOptions::default();
    let pdf_path =
        ImageEngine::images_to_pdf(&[img1.clone(), img2.clone(), img3.clone()], &options).unwrap();

    assert!(pdf_path.exists());
    assert_eq!(pdf_path, temp.path().join("page1.pdf"));

    let pages = PdfEngine::page_count(&pdf_path).unwrap();
    assert_eq!(pages, 3);
}

#[test]
fn test_image_cross_conversion() {
    let temp = tempdir().unwrap();
    let png_path = temp.path().join("source.png");
    create_sample_png(&png_path, 100, 80);

    let options = OutputOptions::default();

    // 1. Convert PNG -> JPG
    let jpg_path =
        ImageEngine::convert_image(&png_path, &Format::Jpeg, Some(90), &options).unwrap();
    assert!(jpg_path.exists());
    assert_eq!(detect_format_from_path(&jpg_path).unwrap(), Format::Jpeg);

    // 2. Convert JPG -> PNG
    let converted_png =
        ImageEngine::convert_image(&jpg_path, &Format::Png, None, &options).unwrap();
    assert!(converted_png.exists());
    assert_eq!(
        detect_format_from_path(&converted_png).unwrap(),
        Format::Png
    );
}

#[test]
fn test_pdf_structural_operations() {
    let temp = tempdir().unwrap();
    let img1 = temp.path().join("p1.png");
    let img2 = temp.path().join("p2.png");
    let img3 = temp.path().join("p3.png");

    create_sample_png(&img1, 100, 100);
    create_sample_png(&img2, 100, 100);
    create_sample_png(&img3, 100, 100);

    let options = OutputOptions::default();
    let initial_pdf = ImageEngine::images_to_pdf(&[img1, img2, img3], &options).unwrap();
    assert_eq!(PdfEngine::page_count(&initial_pdf).unwrap(), 3);

    // 1. Rotate page 2 by 90 degrees
    let rotated = PdfEngine::rotate_pages(&initial_pdf, &[2], 90, &options).unwrap();
    assert!(rotated.exists());
    assert_eq!(PdfEngine::page_count(&rotated).unwrap(), 3);

    // 2. Delete page 2 -> should have 2 pages
    let deleted = PdfEngine::delete_pages(&rotated, &[2], &options).unwrap();
    assert_eq!(PdfEngine::page_count(&deleted).unwrap(), 2);

    // 3. Reorder pages ([2, 1]) -> should have 2 pages
    let reordered = PdfEngine::reorder_pages(&deleted, &[2, 1], &options).unwrap();
    assert_eq!(PdfEngine::page_count(&reordered).unwrap(), 2);

    // 4. Merge initial_pdf (3 pages) and reordered (2 pages) -> should produce 5 pages
    let merged = PdfEngine::merge_pdfs(&[initial_pdf, reordered], &options).unwrap();
    assert_eq!(PdfEngine::page_count(&merged).unwrap(), 5);
}

#[test]
fn test_unicode_and_spaces_in_path() {
    let temp = tempdir().unwrap();
    let subfolder = temp.path().join("Spaced Folder - フォルダ");
    fs::create_dir_all(&subfolder).unwrap();

    let img_path = subfolder.join("photo with spaces - 写真.png");
    create_sample_png(&img_path, 120, 120);

    let options = OutputOptions::default();
    let pdf_path = ImageEngine::images_to_pdf(&[img_path], &options).unwrap();
    assert!(pdf_path.exists());
    assert_eq!(PdfEngine::page_count(&pdf_path).unwrap(), 1);
}

#[test]
fn test_error_handling_empty_and_missing() {
    let temp = tempdir().unwrap();
    let empty_file = temp.path().join("empty.png");
    fs::write(&empty_file, b"").unwrap();

    let missing_file = temp.path().join("non_existent.png");

    let err1 = detect_format_from_path(&empty_file);
    assert!(matches!(err1, Err(MorphError::InvalidInput(_))));

    let err2 = detect_format_from_path(&missing_file);
    assert!(matches!(err2, Err(MorphError::FileNotFound(_))));
}

#[test]
fn test_docx_to_pdf_conversion() {
    use morph_core::engines::document::DocumentEngine;

    let sample_docx = std::path::PathBuf::from("c:/Morph/test_samples/report.docx");
    if sample_docx.exists() && DocumentEngine::is_msword_available() {
        let temp = tempdir().unwrap();
        let target_docx = temp.path().join("report.docx");
        fs::copy(&sample_docx, &target_docx).unwrap();

        let options = OutputOptions::default();
        let pdf_path = DocumentEngine::convert_to_pdf(&target_docx, &options).unwrap();

        assert!(pdf_path.exists());
        assert_eq!(pdf_path, temp.path().join("report.pdf"));

        let format = detect_format_from_path(&pdf_path).unwrap();
        assert_eq!(format, Format::Pdf);

        let pages = PdfEngine::page_count(&pdf_path).unwrap();
        assert!(pages >= 1);
    }
}
