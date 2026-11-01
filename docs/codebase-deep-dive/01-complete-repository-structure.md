# 01: Complete Repository Structure & File Inventory

## 1. Physical Directory Tree

The following diagram illustrates the complete, physical directory hierarchy of the **Morph** repository:

```
C:\Morph\
├── .gitignore                                # Git version control ignore rules
├── ARCHITECTURE.md                           # Core architectural specification and engine designs
├── Cargo.lock                                # Deterministic Cargo dependency lockfile
├── Cargo.toml                                # Cargo workspace virtual manifest
├── LICENSE                                   # MIT open-source license
├── README.md                                 # Project documentation, usage guides, and roadmap highlights
├── ROADMAP.md                                # Phased implementation roadmap and feature milestone tracking
│
├── assets\                                   # Branding assets and embedded Windows icons
│   ├── logo.ico                              # Multi-resolution Windows executable icon (16px to 256px)
│   ├── logo.png                              # Full-resolution master logo
│   ├── logo_fluent.png                       # Windows Fluent Design rounded square logo (96x96 / 512x512)
│   └── logo_square.png                       # High-resolution square logo for branding and installers
│
├── crates\                                   # Cargo workspace member crates
│   │
│   ├── morph-core\                           # Independent, headless conversion and document engine
│   │   ├── Cargo.toml                        # Package manifest and engine dependencies
│   │   ├── examples\                         # Directory reserved for core engine code examples
│   │   ├── src\
│   │   │   ├── detection.rs                  # Magic byte header inspection and MIME identification
│   │   │   ├── error.rs                      # Typed error definitions (MorphError) and Result alias
│   │   │   ├── format.rs                     # Format taxonomy, MIME types, extensions, and categories
│   │   │   ├── job.rs                        # Asynchronous conversion job state machine and lifecycle
│   │   │   ├── lib.rs                        # Public library API exports and crate root
│   │   │   ├── output.rs                     # Atomic staging file writer and collision name resolver
│   │   │   ├── registry.rs                   # Capability-based conversion registry and route discovery
│   │   │   └── engines\                      # Low-level conversion engines
│   │   │       ├── document.rs               # Headless Word COM / LibreOffice bridge for DOCX/DOC/ODT
│   │   │       ├── image.rs                  # Cross-format raster engine and single/multi-image to PDF
│   │   │       ├── mod.rs                    # Engines submodule declaration
│   │   │       └── pdf.rs                    # Lossless structural PDF engine (lopdf AST manipulation)
│   │   └── tests\
│   │       └── engine_integration.rs         # End-to-end integration tests for core conversion engines
│   │
│   ├── morph-cli\                            # Standalone command-line interface & Windows context menu worker
│   │   ├── build.rs                          # Build script embedding logo.ico into morph.exe PE header
│   │   ├── Cargo.toml                        # Package manifest and CLI dependencies
│   │   └── src\
│   │       ├── main.rs                       # Entry point, console attachment, and CLI argument router
│   │       └── windows\                      # Windows operating system integration layer
│   │           ├── context_menu.rs           # HKCU SystemFileAssociations registry verb registration
│   │           ├── mod.rs                    # Windows subsystem module declaration
│   │           └── notify.rs                 # WinRT toast notification dispatcher via PowerShell bridge
│   │
│   └── morph-ui\                             # Native, compact popup document editor (egui/eframe)
│       ├── build.rs                          # Build script embedding logo.ico into morph-ui.exe PE header
│       ├── Cargo.toml                        # Package manifest and GUI dependencies
│       └── src\
│           ├── main.rs                       # UI entry point, event loop, MPSC coordinator, and keyboard shortcuts
│           ├── components\                   # Reusable UI widgets
│           │   ├── button.rs                 # Animated Fluent push-button with vector icon support
│           │   ├── dropzone.rs               # Pulsing full-window drag-and-drop file ingestion overlay
│           │   ├── mod.rs                    # Components submodule declaration
│           │   └── toast.rs                  # In-window floating notification stack manager
│           ├── icons\
│           │   └── mod.rs                    # Procedural vector icons drawn via egui painter primitives
│           ├── motion\
│           │   └── mod.rs                    # Cubic bezier curves and mass-spring-damper physics engine
│           ├── state\
│           │   └── mod.rs                    # Multi-document state, undo/redo history, MPSC worker channels
│           ├── theme\                        # Windows 11 Fluent dark theme design tokens
│           │   ├── colors.rs                 # Centralized 40+ color palette, surfaces, and badges
│           │   ├── elevation.rs              # Realistic drop shadow definitions and container frames
│           │   └── mod.rs                    # Theme application and egui Visuals styling overrides
│           └── views\                        # Subsystem view containers
│               ├── bottom_bar.rs             # Status bar, page indicator, and zoom percentage pill
│               ├── canvas.rs                 # Central document viewer, drop shadow, and GPU vertex rotation
│               ├── document_tabs.rs          # Multi-document tab strip with middle-truncated filenames
│               ├── empty_state.rs            # Initial launch hero card with format badges and file picker
│               ├── header.rs                 # Top window header with branding and export trigger
│               ├── mod.rs                    # Views submodule declaration
│               ├── page_rail.rs              # Collapsible, virtualized vertical page thumbnail carousel
│               ├── toolbar.rs                # Editing toolbar (rotate, flip, duplicate, delete, zoom)
│               └── top_bar.rs                # Multi-file sequence top shelf and insertion controls
│
├── installer\                                # Windows installer definitions and packaging scripts
│   ├── build_msi.ps1                         # PowerShell orchestration script building WiX MSI and Bootstrapper
│   ├── bundle.wxs                            # WiX v4 Bootstrapper bundle specification (setup.exe)
│   ├── morph.iss                             # Inno Setup single-file standalone installer script
│   ├── morph.wxs                             # WiX v4 MSI installer package specification
│   └── .wix\                                 # Cached WiX v4 toolset compiler extensions (UI and Bal)
│
├── dist\                                     # Compiled distribution binaries and installer releases
│   ├── Morph-v0.1.0-windows-x64.msi          # Production WiX Windows Installer package (~5.0 MB)
│   ├── Morph-v0.1.0-windows-x64.zip          # Portable zero-install zip archive (~5.5 MB)
│   └── MorphSetup-v0.1.0-windows-x64.exe     # Standalone Setup executable with embedded icon (~6.0 MB)
│
└── test_samples\                             # Automated test fixtures and validation sample files
    ├── invoice.pdf                           # Single-page sample PDF
    ├── invoice.png                           # Sample raster PNG invoice
    ├── invoice.jpg                           # Sample raster JPEG invoice
    ├── invoice.webp                          # Sample raster WebP invoice
    ├── multi_page_sample.pdf                 # Multi-page test document
    ├── report.docx                           # Word document test fixture for headless COM engine
    └── report.pdf                            # Multi-page reference PDF
```

---

## 2. Comprehensive File Inventory & Categorization

The table below catalogs every file in the repository, explaining its size, language, architectural subsystem, and exact purpose:

| File Path | Size (Bytes) | Language / Format | Architectural Layer | Primary Responsibility |
| :--- | :--- | :--- | :--- | :--- |
| `Cargo.toml` | 113 | TOML | Workspace Root | Declares workspace members (`crates/*`) and resolver version. |
| `Cargo.lock` | 97,811 | TOML | Dependency Graph | Exact cryptographic dependency version lockfile. |
| `.gitignore` | 136 | Text | Version Control | Specifies build directories (`/target`, `/dist`) excluded from git. |
| `LICENSE` | 1,088 | Text | Legal | Official MIT Open Source License terms. |
| `README.md` | 10,407 | Markdown | Documentation | User manual, feature summary, and installation instructions. |
| `ARCHITECTURE.md` | 8,707 | Markdown | Documentation | High-level system design, principles, and compression roadmap. |
| `ROADMAP.md` | 8,765 | Markdown | Documentation | Phased delivery schedule tracking completed and planned phases. |
| `assets/logo.ico` | 114,463 | Binary ICO | Branding | Multi-resolution icon embedded into PE executable headers. |
| `assets/logo.png` | 831,515 | Binary PNG | Branding | Master high-resolution brand artwork. |
| `assets/logo_fluent.png` | 273,455 | Binary PNG | Branding | Windows Fluent rounded square logo used in UI and installer. |
| `assets/logo_square.png` | 273,455 | Binary PNG | Branding | High-resolution square icon asset. |
| `crates/morph-core/Cargo.toml` | 518 | TOML | Core Crate | Declares image, PDF, serialization, and Windows dependencies. |
| `crates/morph-core/src/lib.rs` | 599 | Rust | Core Crate | Crate entry point, submodule declarations, and public re-exports. |
| `crates/morph-core/src/error.rs` | 1,119 | Rust | Core Crate | Defines `MorphError` enum via `thiserror` and `Result<T>`. |
| `crates/morph-core/src/format.rs` | 7,846 | Rust | Core Crate | `Format` and `FormatCategory` enums, MIME types, extensions. |
| `crates/morph-core/src/detection.rs` | 3,858 | Rust | Core Crate | Magic bytes file sniffer defending against spoofed extensions. |
| `crates/morph-core/src/registry.rs` | 6,672 | Rust | Core Crate | Catalog of supported conversion operations and capability discovery. |
| `crates/morph-core/src/output.rs` | 6,174 | Rust | Core Crate | Atomic staging writer and collision renaming (`file (1).ext`). |
| `crates/morph-core/src/job.rs` | 2,088 | Rust | Core Crate | Asynchronous conversion lifecycle state machine (`Job`, `JobStatus`). |
| `crates/morph-core/src/engines/mod.rs` | 46 | Rust | Core Engine | Module declarations for image, PDF, and document submodules. |
| `crates/morph-core/src/engines/image.rs` | 11,702 | Rust | Core Engine | Image cross-conversion, alpha-mask splitting, JPEG embedding. |
| `crates/morph-core/src/engines/pdf.rs` | 14,416 | Rust | Core Engine | Lossless PDF structural merging, rotation, deletion, assembly. |
| `crates/morph-core/src/engines/document.rs` | 7,607 | Rust | Core Engine | Headless Microsoft Word COM and LibreOffice PDF conversion bridge. |
| `crates/morph-core/tests/engine_integration.rs` | 6,251 | Rust | Test Harness | Integration test suite exercising image and PDF operations. |
| `crates/morph-cli/Cargo.toml` | 435 | TOML | CLI Crate | Package manifest configuring `morph.exe` binary and winreg. |
| `crates/morph-cli/build.rs` | 509 | Rust | CLI Build | Windows resource compiler script embedding `logo.ico`. |
| `crates/morph-cli/src/main.rs` | 15,513 | Rust | CLI Binary | Entry point, `AttachConsole` logic, CLI argument dispatcher. |
| `crates/morph-cli/src/windows/mod.rs` | 107 | Rust | CLI Windows | Module declaration re-exporting context menu and notifier. |
| `crates/morph-cli/src/windows/context_menu.rs` | 8,183 | Rust | CLI Windows | Win32 registry management for `HKCU SystemFileAssociations`. |
| `crates/morph-cli/src/windows/notify.rs` | 1,726 | Rust | CLI Windows | WinRT Toast Notification dispatcher via non-blocking PowerShell. |
| `crates/morph-ui/Cargo.toml` | 431 | TOML | UI Crate | Package manifest configuring `morph-ui.exe` and `eframe`/`egui`. |
| `crates/morph-ui/build.rs` | 509 | Rust | UI Build | Windows resource compiler script embedding `logo.ico`. |
| `crates/morph-ui/src/main.rs` | 44,642 | Rust | UI Binary | Main application struct, event loop, shortcuts, MPSC coordinator. |
| `crates/morph-ui/src/components/mod.rs` | 50 | Rust | UI Components | Submodule declaration for buttons, dropzone, and toasts. |
| `crates/morph-ui/src/components/button.rs` | 10,693 | Rust | UI Components | Smoothly animated Fluent buttons with vector icon rendering. |
| `crates/morph-ui/src/components/dropzone.rs` | 2,293 | Rust | UI Components | Full-window drag-and-drop pulsing visual overlay. |
| `crates/morph-ui/src/components/toast.rs` | 5,158 | Rust | UI Components | Non-intrusive floating toast notifications stack. |
| `crates/morph-ui/src/icons/mod.rs` | 17,236 | Rust | UI Graphics | Procedural vector icon definitions drawn via 2D painter math. |
| `crates/morph-ui/src/motion/mod.rs` | 4,903 | Rust | UI Animation | Spring-damper oscillator physics and cubic bezier easings. |
| `crates/morph-ui/src/state/mod.rs` | 17,979 | Rust | UI State | Multi-document state, undo history, MPSC worker thread loops. |
| `crates/morph-ui/src/theme/mod.rs` | 1,750 | Rust | UI Theme | Theme applicator applying Windows 11 Fluent dark visuals. |
| `crates/morph-ui/src/theme/colors.rs` | 3,117 | Rust | UI Theme | Centralized color palette tokens, backgrounds, and text shades. |
| `crates/morph-ui/src/theme/elevation.rs` | 2,571 | Rust | UI Theme | Elevation drop-shadows and styled container frames. |
| `crates/morph-ui/src/views/mod.rs` | 149 | Rust | UI Views | Submodule declaration exporting all view panels. |
| `crates/morph-ui/src/views/bottom_bar.rs` | 3,955 | Rust | UI Views | Status bar displaying page counters, zoom level, and spinners. |
| `crates/morph-ui/src/views/canvas.rs` | 8,232 | Rust | UI Views | Central interactive canvas with 60 FPS GPU vertex rotation. |
| `crates/morph-ui/src/views/document_tabs.rs` | 12,202 | Rust | UI Views | Multi-document tab bar with middle-truncation and close buttons. |
| `crates/morph-ui/src/views/empty_state.rs` | 3,587 | Rust | UI Views | Centered hero card for empty workspace with browse button. |
| `crates/morph-ui/src/views/header.rs` | 5,230 | Rust | UI Views | Consolidated application header with logo, title, and export. |
| `crates/morph-ui/src/views/page_rail.rs` | 13,717 | Rust | UI Views | Collapsible, virtualized vertical thumbnail sidebar. |
| `crates/morph-ui/src/views/toolbar.rs` | 13,246 | Rust | UI Views | Consolidated toolbar for page rotation, zoom, flip, duplicate. |
| `crates/morph-ui/src/views/top_bar.rs` | 24,672 | Rust | UI Views | Workspace top shelf with file chips and insertion controls. |
| `installer/build_msi.ps1` | 3,689 | PowerShell | Installer | Release compilation and WiX v4 packaging automation script. |
| `installer/morph.wxs` | 5,279 | WiX XML | Installer | WiX v4 definition for per-user MSI with context menu hooks. |
| `installer/bundle.wxs` | 729 | WiX XML | Installer | WiX v4 bootstrapper bundle definition compiling `Setup.exe`. |
| `installer/morph.iss` | 3,902 | Inno Setup | Installer | Alternative Inno Setup script for single-file EXE installer. |

---

## 3. Structural Rationale & Workspace Architecture

### The Virtual Workspace Pattern
The root `Cargo.toml` contains no library or binary targets directly; instead, it defines a virtual Cargo workspace:
```toml
[workspace]
members = [
    "crates/morph-core",
    "crates/morph-cli",
    "crates/morph-ui",
]
resolver = "2"
```

This architecture provides critical advantages:
1. **Strict Decoupling**: `morph-core` contains zero UI dependencies (`egui`, `eframe`) and zero CLI-specific console parsing logic. It can be compiled as an independent shared library or consumed by alternative frontends (e.g., a web service, a macOS menu-bar app, or a CLI).
2. **Binary Specialization**:
   - `morph.exe` (`morph-cli`) is built as a console/headless worker optimized for immediate termination and minimal memory overhead.
   - `morph-ui.exe` (`morph-ui`) is built with graphical window subsystems, linking OpenGL bindings and hardware acceleration libraries without bloating the headless CLI.
3. **Shared Dependency Cache**: Cargo caches build artifacts across crates in a unified `target/` directory, preventing redundant recompilation of shared dependencies like `lopdf` or `image`.
4. **Compile-Time Resource Embedding**: Both binary crates utilize `winres` inside `build.rs` to embed Windows PE resources (`logo.ico`) directly into the `.exe` headers at link time, eliminating external runtime file dependencies.

