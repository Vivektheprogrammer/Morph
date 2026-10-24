pub mod detection;
pub mod engines;
pub mod error;
pub mod format;
pub mod job;
pub mod output;
pub mod registry;

pub use detection::{detect_format_from_bytes, detect_format_from_path};
pub use engines::document::DocumentEngine;
pub use engines::image::ImageEngine;
pub use engines::pdf::PdfEngine;
pub use error::{MorphError, Result};
pub use format::{Format, FormatCategory};
pub use job::{Job, JobStatus};
pub use output::{OutputManager, OutputOptions};
pub use registry::{ConversionOperation, ConversionRegistry};

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
