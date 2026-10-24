# Morph

> **A Lightweight, Offline-First, Universal File Conversion and Editing Utility**

*Select a file. Right-click. Transform it. Done.*

Morph is a lightweight, privacy-focused desktop utility designed to integrate directly into the operating system's file manager (Windows File Explorer). It enables instant file conversions without opening a heavy application window, and provides a compact popup editor when multi-file arrangement or fine-grained editing (such as PDF page reordering, page deletion, rotation, or merging) is required.

---

## Key Principles

1. **Offline-First & Private**: 100% local execution. No cloud services, no remote APIs, no telemetry, no tracking.
2. **Instant Native Integration**: Directly accessible via the Windows File Explorer right-click context menu.
3. **Safe File Operations**: Never overwrites source files without explicit instruction. Employs collision detection (`file (1).ext`) and atomic file finalization.
4. **Lightweight & Fast**: Pure native Rust core; fast cold start (<50ms for CLI conversions), bounded memory usage, and zero persistent background services.
5. **Structural Integrity**: Structural PDF manipulation (merge, split, delete, rotate) preserves text, vector paths, and links without lossy rasterization.
6. **Extensible Architecture**: Decoupled core engine (`morph-core`) ready for future platform integrations (macOS Finder Quick Actions).

---

## Workspace Structure

The project is structured as a modular Cargo workspace:

```
Morph/
├── Cargo.toml                    # Workspace root manifest
├── ARCHITECTURE.md               # Technical architecture & design decisions
├── ROADMAP.md                    # Implementation roadmap & phase tracking
├── THIRD_PARTY_NOTICES.md        # Third-party dependencies & licensing
├── README.md                     # Project overview & documentation
├── crates/
│   ├── morph-core/               # Core library: format detection, registry, engines, job/output managers
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs            # Library entrypoint
│   │       ├── detection.rs      # Format sniffing via magic bytes & extensions
│   │       ├── format.rs         # Format taxonomy & capabilities
│   │       ├── registry.rs       # Capability-based conversion registry
│   │       ├── job.rs            # Job manager & lifecycle states
│   │       ├── output.rs         # Safe naming, temp writing & atomic finalize
│   │       ├── error.rs          # Typed domain errors
│   │       └── engines/          # Conversion engine implementations
│   │           ├── mod.rs
│   │           ├── image.rs      # Image format conversions & image-to-PDF
│   │           └── pdf.rs        # Structural PDF manipulation (merge/split/rotate/page ops)
│   ├── morph-cli/                # Command-line interface & context menu worker
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs           # CLI dispatcher & subcommands
│   │       └── windows/          # Windows shell integration (registry verbs, toast notifications)
│   │           ├── mod.rs
│   │           ├── context_menu.rs # Context menu registration/unregistration
│   │           └── notify.rs     # Native Windows notifications
│   └── morph-ui/                 # Compact popup editor (for PDF reordering, image combination, etc.)
│       ├── Cargo.toml
│       └── src/
│           └── main.rs
└── tests/                        # Workspace-wide integration tests
    ├── image_conversions.rs
    ├── pdf_operations.rs
    └── safe_output.rs
```

---

## Development Setup & Prerequisites

### Prerequisites (Windows)
- **Rust Stable** (1.80+ or latest stable; tested on 1.99.1)
- **Visual Studio 2022 C++ Build Tools** (MSVC toolchain)
- **Windows 10/11 SDK** (10.0.19041.0 or higher)
- **Git**

### Building from Source

```powershell
# Clone or navigate to the repository
cd C:\Morph

# Build the entire workspace in debug mode
cargo build

# Build in release mode (optimized binary)
cargo build --release

# Run unit and integration tests
cargo test
```

---

## Usage

### 1. Instant Conversions via CLI
```powershell
# Convert single image to PDF
morph-cli convert document.png --to pdf

# Convert image format
morph-cli convert photo.png --to jpg --quality 90

# Merge multiple files into a single PDF
morph-cli merge page1.pdf page2.jpg page3.png --output combined.pdf

# Rotate PDF pages
morph-cli pdf rotate input.pdf --pages 1,3 --degrees 90
```

### 2. Context Menu Registration
```powershell
# Register Morph into Windows File Explorer context menu (HKCU, no admin needed)
morph-cli register

# Unregister / clean up context menu entries
morph-cli unregister
```

---

## Implementation Status

- [x] Environment inspection & toolchain configuration (Rust 1.99.1 + MSVC + Win11 SDK)
- [x] Workspace architecture & foundational documentation
- [x] `morph-core` format detection & capability registry
- [x] Safe output manager with atomic writes & collision resolution
- [x] Image conversion engine (PNG, JPEG, WebP, BMP, TIFF -> PDF and cross-image conversions)
- [x] Structural PDF engine (merge, split, page deletion, page rotation, page extraction)
- [x] Windows context-menu registration (cascading actions & direct verbs)
- [x] Native Windows notifications
- [x] Compact popup editor for interactive arrangement and reordering
