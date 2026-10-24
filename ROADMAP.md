# Morph Implementation Roadmap

This document outlines the phased development plan for **Morph**, tracking deliverables, status, and validation criteria.

---

## Phase Overview

| Phase | Description | Status |
| :--- | :--- | :--- |
| **Phase 1: Foundation** | Architecture, workspace setup, format detection, registry, safe output manager, job manager, typed errors | In Progress |
| **Phase 2: Image Conversion** | Image-to-image conversions (PNG, JPEG, WebP, BMP, TIFF), Image-to-PDF, multi-image batching | In Progress |
| **Phase 3: Windows Integration** | Explorer context-menu registration (HKCU verbs), CLI worker dispatcher, native notifications, safe arguments | Planned |
| **Phase 4: PDF Structural Engine** | Lossless PDF merge, split, page extraction, page reordering, page deletion, page rotation | Planned |
| **Phase 5: Popup Editor** | Compact UI for multi-file arrangement, PDF page reordering, thumbnail previews, undo/redo, export | Planned |
| **Phase 6: Document Conversion** | Headless engine bridge (DOCX/ODT/RTF/TXT -> PDF, PDF -> TXT) | Planned |
| **Phase 7: Advanced Formats** | Spreadsheets, presentations, media transcoding (FFmpeg bridge), archive extraction safety | Planned |
| **Phase 8: macOS Adapter** | macOS Finder Quick Actions & native packaging abstraction | Future |

---

## Detailed Phase Breakdown

### Phase 1: Foundation
- [x] Configure Rust MSVC toolchain, Git repository, and workspace manifest.
- [x] Create project documentation (`README.md`, `ROADMAP.md`, `ARCHITECTURE.md`, `THIRD_PARTY_NOTICES.md`).
- [ ] Implement `morph-core`:
  - `detection.rs`: Magic byte validation + extension mapping.
  - `format.rs`: Format taxonomy and capabilities (Image, PDF, Document, Audio, Video, Archive).
  - `registry.rs`: Dynamic capability registry with offline status and execution requirements.
  - `output.rs`: Atomic file write, collision avoidance (`name (1).ext`), temp cleanup.
  - `job.rs`: Job lifecycle (`Queued` → `Running` → `Validating` → `Completed` / `Failed`).
  - `error.rs`: Typed, user-friendly error hierarchy (`MorphError`).
- [ ] Comprehensive unit tests for detection, output naming, and registry.

### Phase 2: Image Conversion Engine
- [ ] Implement `engines/image.rs`:
  - Cross-format raster conversions (PNG, JPEG, WebP, BMP, TIFF).
  - Preservation of transparency where supported (PNG, WebP).
  - Configurable quality/compression parameters.
  - Single image to PDF (clean page embedding via `pdf-writer`, preserving aspect ratio and resolution).
  - Multiple images to a single combined PDF.
- [ ] Comprehensive integration tests for image conversions and edge cases (corrupted images, zero-byte files, spaces in filenames, Unicode paths).

### Phase 3: Windows 11 Explorer Integration
- [ ] Implement `morph-cli`:
  - Command-line commands (`convert`, `merge`, `register`, `unregister`).
  - Safe argument handling for Unicode filenames and paths with spaces.
- [ ] Windows context menu registration:
  - Register under `HKCU\Software\Classes\SystemFileAssociations` (per-user, no UAC elevation required).
  - Register cascading menus or direct verbs for images and PDFs.
  - Provide complete uninstallation cleanup.
- [ ] Native Windows notifications:
  - Non-intrusive toast notifications on job completion or failure.

### Phase 4: PDF Conversion and Structural Editing
- [ ] Implement `engines/pdf.rs` using `lopdf`:
  - Structural PDF merging without rasterization (preserving text, vector graphics, fonts).
  - Page deletion and extraction.
  - Page rotation (0, 90, 180, 270 degrees).
  - Page reordering.
  - Splitting PDF documents into single pages or ranges.
- [ ] Integration tests verifying document structure and content preservation.

### Phase 5: Compact Popup Editor
- [ ] Implement `morph-ui`:
  - Compact, native-feeling window designed for quick tasks.
  - Input file list and thumbnail preview grid.
  - Drag-and-drop page/file reordering.
  - Delete and rotate buttons with immediate preview updates.
  - Undo and redo support for manipulation actions.
  - Export action with atomic output writing.

### Phase 6: Document Conversion
- [ ] Text extraction from PDF (`lopdf` / text stream parsing).
- [ ] TXT and Markdown to PDF rendering.
- [ ] Integration with local headless LibreOffice (if installed on user machine) for DOCX/ODT/RTF to PDF.

### Phase 7: Advanced Formats
- [ ] Spreadsheets (CSV <-> XLSX).
- [ ] Media transcoding bridge for audio and video via local FFmpeg binary.
- [ ] Archive operations (ZIP) with path traversal defenses (Zip-Slip mitigation).

### Phase 8: macOS Adapter
- [ ] Finder Quick Actions and Service menu definitions.
- [ ] Unified native notification and launcher abstraction.
