# Third-Party Notices and Licenses

This file lists the third-party open-source components, their licenses, and the technical rationale for their inclusion in **Morph**.

---

## Direct Dependencies

### 1. Core Conversion & Document Manipulation
- **`image`**
  - *License*: MIT / Apache-2.0
  - *Purpose*: High-performance, pure-Rust decoding and encoding for raster image formats (PNG, JPEG, WebP, BMP, TIFF, GIF, ICO). Eliminates native C library vulnerabilities (e.g. libpng/libjpeg buffer overflows).
- **`pdf-writer`**
  - *License*: MIT / Apache-2.0
  - *Purpose*: Low-level, high-efficiency, zero-dependency PDF document writer developed by the Typst project. Used for embedding images directly into clean, compliant PDF documents without raster loss.
- **`lopdf`**
  - *License*: MIT
  - *Purpose*: Pure-Rust structural PDF parser and object-tree manipulator. Used for lossless PDF merging, splitting, page reordering, page deletion, and rotation without re-encoding or rasterizing vector and text contents.

### 2. Utilities & Platform
- **`thiserror`**
  - *License*: MIT / Apache-2.0
  - *Purpose*: Zero-cost typed error definitions and error conversions for domain models in `morph-core`.
- **`serde` & `serde_json`**
  - *License*: MIT / Apache-2.0
  - *Purpose*: Structured configuration and serialization for registry entries and job progress reporting.
- **`tempfile`**
  - *License*: MIT / Apache-2.0
  - *Purpose*: Secure, cross-platform temporary file creation with automatic cleanup upon failure.
- **`windows` / `windows-sys` / `winreg`**
  - *License*: MIT / Apache-2.0
  - *Purpose*: Official Microsoft Windows API bindings for registry management (`HKCU\Software\Classes`), process execution, and native notifications.
- **`egui` & `eframe`**
  - *License*: MIT / Apache-2.0
  - *Purpose*: Immediate-mode, pure-Rust, ultra-lightweight desktop GUI framework for the compact popup editor. Launches in <25ms, requires no webview or Electron runtime, and runs completely offline.
- **`rfd` (Rust File Dialog)**
  - *License*: MIT / Apache-2.0
  - *Purpose*: Native Windows asynchronous file picker dialog used in Morph Editor for selecting PDFs and images to insert.
- **`winres` (Build Dependency)**
  - *License*: MIT
  - *Purpose*: Build-time Windows resource compiler that compiles and embeds the multi-resolution application icon (`logo.ico`) and version metadata into `morph.exe` and `morph-ui.exe`.

---

## Dependency Policy

All dependencies included in Morph must adhere to:
1. Permissive open-source licenses (MIT, Apache-2.0, BSD-2/3-Clause).
2. Pure Rust implementations wherever feasible to prevent memory safety issues and minimize external C/C++ toolchain dependencies.
3. Zero network activity during execution.
4. Active maintenance and security track record.
