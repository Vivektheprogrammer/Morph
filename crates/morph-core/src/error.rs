use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MorphError {
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    #[error("File already exists and overwrite was not permitted: {0}")]
    FileAlreadyExists(PathBuf),

    #[error("Could not detect format for file: {0}")]
    DetectionFailed(PathBuf),

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("Incompatible conversion from {from} to {to}")]
    IncompatibleConversion { from: String, to: String },

    #[error("Empty or invalid file content: {0}")]
    InvalidInput(String),

    #[error("Corrupted file ({format}): {details}")]
    CorruptedFile { format: String, details: String },

    #[error("Image processing error: {0}")]
    ImageError(String),

    #[error("PDF operation error: {0}")]
    PdfError(String),

    #[error("Output validation failed: {0}")]
    ValidationFailed(String),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Operation was cancelled")]
    Cancelled,
}

pub type Result<T> = std::result::Result<T, MorphError>;
