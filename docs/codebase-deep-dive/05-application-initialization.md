# 05: Application Initialization & Runtime Startup

This document traces the exact execution path taken when an operating system process is spawned for either **`morph.exe`** (the command-line interface and context-menu dispatcher) or **`morph-ui.exe`** (the graphical popup document editor).

---

## 1. Process Lifecycle Comparison

```mermaid
flowchart TD
    subgraph CLIStartup ["morph.exe Startup Lifecycle"]
        CLI_Entry["Windows PE Loader calls main()"]
        CLI_Subsystem["#![windows_subsystem = 'windows']<br/>Prevents default console allocation"]
        CLI_Attach["AttachConsole(ATTACH_PARENT_PROCESS)<br/>Attempts Win32 console hook"]
        CLI_Args["Parse env::args()"]
        CLI_Branch{"Parent is Terminal?"}
        CLI_Console["Open 'CONOUT$' for stdout/stderr logging"]
        CLI_Silent["Silent headless mode; no console opened"]
        CLI_Dispatch["Dispatch Command:<br/>convert | merge | pdf | edit | register"]
        CLI_Exit["Clean Process Exit (Code 0 or 1)"]

        CLI_Entry --> CLI_Subsystem --> CLI_Attach --> CLI_Args --> CLI_Branch
        CLI_Branch -->|Yes| CLI_Console --> CLI_Dispatch
        CLI_Branch -->|No (File Explorer)| CLI_Silent --> CLI_Dispatch
        CLI_Dispatch --> CLI_Exit
    end

    subgraph UIStartup ["morph-ui.exe Startup Lifecycle"]
        UI_Entry["Windows PE Loader calls main()"]
        UI_Args["Extract env::args() for initial files"]
        UI_Icon["Decode embedded logo_fluent.png into IconData"]
        UI_Viewport["Configure egui::ViewportBuilder (1120x780, min 760x520)"]
        UI_RunNative["eframe::run_native('Morph Editor', options)"]
        UI_AppNew["MorphApp::new(initial_files)"]
        UI_SpawnWorkers["spawn_workers()<br/>- File Loader Thread<br/>- PDF Thumbnail Thread<br/>- Texture Decoder Thread"]
        UI_Cache["get_cache_dir()<br/>Auto-prune files > 24 hours old"]
        UI_LoadInitial["Dispatch initial file load requests via MPSC"]
        UI_Loop["Enter 60 FPS eframe::App::update event loop"]

        UI_Entry --> UI_Args --> UI_Icon --> UI_Viewport --> UI_RunNative
        UI_RunNative --> UI_AppNew --> UI_SpawnWorkers --> UI_Cache --> UI_LoadInitial --> UI_Loop
    end
```

---

## 2. `morph-cli` Initialization Deep Dive

### Step 1: Subsystem Declaration
In [`crates/morph-cli/src/main.rs`](file:///c:/Morph/crates/morph-cli/src/main.rs#L1):
```rust
#![windows_subsystem = "windows"]
```
When Windows executes a binary, the PE header flag `Subsystem` (offset `0x5C` in the optional header) dictates how the Windows subsystem manager handles process creation:
- `IMAGE_SUBSYSTEM_WINDOWS_CUI` (Console): Spawns a new command prompt window if not launched from an existing console.
- `IMAGE_SUBSYSTEM_WINDOWS_GUI` (GUI): Spawns without allocating a console window.

By declaring `windows_subsystem = "windows"`, Morph prevents File Explorer from flashing a black terminal window when a user selects a right-click conversion action.

### Step 2: Dynamic Console Attachment (`AttachConsole`)
In [`crates/morph-cli/src/main.rs`](file:///c:/Morph/crates/morph-cli/src/main.rs#L74-L81):
```rust
#[cfg(windows)]
unsafe {
    windows_sys::Win32::System::Console::AttachConsole(
        windows_sys::Win32::System::Console::ATTACH_PARENT_PROCESS,
    );
}
```
If `morph.exe` is launched from PowerShell, CMD, or an AI coding agent, `AttachConsole` attaches the GUI process to the calling terminal's existing console buffer. If launched by File Explorer, `AttachConsole` fails with error code `6 (ERROR_INVALID_HANDLE)`, which Morph silently ignores.

### Step 3: Console I/O Redirection via `CONOUT$`
Because standard `println!` or `eprintln!` calls can fail or panic when a Windows GUI binary lacks standard handles, Morph defines dedicated logging macros:
```rust
macro_rules! log_info {
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        #[cfg(windows)]
        {
            use std::io::Write;
            if let Ok(mut conout) = std::fs::OpenOptions::new().write(true).open("CONOUT$") {
                let _ = writeln!(conout, "{}", msg);
            }
        }
    }};
}
```
`CONOUT$` is a reserved Win32 device name referencing the active console screen buffer. Writing directly to `CONOUT$` guarantees terminal output displays reliably without requiring a standard console subsystem allocation.

### Step 4: Argument Routing & Dispatch
In [`crates/morph-cli/src/main.rs`](file:///c:/Morph/crates/morph-cli/src/main.rs#L83-L103):
```rust
let args: Vec<String> = env::args().collect();
if args.len() < 2 {
    print_help();
    return;
}

match args[1].as_str() {
    "convert" => handle_convert(&args[2..]),
    "merge" => handle_merge(&args[2..]),
    "pdf" => handle_pdf(&args[2..]),
    "edit" => handle_edit(&args[2..]),
    "register" => handle_register(),
    "unregister" => handle_unregister(),
    "status" => handle_status(),
    // ...
}
```

---

## 3. `morph-ui` Initialization Deep Dive

### Step 1: Binary Setup & Embedded Branding
In [`crates/morph-ui/src/main.rs`](file:///c:/Morph/crates/morph-ui/src/main.rs#L1104-L1139):
```rust
fn main() -> eframe::Result<()> {
    let args: Vec<String> = env::args().collect();
    let initial_files: Vec<PathBuf> = args.into_iter().skip(1).map(PathBuf::from).collect();

    let icon_data = if let Ok(img) = image::load_from_memory(include_bytes!("../../../assets/logo_fluent.png")) {
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        Some(egui::IconData {
            rgba: rgba.into_raw(),
            width,
            height,
        })
    } else {
        None
    };
```
1. `include_bytes!("../../../assets/logo_fluent.png")`: Embeds the raw PNG bytes of the logo directly into the read-only data segment (`.rdata`) of `morph-ui.exe` at compile time.
2. Decodes the image in memory into raw 32-bit RGBA pixel bytes to construct `egui::IconData`.

### Step 2: Native Viewport Configuration
```rust
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1120.0, 780.0])
        .with_min_inner_size([760.0, 520.0])
        .with_title("Morph Editor");

    if let Some(icon) = icon_data {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
```
The viewport specifies default dimensions ($1120 \times 780$ pixels) and minimum dimensions ($760 \times 520$ pixels) ensuring responsive layout integrity even on smaller laptop displays.

### Step 3: Instantiating `MorphApp` & Spawning Background Channels
When `eframe::run_native` boots, it invokes the constructor closure:
```rust
Box::new(|_cc| Ok(Box::new(MorphApp::new(initial_files))))
```

Inside `MorphApp::new`:
```rust
let (thumb_queue_tx, texture_request_tx, file_loader_tx, event_rx) = spawn_workers();
```
`spawn_workers()` initializes three background worker threads using Rust standard library channels:
1. **Thread 1 (PDF Thumbnail Pipeline)**: Listens on `thumb_queue_tx`. Spawns headless Python scripts to render 72 DPI thumbnail PNGs in `%TEMP%\morph_thumbs\`.
2. **Thread 2 (Texture Decoding Pipeline)**: Listens on `texture_request_tx`. Asynchronously reads PNG files from disk, converts them to `egui::ColorImage`, and dispatches them back to `event_rx`.
3. **Thread 3 (File Loader Pipeline)**: Listens on `file_loader_tx`. Inspects file headers, parses total PDF page counts via `PdfEngine::page_count`, generates `PageItem` records, and pushes `BackgroundEvent::DocumentLoaded` into `event_rx`.

### Step 4: Cache Hygiene on Startup
In [`crates/morph-ui/src/state/mod.rs`](file:///c:/Morph/crates/morph-ui/src/state/mod.rs#L182-L202):
```rust
pub fn get_cache_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("morph_thumbs");
    let _ = std::fs::create_dir_all(&dir);

    // Auto-prune stale temporary preview and thumbnail files older than 24 hours
    if let Ok(entries) = std::fs::read_dir(&dir) {
        let now = std::time::SystemTime::now();
        let max_age = std::time::Duration::from_secs(24 * 60 * 60);
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if let Ok(modified) = meta.modified() {
                    if let Ok(age) = now.duration_since(modified) {
                        if age > max_age {
                            let _ = std::fs::remove_file(entry.path());
                        }
                    }
                }
            }
        }
    }
    dir
}
```
Every time Morph starts, it scans `%TEMP%\morph_thumbs\` and purges any files whose `LastModified` timestamp exceeds 24 hours, preventing cache growth over weeks of usage.

### Step 5: Initial Frame Loop Entry
Once `MorphApp::new` completes, the main thread transitions into `eframe::App::update`:
1. `theme::apply_theme(ctx)`: Sets Fluent dark mode palettes, borders, and rounding.
2. `event_rx.try_recv()`: Drains any events produced by background workers.
3. `Spring::update(dt)`: Advances animation physics.
4. Renders top panels, page rail, and central canvas.

