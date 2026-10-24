# Morph Implementation Roadmap

This document outlines the phased development plan for **Morph**, tracking deliverables, status, and validation criteria.

---

## Phase Overview

| Phase | Description | Status |
| :--- | :--- | :--- |
| **Phase 1: Foundation** | Architecture, workspace setup, format detection, registry, safe output manager, job manager, typed errors | Completed |
| **Phase 2: Image Conversion** | Image-to-image conversions (PNG, JPEG, WebP, BMP, TIFF), Image-to-PDF, multi-image batching | Completed |
| **Phase 3: Windows Integration** | Explorer context-menu registration (HKCU verbs), CLI worker dispatcher, native notifications, safe arguments | Completed |
| **Phase 4: PDF Structural Engine** | Lossless PDF merge, split, page extraction, page reordering, page deletion, page rotation | Completed |
| **Phase 5: Popup Editor** | Compact UI for multi-file arrangement, top shelf navigation, async thumbnail generation, zero-latency GPU rotation, undo/redo, export | Completed |
| **Phase 6: Document Conversion** | Headless engine bridge (DOCX/ODT/RTF -> PDF via local Office engine, PDF text snippet extraction) | In Progress |
| **Phase 7: Advanced Formats** | Spreadsheets (CSV/XLSX), presentations (PPTX), media transcoding (FFmpeg bridge), archive extraction safety | Planned |
| **Phase 8: macOS Adapter** | macOS Finder Quick Actions & native packaging abstraction | Future |

---

## Detailed Phase Breakdown

### Phase 1: Foundation
- [x] Configure Rust MSVC toolchain, Git repository, and workspace manifest.
- [x] Create project documentation (`README.md`, `ROADMAP.md`, `ARCHITECTURE.md`, `THIRD_PARTY_NOTICES.md`).
- [x] Implement `morph-core`:
  - `detection.rs`: Magic byte validation + extension mapping.
  - `format.rs`: Format taxonomy and capabilities (Image, PDF, Document, Audio, Video, Archive).
  - `registry.rs`: Dynamic capability registry with offline status and execution requirements.
  - `output.rs`: Atomic file write, collision avoidance (`name (1).ext`), temp cleanup.
  - `job.rs`: Job lifecycle (`Queued` → `Running` → `Validating` → `Completed` / `Failed`).
  - `error.rs`: Typed, user-friendly error hierarchy (`MorphError`).
- [x] Comprehensive unit and integration tests for detection, output naming, and registry.

### Phase 2: Image Conversion Engine
- [x] Implement `engines/image.rs`:
  - Cross-format raster conversions (PNG, JPEG, WebP, BMP, TIFF).
  - Preservation of transparency where supported (PNG, WebP).
  - Configurable quality/compression parameters.
  - Single image to PDF (clean page embedding via `pdf-writer`, preserving aspect ratio and resolution).
  - Multiple images to a single combined PDF.
- [x] Comprehensive integration tests for image conversions and edge cases (corrupted images, zero-byte files, spaces in filenames, Unicode paths).

### Phase 3: Windows 11 Explorer Integration
- [x] Implement `morph-cli`:
  - Command-line commands (`convert`, `merge`, `register`, `unregister`).
  - Safe argument handling for Unicode filenames and paths with spaces.
- [x] Windows context menu registration:
  - Register under `HKCU\Software\Classes\SystemFileAssociations` (per-user, no UAC elevation required).
  - Register cascading menus and direct verbs for images, PDFs, and documents.
  - Embedded multi-resolution executable icon (`assets/logo.ico`).
  - Provide complete uninstallation cleanup (`morph unregister`).
- [x] Native Windows notifications:
  - Non-intrusive toast notifications on job completion or failure.

### Phase 4: PDF Conversion and Structural Editing
- [x] Implement `engines/pdf.rs` using `lopdf`:
  - Structural PDF merging without rasterization (preserving text, vector graphics, fonts).
  - Page deletion and extraction.
  - Page rotation (0, 90, 180, 270 degrees).
  - Page reordering.
  - Splitting PDF documents into single pages or ranges.
- [x] Integration tests verifying document structure and content preservation.

### Phase 5: Compact Popup Editor
- [x] Implement `morph-ui`:
  - Compact, native-feeling window designed for quick tasks (`eframe`/`egui`).
  - Multi-document loading (PDFs + PNG + JPG + WebP + BMP) with insertion positioning (`At Start`, `Middle`, `At End`).
  - Workspace Top Shelf with file chips, page counters, `[‹]` / `[›]` chevrons, mouse wheel horizontal scrolling, and `Files (N) ▾` full dropdown.
  - Non-blocking asynchronous thumbnail extraction (zero UI freezes or "Not Responding" states).
  - Multi-tier texture resolution with zero-latency 60 FPS GPU vertex rotation (`egui::Image::rotate`).
  - Page reordering, individual and whole-document rotation, page deletion.
  - Full Undo / Redo history state stack.
  - Dynamic zoom levels (`Fit Page`, `Fit Width`, - / +).
  - Automatic temporary file cleanup on exit and document removal.
  - Export action with atomic output writing.
- [ ] Visual Drag-and-Drop page reordering in thumbnail side rail.
- [ ] Selective page export / PDF split options in UI.

### Phase 6: Document Conversion
- [x] Text extraction from PDF (`snip_{hash}_{page}.txt` without blocking).
- [x] Headless Microsoft Word / Office engine bridge for DOCX to PDF.
- [ ] TXT and Markdown to PDF rendering.
- [ ] Headless LibreOffice bridge for ODT/RTF to PDF fallback.

### Phase 7: Advanced Formats
- [ ] Spreadsheets (CSV <-> XLSX).
- [ ] Media transcoding bridge for audio and video via local FFmpeg binary.
- [ ] Archive operations (ZIP) with path traversal defenses (Zip-Slip mitigation).

### Phase 8: macOS Adapter
- [ ] Finder Quick Actions and Service menu definitions.
- [ ] Unified native notification and launcher abstraction.
