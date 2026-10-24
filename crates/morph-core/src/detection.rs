use crate::error::{MorphError, Result};
use crate::format::Format;
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub fn detect_format_from_bytes(bytes: &[u8]) -> Option<Format> {
    if bytes.len() >= 8 && &bytes[0..8] == b"\x89PNG\r\n\x1a\n" {
        return Some(Format::Png);
    }

    if bytes.len() >= 3 && &bytes[0..3] == b"\xFF\xD8\xFF" {
        return Some(Format::Jpeg);
    }

    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Some(Format::Webp);
    }

    if bytes.len() >= 6 && (&bytes[0..6] == b"GIF87a" || &bytes[0..6] == b"GIF89a") {
        return Some(Format::Gif);
    }

    if bytes.len() >= 2 && &bytes[0..2] == b"BM" {
        return Some(Format::Bmp);
    }

    if bytes.len() >= 4
        && ((&bytes[0..4] == b"\x49\x49\x2A\x00") || (&bytes[0..4] == b"\x4D\x4D\x00\x2A"))
    {
        return Some(Format::Tiff);
    }

    if bytes.len() >= 5 && &bytes[0..5] == b"%PDF-" {
        return Some(Format::Pdf);
    }

    if bytes.len() >= 4 && &bytes[0..4] == b"\x50\x4B\x03\x04" {
        return Some(Format::Zip);
    }

    if bytes.len() >= 2 && &bytes[0..2] == b"\x1F\x8B" {
        return Some(Format::Gz);
    }

    if bytes.len() >= 6 && &bytes[0..6] == b"\x37\x7A\xBC\xAF\x27\x1C" {
        return Some(Format::SevenZip);
    }

    if bytes.len() >= 3 && &bytes[0..3] == b"ID3" {
        return Some(Format::Mp3);
    }

    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE" {
        return Some(Format::Wav);
    }

    if bytes.len() >= 4 && &bytes[0..4] == b"fLaC" {
        return Some(Format::Flac);
    }

    if bytes.len() >= 4 && &bytes[0..4] == b"OggS" {
        return Some(Format::Ogg);
    }

    None
}

pub fn detect_format_from_path(path: &Path) -> Result<Format> {
    if !path.exists() {
        return Err(MorphError::FileNotFound(path.to_path_buf()));
    }

    if !path.is_file() {
        return Err(MorphError::InvalidInput(format!(
            "Path '{}' is a directory, not a file",
            path.display()
        )));
    }

    let mut file = File::open(path)?;
    let metadata = file.metadata()?;
    if metadata.len() == 0 {
        return Err(MorphError::InvalidInput(format!(
            "File '{}' is empty (0 bytes)",
            path.display()
        )));
    }

    let mut header = [0u8; 64];
    let bytes_read = file.read(&mut header)?;
    let slice = &header[..bytes_read];

    let magic_format = detect_format_from_bytes(slice);
    let ext_format = path
        .extension()
        .and_then(|ext| ext.to_str())
        .and_then(Format::from_extension);

    match (magic_format, ext_format) {
        // Zip container can be a Office document (DOCX, XLSX, PPTX)
        (Some(Format::Zip), Some(specific @ (Format::Docx | Format::Xlsx | Format::Pptx))) => {
            Ok(specific)
        }
        (Some(Format::Zip), _) => Ok(Format::Zip),

        // If magic byte detection succeeded, trust it over extension
        (Some(detected), _) => Ok(detected),

        // Fall back to extension for text, markdown, csv, svg, etc.
        (None, Some(ext_fmt)) => Ok(ext_fmt),

        (None, None) => Err(MorphError::DetectionFailed(path.to_path_buf())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_png_magic_bytes() {
        let png_header = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR";
        assert_eq!(detect_format_from_bytes(png_header), Some(Format::Png));
    }

    #[test]
    fn test_jpeg_magic_bytes() {
        let jpeg_header = b"\xFF\xD8\xFF\xE0\x00\x10JFIF";
        assert_eq!(detect_format_from_bytes(jpeg_header), Some(Format::Jpeg));
    }

    #[test]
    fn test_pdf_magic_bytes() {
        let pdf_header = b"%PDF-1.7\r\n%\xb5\xed\xae\xfb";
        assert_eq!(detect_format_from_bytes(pdf_header), Some(Format::Pdf));
    }
}
