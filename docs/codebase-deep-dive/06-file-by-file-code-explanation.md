# 06: Comprehensive File-by-File Code Walkthrough

This document contains an exhaustive, deep-dive examination of every source code file in the Morph repository across all three workspace crates (`morph-core`, `morph-cli`, `morph-ui`), build scripts, and installer automation.

---

# SECTION 1: `morph-core` Crate

---

## 1.1 `crates/morph-core/src/lib.rs`

### 1. File Overview
- **Path**: `crates/morph-core/src/lib.rs`
- **Purpose**: Root module for the `morph-core` library crate. Declares all internal submodules and re-exports the public API surface.
- **Role**: Serves as the single integration point through which external consumers (`morph-cli`, `morph-ui`, and tests) access conversion operations, format sniffing, and PDF manipulation.

### 2. Imports and Declarations
```rust
pub mod detection;
pub mod engines;
pub mod error;
pub mod format;
pub mod job;
pub mod output;
pub mod registry;
```
- `pub mod ...`: Declares each respective Rust source file as a public module within the crate's namespace hierarchy.
- Re-exports high-frequency structs and functions to provide a flat, ergonomic import interface:
```rust
pub use detection::{detect_format_from_bytes, detect_format_from_path};
pub use engines::document::DocumentEngine;
pub use engines::image::ImageEngine;
pub use engines::pdf::{PageSpec, PdfEngine};
pub use error::{MorphError, Result};
pub use format::{Format, FormatCategory};
pub use job::{Job, JobStatus};
pub use output::{OutputManager, OutputOptions};
pub use registry::{ConversionOperation, ConversionRegistry};
```

### 3. Functions
```rust
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
```
- **Purpose**: Returns the semantic version string of `morph-core` baked in at compile time.
- **Mechanism**: The macro `env!("CARGO_PKG_VERSION")` reads the version defined in `Cargo.toml` during compilation and injects a static string slice literal (`&'static str`), avoiding runtime string allocations.

---

## 1.2 `crates/morph-core/src/error.rs`

### 1. File Overview
- **Path**: `crates/morph-core/src/error.rs`
- **Purpose**: Defines the centralized error domain `MorphError` and the convenient `Result<T>` type alias.
- **Role**: Guarantees that any operational failure (missing files, invalid magic bytes, corrupted PDFs, I/O exceptions) produces an informative, typed error rather than panicking or crashing.

### 2. Data Structures & Derives
```rust
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
```

### 3. Detailed Mechanics
- `#[derive(Error, Debug)]`: Employs `thiserror` to automatically generate `std::fmt::Display` and `std::error::Error` trait implementations at compile time.
- `#[from] std::io::Error`: Generates an implementation of `From<std::io::Error>` for `MorphError`. This allows any standard library I/O operation (e.g., `fs::File::open`, `fs::rename`) to automatically convert into `MorphError::IoError` when using the `?` operator.
- `pub type Result<T>`: Eliminates boilerplate throughout the engine by standardizing return types to `Result<T, MorphError>`.

---

## 1.3 `crates/morph-core/src/format.rs`

### 1. File Overview
- **Path**: `crates/morph-core/src/format.rs`
- **Purpose**: Defines the format taxonomy, categorizations, standard file extensions, and MIME type mappings.
- **Role**: Provides the strongly-typed foundation used by capability lookups, file sniffers, and engine routers.

### 2. Enums
- **`FormatCategory`**: Classifies file types into `Image`, `Document`, `Spreadsheet`, `Presentation`, `Audio`, `Video`, `Archive`, and `Unknown`.
- **`Format`**: An exhaustive algebraic sum type covering 30+ formats including `Png`, `Jpeg`, `Webp`, `Bmp`, `Tiff`, `Gif`, `Ico`, `Svg`, `Pdf`, `Docx`, `Xlsx`, `Pptx`, `Zip`, and an open-ended `Custom(String)` fallback.

### 3. Key Methods on `Format`
- `pub fn extension(&self) -> &'static str`: Returns the canonical lowercase file extension without leading dots (e.g., `"png"`, `"pdf"`, `"docx"`).
- `pub fn mime_type(&self) -> &'static str`: Returns the standard RFC MIME string (e.g., `"application/pdf"`, `"image/png"`, `"application/vnd.openxmlformats-officedocument.wordprocessingml.document"`).
- `pub fn category(&self) -> FormatCategory`: Maps any format variant to its broader functional category.
- `pub fn from_extension(ext: &str) -> Option<Self>`: Strips whitespace and leading periods, converts to lowercase, and pattern-matches against supported extensions. Returns `Some(Format)` or `None`.

---

## 1.4 `crates/morph-core/src/detection.rs`

### 1. File Overview
- **Path**: `crates/morph-core/src/detection.rs`
- **Purpose**: Binary format sniffing using magic numbers (file header signatures) combined with path extension fallbacks.
- **Role**: Prevents corrupted or malicious files with spoofed extensions (e.g., an executable renamed to `.pdf`) from being processed incorrectly.

### 2. Functions & Implementation Details

#### `detect_format_from_bytes(bytes: &[u8]) -> Option<Format>`
Inspects raw byte slices against known magic byte signatures:
- **PNG**: Byte sequence `\x89PNG\r\n\x1a\n` (`[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]`).
- **JPEG**: Byte sequence `\xFF\xD8\xFF`.
- **WebP**: 12-byte RIFF container checking `bytes[0..4] == b"RIFF"` and `bytes[8..12] == b"WEBP"`.
- **GIF**: Header matching `b"GIF87a"` or `b"GIF89a"`.
- **BMP**: Header matching `b"BM"`.
- **TIFF**: Little-endian `\x49\x49\x2A\x00` (`II*`) or big-endian `\x4D\x4D\x00\x2A` (`MM*`).
- **PDF**: Header sequence `b"%PDF-"`.
- **ZIP**: Header sequence `\x50\x4B\x03\x04` (`PK\x03\x04`).
- **Audio Formats**: MP3 (`ID3`), WAV (`RIFF....WAVE`), FLAC (`fLaC`), Ogg (`OggS`).

#### `detect_format_from_path(path: &Path) -> Result<Format>`
```rust
pub fn detect_format_from_path(path: &Path) -> Result<Format> {
    if !path.exists() {
        return Err(MorphError::FileNotFound(path.to_path_buf()));
    }
    if !path.is_file() {
        return Err(MorphError::InvalidInput(format!("Path '{}' is a directory", path.display())));
    }
    let mut file = File::open(path)?;
    let metadata = file.metadata()?;
    if metadata.len() == 0 {
        return Err(MorphError::InvalidInput(format!("File '{}' is empty (0 bytes)", path.display())));
    }

    let mut header = [0u8; 64];
    let bytes_read = file.read(&mut header)?;
    let slice = &header[..bytes_read];

    let magic_format = detect_format_from_bytes(slice);
    let ext_format = path.extension().and_then(|ext| ext.to_str()).and_then(Format::from_extension);

    match (magic_format, ext_format) {
        // Zip container can be an Office document (DOCX, XLSX, PPTX)
        (Some(Format::Zip), Some(specific @ (Format::Docx | Format::Xlsx | Format::Pptx))) => Ok(specific),
        (Some(Format::Zip), _) => Ok(Format::Zip),
        (Some(detected), _) => Ok(detected),
        (None, Some(ext_fmt)) => Ok(ext_fmt),
        (None, None) => Err(MorphError::DetectionFailed(path.to_path_buf())),
    }
}
```
**Architectural Intelligence**:
Microsoft Office files (`.docx`, `.xlsx`, `.pptx`) are internally ZIP archives containing XML content. If the sniffer detects a ZIP header but the file extension indicates `.docx`, it correctly returns `Format::Docx`. For pure binary formats (PNG, JPEG, PDF), magic byte detection takes precedence over extensions.

---

## 1.5 `crates/morph-core/src/registry.rs`

### 1. File Overview
- **Path**: `crates/morph-core/src/registry.rs`
- **Purpose**: Capability-based conversion registry storing operation descriptors and execution requirements.
- **Role**: Decouples feature availability from engine invocation, allowing the UI and CLI to dynamically discover available conversions.

### 2. Data Structures
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionOperation {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub source_formats: Vec<Format>,
    pub target_format: Format,
    pub offline_capable: bool,
    pub instant_capable: bool,
    pub requires_editor: bool,
    pub lossless: bool,
}

pub struct ConversionRegistry {
    operations: Vec<ConversionOperation>,
}
```

### 3. Registered Core Operations
1. `img_to_pdf`: Direct instant raster-to-PDF conversion (PNG, JPG, WebP, BMP, TIFF, GIF, ICO $\rightarrow$ PDF).
2. `img_to_png`: Cross-format raster conversion to PNG.
3. `img_to_jpg`: Cross-format raster conversion to JPG (lossy, configurable quality).
4. `img_to_webp`: Cross-format raster conversion to WebP.
5. `multi_img_to_pdf`: Combining multiple images into a unified multi-page PDF.
6. `pdf_merge`: Lossless structural PDF merge without rasterization.
7. `doc_to_pdf`: Document bridge converting DOCX, DOC, ODT, RTF to PDF via Word COM or LibreOffice.

---

## 1.6 `crates/morph-core/src/output.rs`

### 1. File Overview
- **Path**: `crates/morph-core/src/output.rs`
- **Purpose**: Collision resolution, safe temporary staging allocation, and atomic file finalization.
- **Role**: Prevents accidental overwrites and ensures partial or failed conversions never corrupt existing user files.

### 2. Key Methods

#### `OutputManager::resolve_destination(source, target_ext, options) -> Result<PathBuf>`
- Determines parent directory (source parent or custom output directory).
- Derives base filename stem.
- If destination does not exist or `options.overwrite == true`, returns the path directly.
- **Collision Loop**: If destination exists, iteratively evaluates `stem (1).ext`, `stem (2).ext`, ... up to 10,000 attempts, returning the first non-colliding candidate path.

#### `OutputManager::create_staging_file(destination_dir) -> Result<NamedTempFile>`
- Uses `tempfile::Builder` to create `.morph-tmp-XXXXXX.tmp` strictly within the destination directory.
- **Why sibling staging is critical**: In Windows NTFS, moving a file across different drive volumes (`C:\` to `D:\`) requires a slow byte copy followed by a delete. Creating the staging file on the *same volume* enables an instantaneous atomic directory entry pointer update via `MoveFileExW` / `fs::rename`.

#### `OutputManager::finalize_atomic(staging, destination) -> Result<PathBuf>`
- Validates that the staging file byte length is greater than zero (`metadata.len() > 0`).
- If destination already exists and overwrite was permitted, removes the destination file first to ensure atomic persist semantics on Windows.
- Calls `staging.persist(destination)` to finalize the file.

---

## 1.7 `crates/morph-core/src/job.rs`

### 1. File Overview
- **Path**: `crates/morph-core/src/job.rs`
- **Purpose**: Conversion lifecycle state machine and progress tracking entity.
- **Role**: Models the state of long-running operations (`Queued`, `Running`, `Validating`, `Completed`, `Failed`, `Cancelled`).

---

## 1.8 `crates/morph-core/src/engines/image.rs`

### 1. File Overview
- **Path**: `crates/morph-core/src/engines/image.rs`
- **Purpose**: Cross-format raster conversions and single/multi-image PDF generation.
- **Role**: Provides fast, high-fidelity image transformations and transparent PDF embedding.

### 2. Methods & Algorithms

#### `ImageEngine::convert_image(input, target_format, quality, options) -> Result<PathBuf>`
1. Decodes source image via `image::open(input)`.
2. **Alpha Flattening Algorithm**: If target is JPEG (which cannot represent alpha channels) and the source has an alpha channel, flattens pixels onto a white background:
   ```rust
   let alpha = pixel[3] as f32 / 255.0;
   let r = (pixel[0] as f32 * alpha + 255.0 * (1.0 - alpha)).round() as u8;
   let g = (pixel[1] as f32 * alpha + 255.0 * (1.0 - alpha)).round() as u8;
   let b = (pixel[2] as f32 * alpha + 255.0 * (1.0 - alpha)).round() as u8;
   ```
3. **ICO Clamping**: If target is ICO and width/height exceed 256 pixels, resizes using Lanczos3 resampling.
4. **Encoding**: Dispatches to `image::codecs::jpeg::JpegEncoder` with quality setting (default 85) or standard format writers.
5. Writes to staging file and finalizes atomically.

#### `ImageEngine::images_to_pdf(image_paths, options) -> Result<PathBuf>`
Constructs a valid PDF from scratch using `pdf-writer`:
1. **JPEG Passthrough Fast Path**: If an input file starts with `\xFF\xD8\xFF` (JPEG), Morph reads the dimensions via `ImageReader::into_dimensions()` without decoding the image, embedding the raw JPEG bytes directly into the PDF using `/Filter /DctDecode`. This achieves instantaneous, mathematically lossless embedding.
2. **Alpha Separation & Soft Masking**: For images with transparency (PNG/WebP), splits the image into:
   - Base RGB stream (3 bytes per pixel).
   - Alpha mask stream (1 byte per pixel, gray).
   - Compresses both streams using `miniz_oxide::deflate::compress_to_vec_zlib(..., 6)`.
   - Links the alpha mask via the `/SMask` dictionary attribute on the main `/XObject`.
3. Sets up PDF Catalog, `/Pages` tree with `/MediaBox [0 0 pw ph]`, and a `/Contents` stream containing transformation matrix scaling:
   ```rust
   content.transform([pw, 0.0, 0.0, ph, 0.0, 0.0]);
   content.x_object(Name(b"Im0"));
   ```

---

## 1.9 `crates/morph-core/src/engines/pdf.rs`

### 1. File Overview
- **Path**: `crates/morph-core/src/engines/pdf.rs`
- **Purpose**: Lossless structural PDF manipulation via `lopdf`.
- **Role**: Merges, rotates, deletes, reorders, and extracts text snippets from PDF documents without lossy rasterization.

### 2. Methods & Algorithms

#### `PdfEngine::merge_pdfs(input_paths, output_options) -> Result<PathBuf>`
1. Iterates through all input PDFs, renumbering objects with `doc.renumber_objects_with(max_id)` so that object IDs across files do not collide.
2. Collects all page object IDs and transfers non-Catalog, non-Pages objects into a new `Document::with_version("1.7")`.
3. Creates a new `/Pages` dictionary with `/Kids` referencing all page IDs and updates each page's `/Parent` pointer to reference this new Pages object.
4. Generates a new `/Catalog` dictionary and assigns it as the document `/Root` in the trailer.
5. Writes the merged document to an atomic staging file.

#### `PdfEngine::rotate_pages(input_path, pages_to_rotate, degrees, options) -> Result<PathBuf>`
- Loads the document, traverses page dictionaries, extracts the `/Rotate` attribute (defaulting to 0), and computes:
  $$\text{new\_rotation} = (\text{current\_rotation} + \text{degrees}) \pmod{360}$$
- Overwrites the `/Rotate` integer in the dictionary and saves the document.

#### `PdfEngine::delete_pages(input_path, pages_to_delete, options) -> Result<PathBuf>`
- Invokes `doc.delete_pages(pages_to_delete)`, which automatically removes page references from the `/Kids` array and decrements `/Count`.

#### `PdfEngine::reorder_pages(input_path, new_order, options) -> Result<PathBuf>`
- Extracts page object IDs in the specified sequence, updates the `/Kids` array and `/Count` attribute of the parent `/Pages` dictionary, and serializes the updated AST.

#### `PdfEngine::assemble_document(pages: &[PageSpec], options) -> Result<PathBuf>`
- Universally combines mixed pages (extracting individual pages from different PDFs, rotating them, converting accompanying images to PDF) into temporary single-page files, and then merges them into the final destination.

---

## 1.10 `crates/morph-core/src/engines/document.rs`

### 1. File Overview
- **Path**: `crates/morph-core/src/engines/document.rs`
- **Purpose**: Headless bridge converting office documents (DOCX, DOC, ODT, RTF) to PDF.
- **Role**: Enables high-fidelity document conversion without cloud dependencies by leveraging locally installed Microsoft Word or LibreOffice.

### 2. Methods
- `is_msword_available()`: Checks registry key `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\winword.exe`.
- `is_libreoffice_available()`: Inspects `C:\Program Files\LibreOffice\program\soffice.exe`.
- `convert_with_msword()`: Executes a hidden PowerShell COM script creating `New-Object -ComObject Word.Application`, calling `$doc.ExportAsFixedFormat(out_path, 17)` (wdExportFormatPDF), and releasing the COM object in a `finally` block.
- `convert_with_libreoffice()`: Invokes `soffice.exe --headless --convert-to pdf --outdir <temp> <input>`.

---

# SECTION 2: `morph-cli` Crate

---

## 2.1 `crates/morph-cli/build.rs`
- Compiles `assets/logo.ico` into an embedded Windows PE resource using `winres::WindowsResource::new()`. Triggers re-run on icon changes.

## 2.2 `crates/morph-cli/src/windows/context_menu.rs`
- **`register_current_exe()`**:
  - Traverses `IMAGE_EXTENSIONS` (`.png`, `.jpg`, `.jpeg`, `.webp`, `.bmp`, `.tiff`) and `DOCUMENT_EXTENSIONS` (`.docx`, `.doc`, `.rtf`, `.odt`), plus `.pdf`.
  - Creates subkeys under `HKCU\Software\Classes\SystemFileAssociations\<ext>\shell\Morph`.
  - Sets `MUIVerb = "Morph"`, `SubCommands = ""`, and `Icon = "<path>\logo.ico"`.
  - Configures cascading submenus: `ConvertToPdf`, `ConvertToPng`, `ConvertToJpg`, `ConvertToWebp`, and `OpenEditor`.
  - Calls `SHChangeNotify(0x08000000, 0, null, null)` (`SHCNE_ASSOCCHANGED`) to notify Windows Explorer immediately.
- **`unregister()`**: Cleanly deletes the `Morph` subkey across all registered extensions.

## 2.3 `crates/morph-cli/src/windows/notify.rs`
- **`send_notification(title, body)`**: Dispatches an asynchronous, hidden PowerShell process (`CREATE_NO_WINDOW = 0x08000000`) executing a WinRT `ToastNotificationManager` script.

## 2.4 `crates/morph-cli/src/main.rs`
- Controls CLI execution flow, handles arguments for `convert`, `merge`, `pdf`, `register`, `unregister`, and attaches to terminal sessions via `AttachConsole`.

---

# SECTION 3: `morph-ui` Crate

---

## 3.1 `crates/morph-ui/src/motion/mod.rs`
- Implements `Spring` (mass-spring-damper oscillator with `stiffness: 180.0`, `damping: 18.0`) and `cubic_bezier` easing algorithms (`ease_standard`, `ease_enter`, `ease_exit`) for smooth 60 FPS transitions.

## 3.2 `crates/morph-ui/src/state/mod.rs`
- Implements `DocumentState`, `PageItem`, and `EditorHistory` (undo/redo state stacks).
- Implements `spawn_workers()` initializing the 3 background MPSC channels.
- Contains `get_cache_dir()` with automated 24-hour cache pruning.

## 3.3 `crates/morph-ui/src/views/canvas.rs`
- Renders the active document page on a simulated white sheet with an Acrobat-grade drop shadow.
- Applies 60 FPS GPU vertex rotation via `egui::Image::rotate(rad, Vec2::splat(0.5))`.
- Automatically requests high-res page previews off-thread when zooming in.

## 3.4 `crates/morph-ui/src/views/page_rail.rs`
- Virtualized vertical sidebar rendering compact Fluent cards for every document page.
- Renders page badges, rotation badges, and handles click-to-jump, rotate, duplicate, and delete actions.

## 3.5 `crates/morph-ui/src/views/document_tabs.rs`
- Renders horizontal document tabs with intelligent middle-truncation (e.g., `quarterly_re...2026.pdf`), close buttons, and active indicators.

## 3.6 `crates/morph-ui/src/views/toolbar.rs`
- Consolidated Fluent toolbar housing sidebar toggle, page rotation buttons ($90^\circ$ CW/CCW, $180^\circ$ flip), duplication, deletion, zoom controls, and insertion mode dropdowns (`⏮ At Start`, `⏺ Middle`, `⏭ At End`).

## 3.7 `crates/morph-ui/src/main.rs`
- Orchestrates the full desktop editor, keyboard shortcut handlers, drag-and-drop overlays, and atomic export pipelines.

