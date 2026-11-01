# 14: Glossary & Technical Terminology

This glossary defines the technical terminology, specifications, and computing concepts utilized throughout the **Morph** codebase.

---

## 1. PDF Specifications & Document Architecture

### Cross-Reference Table (`xref`)
A table located near the end of a PDF file mapping every indirect object number (e.g., `12 0 obj`) to its exact byte offset from the start of the file. Allows PDF readers to seek directly to any page or image without parsing the entire file linearly.

### Cross-Reference Stream (`/XRef`)
Introduced in PDF 1.5, a compressed binary stream replacing the legacy plaintext `xref` table. Compacts object offset data via Flate/Deflate compression.

### Object Stream (`/ObjStm`)
A PDF stream that packs multiple non-stream objects (dictionaries, numbers, strings, arrays) into a single compressed block, significantly reducing file size.

### Document Catalog (`/Catalog` or `/Root`)
The root dictionary of the PDF object hierarchy. Pointed to by the `/Root` key in the document trailer; contains references to the document's `/Pages` tree, metadata, outlines, and interactive forms.

### Pages Tree (`/Pages`, `/Kids`, `/Count`)
A tree structure defining the ordered sequence of pages in a document. The root node has `/Type /Pages`, a `/Count` integer specifying total pages, and a `/Kids` array holding object references (`[3 0 R, 4 0 R, ...]`) to child nodes or individual page dictionaries.

### `/MediaBox`
A four-element array of numbers (`[lower_left_x, lower_left_y, upper_right_x, upper_right_y]`) defining the physical boundaries of the page in default user space units (where 1 unit = $1/72$ of an inch).

### Image XObject (`/Subtype /Image`)
An external visual resource containing binary image data. Can be referenced repeatedly across multiple pages via the `Do` operator without duplicating the underlying image bytes.

### `/Filter /DctDecode`
A PDF decompression filter that accepts raw Discrete Cosine Transform (JPEG) bitstreams. Allows embedding existing JPEG images directly into PDF containers without decoding or recompression.

### `/Filter /FlateDecode`
A PDF decompression filter utilizing zlib/DEFLATE compression. Used by Morph to compress uncompressed RGB pixel arrays and alpha masks.

### Soft Mask (`/SMask`)
An ancillary 8-bit grayscale Image XObject linked to a primary RGB image via the `/SMask` dictionary key. Defines pixel-by-pixel opacity (transparency), enabling full alpha channel representation in PDF documents.

---

## 2. Windows Operating System & Win32 Internals

### Portable Executable (PE / PE32+)
The standard binary executable file format used in Windows 32-bit and 64-bit systems. Contains the DOS stub, COFF header, Optional Header, and section tables (`.text`, `.rdata`, `.data`, `.rsrc`).

### `IMAGE_SUBSYSTEM_WINDOWS_GUI`
A flag in the PE Optional Header indicating that the binary does not require a default console window. Morph sets `#![windows_subsystem = "windows"]` to ensure File Explorer actions launch silently.

### `AttachConsole`
A Win32 API function allowing a GUI-subsystem process to dynamically connect to the console screen buffer of its parent process (`ATTACH_PARENT_PROCESS = 0xFFFFFFFF`). Enables Morph to output progress to PowerShell or terminals when launched from the CLI.

### `CONOUT$`
A special reserved Win32 device name representing the active console screen buffer. Opening `CONOUT$` allows writing console output from GUI-subsystem processes where `stdout` handles may be invalid.

### `SystemFileAssociations`
A Windows Registry path (`HKCU\Software\Classes\SystemFileAssociations\<ext>\shell`) allowing applications to register shell verbs for specific file extensions on a per-user basis without administrative privileges.

### `SHChangeNotify`
A Windows Shell API function (`shell32.dll`) that broadcasts an event notification (`SHCNE_ASSOCCHANGED`) to all running Explorer windows, invalidating their file association caches and updating right-click menus immediately.

### Session 0 Isolation
A Windows security architecture introduced in Windows Vista where system services and non-interactive tasks execute in Session 0, while interactive user desktops run in Session 1+. Prevents non-interactive background tasks from launching interactive GUI or COM applications (such as Microsoft Word) without special DCOM permissions.

### Atomic Rename (`MoveFileExW`)
An operating system primitive that replaces a destination directory entry with a staging file in a single, uninterruptible filesystem operation. Guarantees that readers never observe partially written files.

---

## 3. Systems Programming & GUI Architecture

### Immediate-Mode GUI (IMGUI)
A graphical user interface paradigm where the application redraws the entire visible interface every frame based directly on current data state. Eliminates synchronization bugs, widget tree caches, and DOM reconciliation overhead.

### Mass-Spring-Damper Oscillator
A second-order differential equation simulating mechanical spring physics:
$$F = -k \cdot x - c \cdot v$$
Used by `morph-ui` to drive organic, fluid animations for continuous zooming and rotation transitions.

### Epsilon Snapping
An optimization technique in animation physics where calculations halt once displacement and velocity drop below a tiny threshold ($\epsilon < 0.005$), allowing the GUI event loop to enter an idle state with 0% CPU consumption.

### RAII (Resource Acquisition Is Initialization)
A programming idiom where resource allocation (file handles, memory, temporary files) is tied to object lifetime. When the object goes out of scope, its destructor (`Drop::drop`) executes automatically, guaranteeing zero resource leaks.

### Multi-Producer, Single-Consumer (MPSC)
A thread synchronization primitive providing a channel with multiple transmitting endpoints (`Sender`) and exactly one receiving endpoint (`Receiver`). Used in Morph to stream background loading tasks to the main UI thread.

### Viewport Culling / Virtualized Scrolling
An optimization pattern where a scrollable container only instantiates and renders the elements currently intersecting the visible screen rectangle, maintaining constant memory and processing overhead regardless of collection size.

