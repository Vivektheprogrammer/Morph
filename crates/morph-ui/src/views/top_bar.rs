#![allow(dead_code)]

use crate::icons;
use crate::state::InsertPosition;
use crate::theme::colors;
use crate::theme::elevation;
use eframe::egui::{self, Align, Color32, Frame, Layout, Margin, RichText, Rounding, Sense, Stroke, Ui, Vec2};
use std::path::PathBuf;

pub struct TopBarAction {
    pub undo: bool,
    pub redo: bool,
    pub export: bool,
    pub add_files_clicked: bool,
    pub jump_to_file: Option<PathBuf>,
    pub remove_file: Option<PathBuf>,
    pub new_insert_pos: Option<InsertPosition>,
}

pub fn render_top_bar(
    ui: &mut Ui,
    logo_texture: Option<&egui::TextureHandle>,
    can_undo: bool,
    can_redo: bool,
    pages_count: usize,
    source_files: &[(PathBuf, bool, usize)],
    insert_position: InsertPosition,
    selected_page_idx: Option<usize>,
    shelf_scroll_offset: &mut f32,
) -> TopBarAction {
    let mut action = TopBarAction {
        undo: false,
        redo: false,
        export: false,
        add_files_clicked: false,
        jump_to_file: None,
        remove_file: None,
        new_insert_pos: None,
    };

    ui.add_space(4.0);

    // ROW 1: Branding, Undo/Redo, and Export
    ui.horizontal(|ui| {
        if let Some(logo) = logo_texture {
            ui.image((logo.id(), Vec2::new(26.0, 26.0)));
            ui.add_space(4.0);
        }
        ui.heading(RichText::new("Morph").strong().color(colors::ACCENT_CYAN));
        ui.label(RichText::new("Editor").strong().color(colors::ACCENT_AMBER));

        ui.add_space(14.0);
        ui.separator();
        ui.add_space(10.0);

        // Styled Undo Button with Vector Icon
        let undo_frame = Frame::none()
            .fill(if can_undo { colors::SURFACE_ELEVATED } else { colors::SURFACE_BASE })
            .stroke(Stroke::new(1.0_f32, if can_undo { colors::BORDER_MEDIUM } else { colors::BORDER_SUBTLE }))
            .rounding(Rounding::same(6.0))
            .inner_margin(Margin::symmetric(8.0, 4.0));

        let icon_color = if can_undo { colors::TEXT_PRIMARY } else { colors::TEXT_MUTED };
        let undo_resp = undo_frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
                icons::icon_undo(ui, 13.0, icon_color);
                ui.label(RichText::new("Undo").size(11.0).strong().color(icon_color));
            });
        }).response.interact(Sense::click());

        if can_undo && undo_resp.on_hover_text("Undo last change (Ctrl+Z)").clicked() {
            action.undo = true;
        }

        ui.add_space(4.0);

        // Styled Redo Button with Vector Icon
        let redo_frame = Frame::none()
            .fill(if can_redo { colors::SURFACE_ELEVATED } else { colors::SURFACE_BASE })
            .stroke(Stroke::new(1.0_f32, if can_redo { colors::BORDER_MEDIUM } else { colors::BORDER_SUBTLE }))
            .rounding(Rounding::same(6.0))
            .inner_margin(Margin::symmetric(8.0, 4.0));

        let icon_color = if can_redo { colors::TEXT_PRIMARY } else { colors::TEXT_MUTED };
        let redo_resp = redo_frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
                icons::icon_redo(ui, 13.0, icon_color);
                ui.label(RichText::new("Redo").size(11.0).strong().color(icon_color));
            });
        }).response.interact(Sense::click());

        if can_redo && redo_resp.on_hover_text("Redo last change (Ctrl+Y)").clicked() {
            action.redo = true;
        }

        // Empty document prompt
        if pages_count == 0 {
            ui.add_space(8.0);
            let add_btn = egui::Button::new(RichText::new("+ Add Files...").strong().size(12.0).color(Color32::WHITE))
                .fill(colors::ACCENT_PRIMARY)
                .stroke(Stroke::new(1.0_f32, colors::ACCENT_LIGHT))
                .rounding(Rounding::same(6.0))
                .min_size(Vec2::new(100.0, 28.0));
            if ui.add(add_btn).clicked() {
                action.add_files_clicked = true;
            }
        }

        // Right aligned Export Button & Page Count
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let export_enabled = pages_count > 0;
            let export_btn = egui::Button::new(
                RichText::new("Export PDF")
                    .strong()
                    .color(Color32::WHITE)
                    .size(12.5),
            )
            .fill(if export_enabled { colors::ACCENT_PRIMARY } else { colors::SURFACE_BASE })
            .stroke(if export_enabled { Stroke::new(1.0_f32, colors::ACCENT_LIGHT) } else { Stroke::new(1.0_f32, colors::BORDER_SUBTLE) })
            .rounding(Rounding::same(6.0))
            .min_size(Vec2::new(112.0, 30.0));

            if ui.add_enabled(export_enabled, export_btn).on_hover_text("Save modified document as a new PDF").clicked() {
                action.export = true;
            }

            if export_enabled {
                ui.label(
                    RichText::new(format!("{} Page(s)", pages_count))
                        .size(11.5)
                        .color(colors::TEXT_SECONDARY),
                );
            }
        });
    });

    // ROW 2: File Sequence Shelf & Insertion Controls
    if pages_count > 0 {
        ui.add_space(5.0);
        ui.separator();
        ui.add_space(3.0);

        elevation::frame_shelf().show(ui, |ui| {
            ui.set_height(34.0);

            let total_width = ui.available_width();
            let right_controls_width = 410.0_f32.min(total_width * 0.65);
            let left_area_width = (total_width - right_controls_width - 12.0).max(60.0);

            ui.horizontal(|ui| {
                // Left Area: File chips & navigation
                ui.allocate_ui_with_layout(
                    Vec2::new(left_area_width, 34.0),
                    Layout::left_to_right(Align::Center),
                    |ui| {
                        let mut clip_rect = ui.max_rect();
                        clip_rect.max.x = clip_rect.max.x.min(ui.min_rect().min.x + left_area_width);
                        ui.set_clip_rect(clip_rect);

                        // Dropdown button for all files
                        ui.menu_button(
                            RichText::new(format!("📁 Files ({}) ▾", source_files.len()))
                                .size(11.0)
                                .strong()
                                .color(colors::TEXT_PRIMARY),
                            |ui| {
                                ui.set_min_width(280.0);
                                ui.add_space(3.0);
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("📂 Loaded Documents").size(11.5).strong().color(Color32::WHITE));
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        let total_p: usize = source_files.iter().map(|(_, _, c)| *c).sum();
                                        ui.label(RichText::new(format!("{} pages", total_p)).size(10.0).color(colors::TEXT_MUTED));
                                    });
                                });
                                ui.add_space(2.0);
                                ui.separator();
                                ui.add_space(4.0);

                                egui::ScrollArea::vertical().max_height(250.0).show(ui, |ui| {
                                    for (path, is_image, count) in source_files {
                                        let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
                                        let (tag_bg, tag_txt, tag_label) = if *is_image {
                                            (colors::BADGE_IMG_BG, colors::BADGE_IMG_TXT, "IMG")
                                        } else {
                                            (colors::BADGE_PDF_BG, colors::BADGE_PDF_TXT, "PDF")
                                        };

                                        ui.horizontal(|ui| {
                                            Frame::none()
                                                .fill(tag_bg)
                                                .rounding(Rounding::same(3.0))
                                                .inner_margin(Margin::symmetric(4.0, 1.5))
                                                .show(ui, |ui| {
                                                    ui.label(RichText::new(tag_label).size(8.5).strong().color(tag_txt));
                                                });
                                            ui.add_space(4.0);

                                            let short_name = if fname.chars().count() > 22 {
                                                let prefix: String = fname.chars().take(17).collect();
                                                let ext = path.extension().and_then(|e| e.to_str()).map(|e| format!(".{}", e)).unwrap_or_default();
                                                format!("{}...{}", prefix, ext)
                                            } else {
                                                fname.to_string()
                                            };

                                            if ui.button(RichText::new(short_name).size(11.0).color(colors::TEXT_PRIMARY))
                                                .on_hover_text(format!("{}\nClick to jump to page 1", path.display()))
                                                .clicked()
                                            {
                                                action.jump_to_file = Some(path.clone());
                                                ui.close_menu();
                                            }

                                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                let del_btn = egui::Button::new(RichText::new("✕").size(9.0).strong().color(colors::ACCENT_CRIMSON))
                                                    .fill(colors::SURFACE_HOVER)
                                                    .stroke(Stroke::NONE)
                                                    .rounding(Rounding::same(6.0))
                                                    .min_size(Vec2::new(16.0, 16.0));
                                                if ui.add(del_btn).on_hover_text(format!("Remove all pages from '{}'", fname)).clicked() {
                                                    action.remove_file = Some(path.clone());
                                                    ui.close_menu();
                                                }

                                                let p_str = if *is_image { "1 pg".to_string() } else { format!("{} pgs", count) };
                                                ui.label(RichText::new(p_str).size(9.5).color(colors::TEXT_MUTED));
                                            });
                                        });
                                        ui.add_space(2.0);
                                    }
                                });
                            },
                        );

                        // Chevrons for horizontal scroll
                        if source_files.len() > 1 {
                            ui.add_space(2.0);
                            let left_chev = egui::Button::new(RichText::new("‹").size(13.0).strong().color(colors::TEXT_SECONDARY))
                                .fill(colors::SURFACE_ELEVATED)
                                .stroke(Stroke::new(1.0_f32, colors::BORDER_MEDIUM))
                                .rounding(Rounding::same(4.0))
                                .min_size(Vec2::new(18.0, 22.0));
                            if ui.add(left_chev).on_hover_text("Scroll files left").clicked() {
                                *shelf_scroll_offset = (*shelf_scroll_offset - 160.0).max(0.0);
                            }

                            let right_chev = egui::Button::new(RichText::new("›").size(13.0).strong().color(colors::TEXT_SECONDARY))
                                .fill(colors::SURFACE_ELEVATED)
                                .stroke(Stroke::new(1.0_f32, colors::BORDER_MEDIUM))
                                .rounding(Rounding::same(4.0))
                                .min_size(Vec2::new(18.0, 22.0));
                            if ui.add(right_chev).on_hover_text("Scroll files right").clicked() {
                                *shelf_scroll_offset += 160.0;
                            }
                        }

                        ui.add_space(4.0);

                        let chip_scroll_w = (ui.available_width() - 4.0).max(20.0);
                        let scroll_out = egui::ScrollArea::horizontal()
                            .auto_shrink([false, true])
                            .max_width(chip_scroll_w)
                            .max_height(30.0)
                            .drag_to_scroll(true)
                            .horizontal_scroll_offset(*shelf_scroll_offset)
                            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    for (path, is_image, count) in source_files {
                                        let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
                                        let display_name = if fname.chars().count() > 18 {
                                            let prefix: String = fname.chars().take(13).collect();
                                            let ext = path.extension().and_then(|e| e.to_str()).map(|e| format!(".{}", e)).unwrap_or_default();
                                            format!("{}...{}", prefix, ext)
                                        } else {
                                            fname.to_string()
                                        };

                                        let (tag_bg, tag_txt, tag_label) = if *is_image {
                                            (colors::BADGE_IMG_BG, colors::BADGE_IMG_TXT, "IMG")
                                        } else {
                                            (colors::BADGE_PDF_BG, colors::BADGE_PDF_TXT, "PDF")
                                        };

                                        Frame::none()
                                            .fill(colors::SURFACE_ELEVATED)
                                            .stroke(Stroke::new(1.0_f32, colors::BORDER_MEDIUM))
                                            .rounding(Rounding::same(6.0))
                                            .inner_margin(Margin::symmetric(7.0, 3.0))
                                            .show(ui, |ui| {
                                                ui.horizontal(|ui| {
                                                    Frame::none()
                                                        .fill(tag_bg)
                                                        .rounding(Rounding::same(3.0))
                                                        .inner_margin(Margin::symmetric(4.0, 1.0))
                                                        .show(ui, |ui| {
                                                            ui.label(RichText::new(tag_label).size(8.5).strong().color(tag_txt));
                                                        });
                                                    ui.add_space(3.0);

                                                    let lbl = ui.add(
                                                        egui::Label::new(
                                                            RichText::new(&display_name)
                                                                .size(10.5)
                                                                .strong()
                                                                .color(colors::TEXT_PRIMARY),
                                                        ).sense(Sense::click()),
                                                    ).on_hover_text(format!("{}\nClick to jump to this document", path.display()));

                                                    if lbl.clicked() {
                                                        action.jump_to_file = Some(path.clone());
                                                    }

                                                    ui.add_space(3.0);

                                                    let p_str = if *is_image { "1 pg".to_string() } else { format!("{} pgs", count) };
                                                    Frame::none()
                                                        .fill(colors::SURFACE_BASE)
                                                        .rounding(Rounding::same(3.0))
                                                        .inner_margin(Margin::symmetric(4.0, 1.0))
                                                        .show(ui, |ui| {
                                                            ui.label(RichText::new(p_str).size(9.0).color(colors::TEXT_MUTED));
                                                        });

                                                    ui.add_space(2.0);

                                                    let del_btn = egui::Button::new(RichText::new("✕").size(8.5).strong().color(colors::TEXT_MUTED))
                                                        .fill(colors::SURFACE_HOVER)
                                                        .stroke(Stroke::NONE)
                                                        .rounding(Rounding::same(6.0))
                                                        .min_size(Vec2::new(14.0, 14.0));
                                                    if ui.add(del_btn).on_hover_text(format!("Remove all pages from '{}'", fname)).clicked() {
                                                        action.remove_file = Some(path.clone());
                                                    }
                                                });
                                            });
                                        ui.add_space(4.0);
                                    }
                                });
                            });

                        let strip_hovered = scroll_out.inner_rect.contains(ui.input(|i| i.pointer.hover_pos().unwrap_or(egui::Pos2::ZERO)));
                        let wheel_y = ui.input(|i| i.raw_scroll_delta.y);
                        if strip_hovered && wheel_y.abs() > 0.1 {
                            *shelf_scroll_offset = (*shelf_scroll_offset - wheel_y * 1.5).max(0.0);
                        }
                        let max_scroll = (scroll_out.content_size.x - chip_scroll_w).max(0.0);
                        *shelf_scroll_offset = (*shelf_scroll_offset).min(max_scroll);
                    },
                );

                ui.add_space(2.0);
                ui.add(egui::Separator::default().vertical());
                ui.add_space(2.0);

                // Right Area: Insert position segmented control & Add button
                ui.allocate_ui_with_layout(
                    Vec2::new(right_controls_width - 8.0, 34.0),
                    Layout::right_to_left(Align::Center),
                    |ui| {
                        let add_btn = egui::Button::new(
                            RichText::new("+ Add Files...")
                                .strong()
                                .size(11.0)
                                .color(Color32::WHITE),
                        )
                        .fill(colors::ACCENT_PRIMARY)
                        .stroke(Stroke::new(1.0_f32, colors::ACCENT_LIGHT))
                        .rounding(Rounding::same(6.0))
                        .min_size(Vec2::new(94.0, 24.0));

                        if ui.add(add_btn).on_hover_text("Select PDF or images to insert").clicked() {
                            action.add_files_clicked = true;
                        }

                        ui.add_space(6.0);

                        // Segmented Capsule Dock
                        Frame::none()
                            .fill(colors::SURFACE_BASE)
                            .stroke(Stroke::new(1.0_f32, colors::BORDER_MEDIUM))
                            .rounding(Rounding::same(6.0))
                            .inner_margin(Margin::same(2.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing = Vec2::ZERO;

                                    let is_end = insert_position == InsertPosition::End;
                                    let end_btn = egui::Button::new(
                                        RichText::new("⏭ At End")
                                            .size(10.0)
                                            .strong()
                                            .color(if is_end { Color32::WHITE } else { colors::TEXT_SECONDARY }),
                                    )
                                    .fill(if is_end { colors::ACCENT_PRIMARY } else { Color32::TRANSPARENT })
                                    .stroke(if is_end { Stroke::new(1.0_f32, colors::ACCENT_LIGHT) } else { Stroke::NONE })
                                    .rounding(Rounding::same(5.0))
                                    .min_size(Vec2::new(60.0, 20.0));
                                    if ui.add(end_btn).on_hover_text("Append new files to the end").clicked() {
                                        action.new_insert_pos = Some(InsertPosition::End);
                                    }

                                    let cur_p = selected_page_idx.map(|i| i + 1).unwrap_or(1);
                                    let is_mid = insert_position == InsertPosition::AfterSelected;
                                    let mid_btn = egui::Button::new(
                                        RichText::new(format!("⏺ Middle (p.{})", cur_p))
                                            .size(10.0)
                                            .strong()
                                            .color(if is_mid { Color32::WHITE } else { colors::TEXT_SECONDARY }),
                                    )
                                    .fill(if is_mid { colors::ACCENT_PRIMARY } else { Color32::TRANSPARENT })
                                    .stroke(if is_mid { Stroke::new(1.0_f32, colors::ACCENT_LIGHT) } else { Stroke::NONE })
                                    .rounding(Rounding::same(5.0))
                                    .min_size(Vec2::new(78.0, 20.0));
                                    if ui.add(mid_btn).on_hover_text(format!("Insert right after Page {}", cur_p)).clicked() {
                                        action.new_insert_pos = Some(InsertPosition::AfterSelected);
                                    }

                                    let is_start = insert_position == InsertPosition::Start;
                                    let start_btn = egui::Button::new(
                                        RichText::new("⏮ At Start")
                                            .size(10.0)
                                            .strong()
                                            .color(if is_start { Color32::WHITE } else { colors::TEXT_SECONDARY }),
                                    )
                                    .fill(if is_start { colors::ACCENT_PRIMARY } else { Color32::TRANSPARENT })
                                    .stroke(if is_start { Stroke::new(1.0_f32, colors::ACCENT_LIGHT) } else { Stroke::NONE })
                                    .rounding(Rounding::same(5.0))
                                    .min_size(Vec2::new(64.0, 20.0));
                                    if ui.add(start_btn).on_hover_text("Insert at beginning, before Page 1").clicked() {
                                        action.new_insert_pos = Some(InsertPosition::Start);
                                    }
                                });
                            });

                        ui.add_space(4.0);
                        ui.label(RichText::new("Insert:").size(10.5).color(colors::TEXT_MUTED));
                    },
                );
            });
        });
    }

    action
}
