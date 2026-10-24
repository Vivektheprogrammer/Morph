use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FormatCategory {
    Image,
    Document,
    Spreadsheet,
    Presentation,
    Audio,
    Video,
    Archive,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Format {
    // Images
    Png,
    Jpeg,
    Webp,
    Bmp,
    Tiff,
    Gif,
    Ico,
    Svg,

    // Documents
    Pdf,
    Docx,
    Doc,
    Odt,
    Rtf,
    Txt,
    Markdown,
    Html,

    // Spreadsheets
    Xlsx,
    Xls,
    Ods,
    Csv,

    // Presentations
    Pptx,
    Ppt,
    Odp,

    // Audio
    Mp3,
    Wav,
    Flac,
    Aac,
    Ogg,
    Opus,
    M4a,

    // Video
    Mp4,
    Mkv,
    Mov,
    Avi,
    Webm,

    // Archives
    Zip,
    Tar,
    Gz,
    SevenZip,

    // Other
    Custom(String),
}

impl Format {
    pub fn extension(&self) -> &'static str {
        match self {
            Format::Png => "png",
            Format::Jpeg => "jpg",
            Format::Webp => "webp",
            Format::Bmp => "bmp",
            Format::Tiff => "tiff",
            Format::Gif => "gif",
            Format::Ico => "ico",
            Format::Svg => "svg",

            Format::Pdf => "pdf",
            Format::Docx => "docx",
            Format::Doc => "doc",
            Format::Odt => "odt",
            Format::Rtf => "rtf",
            Format::Txt => "txt",
            Format::Markdown => "md",
            Format::Html => "html",

            Format::Xlsx => "xlsx",
            Format::Xls => "xls",
            Format::Ods => "ods",
            Format::Csv => "csv",

            Format::Pptx => "pptx",
            Format::Ppt => "ppt",
            Format::Odp => "odp",

            Format::Mp3 => "mp3",
            Format::Wav => "wav",
            Format::Flac => "flac",
            Format::Aac => "aac",
            Format::Ogg => "ogg",
            Format::Opus => "opus",
            Format::M4a => "m4a",

            Format::Mp4 => "mp4",
            Format::Mkv => "mkv",
            Format::Mov => "mov",
            Format::Avi => "avi",
            Format::Webm => "webm",

            Format::Zip => "zip",
            Format::Tar => "tar",
            Format::Gz => "gz",
            Format::SevenZip => "7z",

            Format::Custom(_) => "bin",
        }
    }

    pub fn mime_type(&self) -> &'static str {
        match self {
            Format::Png => "image/png",
            Format::Jpeg => "image/jpeg",
            Format::Webp => "image/webp",
            Format::Bmp => "image/bmp",
            Format::Tiff => "image/tiff",
            Format::Gif => "image/gif",
            Format::Ico => "image/x-icon",
            Format::Svg => "image/svg+xml",

            Format::Pdf => "application/pdf",
            Format::Docx => {
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            }
            Format::Doc => "application/msword",
            Format::Odt => "application/vnd.oasis.opendocument.text",
            Format::Rtf => "application/rtf",
            Format::Txt => "text/plain",
            Format::Markdown => "text/markdown",
            Format::Html => "text/html",

            Format::Xlsx => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            Format::Xls => "application/vnd.ms-excel",
            Format::Ods => "application/vnd.oasis.opendocument.spreadsheet",
            Format::Csv => "text/csv",

            Format::Pptx => {
                "application/vnd.openxmlformats-officedocument.presentationml.presentation"
            }
            Format::Ppt => "application/vnd.ms-powerpoint",
            Format::Odp => "application/vnd.oasis.opendocument.presentation",

            Format::Mp3 => "audio/mpeg",
            Format::Wav => "audio/wav",
            Format::Flac => "audio/flac",
            Format::Aac => "audio/aac",
            Format::Ogg => "audio/ogg",
            Format::Opus => "audio/opus",
            Format::M4a => "audio/mp4",

            Format::Mp4 => "video/mp4",
            Format::Mkv => "video/x-matroska",
            Format::Mov => "video/quicktime",
            Format::Avi => "video/x-msvideo",
            Format::Webm => "video/webm",

            Format::Zip => "application/zip",
            Format::Tar => "application/x-tar",
            Format::Gz => "application/gzip",
            Format::SevenZip => "application/x-7z-compressed",

            Format::Custom(_) => "application/octet-stream",
        }
    }

    pub fn category(&self) -> FormatCategory {
        match self {
            Format::Png
            | Format::Jpeg
            | Format::Webp
            | Format::Bmp
            | Format::Tiff
            | Format::Gif
            | Format::Ico
            | Format::Svg => FormatCategory::Image,

            Format::Pdf
            | Format::Docx
            | Format::Doc
            | Format::Odt
            | Format::Rtf
            | Format::Txt
            | Format::Markdown
            | Format::Html => FormatCategory::Document,

            Format::Xlsx | Format::Xls | Format::Ods | Format::Csv => FormatCategory::Spreadsheet,

            Format::Pptx | Format::Ppt | Format::Odp => FormatCategory::Presentation,

            Format::Mp3
            | Format::Wav
            | Format::Flac
            | Format::Aac
            | Format::Ogg
            | Format::Opus
            | Format::M4a => FormatCategory::Audio,

            Format::Mp4 | Format::Mkv | Format::Mov | Format::Avi | Format::Webm => {
                FormatCategory::Video
            }

            Format::Zip | Format::Tar | Format::Gz | Format::SevenZip => FormatCategory::Archive,

            Format::Custom(_) => FormatCategory::Unknown,
        }
    }

    pub fn from_extension(ext: &str) -> Option<Self> {
        let clean_ext = ext.trim().trim_start_matches('.').to_lowercase();
        match clean_ext.as_str() {
            "png" => Some(Format::Png),
            "jpg" | "jpeg" | "jpe" => Some(Format::Jpeg),
            "webp" => Some(Format::Webp),
            "bmp" => Some(Format::Bmp),
            "tiff" | "tif" => Some(Format::Tiff),
            "gif" => Some(Format::Gif),
            "ico" => Some(Format::Ico),
            "svg" => Some(Format::Svg),

            "pdf" => Some(Format::Pdf),
            "docx" => Some(Format::Docx),
            "doc" => Some(Format::Doc),
            "odt" => Some(Format::Odt),
            "rtf" => Some(Format::Rtf),
            "txt" => Some(Format::Txt),
            "md" | "markdown" => Some(Format::Markdown),
            "html" | "htm" => Some(Format::Html),

            "xlsx" => Some(Format::Xlsx),
            "xls" => Some(Format::Xls),
            "ods" => Some(Format::Ods),
            "csv" => Some(Format::Csv),

            "pptx" => Some(Format::Pptx),
            "ppt" => Some(Format::Ppt),
            "odp" => Some(Format::Odp),

            "mp3" => Some(Format::Mp3),
            "wav" => Some(Format::Wav),
            "flac" => Some(Format::Flac),
            "aac" => Some(Format::Aac),
            "ogg" => Some(Format::Ogg),
            "opus" => Some(Format::Opus),
            "m4a" => Some(Format::M4a),

            "mp4" => Some(Format::Mp4),
            "mkv" => Some(Format::Mkv),
            "mov" => Some(Format::Mov),
            "avi" => Some(Format::Avi),
            "webm" => Some(Format::Webm),

            "zip" => Some(Format::Zip),
            "tar" => Some(Format::Tar),
            "gz" => Some(Format::Gz),
            "7z" => Some(Format::SevenZip),

            _ => None,
        }
    }
}

impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.extension().to_uppercase())
    }
}
