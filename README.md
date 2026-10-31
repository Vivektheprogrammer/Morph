<p align="center">
  <img src="assets/logo_fluent.png" width="96" height="96" alt="Morph Logo" />
</p>

<h1 align="center">Morph</h1>

<p align="center">
  <strong>A Lightweight, Offline-First, Universal File Conversion & Document Editing Utility</strong>
</p>

<p align="center">
  <em>Select a file. Right-click. Transform it. Done.</em>
</p>

---

## What Morph Can Do

- **Instant Context Menu Conversions**: Convert raster images (`PNG`, `JPEG`, `WebP`, `BMP`, `TIFF`), documents (`DOCX` to `PDF`), and multi-image batches directly from the Windows File Explorer right-click menu.
- **Interactive Multi-Document Editor (`morph-ui.exe`)**: Open, combine, and arrange multiple PDFs and image files into a single unified workspace with smart insertion controls (`⏮ At Start`, `⏺ Middle`, `⏭ At End`).
- **Lossless Structural PDF Engine**: Merge, split, reorder, rotate ($90^\circ, 180^\circ, 270^\circ$), and delete pages while preserving vector graphics, fonts, metadata, and selectable text without lossy rasterization.
- **Ultra-Fast & Asynchronous**: Dedicated background worker threads render thumbnails asynchronously in parallel without blocking the UI event loop.
- **Zero-Latency GPU Rotation**: 60 FPS GPU-accelerated vertex rotation with a 4-tier texture resolution pipeline ensures thumbnails and document previews never flicker or go blank when rotated.
- **Safe & Collision-Proof**: Zero accidental overwrites. Employs automatic collision resolution (`file (1).ext`), atomic temporary staging files, and automated cache self-cleaning (`%TEMP%\morph_thumbs`).
- **Agentic AI & Scripting Ready**: The standalone CLI (`morph.exe`) allows autonomous AI coding agents (Claude, ChatGPT, Antigravity, Copilot, Cursor, etc.) and PowerShell/batch scripts to perform conversions and PDF structural edits programmatically and completely offline.

---

## Key Principles

1. **Offline-First & 100% Private**: Zero cloud dependencies, zero remote API calls, zero telemetry, and zero tracking. Everything stays strictly on your local machine.
2. **Native Windows Shell Integration**: Static registry verbs registered under `HKCU\Software\Classes\SystemFileAssociations` (requires zero administrator privileges, runs zero background services, and incurs zero right-click delay).
3. **Safe File Operations**: Never overwrites source files without explicit confirmation; uses atomic staging files and collision avoidance.
4. **Lightweight & Fast**: Pure Rust native binaries with cold starts in <50ms and minimal memory footprint (<40MB RAM).
5. **Structural Integrity**: Directly manipulates PDF object trees (`lopdf` / `pdf-writer`) rather than converting pages into lossy raster bitmaps.
6. **Extensible Architecture**: Cleanly decoupled core engine (`morph-core`) ready for future platforms (macOS Finder Quick Actions).

---

## Agentic AI & Automation Ready

Morph is designed from the ground up for both human users and **Autonomous AI Agents**:

Once installed on your system, any coding assistant or autonomous agent (such as **Antigravity**, **Claude**, **ChatGPT**, **Cursor**, **Copilot**, or **Windsurf**) can immediately execute conversions, merges, page extractions, and PDF transformations using the simple, predictable `morph` CLI.

```powershell
# Autonomous agents and scripts can invoke Morph directly:
morph convert invoice.png --to pdf
morph merge section1.pdf section2.pdf --output full_report.pdf
morph pdf rotate document.pdf --pages 1,3 --degrees 90
```

- **Predictable Exit Codes & Stdout**: Clean CLI output designed for machine parsing.
- **Zero Network Requirement**: Fully functional in air-gapped environments.
- **Deterministic Atomic Output**: Agents never risk corrupting source files.

---

## Installation & Downloads

Download the latest version for Windows from **[GitHub Releases](https://github.com/Vivektheprogrammer/Morph/releases/latest)**:

* **Windows Installer (Recommended)**: [`MorphSetup-v0.1.0-windows-x64.exe`](https://github.com/Vivektheprogrammer/Morph/releases/latest/download/MorphSetup-v0.1.0-windows-x64.exe)
  * Sets up File Explorer context menus, Desktop & Start Menu shortcuts, and adds `morph` to your user `PATH`.
* **Portable Archive**: [`Morph-v0.1.0-windows-x64.zip`](https://github.com/Vivektheprogrammer/Morph/releases/latest/download/Morph-v0.1.0-windows-x64.zip)
  * Portable zero-install zip with `install.cmd` / `uninstall.cmd` helper scripts.

> [!NOTE]
> **Windows SmartScreen**: Because Morph is an independent open-source project without an expensive corporate certificate, Windows may initially show a blue *"Windows protected your PC"* prompt. Click **More info** $\rightarrow$ **Run anyway**. Morph is 100% offline, private, and open-source.

---

## Usage

### 1. Instant Conversions via CLI (`morph.exe`)
```powershell
# Convert single image to PDF
morph convert document.png --to pdf

# Convert image format with compression quality
morph convert photo.png --to jpg --quality 90

# Convert Word document to PDF offline
morph convert report.docx --to pdf

# Merge multiple files (PDFs + images) into a single PDF
morph merge page1.pdf page2.jpg page3.png --output combined.pdf

# Rotate specific PDF pages structurally
morph pdf rotate input.pdf --pages 1,3 --degrees 90
```

### 2. Context Menu Registration
```powershell
# Register Morph into Windows File Explorer context menu (per-user, no admin needed)
morph register

# Unregister / clean up all context menu entries cleanly
morph unregister
```

### 3. Interactive Desktop Editor (`morph-ui.exe`)
```powershell
# Launch empty editor
morph-ui.exe

# Open directly with one or more documents
morph-ui.exe document.pdf
```
- **Workspace Top Shelf**:
  - Live category pills showing document name, format badge, and page count.
  - **Click to Jump**: Clicking any file chip selects that document's first page in the main viewer.
  - **Quick Removal**: Click `✕` to unload all pages of a specific document.
  - **`Files (N) ▾` Dropdown**: Vertical popup listing all loaded documents with page summaries for fast navigation.
  - **Horizontal Navigation**: Scroll file chips horizontally using the mouse wheel, click-and-drag, or `[‹]` / `[›]` chevron arrow buttons.
- **Smart Insertion Points**: Choose where newly added documents are placed:
  - `⏮ At Start`: Inserts files before Page 1.
  - `⏺ Middle`: Inserts files immediately after the currently selected page.
  - `⏭ At End`: Appends files to the end of the document.
- **Zero-Latency GPU Rotation**: Rotate individual pages ($90^\circ, 180^\circ, 270^\circ$) or entire documents with 60 FPS GPU vertex rotation.
- **Full History**: Complete Undo / Redo history state stack.
- **Atomic PDF Export**: Lossless structural merge with collision protection.

---

## Upcoming Planned Features

| Feature | Description | Target |
| :--- | :--- | :--- |
| **PDF Stream & Object Deduplication** | Lossless Flate/Deflate stream recompression, unreferenced object pruning, metadata stripping, and cross-reference stream packing (PDF 1.5+). | Phase 6 |
| **Raster Downsampling & Modern Re-encoding** | Inspect embedded PDF images and downsample to 150 DPI (Email) or 72 DPI (Web) with modern JPEG/WebP encoding. | Phase 6 |
| **Target File Size Solver (`--target-size`)** | Automated bisection search algorithm calibrating compression quality and DPI to fit strictly under user limits (e.g. `2MB` for portal uploads). | Phase 6 |
| **GUI Export Size Estimator & Preset Drawer** | Real-time pre-export file size prediction, quick preset buttons (*Lossless*, *Email 150 DPI*, *Smallest Size*), and sliders in the Morph Editor. | Phase 6 |
| **Context Menu "Compress / Shrink File"** | Direct right-click Explorer action to compress single or multiple PDFs/images with native toast feedback showing space saved. | Phase 6 |
| **Visual Drag-and-Drop Reordering** | Drag page thumbnail cards directly within the side rail to drop them into a new sequence visually. | Phase 5.1 |
| **External File Drag-and-Drop** | Drag PDF and image files directly from Windows File Explorer and drop them into the Morph Editor window. | Phase 5.1 |
| **Selective Page Export / Split** | Choose specific page ranges to export (e.g. `1-5, 8, 12-15`) or split into separate PDFs per source file. | Phase 5.2 |
| **Spreadsheets & Presentations Bridge** | Convert Excel (`.xlsx`, `.csv`) and PowerPoint (`.pptx`) documents to PDF offline. | Phase 8 |
| **Media Transcoding Bridge** | Local video and audio format conversion (MP4, MKV, WebM, MP3, WAV, AAC) via an embedded FFmpeg bridge. | Phase 8 |
| **Theme Customization** | Light Mode / Dark Mode toggle and automatic matching with the Windows System Accent Color. | Phase 8 |
| **macOS Adapter** | Finder Quick Actions, native notifications, and macOS menu bar integration. | Phase 9 |

---

## Workspace Structure & Development Setup

### Workspace Layout
```
Morph/
├── Cargo.toml                    # Workspace root manifest
├── ARCHITECTURE.md               # Technical architecture & design decisions
├── ROADMAP.md                    # Implementation roadmap & phase tracking
├── README.md                     # Project overview & documentation
├── assets/                       # Branding icons (logo.ico, logo_fluent.png)
├── crates/
│   ├── morph-core/               # Core library: format detection, registry, engines, job/output managers
│   ├── morph-cli/                # Command-line interface & context menu worker (morph.exe)
│   └── morph-ui/                 # Compact popup editor (morph-ui.exe)
└── tests/                        # Workspace-wide integration tests
```

### Prerequisites
- **Rust Stable** (1.80+; tested on 1.99.1)
- **Visual Studio 2022 C++ Build Tools** (MSVC toolchain)
- **Windows 10/11 SDK** (10.0.19041.0 or higher)

### Building from Source
```powershell
# Clone or navigate to the repository
cd C:\Morph

# Build the entire workspace in release mode (optimized binaries)
cargo build --release

# Run unit and integration tests
cargo test
```

## License

Morph is open-source software licensed under the **[MIT License](LICENSE)**.

Copyright (c) 2026 The Morph Project Contributors. All rights reserved.


## License

Morph is open-source software licensed under the **[MIT License](LICENSE)**.

Copyright (c) 2026 The Morph Project Contributors. All rights reserved.

