#![windows_subsystem = "windows"]

mod components;
mod icons;
mod motion;
mod state;
mod theme;
mod views;

use components::dropzone::render_dropzone_overlay;
use components::toast::{ToastKind, ToastManager};
use eframe::egui;
use morph_core::engines::pdf::{PageSpec, PdfEngine};
use morph_core::output::OutputOptions;
use motion::Spring;
use state::{
    get_cache_dir, hash_path, render_single_page_preview_bg, spawn_workers,
    BackgroundEvent, DocumentState, FileLoadRequest, InsertPosition, TextureRequest,
};
use std::collections::{HashMap, HashSet};
use std::env;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};
use theme::colors;
use views::bottom_bar::render_bottom_bar;
use views::canvas::render_canvas;
use views::document_tabs::render_document_tabs;
use views::empty_state::render_empty_state;
use views::header::render_header;
use views::page_rail::render_page_rail;
use views::toolbar::render_toolbar;

struct MorphApp {
    _input_file: Option<PathBuf>,
    documents: Vec<DocumentState>,
    active_doc_id: Option<usize>,
    next_doc_id: usize,
    next_page_id: usize,
    status_message: String,
    status_is_error: bool,
    is_loading_files: bool,
    sidebar_collapsed: bool,

    // Textures & background pipelines
    thumb_textures: HashMap<usize, egui::TextureHandle>,
    preview_textures: HashMap<usize, egui::TextureHandle>,
    requested_textures: HashSet<(usize, bool)>,
    pending_previews: Arc<Mutex<HashSet<(u64, u32)>>>,
    _thumb_queue_tx: mpsc::Sender<(PathBuf, PathBuf, u64)>,
    texture_request_tx: mpsc::Sender<TextureRequest>,
    file_loader_tx: mpsc::Sender<FileLoadRequest>,
    event_rx: mpsc::Receiver<BackgroundEvent>,
    logo_texture: Option<egui::TextureHandle>,

    // Smooth Motion & Animation Engine
    zoom_spring: Spring,
    canvas_rotation_spring: Spring,
    scroll_accumulator: f32,
    last_page_scroll_time: f64,
    central_panel_rect: egui::Rect,
    insert_position: InsertPosition,
    toasts: ToastManager,
}

impl MorphApp {
    fn new(initial_files: Vec<PathBuf>) -> Self {
        let (thumb_queue_tx, texture_request_tx, file_loader_tx, event_rx) = spawn_workers();

        let mut app = Self {
            _input_file: initial_files.first().cloned(),
            documents: Vec::new(),
            active_doc_id: None,
            next_doc_id: 1,
            next_page_id: 1,
            status_message: "Ready".to_string(),
            status_is_error: false,
            is_loading_files: false,
            thumb_textures: HashMap::new(),
            preview_textures: HashMap::new(),
            requested_textures: HashSet::new(),
            pending_previews: Arc::new(Mutex::new(HashSet::new())),
            _thumb_queue_tx: thumb_queue_tx,
            texture_request_tx,
            file_loader_tx,
            event_rx,
            logo_texture: None,
            zoom_spring: Spring::new(1.0),
            canvas_rotation_spring: Spring::new(0.0),
            scroll_accumulator: 0.0,
            last_page_scroll_time: 0.0,
            central_panel_rect: egui::Rect::NOTHING,
            insert_position: InsertPosition::End,
            toasts: ToastManager::default(),
            sidebar_collapsed: false,
        };

        if !initial_files.is_empty() {
            app.open_documents(initial_files, 0.0);
        }

        app
    }

    fn notify(&mut self, msg: impl Into<String>, is_error: bool, now: f64) {
        let text = msg.into();
        self.status_message = text.clone();
        self.status_is_error = is_error;
        let kind = if is_error {
            ToastKind::Error
        } else {
            ToastKind::Success
        };
        self.toasts.push(text, kind, now);
    }

    fn active_document(&self) -> Option<&DocumentState> {
        self.active_doc_id.and_then(|id| self.documents.iter().find(|d| d.id == id))
    }

    fn active_document_mut(&mut self) -> Option<&mut DocumentState> {
        self.active_doc_id.and_then(|id| self.documents.iter_mut().find(|d| d.id == id))
    }

    fn switch_to_document(&mut self, doc_id: usize) {
        if self.active_doc_id == Some(doc_id) {
            return;
        }

        // Save current document's transient view state
        let cur_zoom = self.zoom_spring.target;
        let cur_rot = self.canvas_rotation_spring.target;
        if let Some(cur) = self.active_document_mut() {
            cur.zoom = cur_zoom;
            cur.visual_rotation = cur_rot;
        }

        self.active_doc_id = Some(doc_id);

        let doc_info = if let Some(new_doc) = self.active_document() {
            let rot = if let Some(idx) = new_doc.selected_page_index {
                new_doc.pages.get(idx).map(|p| p.rotation_degrees as f32).unwrap_or(0.0)
            } else {
                0.0
            };
            Some((new_doc.zoom, rot, new_doc.file_name.clone()))
        } else {
            None
        };

        if let Some((zoom, rot, name)) = doc_info {
            self.zoom_spring.set_target(zoom);
            self.zoom_spring.snap_to_target();
            self.canvas_rotation_spring.set_target(rot);
            self.canvas_rotation_spring.snap_to_target();
            self.status_message = format!("Active: '{}'", name);
            self.status_is_error = false;
        }
    }

    fn close_document(&mut self, doc_id: usize, now: f64) {
        let pos = self.documents.iter().position(|d| d.id == doc_id);
        if let Some(idx) = pos {
            let removed = self.documents.remove(idx);

            // Clean textures for pages in that document
            for p in &removed.pages {
                self.thumb_textures.remove(&p.id);
                self.preview_textures.remove(&p.id);
                self.requested_textures.retain(|(id, _)| *id != p.id);
            }

            // Cleanup disk cache in background
            let p_path = removed.file_path.clone();
            std::thread::spawn(move || {
                let hash = hash_path(&p_path);
                let cache_dir = get_cache_dir();
                let prefix_thumb = format!("thumb_{:016x}_", hash);
                let prefix_prev = format!("page_{:016x}_", hash);
                let prefix_snip = format!("snip_{:016x}_", hash);
                if let Ok(entries) = std::fs::read_dir(&cache_dir) {
                    for entry in entries.flatten() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if name.starts_with(&prefix_thumb)
                            || name.starts_with(&prefix_prev)
                            || name.starts_with(&prefix_snip)
                        {
                            let _ = std::fs::remove_file(entry.path());
                        }
                    }
                }
            });

            if self.active_doc_id == Some(doc_id) {
                if self.documents.is_empty() {
                    self.active_doc_id = None;
                } else {
                    let next_idx = if idx < self.documents.len() { idx } else { self.documents.len() - 1 };
                    let next_id = self.documents[next_idx].id;
                    self.active_doc_id = None;
                    self.switch_to_document(next_id);
                }
            }

            self.notify(format!("Closed '{}'", removed.file_name), false, now);
        }
    }

    fn cycle_document(&mut self, forward: bool) {
        if self.documents.len() <= 1 {
            return;
        }
        if let Some(cur_id) = self.active_doc_id {
            if let Some(cur_idx) = self.documents.iter().position(|d| d.id == cur_id) {
                let next_idx = if forward {
                    (cur_idx + 1) % self.documents.len()
                } else {
                    (cur_idx + self.documents.len() - 1) % self.documents.len()
                };
                let next_id = self.documents[next_idx].id;
                self.switch_to_document(next_id);
            }
        }
    }

    fn open_documents(&mut self, files: Vec<PathBuf>, now: f64) {
        if files.is_empty() {
            return;
        }
        self.is_loading_files = true;
        let count = files.len();
        self.notify(format!("Opening {} document(s)...", count), false, now);

        let start_doc_id = self.next_doc_id;
        self.next_doc_id += files.len() + 10;
        let start_page_id = self.next_page_id;
        self.next_page_id += files.len() * 100 + 100;

        let _ = self.file_loader_tx.send(FileLoadRequest::OpenDocuments {
            files,
            start_doc_id,
            start_page_id,
            cache_dir: get_cache_dir(),
        });
    }

    fn insert_pages_into_active(&mut self, files: Vec<PathBuf>, position: InsertPosition, now: f64) {
        let active_id = match self.active_doc_id {
            Some(id) => id,
            None => {
                self.open_documents(files, now);
                return;
            }
        };

        let insert_index = if let Some(doc) = self.active_document() {
            match position {
                InsertPosition::Start => 0,
                InsertPosition::End => doc.pages.len(),
                InsertPosition::AfterSelected => {
                    if let Some(sel) = doc.selected_page_index {
                        (sel + 1).min(doc.pages.len())
                    } else {
                        doc.pages.len()
                    }
                }
            }
        } else {
            0
        };

        self.is_loading_files = true;
        let count = files.len();
        self.notify(format!("Inserting pages from {} file(s)...", count), false, now);

        let start_page_id = self.next_page_id;
        self.next_page_id += files.len() * 100 + 100;

        let _ = self.file_loader_tx.send(FileLoadRequest::InsertPages {
            doc_id: active_id,
            files,
            insert_index,
            start_page_id,
            cache_dir: get_cache_dir(),
        });
    }

    fn request_texture(&mut self, id: usize, path: PathBuf, is_preview: bool) {
        let key = (id, is_preview);
        if self.requested_textures.insert(key) {
            let _ = self.texture_request_tx.send(TextureRequest {
                id,
                path,
                is_preview,
            });
        }
    }

    fn undo(&mut self, now: f64) {
        if let Some(doc) = self.active_document_mut() {
            if doc.undo() {
                self.notify("Undo performed", false, now);
            }
        }
    }

    fn redo(&mut self, now: f64) {
        if let Some(doc) = self.active_document_mut() {
            if doc.redo() {
                self.notify("Redo performed", false, now);
            }
        }
    }

    fn rotate_single_page(&mut self, idx: usize, degrees: i32, now: f64) {
        let mut rot_info = None;
        if let Some(doc) = self.active_document_mut() {
            if idx < doc.pages.len() {
                let page = &mut doc.pages[idx];
                let new_deg = (page.rotation_degrees + degrees) % 360;
                page.rotation_degrees = if new_deg < 0 { new_deg + 360 } else { new_deg };
                let final_deg = page.rotation_degrees;
                doc.selected_page_index = Some(idx);
                doc.save_history();
                rot_info = Some(final_deg);
            }
        }
        if let Some(final_deg) = rot_info {
            self.canvas_rotation_spring.set_target(final_deg as f32);
            self.canvas_rotation_spring.snap_to_target();
            self.notify(format!("Rotated page {} to {}°", idx + 1, final_deg), false, now);
        }
    }

    fn rotate_all_pages(&mut self, degrees: i32, now: f64) {
        let mut total_pages = None;
        if let Some(doc) = self.active_document_mut() {
            if !doc.pages.is_empty() {
                for page in &mut doc.pages {
                    let new_deg = (page.rotation_degrees + degrees) % 360;
                    page.rotation_degrees = if new_deg < 0 { new_deg + 360 } else { new_deg };
                }
                doc.save_history();
                total_pages = Some(doc.pages.len());
            }
        }
        if let Some(total) = total_pages {
            if let Some(doc) = self.active_document() {
                if let Some(idx) = doc.selected_page_index {
                    let deg = doc.pages[idx].rotation_degrees as f32;
                    self.canvas_rotation_spring.set_target(deg);
                    self.canvas_rotation_spring.snap_to_target();
                }
            }
            self.notify(format!("Rotated all {} pages by {}°", total, degrees), false, now);
        }
    }

    fn delete_selected(&mut self, now: f64) {
        let sel = self.active_document().and_then(|d| d.selected_page_index);
        if let Some(idx) = sel {
            self.delete_page_at(idx, now);
        }
    }

    fn delete_page_at(&mut self, idx: usize, now: f64) {
        let mut removed_page_id = None;
        if let Some(doc) = self.active_document_mut() {
            if idx < doc.pages.len() {
                let page_id = doc.pages[idx].id;
                doc.pages.remove(idx);

                if doc.pages.is_empty() {
                    doc.selected_page_index = None;
                } else if let Some(sel) = doc.selected_page_index {
                    if sel >= doc.pages.len() {
                        doc.selected_page_index = Some(doc.pages.len() - 1);
                    } else if sel == idx {
                        doc.selected_page_index = Some(idx.min(doc.pages.len() - 1));
                    }
                }
                doc.save_history();
                removed_page_id = Some(page_id);
            }
        }

        if let Some(page_id) = removed_page_id {
            self.thumb_textures.remove(&page_id);
            self.preview_textures.remove(&page_id);
            self.requested_textures.retain(|(rid, _)| *rid != page_id);
            self.notify(format!("Page {} removed", idx + 1), false, now);
        }
    }

    fn duplicate_page_at(&mut self, idx: usize, now: f64) {
        let new_id = self.next_page_id;
        self.next_page_id += 1;

        if let Some(doc) = self.active_document_mut() {
            if idx < doc.pages.len() {
                let mut new_page = doc.pages[idx].clone();
                new_page.id = new_id;
                doc.pages.insert(idx + 1, new_page);
                doc.selected_page_index = Some(idx + 1);
                doc.save_history();
                self.notify(format!("Duplicated page {}", idx + 1), false, now);
            }
        }
    }

    fn move_page_up_at(&mut self, idx: usize) {
        if let Some(doc) = self.active_document_mut() {
            if idx > 0 && idx < doc.pages.len() {
                doc.pages.swap(idx, idx - 1);
                doc.selected_page_index = Some(idx - 1);
                doc.save_history();
            }
        }
    }

    fn move_page_down_at(&mut self, idx: usize) {
        if let Some(doc) = self.active_document_mut() {
            if idx + 1 < doc.pages.len() {
                doc.pages.swap(idx, idx + 1);
                doc.selected_page_index = Some(idx + 1);
                doc.save_history();
            }
        }
    }

    fn reorder_page(&mut self, from_idx: usize, to_idx: usize, now: f64) {
        if let Some(doc) = self.active_document_mut() {
            if from_idx < doc.pages.len() && to_idx < doc.pages.len() && from_idx != to_idx {
                let item = doc.pages.remove(from_idx);
                doc.pages.insert(to_idx, item);
                doc.selected_page_index = Some(to_idx);
                doc.save_history();
                self.notify(format!("Moved page {} to position {}", from_idx + 1, to_idx + 1), false, now);
            }
        }
    }

    fn export_result(&mut self, now: f64) {
        let (first_file, page_specs) = {
            let doc = match self.active_document() {
                Some(d) => d,
                None => {
                    self.notify("Cannot export: No active document", true, now);
                    return;
                }
            };

            if doc.pages.is_empty() {
                self.notify("Cannot export: Active document has no pages", true, now);
                return;
            }

            let specs: Vec<PageSpec> = doc
                .pages
                .iter()
                .map(|p| PageSpec {
                    source_path: p.source_file.clone(),
                    page_number: p.original_page_num,
                    is_image: p.is_image,
                    rotation: p.rotation_degrees,
                })
                .collect();
            (doc.file_path.clone(), specs)
        };

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
            None => return,
        };

        let output_options = OutputOptions {
            overwrite: true,
            custom_output_dir: target_path.parent().map(|p| p.to_path_buf()),
            custom_filename: target_path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string()),
        };

        match PdfEngine::assemble_document(&page_specs, &output_options) {
            Ok(out) => {
                let fname = out
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("output.pdf");
                self.notify(format!("Exported successfully to '{}'", fname), false, now);

                #[cfg(windows)]
                {
                    use std::os::windows::process::CommandExt;
                    const CREATE_NO_WINDOW: u32 = 0x08000000;
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
                        fname
                    );
                    let mut cmd = std::process::Command::new("powershell.exe");
                    cmd.creation_flags(CREATE_NO_WINDOW);
                    let _ = cmd
                        .args([
                            "-NoProfile",
                            "-NonInteractive",
                            "-WindowStyle",
                            "Hidden",
                            "-Command",
                            &ps_script,
                        ])
                        .spawn();
                }
            }
            Err(e) => {
                self.notify(format!("Export failed: {}", e), true, now);
            }
        }
    }
}

impl eframe::App for MorphApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        theme::apply_theme(ctx);
        let now = ctx.input(|i| i.time);
        let dt = ctx.input(|i| i.stable_dt).min(0.05);

        // 1. Process asynchronous background events (off UI thread)
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                BackgroundEvent::DocumentLoaded {
                    doc_id,
                    file_path,
                    is_image,
                    pages,
                } => {
                    self.is_loading_files = false;
                    let page_count = pages.len();
                    let new_doc = DocumentState::new(doc_id, file_path, is_image, pages);
                    let fname = new_doc.file_name.clone();
                    self.documents.push(new_doc);
                    self.active_doc_id = Some(doc_id);
                    self.zoom_spring.set_target(1.0);
                    self.zoom_spring.snap_to_target();
                    self.canvas_rotation_spring.set_target(0.0);
                    self.canvas_rotation_spring.snap_to_target();
                    self.notify(format!("Opened '{}' ({} pages)", fname, page_count), false, now);
                }
                BackgroundEvent::PagesInserted {
                    doc_id,
                    insert_index,
                    pages,
                } => {
                    self.is_loading_files = false;
                    let count = pages.len();
                    if let Some(doc) = self.documents.iter_mut().find(|d| d.id == doc_id) {
                        let actual_idx = insert_index.min(doc.pages.len());
                        doc.pages.splice(actual_idx..actual_idx, pages);
                        if doc.selected_page_index.is_none() {
                            doc.selected_page_index = Some(actual_idx);
                        }
                        doc.save_history();
                        self.notify(format!("Inserted {} page(s)", count), false, now);
                    }
                }
                BackgroundEvent::TextureDecoded { id, is_preview, image } => {
                    if is_preview {
                        let handle = ctx.load_texture(format!("prev_{}", id), image, egui::TextureOptions::LINEAR);
                        self.preview_textures.insert(id, handle);
                    } else {
                        let handle = ctx.load_texture(format!("thumb_{}", id), image, egui::TextureOptions::LINEAR);
                        self.thumb_textures.insert(id, handle);
                    }
                    ctx.request_repaint();
                }
                BackgroundEvent::SnippetLoaded { id, text } => {
                    for doc in &mut self.documents {
                        if let Some(p) = doc.pages.iter_mut().find(|p| p.id == id) {
                            p.text_snippet = Some(text);
                            break;
                        }
                    }
                }
            }
        }

        // 2. Advance motion springs (only request repaint while actively in motion)
        let mut spring_active = self.zoom_spring.update(dt);
        if self.canvas_rotation_spring.update(dt) {
            spring_active = true;
        }
        if spring_active {
            ctx.request_repaint();
        }

        // Lazy-load logo
        if self.logo_texture.is_none() {
            if let Ok(img) = image::load_from_memory(include_bytes!("../../../assets/logo_fluent.png")) {
                let rgba = img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let color_img = egui::ColorImage::from_rgba_unmultiplied(size, &rgba.into_raw());
                self.logo_texture = Some(ctx.load_texture("app_logo", color_img, egui::TextureOptions::LINEAR));
            }
        }

        // Native OS Drag and Drop Files handling
        let hovered_files = ctx.input(|i| i.raw.hovered_files.clone());
        let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());
        let is_hovering = !hovered_files.is_empty();

        if !dropped_files.is_empty() {
            let paths: Vec<PathBuf> = dropped_files
                .into_iter()
                .filter_map(|d| d.path)
                .collect();
            if !paths.is_empty() {
                if self.documents.is_empty() {
                    self.open_documents(paths, now);
                } else {
                    self.insert_pages_into_active(paths, self.insert_position, now);
                }
            }
        }

        // Continuous Smooth Zoom via Ctrl + Mouse Wheel
        ctx.input(|i| {
            if i.modifiers.ctrl && i.raw_scroll_delta.y.abs() > 0.0 {
                let delta = i.raw_scroll_delta.y * 0.002;
                let new_target = (self.zoom_spring.target + delta).clamp(0.25, 4.0);
                self.zoom_spring.set_target(new_target);
            }
        });

        // Mouse Wheel Page Navigation on Central Canvas (when not holding Ctrl)
        let (scroll_y, is_ctrl) = ctx.input(|i| (i.raw_scroll_delta.y, i.modifiers.ctrl));
        let pointer_in_central = ctx.input(|i| {
            if let Some(pos) = i.pointer.hover_pos() {
                self.central_panel_rect.contains(pos)
            } else {
                false
            }
        });

        if !is_ctrl && pointer_in_central && scroll_y.abs() > 0.5 && self.zoom_spring.value <= 1.25 {
            self.scroll_accumulator += scroll_y;
            if self.scroll_accumulator <= -30.0 && (now - self.last_page_scroll_time) > 0.12 {
                let mut nav_info = None;
                if let Some(doc) = self.active_document_mut() {
                    if let Some(idx) = doc.selected_page_index {
                        if idx + 1 < doc.pages.len() {
                            doc.selected_page_index = Some(idx + 1);
                            let deg = doc.pages[idx + 1].rotation_degrees as f32;
                            nav_info = Some((idx + 2, doc.pages.len(), deg));
                        }
                    }
                }
                if let Some((page_num, total, deg)) = nav_info {
                    self.canvas_rotation_spring.set_target(deg);
                    self.canvas_rotation_spring.snap_to_target();
                    self.status_message = format!("Page {} of {}", page_num, total);
                    self.status_is_error = false;
                    self.last_page_scroll_time = now;
                    self.scroll_accumulator = 0.0;
                }
            } else if self.scroll_accumulator >= 30.0 && (now - self.last_page_scroll_time) > 0.12 {
                let mut nav_info = None;
                if let Some(doc) = self.active_document_mut() {
                    if let Some(idx) = doc.selected_page_index {
                        if idx > 0 {
                            doc.selected_page_index = Some(idx - 1);
                            let deg = doc.pages[idx - 1].rotation_degrees as f32;
                            nav_info = Some((idx, doc.pages.len(), deg));
                        }
                    }
                }
                if let Some((page_num, total, deg)) = nav_info {
                    self.canvas_rotation_spring.set_target(deg);
                    self.canvas_rotation_spring.snap_to_target();
                    self.status_message = format!("Page {} of {}", page_num, total);
                    self.status_is_error = false;
                    self.last_page_scroll_time = now;
                    self.scroll_accumulator = 0.0;
                }
            }
        } else if scroll_y.abs() < 0.5 {
            self.scroll_accumulator *= 0.5;
            if self.scroll_accumulator.abs() < 1.0 {
                self.scroll_accumulator = 0.0;
            }
        }

        // Global Keyboard Shortcuts
        // Document cycling: Ctrl+Tab and Ctrl+Shift+Tab
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Tab)) {
            let shift = ctx.input(|i| i.modifiers.shift);
            self.cycle_document(!shift);
        }

        // Close active tab: Ctrl+W
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::W)) {
            if let Some(doc_id) = self.active_doc_id {
                self.close_document(doc_id, now);
            }
        }

        // Open document: Ctrl+O
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::O)) {
            if let Some(files) = rfd::FileDialog::new()
                .add_filter("Supported Files", &["pdf", "png", "jpg", "jpeg", "webp", "bmp"])
                .pick_files()
            {
                self.open_documents(files, now);
            }
        }

        // Export PDF: Ctrl+S
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::S)) {
            self.export_result(now);
        }

        // Delete selected page
        if ctx.input(|i| i.key_pressed(egui::Key::Delete)) {
            self.delete_selected(now);
        }

        // Rotate CW: R, CCW: Shift+R
        if ctx.input(|i| i.key_pressed(egui::Key::R)) {
            let shift = ctx.input(|i| i.modifiers.shift);
            if let Some(idx) = self.active_document().and_then(|d| d.selected_page_index) {
                self.rotate_single_page(idx, if shift { -90 } else { 90 }, now);
            }
        }

        // Undo: Ctrl+Z
        if ctx.input(|i| i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::Z)) {
            self.undo(now);
        }

        // Redo: Ctrl+Y or Ctrl+Shift+Z
        if ctx.input(|i| {
            i.modifiers.ctrl
                && (i.key_pressed(egui::Key::Y)
                    || (i.modifiers.shift && i.key_pressed(egui::Key::Z)))
        }) {
            self.redo(now);
        }

        // Duplicate page: Ctrl+D
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::D)) {
            if let Some(idx) = self.active_document().and_then(|d| d.selected_page_index) {
                self.duplicate_page_at(idx, now);
            }
        }

        // Toggle Sidebar: Ctrl+B
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::B)) {
            self.sidebar_collapsed = !self.sidebar_collapsed;
        }

        // Arrow navigation
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp) || i.key_pressed(egui::Key::PageUp)) {
            let mut nav_rot = None;
            if let Some(doc) = self.active_document_mut() {
                if let Some(idx) = doc.selected_page_index {
                    if idx > 0 {
                        doc.selected_page_index = Some(idx - 1);
                        let deg = doc.pages[idx - 1].rotation_degrees as f32;
                        nav_rot = Some(deg);
                    }
                }
            }
            if let Some(deg) = nav_rot {
                self.canvas_rotation_spring.set_target(deg);
                self.canvas_rotation_spring.snap_to_target();
            }
        }
        if ctx.input(|i| {
            i.key_pressed(egui::Key::ArrowDown)
                || i.key_pressed(egui::Key::PageDown)
                || i.key_pressed(egui::Key::Space)
        }) {
            let mut nav_rot = None;
            if let Some(doc) = self.active_document_mut() {
                if let Some(idx) = doc.selected_page_index {
                    if idx + 1 < doc.pages.len() {
                        doc.selected_page_index = Some(idx + 1);
                        let deg = doc.pages[idx + 1].rotation_degrees as f32;
                        nav_rot = Some(deg);
                    }
                }
            }
            if let Some(deg) = nav_rot {
                self.canvas_rotation_spring.set_target(deg);
                self.canvas_rotation_spring.snap_to_target();
            }
        }

        // Active document data cache
        let (active_name_owned, active_pages_count, can_undo, can_redo, active_sel_idx) = if let Some(doc) = self.active_document() {
            (
                Some(doc.file_name.clone()),
                doc.pages.len(),
                doc.can_undo(),
                doc.can_redo(),
                doc.selected_page_index,
            )
        } else {
            (None, 0, false, false, None)
        };

        // 1. TOP PANELS (Header, Document Tabs, Main Consolidated Toolbar)
        egui::TopBottomPanel::top("top_panel")
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = egui::Vec2::ZERO;

                // A. Top Application Header
                let header_act = render_header(
                    ui,
                    self.logo_texture.as_ref(),
                    active_name_owned.as_deref(),
                    can_undo,
                    can_redo,
                    active_pages_count,
                );

                if header_act.undo {
                    self.undo(now);
                }
                if header_act.redo {
                    self.redo(now);
                }
                if header_act.export {
                    self.export_result(now);
                }

                // B. Document Tabs (rendered when documents exist)
                let tabs_act = render_document_tabs(
                    ui,
                    &self.documents,
                    self.active_doc_id,
                );

                if tabs_act.open_new_document {
                    if let Some(files) = rfd::FileDialog::new()
                        .add_filter("Supported Files", &["pdf", "png", "jpg", "jpeg", "webp", "bmp"])
                        .pick_files()
                    {
                        self.open_documents(files, now);
                    }
                }
                if let Some(target_id) = tabs_act.switch_to_document {
                    self.switch_to_document(target_id);
                }
                if let Some(target_id) = tabs_act.close_document {
                    self.close_document(target_id, now);
                }

                // C. Main Consolidated Editing Toolbar
                let toolbar_act = render_toolbar(
                    ui,
                    active_sel_idx.is_some(),
                    active_pages_count,
                    self.zoom_spring.value,
                    self.insert_position,
                    self.sidebar_collapsed,
                );

                if toolbar_act.toggle_sidebar {
                    self.sidebar_collapsed = !self.sidebar_collapsed;
                }
                if toolbar_act.rotate_left {
                    if let Some(idx) = active_sel_idx {
                        self.rotate_single_page(idx, -90, now);
                    }
                }
                if toolbar_act.rotate_right {
                    if let Some(idx) = active_sel_idx {
                        self.rotate_single_page(idx, 90, now);
                    }
                }
                if toolbar_act.flip_180 {
                    if let Some(idx) = active_sel_idx {
                        self.rotate_single_page(idx, 180, now);
                    }
                }
                if toolbar_act.duplicate_page {
                    if let Some(idx) = active_sel_idx {
                        self.duplicate_page_at(idx, now);
                    }
                }
                if toolbar_act.delete_page {
                    self.delete_selected(now);
                }
                if toolbar_act.zoom_delta.abs() > 0.001 {
                    let new_target = (self.zoom_spring.target + toolbar_act.zoom_delta).clamp(0.25, 4.0);
                    self.zoom_spring.set_target(new_target);
                }
                if toolbar_act.reset_zoom {
                    self.zoom_spring.set_target(1.0);
                }
                if toolbar_act.fit_page {
                    self.zoom_spring.set_target(1.0);
                }
                if toolbar_act.fit_width {
                    let avail_w = (self.central_panel_rect.width() - 60.0).max(300.0);
                    let fit_h = (self.central_panel_rect.height() - 48.0).max(200.0);
                    let base_w = fit_h * 0.7727_f32;
                    if base_w > 0.0 {
                        self.zoom_spring.set_target((avail_w / base_w).clamp(0.5, 3.0));
                    }
                }
                if toolbar_act.insert_pages {
                    if let Some(files) = rfd::FileDialog::new()
                        .add_filter("Supported Files", &["pdf", "png", "jpg", "jpeg", "webp", "bmp"])
                        .pick_files()
                    {
                        self.insert_pages_into_active(files, self.insert_position, now);
                    }
                }
                if toolbar_act.rotate_all {
                    self.rotate_all_pages(90, now);
                }
                if let Some(pos) = toolbar_act.insert_pos_changed {
                    self.insert_position = pos;
                }
            });

        // 2. BOTTOM STATUS BAR
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            render_bottom_bar(
                ui,
                &self.status_message,
                self.status_is_error,
                active_pages_count,
                active_sel_idx,
                self.zoom_spring.value,
                self.is_loading_files,
            );
        });

        // 3. CENTRAL WORKSPACE AREA & SIDEBAR
        if self.documents.is_empty() {
            egui::CentralPanel::default().show(ctx, |ui| {
                if let Some(picked) = render_empty_state(ui, self.logo_texture.as_ref()) {
                    self.open_documents(picked, now);
                }
            });
        } else {
            // D. LEFT PAGE NAVIGATION SIDEBAR (Collapsible, virtualized)
            if !self.sidebar_collapsed {
                let active_pages: Vec<state::PageItem> = self.active_document()
                    .map(|d| d.pages.clone())
                    .unwrap_or_default();

                egui::SidePanel::left("vertical_page_rail")
                    .exact_width(220.0)
                    .resizable(false)
                    .frame(egui::Frame::none().fill(colors::BG_SECONDARY))
                    .show(ctx, |ui| {
                        let action = render_page_rail(
                            ui,
                            &active_pages,
                            active_sel_idx,
                            &self.thumb_textures,
                            &self.preview_textures,
                        );

                        // Queue decoded textures for visible cards on-demand off UI thread
                        for (id, path) in action.request_textures {
                            self.request_texture(id, path, false);
                        }

                        if action.toggle_sidebar {
                            self.sidebar_collapsed = true;
                        }
                        if let Some(idx) = action.selected {
                            let mut target_rot = None;
                            if let Some(doc) = self.active_document_mut() {
                                doc.selected_page_index = Some(idx);
                                if idx < doc.pages.len() {
                                    target_rot = Some(doc.pages[idx].rotation_degrees as f32);
                                }
                            }
                            if let Some(deg) = target_rot {
                                self.canvas_rotation_spring.set_target(deg);
                                self.canvas_rotation_spring.snap_to_target();
                            }
                        }
                        if let Some(idx) = action.rotate_cw {
                            self.rotate_single_page(idx, 90, now);
                        }
                        if let Some(idx) = action.rotate_ccw {
                            self.rotate_single_page(idx, -90, now);
                        }
                        if let Some(idx) = action.flip_180 {
                            self.rotate_single_page(idx, 180, now);
                        }
                        if let Some(idx) = action.duplicate {
                            self.duplicate_page_at(idx, now);
                        }
                        if let Some(idx) = action.move_up {
                            self.move_page_up_at(idx);
                        }
                        if let Some(idx) = action.move_down {
                            self.move_page_down_at(idx);
                        }
                        if let Some((from, to)) = action.reorder {
                            self.reorder_page(from, to, now);
                        }
                        if let Some(idx) = action.delete {
                            self.delete_page_at(idx, now);
                        }
                    });
            }

            // E. MAIN DOCUMENT CANVAS
            egui::CentralPanel::default()
                .frame(egui::Frame::none().fill(colors::BG_APP))
                .show(ctx, |ui| {
                self.central_panel_rect = ui.max_rect();

                let active_page_opt = self.active_document().and_then(|doc| {
                    doc.selected_page_index.and_then(|idx| doc.pages.get(idx).cloned())
                });

                if let Some(page) = active_page_opt {
                    let action = render_canvas(
                        ui,
                        &page,
                        self.zoom_spring.value,
                        self.canvas_rotation_spring.value,
                        &self.preview_textures,
                        &self.thumb_textures,
                    );

                    // Queue high-res preview decode in background if needed
                    if let Some((id, path)) = action.request_preview {
                        if !page.is_image {
                            let hash = hash_path(&page.source_file);
                            let p_num = page.original_page_num;
                            let should_spawn = if let Ok(mut set) = self.pending_previews.lock() {
                                set.insert((hash, p_num))
                            } else {
                                false
                            };
                            if should_spawn {
                                let bg_path = page.source_file.clone();
                                let bg_cache = get_cache_dir();
                                let bg_pending = self.pending_previews.clone();
                                std::thread::spawn(move || {
                                    render_single_page_preview_bg(&bg_path, &bg_cache, hash, p_num);
                                    if let Ok(mut set) = bg_pending.lock() {
                                        set.remove(&(hash, p_num));
                                    }
                                });
                            }
                        }
                        self.request_texture(id, path, true);
                    }
                }
            });
        }

        // RENDER DROPZONE OVERLAY
        render_dropzone_overlay(ctx, is_hovering);

        // RENDER TOAST STACK
        self.toasts.render(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
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
        Box::new(|_cc| Ok(Box::new(MorphApp::new(initial_files)))),
    )
}
