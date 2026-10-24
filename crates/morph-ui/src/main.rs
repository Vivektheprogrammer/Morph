#![windows_subsystem = "windows"]

use eframe::egui::{self, Align, Color32, Layout, RichText, Sense, Stroke, Vec2};
use morph_core::engines::pdf::{PageSpec, PdfEngine};
use morph_core::format::Format;
use morph_core::output::OutputOptions;
use morph_core::detect_format_from_path;
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};

fn hash_path(path: &Path) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut s = DefaultHasher::new();
    path.to_string_lossy().hash(&mut s);
    s.finish()
}

fn get_cache_dir() -> PathBuf {
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

/// Renders 72 DPI thumbnails asynchronously in a background worker thread.
/// Also extracts text snippets into snip_{hash}_{page}.txt without blocking.
/// NEVER blocks the UI event loop -> fluid 60 FPS interface during multi-page rendering.
fn render_pdf_thumbnails_bg(pdf_path: PathBuf, cache_dir: PathBuf, hash: u64) {
    let script = format!(
        r#"import fitz, os
try:
    doc = fitz.open(r"{pdf_path}")
    for i in range(len(doc)):
        p = doc[i]
        t_path = os.path.join(r"{cache_dir}", f"thumb_{path_hash:016x}_{{i+1}}.png")
        if not os.path.exists(t_path):
            p.get_pixmap(dpi=72).save(t_path)
        txt = p.get_text()[:280].strip()
        if txt:
            snip_path = os.path.join(r"{cache_dir}", f"snip_{path_hash:016x}_{{i+1}}.txt")
            if not os.path.exists(snip_path):
                with open(snip_path, "w", encoding="utf-8") as f:
                    f.write(txt)
except Exception:
    pass
"#,
        pdf_path = pdf_path.display(),
        cache_dir = cache_dir.display(),
        path_hash = hash,
    );

    #[cfg(windows)]
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let mut cmd = std::process::Command::new("python");
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let _ = cmd.args(["-c", &script]).output();
}

/// Renders high-resolution 180 DPI page image in a background thread for the single page currently being viewed.
/// Only renders 1 page (~20ms) instead of the entire document, keeping memory low and speed instant.
fn render_single_page_preview_bg(pdf_path: &Path, cache_dir: &Path, hash: u64, page_num: u32) {
    let p_path = cache_dir.join(format!("page_{:016x}_{}.png", hash, page_num));
    if p_path.exists() {
        return;
    }

    let script = format!(
        r#"import fitz, os
try:
    doc = fitz.open(r"{pdf_path}")
    idx = {page_idx}
    if idx < len(doc):
        p_path = r"{p_path}"
        if not os.path.exists(p_path):
            doc[idx].get_pixmap(dpi=180).save(p_path)
except Exception:
    pass
"#,
        pdf_path = pdf_path.display(),
        page_idx = page_num.saturating_sub(1),
        p_path = p_path.display(),
    );

    #[cfg(windows)]
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let mut cmd = std::process::Command::new("python");
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let _ = cmd.args(["-c", &script]).output();
}

#[derive(Clone, Debug, PartialEq)]
struct PageItem {
    id: usize,
    original_page_num: u32,
    source_file: PathBuf,
    rotation_degrees: i32,
    is_image: bool,
    thumbnail_path: Option<PathBuf>,
    highres_path: Option<PathBuf>,
    text_snippet: Option<String>,
}

#[derive(Clone, Debug)]
struct EditorHistory {
    pages: Vec<PageItem>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InsertPosition {
    Start,
    AfterSelected,
    End,
}

struct MorphApp {
    _input_file: Option<PathBuf>,
    pages: Vec<PageItem>,
    selected_page_index: Option<usize>,
    history: Vec<EditorHistory>,
    history_index: usize,
    status_message: String,
    status_is_error: bool,
    next_id: usize,
    // Separate textures for compact thumbnails and high-res view
    thumb_textures: HashMap<(usize, i32), egui::TextureHandle>,
    preview_textures: HashMap<(usize, i32), egui::TextureHandle>,
    pending_previews: std::sync::Arc<std::sync::Mutex<std::collections::HashSet<(u64, u32)>>>,
    thumb_queue_tx: std::sync::mpsc::Sender<(PathBuf, PathBuf, u64)>,
    logo_texture: Option<egui::TextureHandle>,
    zoom_level: f32,
    scroll_accumulator: f32,
    last_page_scroll_time: f64,
    central_panel_rect: egui::Rect,
    insert_position: InsertPosition,
    shelf_scroll_offset: f32,
}

impl MorphApp {
    fn new(initial_file: Option<PathBuf>) -> Self {
        let (thumb_queue_tx, thumb_queue_rx) = std::sync::mpsc::channel::<(PathBuf, PathBuf, u64)>();
        std::thread::spawn(move || {
            while let Ok((pdf_path, cache_dir, hash)) = thumb_queue_rx.recv() {
                render_pdf_thumbnails_bg(pdf_path, cache_dir, hash);
            }
        });

        let mut app = Self {
            _input_file: initial_file.clone(),
            pages: Vec::new(),
            selected_page_index: None,
            history: Vec::new(),
            history_index: 0,
            status_message: "Ready".to_string(),
            status_is_error: false,
            next_id: 1,
            thumb_textures: HashMap::new(),
            preview_textures: HashMap::new(),
            pending_previews: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashSet::new())),
            thumb_queue_tx,
            logo_texture: None,
            zoom_level: 1.0,
            scroll_accumulator: 0.0,
            last_page_scroll_time: 0.0,
            central_panel_rect: egui::Rect::NOTHING,
            insert_position: InsertPosition::End,
            shelf_scroll_offset: 0.0,
        };

        if let Some(ref path) = initial_file {
            app.load_file(path.clone());
        }

        app.save_history();
        app
    }

    /// Returns list of unique source files currently in the sequence with (path, is_image, page_count)
    fn get_source_files(&self) -> Vec<(PathBuf, bool, usize)> {
        let mut files: Vec<(PathBuf, bool, usize)> = Vec::new();
        for page in &self.pages {
            if let Some(entry) = files.iter_mut().find(|(p, _, _)| *p == page.source_file) {
                entry.2 += 1;
            } else {
                files.push((page.source_file.clone(), page.is_image, 1));
            }
        }
        files
    }

    /// Removes all pages belonging to the specified source file
    fn remove_source_file(&mut self, path: &Path) {
        let initial_count = self.pages.len();
        let page_ids_to_remove: Vec<usize> = self.pages
            .iter()
            .filter(|p| p.source_file == path)
            .map(|p| p.id)
            .collect();

        self.pages.retain(|p| p.source_file != path);

        for id in page_ids_to_remove {
            self.thumb_textures.retain(|(tid, _), _| *tid != id);
            self.preview_textures.retain(|(pid, _), _| *pid != id);
        }

        // Clean up temporary thumbnail, preview, and snippet files from disk
        let hash = hash_path(path);
        let cache_dir = get_cache_dir();
        let prefix_thumb = format!("thumb_{:016x}_", hash);
        let prefix_prev = format!("prev_{:016x}_", hash);
        let prefix_snip = format!("snip_{:016x}_", hash);
        if let Ok(entries) = std::fs::read_dir(&cache_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with(&prefix_thumb) || name.starts_with(&prefix_prev) || name.starts_with(&prefix_snip) {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }

        let removed = initial_count - self.pages.len();
        if removed > 0 {
            if self.pages.is_empty() {
                self.selected_page_index = None;
            } else if let Some(sel) = self.selected_page_index {
                if sel >= self.pages.len() {
                    self.selected_page_index = Some(self.pages.len() - 1);
                }
            }
            let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
            self.status_message = format!("Removed {} page(s) from '{}'", removed, fname);
            self.status_is_error = false;
            self.save_history();
        }
    }

    fn load_file(&mut self, path: PathBuf) {
        self.load_files_at_position(vec![path], self.insert_position);
    }

    fn load_files_at_position(&mut self, files: Vec<PathBuf>, position: InsertPosition) {
        let insert_index = match position {
            InsertPosition::Start => 0,
            InsertPosition::End => self.pages.len(),
            InsertPosition::AfterSelected => {
                if let Some(sel) = self.selected_page_index {
                    (sel + 1).min(self.pages.len())
                } else {
                    self.pages.len()
                }
            }
        };
        self.load_files_at(files, insert_index);
    }

    fn load_files_at(&mut self, files: Vec<PathBuf>, mut insert_index: usize) {
        let mut total_added = 0;
        let initial_insert = insert_index;
        let cache_dir = get_cache_dir();

        for path in files {
            if !path.exists() {
                continue;
            }

            let fmt = detect_format_from_path(&path).unwrap_or(Format::Pdf);
            let path_hash = hash_path(&path);

            if fmt == Format::Pdf {
                if let Ok(count) = PdfEngine::page_count(&path) {
                    // Send to background queue to render 72 DPI thumbnails and snippets sequentially
                    // NEVER blocks the UI event loop -> zero UI freeze!
                    let _ = self.thumb_queue_tx.send((path.clone(), cache_dir.clone(), path_hash));

                    let mut new_pages = Vec::with_capacity(count);
                    for page_num in 1..=count as u32 {
                        let thumb_file = cache_dir.join(format!("thumb_{:016x}_{}.png", path_hash, page_num));
                        let page_file = cache_dir.join(format!("page_{:016x}_{}.png", path_hash, page_num));

                        new_pages.push(PageItem {
                            id: self.next_id,
                            original_page_num: page_num,
                            source_file: path.clone(),
                            rotation_degrees: 0,
                            is_image: false,
                            thumbnail_path: Some(thumb_file),
                            highres_path: Some(page_file),
                            text_snippet: None,
                        });
                        self.next_id += 1;
                    }

                    let added_len = new_pages.len();
                    if insert_index > self.pages.len() {
                        insert_index = self.pages.len();
                    }
                    self.pages.splice(insert_index..insert_index, new_pages);
                    insert_index += added_len;
                    total_added += added_len;
                }
            } else if fmt.category() == morph_core::FormatCategory::Image {
                let thumb_file = cache_dir.join(format!("img_thumb_{:016x}.png", path_hash));
                if !thumb_file.exists() {
                    let img_src = path.clone();
                    let img_dst = thumb_file.clone();
                    std::thread::spawn(move || {
                        if let Ok(img) = image::open(&img_src) {
                            let thumb = img.thumbnail(320, 440);
                            let _ = thumb.save(&img_dst);
                        }
                    });
                }

                let thumb_opt = if thumb_file.exists() {
                    Some(thumb_file)
                } else {
                    Some(path.clone())
                };

                let new_page = PageItem {
                    id: self.next_id,
                    original_page_num: 1,
                    source_file: path.clone(),
                    rotation_degrees: 0,
                    is_image: true,
                    thumbnail_path: thumb_opt,
                    highres_path: Some(path.clone()),
                    text_snippet: None,
                };
                self.next_id += 1;

                if insert_index > self.pages.len() {
                    insert_index = self.pages.len();
                }
                self.pages.insert(insert_index, new_page);
                insert_index += 1;
                total_added += 1;
            }
        }

        if total_added > 0 {
            self.selected_page_index = Some(initial_insert.min(self.pages.len() - 1));
            let pos_desc = if initial_insert == 0 {
                "at beginning".to_string()
            } else if initial_insert >= self.pages.len() - total_added {
                "at end".to_string()
            } else {
                format!("after page {}", initial_insert)
            };
            self.status_message = format!("Inserted {} page(s) {}", total_added, pos_desc);
            self.status_is_error = false;
            self.save_history();
        }
    }

    fn save_history(&mut self) {
        if self.history_index < self.history.len() {
            self.history.truncate(self.history_index);
        }
        self.history.push(EditorHistory {
            pages: self.pages.clone(),
        });
        self.history_index = self.history.len();
    }

    fn undo(&mut self) {
        if self.history_index > 1 {
            self.history_index -= 1;
            self.pages = self.history[self.history_index - 1].pages.clone();
            if let Some(idx) = self.selected_page_index {
                if idx >= self.pages.len() && !self.pages.is_empty() {
                    self.selected_page_index = Some(self.pages.len() - 1);
                }
            }
            self.status_message = "Undo performed".to_string();
            self.status_is_error = false;
        }
    }

    fn redo(&mut self) {
        if self.history_index < self.history.len() {
            self.history_index += 1;
            self.pages = self.history[self.history_index - 1].pages.clone();
            if let Some(idx) = self.selected_page_index {
                if idx >= self.pages.len() && !self.pages.is_empty() {
                    self.selected_page_index = Some(self.pages.len() - 1);
                }
            }
            self.status_message = "Redo performed".to_string();
            self.status_is_error = false;
        }
    }

    /// Rotates ONLY the specified page by `degrees` (e.g. 90 or 180)
    fn rotate_single_page(&mut self, idx: usize, degrees: i32) {
        if idx < self.pages.len() {
            self.pages[idx].rotation_degrees =
                (self.pages[idx].rotation_degrees + degrees) % 360;
            if self.pages[idx].rotation_degrees < 0 {
                self.pages[idx].rotation_degrees += 360;
            }
            self.selected_page_index = Some(idx);
            self.status_message = format!(
                "Rotated page {} to {}°",
                idx + 1,
                self.pages[idx].rotation_degrees
            );
            self.status_is_error = false;
            self.save_history();
        }
    }

    /// Rotates ALL pages across the ENTIRE document by `degrees` (e.g. 90)
    fn rotate_all_pages(&mut self, degrees: i32) {
        if self.pages.is_empty() {
            return;
        }
        for page in &mut self.pages {
            page.rotation_degrees = (page.rotation_degrees + degrees) % 360;
            if page.rotation_degrees < 0 {
                page.rotation_degrees += 360;
            }
        }
        self.status_message = format!(
            "Rotated all {} pages by {}°",
            self.pages.len(),
            degrees
        );
        self.status_is_error = false;
        self.save_history();
    }

    fn delete_selected(&mut self) {
        if let Some(idx) = self.selected_page_index {
            self.delete_page_at(idx);
        }
    }

    fn delete_page_at(&mut self, idx: usize) {
        if idx < self.pages.len() {
            let page_id = self.pages[idx].id;
            self.pages.remove(idx);

            // Clean up cached textures for removed page
            self.thumb_textures.retain(|(id, _), _| *id != page_id);
            self.preview_textures.retain(|(id, _), _| *id != page_id);

            if self.pages.is_empty() {
                self.selected_page_index = None;
            } else if let Some(sel) = self.selected_page_index {
                if sel >= self.pages.len() {
                    self.selected_page_index = Some(self.pages.len() - 1);
                } else if sel == idx {
                    self.selected_page_index = Some(idx.min(self.pages.len() - 1));
                }
            }
            self.status_message = format!("Page {} removed", idx + 1);
            self.status_is_error = false;
            self.save_history();
        }
    }

    fn move_page_up_at(&mut self, idx: usize) {
        if idx > 0 && idx < self.pages.len() {
            self.pages.swap(idx, idx - 1);
            self.selected_page_index = Some(idx - 1);
            self.save_history();
        }
    }

    fn move_page_down_at(&mut self, idx: usize) {
        if idx + 1 < self.pages.len() {
            self.pages.swap(idx, idx + 1);
            self.selected_page_index = Some(idx + 1);
            self.save_history();
        }
    }

    fn export_result(&mut self) {
        if self.pages.is_empty() {
            self.status_message = "Cannot export: Document has no pages".to_string();
            self.status_is_error = true;
            return;
        }

        let first_file = &self.pages[0].source_file;
        let default_name = first_file
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| format!("{}_edited.pdf", s))
            .unwrap_or_else(|| "morph_export.pdf".to_string());

        let default_dir = first_file.parent().unwrap_or_else(|| Path::new("."));

        let save_target = rfd::FileDialog::new()
            .set_directory(default_dir)
            .set_file_name(&default_name)
            .add_filter("PDF Document (*.pdf)", &["pdf"])
            .save_file();

        let target_path = match save_target {
            Some(p) => p,
            None => {
                return; // User cancelled save dialog
            }
        };

        let output_options = OutputOptions {
            overwrite: true,
            custom_output_dir: target_path.parent().map(|p| p.to_path_buf()),
            custom_filename: target_path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string()),
        };

        // Convert sequence into PageSpecs
        let page_specs: Vec<PageSpec> = self
            .pages
            .iter()
            .map(|p| PageSpec {
                source_path: p.source_file.clone(),
                page_number: p.original_page_num,
                is_image: p.is_image,
                rotation: p.rotation_degrees,
            })
            .collect();

        match PdfEngine::assemble_document(&page_specs, &output_options) {
            Ok(out) => {
                self.status_message = format!("Exported successfully to '{}'", out.display());
                self.status_is_error = false;
                
                // Show native Windows notification
                #[cfg(windows)]
                {
                    use std::os::windows::process::CommandExt;
                    const CREATE_NO_WINDOW: u32 = 0x08000000;
                    let filename = out.file_name().and_then(|n| n.to_str()).unwrap_or("output.pdf");
                    let ps_script = format!(
                        r#"[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null;
$template = [Windows.UI.Notifications.ToastTemplateType]::ToastText02;
$xml = [Windows.UI.Notifications.ToastNotificationManager]::GetTemplateContent($template);
$textNodes = $xml.GetElementsByTagName('text');
$textNodes.Item(0).AppendChild($xml.CreateTextNode("Morph - Export Complete")) | Out-Null;
$textNodes.Item(1).AppendChild($xml.CreateTextNode("Saved {}")) | Out-Null;
$toast = [Windows.UI.Notifications.ToastNotification]::new($xml);
$notifier = [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('Morph');
$notifier.Show($toast);
"#,
                        filename
                    );
                    let mut cmd = std::process::Command::new("powershell.exe");
                    cmd.creation_flags(CREATE_NO_WINDOW);
                    let _ = cmd.args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", &ps_script]).spawn();
                }
            }
            Err(e) => {
                self.status_message = format!("Export failed: {}", e);
                self.status_is_error = true;
            }
        }
    }

    /// Loads and caches thumbnail textures for the side rail
    fn ensure_thumb_texture(
        textures: &mut HashMap<(usize, i32), egui::TextureHandle>,
        ctx: &egui::Context,
        page: &PageItem,
    ) {
        let key = (page.id, page.rotation_degrees);
        if !textures.contains_key(&key) {
            let path_opt = page.thumbnail_path.as_ref().filter(|p| p.exists())
                .or_else(|| page.highres_path.as_ref().filter(|p| p.exists()))
                .or_else(|| if page.is_image && page.source_file.exists() { Some(&page.source_file) } else { None });

            if let Some(path) = path_opt {
                if let Ok(dyn_img) = image::open(path) {
                    let deg = ((page.rotation_degrees % 360) + 360) % 360;
                    let rotated_img = match deg {
                        90 => dyn_img.rotate90(),
                        180 => dyn_img.rotate180(),
                        270 => dyn_img.rotate270(),
                        _ => dyn_img,
                    };
                    let rgba = rotated_img.to_rgba8();
                    let size = [rgba.width() as usize, rgba.height() as usize];
                    let pixels = rgba.into_raw();
                    let color_img = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
                    let handle = ctx.load_texture(
                        format!("thumb_{}_{}", page.id, deg),
                        color_img,
                        egui::TextureOptions::LINEAR,
                    );
                    textures.insert(key, handle);
                }
            } else {
                // Background thread is still generating thumbnails; schedule repaint to pick up ready frames
                ctx.request_repaint_after(std::time::Duration::from_millis(120));
            }
        }
    }

    /// Loads and caches high-resolution textures for the central document viewer asynchronously
    fn ensure_preview_texture(
        textures: &mut HashMap<(usize, i32), egui::TextureHandle>,
        pending: &std::sync::Arc<std::sync::Mutex<std::collections::HashSet<(u64, u32)>>>,
        ctx: &egui::Context,
        page: &PageItem,
    ) {
        let key = (page.id, page.rotation_degrees);
        if !textures.contains_key(&key) {
            let cache_dir = get_cache_dir();
            let hash = hash_path(&page.source_file);

            if !page.is_image {
                let page_file = cache_dir.join(format!("page_{:016x}_{}.png", hash, page.original_page_num));
                if !page_file.exists() {
                    // Spawn background preview render if not already running
                    let should_spawn = if let Ok(mut set) = pending.lock() {
                        set.insert((hash, page.original_page_num))
                    } else {
                        false
                    };

                    if should_spawn {
                        let bg_path = page.source_file.clone();
                        let bg_cache = cache_dir.clone();
                        let p_num = page.original_page_num;
                        let bg_pending = pending.clone();
                        std::thread::spawn(move || {
                            render_single_page_preview_bg(&bg_path, &bg_cache, hash, p_num);
                            if let Ok(mut set) = bg_pending.lock() {
                                set.remove(&(hash, p_num));
                            }
                        });
                    }

                    // Schedule a repaint so as soon as the background thread finishes, the highres image appears
                    ctx.request_repaint_after(std::time::Duration::from_millis(60));
                }
            }

            if let Some(path) = page.highres_path.as_ref() {
                if path.exists() {
                    if let Ok(dyn_img) = image::open(path) {
                        let deg = ((page.rotation_degrees % 360) + 360) % 360;
                        let rotated_img = match deg {
                            90 => dyn_img.rotate90(),
                            180 => dyn_img.rotate180(),
                            270 => dyn_img.rotate270(),
                            _ => dyn_img,
                        };
                        let rgba = rotated_img.to_rgba8();
                        let size = [rgba.width() as usize, rgba.height() as usize];
                        let pixels = rgba.into_raw();
                        let color_img = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
                        let handle = ctx.load_texture(
                            format!("preview_{}_{}", page.id, deg),
                            color_img,
                            egui::TextureOptions::LINEAR,
                        );
                        textures.insert(key, handle);
                    }
                }
            }
        }
    }

    /// Lazily populates text snippets as background processing writes snip_*.txt
    fn check_snippets_ready(&mut self) {
        let cache_dir = get_cache_dir();
        for page in &mut self.pages {
            if !page.is_image && page.text_snippet.is_none() {
                let hash = hash_path(&page.source_file);
                let snip_path = cache_dir.join(format!("snip_{:016x}_{}.txt", hash, page.original_page_num));
                if snip_path.exists() {
                    if let Ok(txt) = std::fs::read_to_string(&snip_path) {
                        let clean = txt.trim().to_string();
                        if !clean.is_empty() {
                            page.text_snippet = Some(clean);
                        }
                    }
                }
            }
        }
    }
}

impl eframe::App for MorphApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // High-end Windows 11 Dark Design System
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = Color32::from_rgb(14, 16, 22);
        visuals.window_fill = Color32::from_rgb(18, 20, 28);
        visuals.selection.bg_fill = Color32::from_rgb(37, 99, 235);
        visuals.override_text_color = Some(Color32::from_rgb(241, 245, 249));

        // Modern curved buttons across the entire app
        visuals.widgets.inactive.rounding = egui::Rounding::same(6.0);
        visuals.widgets.hovered.rounding = egui::Rounding::same(6.0);
        visuals.widgets.active.rounding = egui::Rounding::same(6.0);
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(28, 33, 46);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(38, 46, 64);
        visuals.widgets.active.bg_fill = Color32::from_rgb(45, 55, 78);
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(45, 52, 70));
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(59, 130, 246));
        ctx.set_visuals(visuals);

        self.check_snippets_ready();

        // Pre-load logo if needed
        if self.logo_texture.is_none() {
            if let Ok(img) = image::load_from_memory(include_bytes!("../../../assets/logo_fluent.png")) {
                let rgba = img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let color_img = egui::ColorImage::from_rgba_unmultiplied(size, &rgba.into_raw());
                self.logo_texture = Some(ctx.load_texture("app_logo", color_img, egui::TextureOptions::LINEAR));
            }
        }

        // Ctrl + Mouse Wheel continuous zoom
        ctx.input(|i| {
            if i.modifiers.ctrl && i.raw_scroll_delta.y.abs() > 0.0 {
                let delta = i.raw_scroll_delta.y * 0.002;
                self.zoom_level = (self.zoom_level + delta).clamp(0.25, 4.0);
            }
        });

        // Mouse Wheel Page Navigation on Home Screen (when not holding Ctrl)
        let (scroll_y, is_ctrl) = ctx.input(|i| (i.raw_scroll_delta.y, i.modifiers.ctrl));
        let pointer_in_central = ctx.input(|i| {
            if let Some(pos) = i.pointer.hover_pos() {
                self.central_panel_rect.contains(pos)
            } else {
                false
            }
        });

        if !is_ctrl && pointer_in_central && scroll_y.abs() > 0.5 && self.zoom_level <= 1.25 {
            self.scroll_accumulator += scroll_y;
            let current_time = ctx.input(|i| i.time);
            // Scrolling DOWN -> Next Page
            if self.scroll_accumulator <= -30.0 && (current_time - self.last_page_scroll_time) > 0.12 {
                if let Some(idx) = self.selected_page_index {
                    if idx + 1 < self.pages.len() {
                        self.selected_page_index = Some(idx + 1);
                        self.status_message = format!("Page {} of {}", idx + 2, self.pages.len());
                        self.status_is_error = false;
                        self.last_page_scroll_time = current_time;
                        self.scroll_accumulator = 0.0;
                    }
                }
            // Scrolling UP -> Previous Page
            } else if self.scroll_accumulator >= 30.0 && (current_time - self.last_page_scroll_time) > 0.12 {
                if let Some(idx) = self.selected_page_index {
                    if idx > 0 {
                        self.selected_page_index = Some(idx - 1);
                        self.status_message = format!("Page {} of {}", idx, self.pages.len());
                        self.status_is_error = false;
                        self.last_page_scroll_time = current_time;
                        self.scroll_accumulator = 0.0;
                    }
                }
            }
        } else if scroll_y.abs() < 0.5 {
            self.scroll_accumulator *= 0.5;
            if self.scroll_accumulator.abs() < 1.0 {
                self.scroll_accumulator = 0.0;
            }
        }

        // Handle keyboard shortcuts
        if ctx.input(|i| i.key_pressed(egui::Key::Delete)) {
            self.delete_selected();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::R)) {
            if let Some(idx) = self.selected_page_index {
                self.rotate_single_page(idx, 90);
            }
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Z)) {
            self.undo();
        }
        if ctx.input(|i| i.modifiers.ctrl && (i.key_pressed(egui::Key::Y) || (i.modifiers.shift && i.key_pressed(egui::Key::Z)))) {
            self.redo();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp) || i.key_pressed(egui::Key::PageUp)) {
            if let Some(idx) = self.selected_page_index {
                if idx > 0 {
                    self.selected_page_index = Some(idx - 1);
                }
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown) || i.key_pressed(egui::Key::PageDown) || i.key_pressed(egui::Key::Space)) {
            if let Some(idx) = self.selected_page_index {
                if idx + 1 < self.pages.len() {
                    self.selected_page_index = Some(idx + 1);
                }
            }
        }

        // Top Navigation Bar
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.add_space(6.0);

            // Row 1: Brand, Undo/Redo, and Export
            ui.horizontal(|ui| {
                if let Some(ref logo) = self.logo_texture {
                    ui.image((logo.id(), Vec2::new(26.0, 26.0)));
                    ui.add_space(4.0);
                }
                ui.heading(RichText::new("Morph").strong().color(Color32::from_rgb(56, 189, 248)));
                ui.label(RichText::new("Editor").strong().color(Color32::from_rgb(251, 146, 60)));

                ui.add_space(16.0);
                ui.separator();
                ui.add_space(12.0);

                // Beautiful, Styled Undo button
                let can_undo = self.history_index > 1;
                let undo_fill = if can_undo {
                    Color32::from_rgb(30, 41, 59)
                } else {
                    Color32::from_rgb(18, 22, 30)
                };
                let undo_stroke = if can_undo {
                    Stroke::new(1.0_f32, Color32::from_rgb(71, 85, 105))
                } else {
                    Stroke::new(1.0_f32, Color32::from_rgb(33, 41, 54))
                };
                let undo_text_color = if can_undo {
                    Color32::from_rgb(241, 245, 249)
                } else {
                    Color32::from_rgb(100, 116, 139)
                };

                let undo_btn = egui::Button::new(
                    RichText::new("Undo")
                        .size(12.0)
                        .strong()
                        .color(undo_text_color),
                )
                .fill(undo_fill)
                .stroke(undo_stroke)
                .rounding(6.0)
                .min_size(Vec2::new(64.0, 28.0));

                let undo_resp = ui.add_enabled(can_undo, undo_btn)
                    .on_hover_text(if can_undo { "Undo last change (Ctrl+Z)" } else { "No actions to undo" });
                if undo_resp.clicked() {
                    self.undo();
                }

                ui.add_space(4.0);

                // Beautiful, Styled Redo button
                let can_redo = self.history_index < self.history.len();
                let redo_fill = if can_redo {
                    Color32::from_rgb(30, 41, 59)
                } else {
                    Color32::from_rgb(18, 22, 30)
                };
                let redo_stroke = if can_redo {
                    Stroke::new(1.0_f32, Color32::from_rgb(71, 85, 105))
                } else {
                    Stroke::new(1.0_f32, Color32::from_rgb(33, 41, 54))
                };
                let redo_text_color = if can_redo {
                    Color32::from_rgb(241, 245, 249)
                } else {
                    Color32::from_rgb(100, 116, 139)
                };

                let redo_btn = egui::Button::new(
                    RichText::new("Redo")
                        .size(12.0)
                        .strong()
                        .color(redo_text_color),
                )
                .fill(redo_fill)
                .stroke(redo_stroke)
                .rounding(6.0)
                .min_size(Vec2::new(64.0, 28.0));

                let redo_resp = ui.add_enabled(can_redo, redo_btn)
                    .on_hover_text(if can_redo { "Redo last undone change (Ctrl+Y)" } else { "No actions to redo" });
                if redo_resp.clicked() {
                    self.redo();
                }

                // If empty document, show Add Files here directly
                if self.pages.is_empty() {
                    ui.add_space(8.0);
                    let add_btn = egui::Button::new(RichText::new("+ Add Files...").strong().size(12.0))
                        .fill(Color32::from_rgb(37, 99, 235))
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(96, 165, 250)))
                        .min_size(Vec2::new(104.0, 28.0));
                    if ui.add(add_btn).clicked() {
                        if let Some(files) = rfd::FileDialog::new()
                            .add_filter("Supported Files", &["pdf", "png", "jpg", "jpeg", "webp", "bmp"])
                            .pick_files()
                        {
                            self.load_files_at_position(files, InsertPosition::End);
                        }
                    }
                }

                // Right aligned Export button
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let export_btn = egui::Button::new(
                        RichText::new("Export PDF")
                            .strong()
                            .color(Color32::WHITE)
                            .size(13.0),
                    )
                    .fill(Color32::from_rgb(37, 99, 235))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(96, 165, 250)))
                    .rounding(6.0)
                    .min_size(Vec2::new(115.0, 32.0));

                    if ui.add_enabled(!self.pages.is_empty(), export_btn).on_hover_text("Save modified document as a new PDF").clicked() {
                        self.export_result();
                    }

                    if !self.pages.is_empty() {
                        let total = self.pages.len();
                        ui.label(
                            RichText::new(format!("{} Page(s) in sequence", total))
                                .size(12.0)
                                .color(Color32::from_rgb(148, 163, 184)),
                        );
                    }
                });
            });

            // Row 2: Added Source Files Strip & Multi-File Insertion Location Controls
            if !self.pages.is_empty() {
                ui.add_space(5.0);
                ui.separator();
                ui.add_space(4.0);

                // Sleek Modern Workspace Shelf Container
                egui::Frame::none()
                    .fill(Color32::from_rgb(15, 19, 28))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(30, 39, 56)))
                    .rounding(8.0)
                    .inner_margin(egui::Margin::symmetric(10.0, 4.0))
                    .show(ui, |ui| {
                        ui.set_height(36.0);
                        let source_files = self.get_source_files();
                        let mut remove_file_target = None;
                        let mut jump_to_file_target = None;

                        let total_width = ui.available_width();
                        let right_controls_width = 430.0_f32.min(total_width * 0.65);
                        let left_area_width = (total_width - right_controls_width - 12.0).max(60.0);

                        ui.horizontal(|ui| {
                            // 1. LEFT SECTION: Files Dropdown + Navigation Chevrons + Scrollable Chips
                            ui.allocate_ui_with_layout(
                                Vec2::new(left_area_width, 36.0),
                                Layout::left_to_right(Align::Center),
                                |ui| {
                                    // Strictly clip this area so file chips can NEVER overflow onto right controls
                                    let mut clip_rect = ui.max_rect();
                                    clip_rect.max.x = clip_rect.max.x.min(ui.min_rect().min.x + left_area_width);
                                    ui.set_clip_rect(clip_rect);

                                    // Interactive Dropdown Menu Button for ALL files
                                    ui.menu_button(
                                        RichText::new(format!("📁 Files ({}) ▾", source_files.len()))
                                            .size(11.0)
                                            .strong()
                                            .color(Color32::from_rgb(203, 213, 225)),
                                        |ui| {
                                            ui.set_min_width(280.0);
                                            ui.add_space(3.0);
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    RichText::new("📂 Loaded Documents")
                                                        .size(11.5)
                                                        .strong()
                                                        .color(Color32::WHITE),
                                                );
                                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                    let total_pages = source_files.iter().map(|(_, _, c)| *c).sum::<usize>();
                                                    ui.label(
                                                        RichText::new(format!("{} total pages", total_pages))
                                                            .size(10.0)
                                                            .color(Color32::from_rgb(148, 163, 184)),
                                                    );
                                                });
                                            });
                                            ui.add_space(2.0);
                                            ui.separator();
                                            ui.add_space(4.0);

                                            egui::ScrollArea::vertical()
                                                .max_height(250.0)
                                                .show(ui, |ui| {
                                                    for (path, is_image, count) in &source_files {
                                                        let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
                                                        let (tag_bg, tag_txt, tag_label) = if *is_image {
                                                            (Color32::from_rgb(14, 40, 30), Color32::from_rgb(52, 211, 153), "IMG")
                                                        } else {
                                                            (Color32::from_rgb(48, 18, 24), Color32::from_rgb(251, 113, 133), "PDF")
                                                        };

                                                        ui.horizontal(|ui| {
                                                            egui::Frame::none()
                                                                .fill(tag_bg)
                                                                .rounding(3.0)
                                                                .inner_margin(egui::Margin::symmetric(4.0, 1.5))
                                                                .show(ui, |ui| {
                                                                    ui.label(RichText::new(tag_label).size(8.5).strong().color(tag_txt));
                                                                });

                                                            ui.add_space(4.0);

                                                            let short_name = if fname.chars().count() > 24 {
                                                                let prefix: String = fname.chars().take(18).collect();
                                                                let ext = path.extension().and_then(|e| e.to_str()).map(|e| format!(".{}", e)).unwrap_or_default();
                                                                format!("{}...{}", prefix, ext)
                                                            } else {
                                                                fname.to_string()
                                                            };

                                                            if ui.button(RichText::new(short_name).size(11.0).color(Color32::from_rgb(226, 232, 240)))
                                                                .on_hover_text(format!("{}\nClick to jump to page 1 of this document", path.display()))
                                                                .clicked()
                                                            {
                                                                jump_to_file_target = Some(path.clone());
                                                                ui.close_menu();
                                                            }

                                                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                                let del_btn = egui::Button::new(
                                                                    RichText::new("✕")
                                                                        .size(9.0)
                                                                        .strong()
                                                                        .color(Color32::from_rgb(239, 68, 68)),
                                                                )
                                                                .fill(Color32::from_rgb(30, 38, 54))
                                                                .stroke(Stroke::NONE)
                                                                .rounding(6.0)
                                                                .min_size(Vec2::new(16.0, 16.0));

                                                                if ui.add(del_btn).on_hover_text(format!("Remove all pages from '{}'", fname)).clicked() {
                                                                    remove_file_target = Some(path.clone());
                                                                    ui.close_menu();
                                                                }

                                                                ui.add_space(4.0);

                                                                let page_str = if *is_image { "1 pg".to_string() } else { format!("{} pgs", count) };
                                                                ui.label(RichText::new(page_str).size(9.5).color(Color32::from_rgb(148, 163, 184)));
                                                            });
                                                        });
                                                        ui.add_space(2.0);
                                                    }
                                                });
                                        },
                                    );

                                    // Chevron navigation buttons when multiple files are loaded
                                    if source_files.len() > 1 {
                                        ui.add_space(3.0);
                                        let left_chevron = egui::Button::new(
                                            RichText::new("‹")
                                                .size(13.0)
                                                .strong()
                                                .color(Color32::from_rgb(203, 213, 225)),
                                        )
                                        .fill(Color32::from_rgb(24, 30, 44))
                                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(45, 55, 78)))
                                        .rounding(4.0)
                                        .min_size(Vec2::new(18.0, 22.0));

                                        if ui.add(left_chevron).on_hover_text("Scroll files left (or roll mouse wheel)").clicked() {
                                            self.shelf_scroll_offset = (self.shelf_scroll_offset - 160.0).max(0.0);
                                        }

                                        let right_chevron = egui::Button::new(
                                            RichText::new("›")
                                                .size(13.0)
                                                .strong()
                                                .color(Color32::from_rgb(203, 213, 225)),
                                        )
                                        .fill(Color32::from_rgb(24, 30, 44))
                                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(45, 55, 78)))
                                        .rounding(4.0)
                                        .min_size(Vec2::new(18.0, 22.0));

                                        if ui.add(right_chevron).on_hover_text("Scroll files right (or roll mouse wheel)").clicked() {
                                            self.shelf_scroll_offset += 160.0;
                                        }
                                    }

                                    ui.add_space(4.0);

                                    let chip_scroll_w = (ui.available_width() - 4.0).max(20.0);

                                    // Smooth horizontal scroll area for chips
                                    let scroll_out = egui::ScrollArea::horizontal()
                                        .auto_shrink([false, true])
                                        .max_width(chip_scroll_w)
                                        .max_height(32.0)
                                        .drag_to_scroll(true)
                                        .horizontal_scroll_offset(self.shelf_scroll_offset)
                                        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                for (path, is_image, count) in &source_files {
                                                    let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");

                                                    let display_name = if fname.chars().count() > 18 {
                                                        let prefix: String = fname.chars().take(13).collect();
                                                        let ext = path.extension().and_then(|e| e.to_str()).map(|e| format!(".{}", e)).unwrap_or_default();
                                                        format!("{}...{}", prefix, ext)
                                                    } else {
                                                        fname.to_string()
                                                    };

                                                    let page_text = if *is_image {
                                                        "IMG".to_string()
                                                    } else if *count == 1 {
                                                        "1 pg".to_string()
                                                    } else {
                                                        format!("{} pgs", count)
                                                    };

                                                    let chip_frame = egui::Frame::none()
                                                        .fill(Color32::from_rgb(22, 28, 40))
                                                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(40, 50, 70)))
                                                        .rounding(6.0)
                                                        .inner_margin(egui::Margin::symmetric(8.0, 3.5));

                                                    chip_frame.show(ui, |ui| {
                                                        ui.horizontal(|ui| {
                                                            // Format tag (PDF / IMG)
                                                            let (tag_bg, tag_txt, tag_label) = if *is_image {
                                                                (Color32::from_rgb(14, 40, 30), Color32::from_rgb(52, 211, 153), "IMG")
                                                            } else {
                                                                (Color32::from_rgb(48, 18, 24), Color32::from_rgb(251, 113, 133), "PDF")
                                                            };

                                                            egui::Frame::none()
                                                                .fill(tag_bg)
                                                                .rounding(3.0)
                                                                .inner_margin(egui::Margin::symmetric(4.0, 1.5))
                                                                .show(ui, |ui| {
                                                                    ui.label(RichText::new(tag_label).size(8.5).strong().color(tag_txt));
                                                                });

                                                            ui.add_space(4.0);

                                                            // Clickable document title
                                                            let lbl = ui.add(
                                                                egui::Label::new(
                                                                    RichText::new(&display_name)
                                                                        .size(11.0)
                                                                        .strong()
                                                                        .color(Color32::from_rgb(226, 232, 240)),
                                                                )
                                                                .sense(Sense::click()),
                                                            ).on_hover_text(format!("{}\nPages: {}\nClick to jump to this document", path.display(), count));

                                                            if lbl.clicked() {
                                                                jump_to_file_target = Some(path.clone());
                                                            }

                                                            ui.add_space(3.0);

                                                            // Page count capsule pill
                                                            egui::Frame::none()
                                                                .fill(Color32::from_rgb(13, 17, 25))
                                                                .rounding(4.0)
                                                                .inner_margin(egui::Margin::symmetric(5.0, 1.5))
                                                                .show(ui, |ui| {
                                                                    ui.label(
                                                                        RichText::new(page_text)
                                                                            .size(9.5)
                                                                            .color(Color32::from_rgb(148, 163, 184)),
                                                                    );
                                                                });

                                                            ui.add_space(3.0);

                                                            // Elegant subtle Delete Button
                                                            let del_btn = egui::Button::new(
                                                                RichText::new("✕")
                                                                    .size(9.0)
                                                                    .strong()
                                                                    .color(Color32::from_rgb(148, 163, 184)),
                                                            )
                                                            .fill(Color32::from_rgb(30, 38, 54))
                                                            .stroke(Stroke::NONE)
                                                            .rounding(8.0)
                                                            .min_size(Vec2::new(16.0, 16.0));

                                                            if ui.add(del_btn).on_hover_text(format!("Remove all {} pages of '{}'", count, fname)).clicked() {
                                                                remove_file_target = Some(path.clone());
                                                            }
                                                        });
                                                    });
                                                    ui.add_space(5.0);
                                                }
                                            });
                                        });

                                    // Enable mouse wheel scrolling when hovering over the chips strip
                                    let strip_hovered = scroll_out.inner_rect.contains(ui.input(|i| i.pointer.hover_pos().unwrap_or(egui::Pos2::ZERO)));
                                    let wheel_y = ui.input(|i| i.raw_scroll_delta.y);
                                    if strip_hovered && wheel_y.abs() > 0.1 {
                                        self.shelf_scroll_offset = (self.shelf_scroll_offset - wheel_y * 1.5).max(0.0);
                                    }
                                    let max_scroll = (scroll_out.content_size.x - chip_scroll_w).max(0.0);
                                    self.shelf_scroll_offset = self.shelf_scroll_offset.min(max_scroll);
                                },
                            );

                            // Subtle vertical separator between file chips and right dock
                            ui.add_space(3.0);
                            ui.add(egui::Separator::default().vertical());
                            ui.add_space(3.0);

                            // 2. RIGHT SECTION: Pinned controls (Add button, Segmented Capsule Dock, Label)
                            ui.allocate_ui_with_layout(
                                Vec2::new(right_controls_width - 8.0, 36.0),
                                Layout::right_to_left(Align::Center),
                                |ui| {
                                    let pos_desc = match self.insert_position {
                                        InsertPosition::Start => "Insert at beginning (before Page 1)",
                                        InsertPosition::End => "Append to end of document",
                                        InsertPosition::AfterSelected => "Insert in middle (after current page)",
                                    };

                                    let add_btn = egui::Button::new(
                                        RichText::new("➕ Add Files...")
                                            .strong()
                                            .size(11.5)
                                            .color(Color32::WHITE),
                                    )
                                    .fill(Color32::from_rgb(37, 99, 235))
                                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(96, 165, 250)))
                                    .rounding(6.0)
                                    .min_size(Vec2::new(98.0, 26.0));

                                    if ui.add(add_btn).on_hover_text(format!("Pick PDF or images — will {}", pos_desc.to_lowercase())).clicked() {
                                        if let Some(files) = rfd::FileDialog::new()
                                            .add_filter("Supported Files", &["pdf", "png", "jpg", "jpeg", "webp", "bmp"])
                                            .pick_files()
                                        {
                                            self.load_files_at_position(files, self.insert_position);
                                        }
                                    }

                                    ui.add_space(6.0);

                                    // Integrated Segmented Capsule Dock
                                    egui::Frame::none()
                                        .fill(Color32::from_rgb(11, 14, 20))
                                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(45, 55, 78)))
                                        .rounding(6.0)
                                        .inner_margin(2.0)
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.spacing_mut().item_spacing = Vec2::ZERO;

                                                // [ At End ]
                                                let is_end = self.insert_position == InsertPosition::End;
                                                let end_btn = egui::Button::new(
                                                    RichText::new("⏭ At End")
                                                        .size(10.5)
                                                        .strong()
                                                        .color(if is_end { Color32::WHITE } else { Color32::from_rgb(148, 163, 184) }),
                                                )
                                                .fill(if is_end { Color32::from_rgb(37, 99, 235) } else { Color32::TRANSPARENT })
                                                .stroke(if is_end { Stroke::new(1.0_f32, Color32::from_rgb(96, 165, 250)) } else { Stroke::NONE })
                                                .rounding(5.0)
                                                .min_size(Vec2::new(64.0, 22.0));
                                                if ui.add(end_btn).on_hover_text("New files will be appended to the end of document").clicked() {
                                                    self.insert_position = InsertPosition::End;
                                                }

                                                // [ Middle (p.X) ]
                                                let cur_page_num = self.selected_page_index.map(|i| i + 1).unwrap_or(1);
                                                let middle_label = format!("⏺ Middle (p.{})", cur_page_num);
                                                let is_mid = self.insert_position == InsertPosition::AfterSelected;
                                                let mid_btn = egui::Button::new(
                                                    RichText::new(middle_label)
                                                        .size(10.5)
                                                        .strong()
                                                        .color(if is_mid { Color32::WHITE } else { Color32::from_rgb(148, 163, 184) }),
                                                )
                                                .fill(if is_mid { Color32::from_rgb(37, 99, 235) } else { Color32::TRANSPARENT })
                                                .stroke(if is_mid { Stroke::new(1.0_f32, Color32::from_rgb(96, 165, 250)) } else { Stroke::NONE })
                                                .rounding(5.0)
                                                .min_size(Vec2::new(84.0, 22.0));
                                                if ui.add(mid_btn).on_hover_text(format!("New files will be inserted right after Page {}", cur_page_num)).clicked() {
                                                    self.insert_position = InsertPosition::AfterSelected;
                                                }

                                                // [ At Start ]
                                                let is_start = self.insert_position == InsertPosition::Start;
                                                let start_btn = egui::Button::new(
                                                    RichText::new("⏮ At Start")
                                                        .size(10.5)
                                                        .strong()
                                                        .color(if is_start { Color32::WHITE } else { Color32::from_rgb(148, 163, 184) }),
                                                )
                                                .fill(if is_start { Color32::from_rgb(37, 99, 235) } else { Color32::TRANSPARENT })
                                                .stroke(if is_start { Stroke::new(1.0_f32, Color32::from_rgb(96, 165, 250)) } else { Stroke::NONE })
                                                .rounding(5.0)
                                                .min_size(Vec2::new(68.0, 22.0));
                                                if ui.add(start_btn).on_hover_text("New files will be inserted at the beginning, before Page 1").clicked() {
                                                    self.insert_position = InsertPosition::Start;
                                                }
                                            });
                                        });

                                    ui.add_space(6.0);
                                    ui.label(
                                        RichText::new("Apply New Files:")
                                            .size(11.0)
                                            .color(Color32::from_rgb(148, 163, 184)),
                                    );
                                },
                            );
                        });

                        if let Some(target) = remove_file_target {
                            self.remove_source_file(&target);
                        }
                        if let Some(target) = jump_to_file_target {
                            if let Some(pos) = self.pages.iter().position(|p| p.source_file == target) {
                                self.selected_page_index = Some(pos);
                            }
                        }
                    });
            }

            ui.add_space(4.0);
        });

        // Bottom Status Bar
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let dot_color = if self.status_is_error {
                    Color32::from_rgb(239, 68, 68)
                } else {
                    Color32::from_rgb(34, 197, 94)
                };
                ui.label(RichText::new("●").color(dot_color).size(12.0));
                ui.label(
                    RichText::new(&self.status_message)
                        .size(12.0)
                        .color(if self.status_is_error {
                            Color32::from_rgb(248, 113, 113)
                        } else {
                            Color32::from_rgb(203, 213, 225)
                        }),
                );

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new("Shortcuts: [R] Rotate Page • [Del] Delete • [Ctrl+Scroll] Zoom • [Ctrl+Z] Undo • [Ctrl+Y] Redo")
                            .size(11.0)
                            .weak(),
                    );
                });
            });
            ui.add_space(4.0);
        });

        if self.pages.is_empty() {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(140.0);
                    if let Some(ref logo) = self.logo_texture {
                        ui.image((logo.id(), Vec2::new(80.0, 80.0)));
                        ui.add_space(16.0);
                    }
                    ui.heading(RichText::new("No Document or Pages Loaded").strong().size(22.0));
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("Click '+ Add Files...' or right-click any PDF or Image in File Explorer to begin.")
                            .size(14.0)
                            .color(Color32::from_rgb(148, 163, 184)),
                    );
                    ui.add_space(20.0);
                    let add_empty_btn = egui::Button::new(RichText::new("+ Add Files to Edit").strong().size(14.0))
                        .fill(Color32::from_rgb(37, 99, 235))
                        .rounding(6.0)
                        .min_size(Vec2::new(180.0, 38.0));
                    if ui.add(add_empty_btn).clicked() {
                        if let Some(files) = rfd::FileDialog::new()
                            .add_filter("Supported Files", &["pdf", "png", "jpg", "jpeg", "webp", "bmp"])
                            .pick_files()
                        {
                            for f in files {
                                self.load_file(f);
                            }
                            self.save_history();
                        }
                    }
                });
            });
            return;
        }

        // Left Side Panel: Resizable Vertical Page Sequence Rail (default 220px to utilize more space)
        egui::SidePanel::left("vertical_page_rail")
            .default_width(220.0)
            .width_range(160.0..=360.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("PAGE SEQUENCE").strong().size(11.0).color(Color32::from_rgb(148, 163, 184)));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new(format!("{} Total", self.pages.len())).strong().color(Color32::from_rgb(56, 189, 248)));
                    });
                });
                ui.separator();
                ui.add_space(4.0);

                egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
                    // Top quick-insert button in side rail
                    let add_start_btn = egui::Button::new(RichText::new("+ Insert at Start").size(10.5).color(Color32::from_rgb(148, 163, 184)))
                        .fill(Color32::from_rgb(20, 24, 34))
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(45, 52, 70)))
                        .rounding(4.0)
                        .min_size(Vec2::new(ui.available_width(), 22.0));
                    if ui.add(add_start_btn).on_hover_text("Insert new PDF/images at the very beginning (before Page 1)").clicked() {
                        if let Some(files) = rfd::FileDialog::new()
                            .add_filter("Supported Files", &["pdf", "png", "jpg", "jpeg", "webp", "bmp"])
                            .pick_files()
                        {
                            self.load_files_at(files, 0);
                        }
                    }
                    ui.add_space(4.0);

                    let mut newly_selected = None;
                    let mut action_rotate = None;
                    let mut action_up = None;
                    let mut action_down = None;
                    let mut action_delete = None;
                    let mut action_insert_after: Option<(usize, Vec<PathBuf>)> = None;

                    for (idx, page) in self.pages.iter().enumerate() {
                        let is_selected = self.selected_page_index == Some(idx);

                        let card_bg = if is_selected {
                            Color32::from_rgb(30, 41, 59)
                        } else {
                            Color32::from_rgb(20, 24, 34)
                        };

                        let border_stroke = if is_selected {
                            Stroke::new(2.0_f32, Color32::from_rgb(59, 130, 246))
                        } else {
                            Stroke::new(1.0_f32, Color32::from_rgb(38, 45, 62))
                        };

                        egui::Frame::none()
                            .fill(card_bg)
                            .stroke(border_stroke)
                            .rounding(8.0)
                            .inner_margin(6.0)
                            .show(ui, |ui| {
                                ui.vertical(|ui| {
                                    // Card Header: Page badge, format badge, rotation pill, styled Delete button
                                    ui.horizontal(|ui| {
                                        let pill_color = if is_selected {
                                            Color32::from_rgb(37, 99, 235)
                                        } else {
                                            Color32::from_rgb(51, 65, 85)
                                        };
                                        let badge_resp = egui::Frame::none()
                                            .fill(pill_color)
                                            .rounding(4.0)
                                            .inner_margin(egui::Margin::symmetric(6.0, 2.0))
                                            .show(ui, |ui| {
                                                ui.label(RichText::new(format!("#{}", idx + 1)).strong().size(11.0).color(Color32::WHITE));
                                            });
                                        if badge_resp.response.interact(Sense::click()).clicked() {
                                            newly_selected = Some(idx);
                                        }

                                        if page.is_image {
                                            ui.label(RichText::new("IMG").size(10.0).weak());
                                        } else {
                                            ui.label(RichText::new(format!("p.{}", page.original_page_num)).size(10.0).weak());
                                        }

                                        if page.rotation_degrees != 0 {
                                            egui::Frame::none()
                                                .fill(Color32::from_rgb(180, 83, 9))
                                                .rounding(3.0)
                                                .inner_margin(egui::Margin::symmetric(4.0, 1.0))
                                                .show(ui, |ui| {
                                                    ui.label(RichText::new(format!("{}°", page.rotation_degrees)).size(9.0).strong().color(Color32::WHITE));
                                                });
                                        }

                                        // Beautiful, Styled Delete button with clear hit-target on top-right
                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                            let del_btn = egui::Button::new(
                                                RichText::new("Del")
                                                    .strong()
                                                    .size(10.5)
                                                    .color(Color32::from_rgb(254, 202, 202)),
                                            )
                                            .fill(Color32::from_rgb(127, 29, 29))
                                            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(220, 38, 38)))
                                            .rounding(4.0)
                                            .min_size(Vec2::new(32.0, 20.0));

                                            if ui.add(del_btn).on_hover_text("Delete this page").clicked() {
                                                action_delete = Some(idx);
                                            }
                                        });
                                    });

                                    ui.add_space(4.0);

                                    // Thumbnail Preview Area: Clicking the preview selects the page
                                    let aspect_height = 125.0;
                                    let key = (page.id, page.rotation_degrees);
                                    let base_key = (page.id, 0);

                                    Self::ensure_thumb_texture(&mut self.thumb_textures, ctx, page);

                                    let deg = ((page.rotation_degrees % 360) + 360) % 360;
                                    let rad = (deg as f32).to_radians();

                                    // Multi-tier texture resolver:
                                    // 1. Dedicated rotated thumbnail texture
                                    // 2. Dedicated rotated high-res preview texture
                                    // 3. Base unrotated thumbnail (with zero-latency GPU vertex rotation)
                                    // 4. Base unrotated high-res preview (with zero-latency GPU vertex rotation)
                                    let tex_opt = self.thumb_textures.get(&key)
                                        .or_else(|| self.preview_textures.get(&key))
                                        .map(|t| (t, 0.0))
                                        .or_else(|| {
                                            self.thumb_textures.get(&base_key)
                                                .or_else(|| self.preview_textures.get(&base_key))
                                                .map(|t| (t, rad))
                                        });

                                    if let Some((tex, extra_rad)) = tex_opt {
                                        let mut img_widget = egui::Image::new(tex)
                                            .max_height(aspect_height)
                                            .rounding(4.0)
                                            .sense(Sense::click());

                                        if extra_rad.abs() > 0.001 {
                                            img_widget = img_widget.rotate(extra_rad, egui::Vec2::splat(0.5));
                                        }

                                        let resp = ui.vertical_centered(|ui| {
                                            ui.add(img_widget)
                                        });
                                        if resp.inner.clicked() {
                                            newly_selected = Some(idx);
                                        }
                                    } else if let Some(ref text) = page.text_snippet {
                                        // Miniature Paper Sheet with real extracted text
                                        let resp = egui::Frame::none()
                                            .fill(Color32::from_rgb(248, 250, 252))
                                            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(203, 213, 225)))
                                            .rounding(3.0)
                                            .inner_margin(6.0)
                                            .show(ui, |ui| {
                                                ui.set_height(aspect_height);
                                                ui.set_width(ui.available_width());
                                                ui.vertical(|ui| {
                                                    ui.label(
                                                        RichText::new("DOCUMENT CONTENT")
                                                            .size(8.0)
                                                            .strong()
                                                            .color(Color32::from_rgb(71, 85, 105)),
                                                    );
                                                    ui.separator();
                                                    ui.label(
                                                        RichText::new(text)
                                                            .size(7.5)
                                                            .color(Color32::from_rgb(30, 41, 59)),
                                                    );
                                                });
                                            });
                                        if resp.response.interact(Sense::click()).clicked() {
                                            newly_selected = Some(idx);
                                        }
                                    } else {
                                        let resp = egui::Frame::none()
                                            .fill(Color32::from_rgb(30, 35, 48))
                                            .rounding(4.0)
                                            .inner_margin(6.0)
                                            .show(ui, |ui| {
                                                ui.set_height(aspect_height);
                                                ui.set_width(ui.available_width());
                                                ui.vertical_centered(|ui| {
                                                    ui.add_space(35.0);
                                                    ui.label(RichText::new("Page").size(14.0));
                                                    ui.label(RichText::new(format!("Page {}", page.original_page_num)).size(11.0).weak());
                                                });
                                            });
                                        if resp.response.interact(Sense::click()).clicked() {
                                            newly_selected = Some(idx);
                                        }
                                    }

                                    ui.add_space(3.0);

                                    // Filename subtitle
                                    let filename = page.source_file.file_name().and_then(|n| n.to_str()).unwrap_or("document");
                                    ui.label(RichText::new(filename).size(10.0).weak());

                                    ui.add_space(2.0);

                                    // Action Buttons Row: Rotate Page + Move Up/Down + Insert Here
                                    ui.horizontal(|ui| {
                                        let rot_btn = egui::Button::new(
                                            RichText::new("90°")
                                                .size(11.0)
                                                .strong()
                                                .color(Color32::from_rgb(56, 189, 248)),
                                        )
                                        .fill(Color32::from_rgb(24, 32, 47))
                                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(3, 105, 161)))
                                        .rounding(4.0)
                                        .min_size(Vec2::new(44.0, 24.0));
                                        if ui.add(rot_btn).on_hover_text("Rotate this page 90° clockwise").clicked() {
                                            action_rotate = Some(idx);
                                        }

                                        let up_btn = egui::Button::new(
                                            RichText::new("Up")
                                                .size(11.0)
                                                .color(if idx > 0 { Color32::from_rgb(226, 232, 240) } else { Color32::from_rgb(71, 85, 105) }),
                                        )
                                        .fill(Color32::from_rgb(24, 28, 38))
                                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(45, 52, 70)))
                                        .rounding(4.0)
                                        .min_size(Vec2::new(36.0, 24.0));
                                        if ui.add_enabled(idx > 0, up_btn).on_hover_text("Move page up in sequence").clicked() {
                                            action_up = Some(idx);
                                        }

                                        let down_btn = egui::Button::new(
                                            RichText::new("Down")
                                                .size(11.0)
                                                .color(if idx + 1 < self.pages.len() { Color32::from_rgb(226, 232, 240) } else { Color32::from_rgb(71, 85, 105) }),
                                        )
                                        .fill(Color32::from_rgb(24, 28, 38))
                                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(45, 52, 70)))
                                        .rounding(4.0)
                                        .min_size(Vec2::new(42.0, 24.0));
                                        if ui.add_enabled(idx + 1 < self.pages.len(), down_btn).on_hover_text("Move page down in sequence").clicked() {
                                            action_down = Some(idx);
                                        }

                                        if is_selected {
                                            let ins_btn = egui::Button::new(RichText::new("+ Insert").size(10.5).color(Color32::from_rgb(56, 189, 248)))
                                                .fill(Color32::from_rgb(20, 32, 48))
                                                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(37, 99, 235)))
                                                .rounding(4.0)
                                                .min_size(Vec2::new(50.0, 24.0));
                                            if ui.add(ins_btn).on_hover_text(format!("Insert new files right after Page {}", idx + 1)).clicked() {
                                                if let Some(files) = rfd::FileDialog::new()
                                                    .add_filter("Supported Files", &["pdf", "png", "jpg", "jpeg", "webp", "bmp"])
                                                    .pick_files()
                                                {
                                                    action_insert_after = Some((idx + 1, files));
                                                }
                                            }
                                        }
                                    });
                                });
                            });

                        ui.add_space(6.0);
                    }

                    // Bottom quick-insert button in side rail
                    ui.add_space(4.0);
                    let add_end_btn = egui::Button::new(RichText::new("+ Append at End").size(10.5).color(Color32::from_rgb(148, 163, 184)))
                        .fill(Color32::from_rgb(20, 24, 34))
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(45, 52, 70)))
                        .rounding(4.0)
                        .min_size(Vec2::new(ui.available_width(), 24.0));
                    if ui.add(add_end_btn).on_hover_text("Add new PDF/images to the very end of document").clicked() {
                        if let Some(files) = rfd::FileDialog::new()
                            .add_filter("Supported Files", &["pdf", "png", "jpg", "jpeg", "webp", "bmp"])
                            .pick_files()
                        {
                            let end_pos = self.pages.len();
                            self.load_files_at(files, end_pos);
                        }
                    }
                    ui.add_space(8.0);

                    // Execute actions cleanly with zero interference
                    if let Some((ins_idx, files)) = action_insert_after {
                        self.load_files_at(files, ins_idx);
                    } else if let Some(idx) = action_delete {
                        self.delete_page_at(idx);
                    } else {
                        if let Some(sel) = newly_selected {
                            self.selected_page_index = Some(sel);
                        }
                        if let Some(idx) = action_rotate {
                            self.rotate_single_page(idx, 90);
                        }
                        if let Some(idx) = action_up {
                            self.move_page_up_at(idx);
                        }
                        if let Some(idx) = action_down {
                            self.move_page_down_at(idx);
                        }
                    }
                });
            });

        // Main Detail Inspection Panel: Expansive High-Resolution Document Canvas
        egui::CentralPanel::default().show(ctx, |ui| {
            self.central_panel_rect = ui.max_rect();

            if let Some(idx) = self.selected_page_index {
                if idx < self.pages.len() {
                    let page = self.pages[idx].clone();

                    ui.vertical(|ui| {
                        // Inspector Top Header with Page Navigation
                        ui.horizontal(|ui| {
                            ui.heading(
                                RichText::new(format!("Page {} of {}", idx + 1, self.pages.len()))
                                    .strong()
                                    .size(16.0)
                                    .color(Color32::from_rgb(56, 189, 248)),
                            );
                            ui.add_space(8.0);

                            // Page Navigation buttons (Prev / Next)
                            let prev_btn = egui::Button::new(RichText::new("◀ Prev").size(11.0))
                                .fill(Color32::from_rgb(30, 41, 59))
                                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(51, 65, 85)))
                                .rounding(4.0)
                                .min_size(Vec2::new(54.0, 22.0));
                            if ui.add_enabled(idx > 0, prev_btn).on_hover_text("Previous page (Up arrow / Scroll up)").clicked() {
                                self.selected_page_index = Some(idx - 1);
                            }

                            let next_btn = egui::Button::new(RichText::new("Next ▶").size(11.0))
                                .fill(Color32::from_rgb(30, 41, 59))
                                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(51, 65, 85)))
                                .rounding(4.0)
                                .min_size(Vec2::new(54.0, 22.0));
                            if ui.add_enabled(idx + 1 < self.pages.len(), next_btn).on_hover_text("Next page (Down arrow / Scroll down)").clicked() {
                                self.selected_page_index = Some(idx + 1);
                            }

                            ui.add_space(8.0);
                            let src_name = page.source_file.file_name().and_then(|n| n.to_str()).unwrap_or("document");
                            ui.label(RichText::new(format!("Source: {}", src_name)).size(12.0).weak());

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if page.rotation_degrees != 0 {
                                    ui.label(RichText::new(format!("Orientation: Rotated {}°", page.rotation_degrees)).strong().color(Color32::from_rgb(251, 146, 60)));
                                } else {
                                    ui.label(RichText::new("Orientation: Normal (0°)").size(12.0).weak());
                                }
                            });
                        });
                        ui.separator();
                        ui.add_space(4.0);

                        // Unified Modern Action Bar: Rotation, Delete & Full Zoom Controls
                        ui.horizontal(|ui| {
                            // Rotate Entire PDF button (rotates all pages at once)
                            let rot_all_btn = egui::Button::new(
                                RichText::new("Rotate Entire PDF (90°)")
                                    .strong()
                                    .color(Color32::from_rgb(254, 215, 170))
                                    .size(12.0),
                            )
                            .fill(Color32::from_rgb(120, 53, 15))
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(180, 83, 9)))
                            .rounding(6.0)
                            .min_size(Vec2::new(150.0, 28.0));
                            if ui.add(rot_all_btn).on_hover_text("Rotate all pages in document clockwise by 90°").clicked() {
                                self.rotate_all_pages(90);
                            }

                            ui.add_space(4.0);

                            // Rotate 180° button
                            let rot_180_btn = egui::Button::new(
                                RichText::new("Rotate 180°")
                                    .strong()
                                    .color(Color32::from_rgb(186, 230, 253))
                                    .size(12.0),
                            )
                            .fill(Color32::from_rgb(12, 74, 110))
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(3, 105, 161)))
                            .rounding(6.0)
                            .min_size(Vec2::new(100.0, 28.0));
                            if ui.add(rot_180_btn).on_hover_text("Flip only this page upside down (180°)").clicked() {
                                self.rotate_single_page(idx, 180);
                            }

                            ui.add_space(4.0);

                            // Styled Delete Button
                            let del_btn = egui::Button::new(
                                RichText::new("Delete Page")
                                    .strong()
                                    .color(Color32::from_rgb(254, 202, 202))
                                    .size(12.0),
                            )
                            .fill(Color32::from_rgb(127, 29, 29))
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(220, 38, 38)))
                            .rounding(6.0)
                            .min_size(Vec2::new(105.0, 28.0));
                            if ui.add(del_btn).on_hover_text("Delete currently selected page (Delete key)").clicked() {
                                self.delete_selected();
                            }

                            ui.add_space(12.0);
                            ui.separator();
                            ui.add_space(8.0);

                            // Zoom Controls Toolbar
                            ui.label(RichText::new("Zoom:").size(12.0).weak());

                            let zoom_out_btn = egui::Button::new(RichText::new("-").strong().size(12.0))
                                .fill(Color32::from_rgb(30, 41, 59))
                                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(51, 65, 85)))
                                .rounding(4.0)
                                .min_size(Vec2::new(26.0, 26.0));
                            if ui.add(zoom_out_btn).on_hover_text("Zoom Out (Ctrl -)").clicked() {
                                self.zoom_level = (self.zoom_level - 0.25).max(0.25);
                            }

                            // Zoom Level pill (clicking resets to 100%)
                            let zoom_text = format!("{:.0}%", self.zoom_level * 100.0);
                            let zoom_pill = egui::Button::new(
                                RichText::new(zoom_text)
                                    .strong()
                                    .size(11.0)
                                    .color(Color32::from_rgb(56, 189, 248)),
                            )
                            .fill(Color32::from_rgb(24, 32, 47))
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(3, 105, 161)))
                            .rounding(4.0)
                            .min_size(Vec2::new(48.0, 26.0));
                            if ui.add(zoom_pill).on_hover_text("Click to reset zoom to 100%").clicked() {
                                self.zoom_level = 1.0;
                            }

                            let zoom_in_btn = egui::Button::new(RichText::new("+").strong().size(12.0))
                                .fill(Color32::from_rgb(30, 41, 59))
                                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(51, 65, 85)))
                                .rounding(4.0)
                                .min_size(Vec2::new(26.0, 26.0));
                            if ui.add(zoom_in_btn).on_hover_text("Zoom In (Ctrl +)").clicked() {
                                self.zoom_level = (self.zoom_level + 0.25).min(4.0);
                            }

                            ui.add_space(4.0);

                            let fit_page_btn = egui::Button::new(RichText::new("Fit Page").size(11.0))
                                .fill(Color32::from_rgb(30, 41, 59))
                                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(51, 65, 85)))
                                .rounding(4.0)
                                .min_size(Vec2::new(58.0, 26.0));
                            if ui.add(fit_page_btn).on_hover_text("Fit page vertically to window").clicked() {
                                self.zoom_level = 1.0;
                            }

                            let fit_width_btn = egui::Button::new(RichText::new("Fit Width").size(11.0))
                                .fill(Color32::from_rgb(30, 41, 59))
                                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(51, 65, 85)))
                                .rounding(4.0)
                                .min_size(Vec2::new(60.0, 26.0));
                            if ui.add(fit_width_btn).on_hover_text("Fit page horizontally to window width").clicked() {
                                let avail_w = (ui.available_width() - 40.0).max(300.0);
                                let base_w = 480.0;
                                self.zoom_level = (avail_w / base_w).clamp(0.5, 3.5);
                            }
                        });

                        ui.add_space(6.0);

                        // Centered High-Resolution Document Canvas with Smooth Dual-Axis Scrolling & Full Zoom
                        egui::ScrollArea::both()
                            .vscroll(self.zoom_level > 1.25)
                            .hscroll(self.zoom_level > 1.25)
                            .auto_shrink([false; 2])
                            .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                egui::Frame::none()
                                    .fill(Color32::from_rgb(18, 20, 28))
                                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(38, 44, 58)))
                                    .rounding(6.0)
                                    .inner_margin(8.0) // Maximizes viewing space
                                    .show(ui, |ui| {
                                        Self::ensure_preview_texture(&mut self.preview_textures, &self.pending_previews, ctx, &page);
                                        let key = (page.id, page.rotation_degrees);
                                        let base_key = (page.id, 0);

                                        let deg = ((page.rotation_degrees % 360) + 360) % 360;
                                        let rad = (deg as f32).to_radians();

                                        let tex_opt = self.preview_textures.get(&key)
                                            .or_else(|| self.thumb_textures.get(&key))
                                            .map(|t| (t, 0.0))
                                            .or_else(|| {
                                                self.preview_textures.get(&base_key)
                                                    .or_else(|| self.thumb_textures.get(&base_key))
                                                    .map(|t| (t, rad))
                                            });

                                        if let Some((tex, extra_rad)) = tex_opt {
                                            let tex_size = tex.size_vec2();
                                            let is_rotated_extra = extra_rad.abs() > 0.001 && (deg % 180 != 0);
                                            let aspect = if tex_size.y > 0.0 {
                                                if is_rotated_extra {
                                                    tex_size.y / tex_size.x
                                                } else {
                                                    tex_size.x / tex_size.y
                                                }
                                            } else {
                                                0.707
                                            };

                                            // Scale image according to zoom_level, expanding above 100% smoothly
                                            let base_height = (ui.available_height() - 20.0).clamp(540.0, 960.0);
                                            let target_height = base_height * self.zoom_level;
                                            let target_width = target_height * aspect;

                                            let mut img = egui::Image::new(tex)
                                                .fit_to_exact_size(Vec2::new(target_width, target_height))
                                                .rounding(4.0);
                                            if extra_rad.abs() > 0.001 {
                                                img = img.rotate(extra_rad, egui::Vec2::splat(0.5));
                                            }
                                            ui.add(img);
                                        } else if let Some(ref text) = page.text_snippet {
                                            // Document Reader Canvas fallback
                                            let (w, h) = if page.rotation_degrees % 180 != 0 {
                                                (580.0 * self.zoom_level, 440.0 * self.zoom_level)
                                            } else {
                                                (460.0 * self.zoom_level, 580.0 * self.zoom_level)
                                            };
                                            egui::Frame::none()
                                                .fill(Color32::from_rgb(255, 255, 255))
                                                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(226, 232, 240)))
                                                .rounding(6.0)
                                                .inner_margin(16.0 * self.zoom_level.clamp(0.5, 2.0))
                                                .show(ui, |ui| {
                                                    ui.set_min_width(w);
                                                    ui.set_min_height(h);
                                                    ui.vertical(|ui| {
                                                        ui.horizontal(|ui| {
                                                            ui.label(RichText::new(format!("PAGE {}", page.original_page_num)).strong().size(14.0 * self.zoom_level.clamp(0.8, 1.8)).color(Color32::from_rgb(71, 85, 105)));
                                                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                                ui.label(RichText::new("MORPH DOCUMENT VIEWER").size(10.0 * self.zoom_level.clamp(0.8, 1.5)).weak().color(Color32::from_rgb(148, 163, 184)));
                                                            });
                                                        });
                                                        ui.add_space(8.0);
                                                        ui.separator();
                                                        ui.add_space(12.0);
                                                        ui.label(RichText::new(text).size(13.0 * self.zoom_level.clamp(0.8, 2.0)).color(Color32::from_rgb(15, 23, 42)));
                                                    });
                                                });
                                        } else {
                                            ui.vertical_centered(|ui| {
                                                ui.add_space(80.0);
                                                ui.label(RichText::new("📄").size(48.0));
                                                ui.heading("Page Preview");
                                                ui.label(RichText::new(format!("Original Page #{}", page.original_page_num)).weak());
                                                ui.add_space(80.0);
                                            });
                                        }
                                    });
                            });
                        });
                    });
                }
            }
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Automatically clean up all temporary preview & thumbnail files on exit
        let cache_dir = get_cache_dir();
        if let Ok(entries) = std::fs::read_dir(&cache_dir) {
            for entry in entries.flatten() {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
}

fn main() -> eframe::Result<()> {
    let args: Vec<String> = env::args().collect();
    let initial_file = if args.len() > 1 {
        Some(PathBuf::from(&args[1]))
    } else {
        None
    };

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

    eframe::run_native(
        "Morph Editor",
        options,
        Box::new(|_cc| Ok(Box::new(MorphApp::new(initial_file)))),
    )
}
