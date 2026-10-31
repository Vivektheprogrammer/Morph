use eframe::egui;
use morph_core::detect_format_from_path;
use morph_core::engines::pdf::PdfEngine;
use morph_core::format::Format;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};

#[derive(Clone, Debug, PartialEq)]
pub struct PageItem {
    pub id: usize,
    pub original_page_num: u32,
    pub source_file: PathBuf,
    pub rotation_degrees: i32,
    pub is_image: bool,
    pub thumbnail_path: Option<PathBuf>,
    pub highres_path: Option<PathBuf>,
    pub text_snippet: Option<String>,
}

#[derive(Clone, Debug)]
pub struct EditorHistory {
    pub pages: Vec<PageItem>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InsertPosition {
    Start,
    AfterSelected,
    End,
}

#[derive(Clone, Debug)]
pub struct TextureRequest {
    pub id: usize,
    pub path: PathBuf,
    pub is_preview: bool,
}

/// Represents the isolated, dedicated state of an open document in a tab.
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct DocumentState {
    pub id: usize,
    pub file_path: PathBuf,
    pub file_name: String,
    pub is_image: bool,
    pub pages: Vec<PageItem>,
    pub selected_page_index: Option<usize>,
    pub history: Vec<EditorHistory>,
    pub history_index: usize,
    pub zoom: f32,
    pub visual_rotation: f32,
    pub scroll_y: f32,
    pub next_page_id: usize,
}

impl DocumentState {
    pub fn new(id: usize, file_path: PathBuf, is_image: bool, pages: Vec<PageItem>) -> Self {
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("document.pdf")
            .to_string();
        let selected = if pages.is_empty() { None } else { Some(0) };
        let initial_history = vec![EditorHistory { pages: pages.clone() }];
        let max_id = pages.iter().map(|p| p.id).max().unwrap_or(0);

        Self {
            id,
            file_path,
            file_name,
            is_image,
            pages,
            selected_page_index: selected,
            history: initial_history,
            history_index: 1,
            zoom: 1.0,
            visual_rotation: 0.0,
            scroll_y: 0.0,
            next_page_id: max_id + 1,
        }
    }

    pub fn save_history(&mut self) {
        if self.history_index < self.history.len() {
            self.history.truncate(self.history_index);
        }
        self.history.push(EditorHistory {
            pages: self.pages.clone(),
        });
        self.history_index = self.history.len();
    }

    pub fn undo(&mut self) -> bool {
        if self.history_index > 1 {
            self.history_index -= 1;
            self.pages = self.history[self.history_index - 1].pages.clone();
            if let Some(idx) = self.selected_page_index {
                if idx >= self.pages.len() && !self.pages.is_empty() {
                    self.selected_page_index = Some(self.pages.len() - 1);
                }
            }
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if self.history_index < self.history.len() {
            self.history_index += 1;
            self.pages = self.history[self.history_index - 1].pages.clone();
            if let Some(idx) = self.selected_page_index {
                if idx >= self.pages.len() && !self.pages.is_empty() {
                    self.selected_page_index = Some(self.pages.len() - 1);
                }
            }
            true
        } else {
            false
        }
    }

    pub fn can_undo(&self) -> bool {
        self.history_index > 1
    }

    pub fn can_redo(&self) -> bool {
        self.history_index < self.history.len()
    }
}

#[derive(Clone, Debug)]
pub enum FileLoadRequest {
    OpenDocuments {
        files: Vec<PathBuf>,
        start_doc_id: usize,
        start_page_id: usize,
        cache_dir: PathBuf,
    },
    InsertPages {
        doc_id: usize,
        files: Vec<PathBuf>,
        insert_index: usize,
        start_page_id: usize,
        cache_dir: PathBuf,
    },
}

#[allow(dead_code)]
pub enum BackgroundEvent {
    DocumentLoaded {
        doc_id: usize,
        file_path: PathBuf,
        is_image: bool,
        pages: Vec<PageItem>,
    },
    PagesInserted {
        doc_id: usize,
        insert_index: usize,
        pages: Vec<PageItem>,
    },
    TextureDecoded {
        id: usize,
        is_preview: bool,
        image: egui::ColorImage,
    },
    SnippetLoaded {
        id: usize,
        text: String,
    },
}

pub fn hash_path(path: &Path) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut s = DefaultHasher::new();
    path.to_string_lossy().hash(&mut s);
    s.finish()
}

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

pub fn render_pdf_thumbnails_bg(pdf_path: PathBuf, cache_dir: PathBuf, hash: u64) {
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

pub fn render_single_page_preview_bg(pdf_path: &Path, cache_dir: &Path, hash: u64, page_num: u32) {
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

/// Spawns the dedicated asynchronous worker threads:
/// 1. PDF thumbnail rendering queue (runs Python in background)
/// 2. Asynchronous texture decoder (decodes PNGs into ColorImage off UI thread)
/// 3. Asynchronous file loader channel (inspects page counts and organizes multi-document state without freezing UI)
pub fn spawn_workers() -> (
    Sender<(PathBuf, PathBuf, u64)>,
    Sender<TextureRequest>,
    Sender<FileLoadRequest>,
    Receiver<BackgroundEvent>,
) {
    let (event_tx, event_rx) = channel::<BackgroundEvent>();

    // 1. Python PDF thumbnail worker
    let (thumb_tx, thumb_rx) = channel::<(PathBuf, PathBuf, u64)>();
    std::thread::spawn(move || {
        while let Ok((pdf_path, cache_dir, hash)) = thumb_rx.recv() {
            render_pdf_thumbnails_bg(pdf_path, cache_dir, hash);
        }
    });

    // 2. Texture decoding worker (reads & decodes images off the UI thread)
    let (texture_tx, texture_rx) = channel::<TextureRequest>();
    let tex_event_tx = event_tx.clone();
    std::thread::spawn(move || {
        while let Ok(req) = texture_rx.recv() {
            // If file is still being rendered by Python worker, wait up to 1.5s
            let mut attempts = 0;
            while !req.path.exists() && attempts < 15 {
                std::thread::sleep(std::time::Duration::from_millis(100));
                attempts += 1;
            }

            if req.path.exists() {
                if let Ok(dyn_img) = image::open(&req.path) {
                    let rgba = dyn_img.to_rgba8();
                    let size = [rgba.width() as usize, rgba.height() as usize];
                    let color_img = egui::ColorImage::from_rgba_unmultiplied(size, &rgba.into_raw());

                    let _ = tex_event_tx.send(BackgroundEvent::TextureDecoded {
                        id: req.id,
                        is_preview: req.is_preview,
                        image: color_img,
                    });
                }
            }
        }
    });

    // 3. File Loader worker (loads documents and page counts off the UI thread)
    let (file_tx, file_rx) = channel::<FileLoadRequest>();
    let file_event_tx = event_tx.clone();
    let file_thumb_tx = thumb_tx.clone();

    std::thread::spawn(move || {
        while let Ok(req) = file_rx.recv() {
            match req {
                FileLoadRequest::OpenDocuments {
                    files,
                    start_doc_id,
                    start_page_id,
                    cache_dir,
                } => {
                    let mut doc_id_counter = start_doc_id;
                    let mut page_id_counter = start_page_id;

                    for path in files {
                        if !path.exists() {
                            continue;
                        }

                        let fmt = detect_format_from_path(&path).unwrap_or(Format::Pdf);
                        let path_hash = hash_path(&path);
                        let this_doc_id = doc_id_counter;
                        doc_id_counter += 1;

                        if fmt == Format::Pdf {
                            if let Ok(count) = PdfEngine::page_count(&path) {
                                let _ = file_thumb_tx.send((path.clone(), cache_dir.clone(), path_hash));

                                let mut pages = Vec::with_capacity(count);
                                for page_num in 1..=count as u32 {
                                    let thumb_file = cache_dir.join(format!("thumb_{:016x}_{}.png", path_hash, page_num));
                                    let page_file = cache_dir.join(format!("page_{:016x}_{}.png", path_hash, page_num));

                                    pages.push(PageItem {
                                        id: page_id_counter,
                                        original_page_num: page_num,
                                        source_file: path.clone(),
                                        rotation_degrees: 0,
                                        is_image: false,
                                        thumbnail_path: Some(thumb_file),
                                        highres_path: Some(page_file),
                                        text_snippet: None,
                                    });
                                    page_id_counter += 1;
                                }

                                let _ = file_event_tx.send(BackgroundEvent::DocumentLoaded {
                                    doc_id: this_doc_id,
                                    file_path: path.clone(),
                                    is_image: false,
                                    pages,
                                });
                            }
                        } else if fmt.category() == morph_core::FormatCategory::Image {
                            let thumb_file = cache_dir.join(format!("img_thumb_{:016x}.png", path_hash));
                            if !thumb_file.exists() {
                                let img_src = path.clone();
                                let img_dst = thumb_file.clone();
                                if let Ok(img) = image::open(&img_src) {
                                    let thumb = img.thumbnail(320, 440);
                                    let _ = thumb.save(&img_dst);
                                }
                            }

                            let thumb_opt = if thumb_file.exists() {
                                Some(thumb_file)
                            } else {
                                Some(path.clone())
                            };

                            let pages = vec![PageItem {
                                id: page_id_counter,
                                original_page_num: 1,
                                source_file: path.clone(),
                                rotation_degrees: 0,
                                is_image: true,
                                thumbnail_path: thumb_opt,
                                highres_path: Some(path.clone()),
                                text_snippet: None,
                            }];
                            page_id_counter += 1;

                            let _ = file_event_tx.send(BackgroundEvent::DocumentLoaded {
                                doc_id: this_doc_id,
                                file_path: path.clone(),
                                is_image: true,
                                pages,
                            });
                        }
                    }
                }
                FileLoadRequest::InsertPages {
                    doc_id,
                    files,
                    insert_index,
                    start_page_id,
                    cache_dir,
                } => {
                    let mut page_id_counter = start_page_id;
                    let mut inserted_pages = Vec::new();

                    for path in files {
                        if !path.exists() {
                            continue;
                        }

                        let fmt = detect_format_from_path(&path).unwrap_or(Format::Pdf);
                        let path_hash = hash_path(&path);

                        if fmt == Format::Pdf {
                            if let Ok(count) = PdfEngine::page_count(&path) {
                                let _ = file_thumb_tx.send((path.clone(), cache_dir.clone(), path_hash));

                                for page_num in 1..=count as u32 {
                                    let thumb_file = cache_dir.join(format!("thumb_{:016x}_{}.png", path_hash, page_num));
                                    let page_file = cache_dir.join(format!("page_{:016x}_{}.png", path_hash, page_num));

                                    inserted_pages.push(PageItem {
                                        id: page_id_counter,
                                        original_page_num: page_num,
                                        source_file: path.clone(),
                                        rotation_degrees: 0,
                                        is_image: false,
                                        thumbnail_path: Some(thumb_file),
                                        highres_path: Some(page_file),
                                        text_snippet: None,
                                    });
                                    page_id_counter += 1;
                                }
                            }
                        } else if fmt.category() == morph_core::FormatCategory::Image {
                            let thumb_file = cache_dir.join(format!("img_thumb_{:016x}.png", path_hash));
                            if !thumb_file.exists() {
                                let img_src = path.clone();
                                let img_dst = thumb_file.clone();
                                if let Ok(img) = image::open(&img_src) {
                                    let thumb = img.thumbnail(320, 440);
                                    let _ = thumb.save(&img_dst);
                                }
                            }

                            let thumb_opt = if thumb_file.exists() {
                                Some(thumb_file)
                            } else {
                                Some(path.clone())
                            };

                            inserted_pages.push(PageItem {
                                id: page_id_counter,
                                original_page_num: 1,
                                source_file: path.clone(),
                                rotation_degrees: 0,
                                is_image: true,
                                thumbnail_path: thumb_opt,
                                highres_path: Some(path.clone()),
                                text_snippet: None,
                            });
                            page_id_counter += 1;
                        }
                    }

                    let _ = file_event_tx.send(BackgroundEvent::PagesInserted {
                        doc_id,
                        insert_index,
                        pages: inserted_pages,
                    });
                }
            }
        }
    });

    (thumb_tx, texture_tx, file_tx, event_rx)
}
