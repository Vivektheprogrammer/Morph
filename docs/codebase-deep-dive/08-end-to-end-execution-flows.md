# 08: End-to-End Execution Flows & Sequence Traces

This chapter traces five critical end-to-end workflows in **Morph**, mapping every step from user trigger to final operating system output.

---

## 1. Flow 1: Context Menu Instant Conversion

This flow traces what happens when a user right-clicks `invoice.png` in Windows File Explorer and selects **Morph $\rightarrow$ Convert to PDF**:

```mermaid
sequenceDiagram
    autonumber
    actor User as Windows Explorer User
    participant Explorer as Windows File Explorer (explorer.exe)
    participant Registry as Windows Registry (HKCU)
    participant CLI as morph.exe (morph-cli)
    participant CoreSniff as detection.rs
    participant CoreImg as engines/image.rs
    participant OutMgr as output.rs
    participant Disk as Local Filesystem
    participant Toast as Windows Action Center

    User->>Explorer: Right-click "invoice.png" -> Morph -> "Convert to PDF"
    Explorer->>Registry: Read command under SystemFileAssociations\.png\shell\Morph\shell\ConvertToPdf\command
    Registry-->>Explorer: "morph.exe" convert "%1" --to pdf
    Explorer->>CLI: Spawn morph.exe convert "C:\Docs\invoice.png" --to pdf
    CLI->>CLI: #![windows_subsystem="windows"] prevents console allocation
    CLI->>CLI: AttachConsole(ATTACH_PARENT_PROCESS) fails silently

    CLI->>CoreSniff: detect_format_from_path("C:\Docs\invoice.png")
    CoreSniff->>Disk: Read first 64 bytes
    Disk-->>CoreSniff: "\x89PNG\r\n\x1a\n..."
    CoreSniff-->>CLI: Ok(Format::Png)

    CLI->>OutMgr: resolve_destination("C:\Docs\invoice.png", "pdf", options)
    OutMgr->>Disk: Check exists("C:\Docs\invoice.pdf")
    Disk-->>OutMgr: false
    OutMgr-->>CLI: Ok("C:\Docs\invoice.pdf")

    CLI->>OutMgr: create_staging_file("C:\Docs")
    OutMgr->>Disk: Create ".morph-tmp-82194.tmp"
    OutMgr-->>CLI: Ok(NamedTempFile)

    CLI->>CoreImg: images_to_pdf(["C:\Docs\invoice.png"], options)
    CoreImg->>Disk: Read invoice.png
    CoreImg->>CoreImg: Extract RGBA, compress RGB & Alpha mask via miniz_oxide
    CoreImg->>CoreImg: Construct PDF Catalog, Pages tree & XObjects via pdf-writer
    CoreImg->>Disk: Write serialized PDF bytes into ".morph-tmp-82194.tmp"
    CoreImg-->>CLI: Ok()

    CLI->>OutMgr: finalize_atomic(staging, "C:\Docs\invoice.pdf")
    OutMgr->>Disk: Verify length > 0
    OutMgr->>Disk: MoveFileExW(".morph-tmp-82194.tmp", "C:\Docs\invoice.pdf")
    OutMgr-->>CLI: Ok("C:\Docs\invoice.pdf")

    CLI->>Toast: send_notification("Morph - Conversion Complete", "Saved to invoice.pdf")
    Toast-->>User: Native Windows Toast Notification slides onto screen
    CLI-->>Explorer: Process terminates (Exit code 0)
```

---

## 2. Flow 2: Multi-File CLI Merge Command

This flow traces an AI coding agent or automation script executing a multi-format merge command:
```powershell
morph merge cover.pdf receipt.jpg summary.png --output final_package.pdf
```

```mermaid
sequenceDiagram
    autonumber
    actor Agent as Autonomous AI Agent / PowerShell
    participant CLI as morph.exe (morph-cli)
    participant CorePdf as engines/pdf.rs
    participant CoreImg as engines/image.rs
    participant OutMgr as output.rs
    participant Disk as Local Filesystem

    Agent->>CLI: morph merge cover.pdf receipt.jpg summary.png --output final_package.pdf
    CLI->>CLI: AttachConsole(ATTACH_PARENT_PROCESS) attaches to Agent terminal
    CLI->>CLI: log_info! writes to CONOUT$
    CLI->>CorePdf: Detect cover.pdf format -> Format::Pdf

    Note over CLI,CorePdf: Mixing PDFs and images triggers Universal Assembly via PageSpec

    CLI->>Disk: Create temp staging directory
    CLI->>CoreImg: Convert receipt.jpg -> temp_0001.pdf
    CLI->>CoreImg: Convert summary.png -> temp_0002.pdf
    CLI->>CorePdf: merge_pdfs([cover.pdf, temp_0001.pdf, temp_0002.pdf], options)

    CorePdf->>CorePdf: Renumber object IDs across all 3 documents
    CorePdf->>CorePdf: Merge object tables, assemble new unified /Pages catalog
    CorePdf->>OutMgr: finalize_atomic(staging, "final_package.pdf")
    OutMgr-->>CLI: Ok("final_package.pdf")

    CLI->>Agent: Console output: "Success: Created 'final_package.pdf'"
    CLI-->>Agent: Process terminates with exit code 0
```

---

## 3. Flow 3: Interactive Editor File Drag-and-Drop & GPU Rotation

This flow traces what happens inside the desktop editor (`morph-ui.exe`) when a user drags a file into the window and rotates a page:

```mermaid
sequenceDiagram
    autonumber
    actor User as User
    participant Viewport as egui Viewport
    participant App as MorphApp (UI Thread)
    participant MPSC as File Loader Channel
    participant Worker as Background File Worker
    participant GPU as Hardware GPU (OpenGL)

    User->>Viewport: Drag "contract.pdf" over Morph Editor window
    Viewport->>App: ctx.input().raw.hovered_files populated
    App->>Viewport: render_dropzone_overlay(ctx, is_hovering=true)
    Viewport-->>User: Pulsing blue dropzone overlay appears

    User->>Viewport: Drop file into window
    Viewport->>App: ctx.input().raw.dropped_files received
    App->>MPSC: file_loader_tx.send(FileLoadRequest::OpenDocuments)
    Note over App: Main UI thread continues at 60 FPS; zero freeze!

    Worker->>MPSC: Read page count from contract.pdf (12 pages)
    Worker->>MPSC: Dispatch thumbnail requests to thumb_queue_tx
    Worker->>App: event_tx.send(BackgroundEvent::DocumentLoaded)

    App->>App: Add DocumentState to documents array
    App->>Viewport: Render DocumentTab & PageRail thumbnail cards

    User->>Viewport: Click "Rotate Clockwise" (R key)
    App->>App: page.rotation_degrees = (0 + 90) % 360 = 90
    App->>App: canvas_rotation_spring.set_target(90.0)
    App->>GPU: egui::Image::rotate(1.5708 rad) modifies vertex UV matrix
    GPU-->>User: Page rotates instantaneously at 60 FPS without disk read!
```

---

## 4. Flow 4: Context Menu Shell Registration

This flow traces executing `morph register` to configure Windows File Explorer:

```mermaid
sequenceDiagram
    autonumber
    actor Admin as User / Installer
    participant CLI as morph.exe
    participant CtxMgr as context_menu.rs
    participant WinReg as Windows Registry (HKCU)
    participant Shell32 as shell32.dll (SHChangeNotify)
    participant Explorer as Windows Explorer

    Admin->>CLI: morph register
    CLI->>CtxMgr: register_current_exe()
    CtxMgr->>CtxMgr: Resolve absolute paths to morph.exe, morph-ui.exe, logo.ico

    loop For each image and document extension (.png, .jpg, .pdf, .docx...)
        CtxMgr->>WinReg: CreateSubKey HKCU\Software\Classes\SystemFileAssociations\<ext>\shell\Morph
        CtxMgr->>WinReg: SetValue "MUIVerb" = "Morph"
        CtxMgr->>WinReg: SetValue "Icon" = "C:\Morph\assets\logo.ico"
        CtxMgr->>WinReg: CreateSubKey shell\ConvertToPdf\command
        CtxMgr->>WinReg: SetValue "" = '"C:\Morph\target\release\morph.exe" convert "%1" --to pdf'
    end

    CtxMgr->>Shell32: SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, null, null)
    Shell32-->>Explorer: Invalidate file association icon & verb caches
    Explorer-->>Explorer: File Explorer refreshes instantly without restarting!
    CLI-->>Admin: "Success: Morph context menu successfully registered."
```

---

## 5. Flow 5: Background Thumbnail Rendering & In-Flight Deduplication

This flow traces how thumbnail requests are deduplicated and processed asynchronously off the UI thread:

```mermaid
sequenceDiagram
    autonumber
    participant UI as UI Render Loop (page_rail.rs)
    participant Dedupe as pending_previews (Arc<Mutex<HashSet>>)
    participant Worker as Background Thread
    participant Python as Headless Python (PyMuPDF)
    participant Disk as Cache Directory (%TEMP%\morph_thumbs)
    participant TexWorker as Texture Decoder Worker
    participant EventRx as UI Event Receiver

    UI->>Dedupe: Check if (hash, page_num) in pending set
    Dedupe-->>UI: Not present -> Insert into set & spawn task

    UI->>Worker: Spawn background task
    Worker->>Python: Execute script with CREATE_NO_WINDOW
    Python->>Disk: Render 72 DPI pixmap -> thumb_00a12f_1.png
    Python-->>Worker: Complete
    Worker->>Dedupe: Remove (hash, page_num) from pending set

    UI->>TexWorker: texture_request_tx.send(TextureRequest)
    TexWorker->>Disk: Read thumb_00a12f_1.png
    TexWorker->>TexWorker: Decode PNG bytes to ColorImage RGBA
    TexWorker->>EventRx: event_tx.send(BackgroundEvent::TextureDecoded)

    EventRx->>UI: Receive TextureDecoded in update()
    UI->>UI: ctx.load_texture() loads into GPU memory
    UI-->>UI: Thumbnail appears smoothly in PageRail!
```

