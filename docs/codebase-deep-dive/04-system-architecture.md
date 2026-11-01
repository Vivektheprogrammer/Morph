# 04: System Architecture & Design Patterns

This chapter details the overarching system architecture of **Morph**. It explains the structural patterns, layer responsibilities, data flow, component boundaries, and internal communication mechanisms that govern the codebase.

---

## 1. Architectural Pattern: Decoupled Multi-Tier Architecture

Morph is structured around a **Decoupled Multi-Tier Architecture** that enforces a strict unidirectional dependency flow:

```mermaid
flowchart TD
    subgraph PresentationTier ["Tier 1: Presentation & Invocation Layer"]
        CLI["morph.exe (morph-cli)<br/>- Console Attachment<br/>- CLI Argument Parsing<br/>- Shell Dispatch"]
        UI["morph-ui.exe (morph-ui)<br/>- egui Immediate-Mode Loop<br/>- Spring Animation Physics<br/>- MPSC Worker Threads"]
    end

    subgraph BusinessTier ["Tier 2: Core Domain & Engine Layer (morph-core)"]
        Registry["Conversion Registry<br/>(Capability Table)"]
        Detection["Format Sniffer<br/>(Magic Bytes)"]
        JobStateMachine["Job State Machine"]

        subgraph Engines ["Domain Engines"]
            ImageEngine["ImageEngine"]
            PdfEngine["PdfEngine"]
            DocumentEngine["DocumentEngine"]
        end
    end

    subgraph PersistenceTier ["Tier 3: Persistence & OS Integration Layer"]
        OutMgr["OutputManager<br/>- Collision Resolver<br/>- Staging File Manager<br/>- Atomic Finalizer"]
        WinReg["Windows Registry<br/>(HKCU SystemFileAssociations)"]
        WinNotify["Windows Action Center<br/>(WinRT Toast Notifications)"]
        Disk["Filesystem Storage<br/>(NTFS / ReFS Volumes)"]
        DiskCache["Thumbnail Cache<br/>(%TEMP%\\morph_thumbs\\)"]
    end

    CLI -->|Depends on| BusinessTier
    UI -->|Depends on| BusinessTier

    CLI -->|Interacts with| WinReg
    CLI -->|Dispatches| WinNotify
    UI -->|Disk Caching| DiskCache

    Registry --> Detection
    Registry --> Engines
    JobStateMachine --> Engines
    Engines --> OutMgr
    OutMgr -->|Atomic Rename| Disk
```

### Architectural Principles Governing This Structure
1. **Engine Independence**: `morph-core` has zero knowledge of `morph-cli` or `morph-ui`. It can be imported by any Rust binary, compiled as a C-compatible dynamic library (`cdylib`), or integrated into a web service.
2. **Headless Execution**: All business logic (format sniffing, PDF tree manipulation, image filtering) executes without initializing a window or graphics context.
3. **No Circular Dependencies**: Dependencies flow strictly downward. Tier 1 depends on Tier 2; Tier 2 depends on standard libraries and OS primitives. Tier 2 never calls back into Tier 1.

---

## 2. Layer Responsibilities & Component Details

### Layer 1: Presentation & Invocation Layer
- **`morph-cli`**:
  - Handles process bootstrapping.
  - Dynamically binds to parent console via `AttachConsole`.
  - Parses command-line tokens (`convert`, `merge`, `pdf rotate`, `pdf delete`, `register`, `unregister`, `status`).
  - Routes complex multi-file visual edit requests to `morph-ui.exe`.
  - Dispatches non-blocking WinRT Toast Notifications on job completion or failure.
- **`morph-ui`**:
  - Runs an immediate-mode 60 FPS graphics loop via `eframe` and hardware OpenGL (`glow`).
  - Implements physics-based mass-spring-damper animations for zoom levels and canvas rotation.
  - Manages multi-document tab state, undo/redo history stacks, and drag-and-drop overlays.
  - Coordinates three dedicated background threads via MPSC channels to keep the UI completely non-blocking.

### Layer 2: Core Domain & Engine Layer (`morph-core`)
- **`detection`**:
  - Validates input files by reading the initial 64 bytes (magic headers).
  - Distinguishes between true file formats and deceptive or misnamed extensions.
- **`registry`**:
  - Maintains a catalog of available conversion capabilities (`ConversionOperation`).
  - Exposes query methods: `find_instant_operations`, `find_editor_operations`, `can_convert`.
- **`engines::image`**:
  - Manages cross-format raster encoding/decoding via `image`.
  - Implements single-pass JPEG embedding into PDF `/XObject` streams via `/DctDecode`.
  - Splits RGBA images into raw RGB streams and separate 8-bit alpha soft masks (`/SMask`) with zlib compression.
- **`engines::pdf`**:
  - Manipulates PDF object trees directly via `lopdf`.
  - Renumbers object IDs dynamically to merge disparate PDF documents into a unified object catalog.
  - Modifies page `/Rotate` dictionary attributes losslessly ($90^\circ, 180^\circ, 270^\circ$).
  - Re-indexes `/Kids` and `/Count` arrays in the parent `/Pages` tree for page deletion and reordering.
  - Assembles unified documents from mixed arrays of `PageSpec` (PDF pages + images).
- **`engines::document`**:
  - Bridges office document formats (DOCX, DOC, ODT, RTF) to PDF using local Microsoft Word COM automation or headless LibreOffice.

### Layer 3: Persistence & OS Integration Layer
- **`output::OutputManager`**:
  - Resolves non-colliding destination paths (`filename (1).ext`).
  - Creates hidden sibling staging files (`.morph-tmp-XXXX`).
  - Performs atomic finalization using OS filesystem rename primitives.
  - Validates output integrity (nonzero byte lengths).

---

## 3. Data Flow Architecture

The diagram below tracks the complete lifecycle of a file conversion request:

```mermaid
sequenceDiagram
    autonumber
    actor User as User / Shell / AI Agent
    participant CLI as morph-cli
    participant Sniffer as detection.rs
    participant Registry as registry.rs
    participant Engine as engines (image/pdf)
    participant OutMgr as output.rs
    participant FS as Local Filesystem
    participant Toast as Windows Toast Notifier

    User->>CLI: morph convert invoice.png --to pdf
    CLI->>Sniffer: detect_format_from_path(invoice.png)
    Sniffer->>FS: Read first 64 bytes
    FS-->>Sniffer: Header: \x89PNG\r\n\x1a\n
    Sniffer-->>CLI: Format::Png
    CLI->>Registry: can_convert(Png, Pdf)
    Registry-->>CLI: true (img_to_pdf operation)

    CLI->>OutMgr: resolve_destination(invoice.png, "pdf", options)
    OutMgr->>FS: Check if invoice.pdf exists
    FS-->>OutMgr: File does not exist
    OutMgr-->>CLI: Path: invoice.pdf

    CLI->>OutMgr: create_staging_file(destination_dir)
    OutMgr->>FS: Create .morph-tmp-982341.tmp
    OutMgr-->>Engine: Staging NamedTempFile

    CLI->>Engine: images_to_pdf([invoice.png], options)
    Engine->>Engine: Decode dimensions, build PDF Catalog, Pages & XObject
    Engine->>FS: Write PDF stream into staging file
    Engine-->>CLI: Ok()

    CLI->>OutMgr: finalize_atomic(staging, invoice.pdf)
    OutMgr->>FS: Validate length > 0
    OutMgr->>FS: Atomic Rename: .morph-tmp-982341.tmp -> invoice.pdf
    OutMgr-->>CLI: Ok(invoice.pdf)

    CLI->>Toast: send_notification("Conversion Complete", "Saved to invoice.pdf")
    CLI-->>User: Exit Code 0 (Success)
```

---

## 4. UI Architecture & Asynchronous Worker Pipeline

`morph-ui` implements an **Asynchronous Multi-Threaded Reactive Pipeline**. The UI thread never blocks on disk I/O, PDF rendering, or image decoding:

```mermaid
flowchart TD
    subgraph UIThread ["Main UI Thread (60 FPS egui Event Loop)"]
        InputHandler["Input & Drag-Drop Handler"]
        LayoutEngine["View Layout Engine<br/>(Header, Tabs, PageRail, Canvas, Toolbar)"]
        Springs["Mass-Spring-Damper Physics<br/>(Zoom & Canvas Rotation)"]
        EventConsumer["try_recv() Event Consumer"]
        TextureManager["GPU Texture Manager<br/>(load_texture to OpenGL)"]
    end

    subgraph MPSCChannels ["MPSC Channel Infrastructure"]
        FileLoadTx["file_loader_tx<br/>(FileLoadRequest)"]
        ThumbTx["thumb_queue_tx<br/>(PathBuf, PathBuf, u64)"]
        TexReqTx["texture_request_tx<br/>(TextureRequest)"]
        EventTx["event_tx / event_rx<br/>(BackgroundEvent)"]
    end

    subgraph BackgroundWorkers ["Dedicated Background Worker Threads"]
        FileWorker["Worker 1: File Loader Thread<br/>- Inspects page counts<br/>- Maps PageItem structures<br/>- Creates document tabs"]
        ThumbWorker["Worker 2: PDF Thumbnail Thread<br/>- Runs background python/fitz<br/>- Renders 72 DPI thumbnail PNGs<br/>- Extracts 280-char snippets"]
        TexWorker["Worker 3: Texture Decoder Thread<br/>- Reads PNG bytes off disk<br/>- Decodes to ColorImage RGBA<br/>- Dispatches to UI thread"]
    end

    InputHandler -->|Send request| FileLoadTx
    FileLoadTx --> FileWorker
    FileWorker -->|Send thumb tasks| ThumbTx
    ThumbTx --> ThumbWorker
    FileWorker -->|Send DocumentLoaded| EventTx

    LayoutEngine -->|Visible thumb request| TexReqTx
    TexReqTx --> TexWorker
    TexWorker -->|Send TextureDecoded| EventTx

    EventTx --> EventConsumer
    EventConsumer --> TextureManager
    TextureManager --> LayoutEngine
    Springs --> LayoutEngine
```

### The 4-Tier GPU Texture Pipeline
To guarantee that rotating pages executes at a fluid 60 FPS without disk latency or white flickering, `morph-ui` utilizes a 4-tier texture fallback mechanism:
1. **Tier 1 (Dedicated Rotated Thumbnail)**: If a thumbnail rendered at the target rotation angle exists in cache, paint it directly.
2. **Tier 2 (Dedicated Rotated Preview)**: If a high-res preview at the target rotation angle exists, paint it directly.
3. **Tier 3 (Base Unrotated Thumbnail with GPU Vertex Rotation)**: If only the unrotated thumbnail is in memory, apply a hardware rotation transform (`egui::Image::rotate`) in the vertex shader.
4. **Tier 4 (Base Preview with GPU Vertex Rotation)**: Fall back to rotating the high-res unrotated texture on the GPU.

This guarantees that rotating a document never displays a blank card.

---

## 5. Security & Boundary Architecture

| Boundary | Mechanism | Verification / Guarantee |
| :--- | :--- | :--- |
| **Network Boundary** | Zero socket calls | The codebase contains zero HTTP, TCP, or UDP imports. Fully air-gapped. |
| **Filesystem Boundary** | Sibling staging files | Temporary files are created strictly in the target destination's parent folder, guaranteeing same-volume atomic renames without cross-drive copy delays. |
| **Execution Boundary** | HKCU Shell Verbs | Context menu registration occurs strictly in `HKEY_CURRENT_USER`, ensuring unprivileged operation without administrative UAC triggers. |
| **Process Isolation** | `CREATE_NO_WINDOW` | Subprocess invocations (PowerShell for toasts, Python for thumbnails, Word COM) enforce the `0x08000000` creation flag, preventing background console window flashes. |

