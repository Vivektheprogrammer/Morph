use crate::state::PageItem;
use crate::theme::colors;
use crate::theme::elevation;
use eframe::egui::{self, Align, Color32, Layout, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2};
use std::collections::HashMap;
use std::path::PathBuf;

pub struct CanvasAction {
    pub request_preview: Option<(usize, PathBuf)>,
}

pub fn render_canvas(
    ui: &mut Ui,
    page: &PageItem,
    zoom_level: f32,
    visual_rotation: f32,
    preview_textures: &HashMap<usize, egui::TextureHandle>,
    thumb_textures: &HashMap<usize, egui::TextureHandle>,
) -> CanvasAction {
    let mut action = CanvasAction {
        request_preview: None,
    };

    // Request high-res preview if not already decoded
    if !preview_textures.contains_key(&page.id) {
        if let Some(path) = page.highres_path.as_ref().or(page.thumbnail_path.as_ref()) {
            action.request_preview = Some((page.id, path.to_path_buf()));
        }
    }

    let tex_opt = preview_textures.get(&page.id).or_else(|| thumb_textures.get(&page.id));

    let base_aspect = if let Some(tex) = tex_opt {
        let tex_size = tex.size_vec2();
        if tex_size.y > 0.0 {
            tex_size.x / tex_size.y
        } else {
            0.707
        }
    } else {
        0.707
    };

    let deg_int = ((visual_rotation.round() as i32 % 360) + 360) % 360;
    let is_rotated_sideways = deg_int == 90 || deg_int == 270;
    let aspect = if is_rotated_sideways {
        1.0 / base_aspect
    } else {
        base_aspect
    };

    // Responsive document dimensions (Fit to window height by default at 100% zoom)
    let margin = 48.0_f32;
    let fit_height = (ui.available_height() - margin).max(240.0);
    let fit_width_limit = (ui.available_width() - margin).max(240.0);

    // If landscape page would overflow horizontally at 100% zoom, fit to width instead
    let base_height = if (fit_height * aspect) > fit_width_limit {
        fit_width_limit / aspect
    } else {
        fit_height
    };

    let target_height = base_height * zoom_level;
    let target_width = target_height * aspect;

    // Dark canvas background
    ui.painter().rect_filled(ui.max_rect(), 0.0, colors::CANVAS_WORKSPACE);

    let canvas_bounds = ui.max_rect();
    let is_zoomed_h = target_width > (canvas_bounds.width() - 20.0);
    let is_zoomed_v = target_height > (canvas_bounds.height() - 20.0);

    // ScrollArea allows smooth scrolling when zoomed in beyond viewport
    egui::ScrollArea::both()
        .vscroll(is_zoomed_v)
        .hscroll(is_zoomed_h)
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            let total_content_width = if is_zoomed_h { target_width + 48.0 } else { ui.available_width() };
            let total_content_height = if is_zoomed_v { target_height + 48.0 } else { ui.available_height() };

            ui.allocate_ui_with_layout(
                Vec2::new(total_content_width, total_content_height),
                Layout::top_down(Align::Center),
                |ui| {
                    let vert_pad = ((total_content_height - target_height) * 0.5).max(20.0);
                    ui.add_space(vert_pad);

                    // Allocate EXACT size for the sheet
                    let (sheet_rect, _) = ui.allocate_exact_size(
                        Vec2::new(target_width, target_height),
                        Sense::hover(),
                    );

                    // 1. Acrobat-grade realistic drop shadow behind sheet
                    let shadow = elevation::shadow_document();
                    ui.painter().add(shadow.as_shape(sheet_rect, 2.0));

                    // 2. White document sheet background
                    ui.painter().rect_filled(sheet_rect, 2.0, colors::DOCUMENT_SHEET);
                    ui.painter().rect_stroke(
                        sheet_rect,
                        2.0,
                        Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
                    );

                    // 3. Render page content smoothly on GPU with zero flickering
                    let rad = visual_rotation.to_radians();

                    if let Some(tex) = tex_opt {
                        if rad.abs() > 0.001 {
                            // Smooth hardware-accelerated rotation
                            let unrotated_size = if is_rotated_sideways {
                                Vec2::new(target_height, target_width)
                            } else {
                                Vec2::new(target_width, target_height)
                            };
                            let img = egui::Image::new(tex)
                                .fit_to_exact_size(unrotated_size)
                                .rounding(1.0)
                                .rotate(rad, Vec2::splat(0.5));
                            let unrotated_rect = Rect::from_center_size(sheet_rect.center(), unrotated_size);
                            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(unrotated_rect), |ui| {
                                ui.add(img);
                            });
                        } else {
                            // Direct crisp texture paint
                            ui.painter().image(
                                tex.id(),
                                sheet_rect,
                                Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                                Color32::WHITE,
                            );
                        }
                    } else if let Some(ref text) = page.text_snippet {
                        // Miniature Document Reader Paper
                        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(sheet_rect), |ui| {
                            ui.vertical(|ui| {
                                ui.add_space(16.0 * zoom_level.clamp(0.6, 1.5));
                                ui.horizontal(|ui| {
                                    ui.add_space(16.0);
                                    ui.label(
                                        RichText::new(format!("PAGE {}", page.original_page_num))
                                            .strong()
                                            .size(12.0 * zoom_level.clamp(0.8, 1.5))
                                            .color(Color32::from_rgb(71, 85, 105)),
                                    );
                                });
                                ui.add_space(8.0);
                                let (sep_r, _) = ui.allocate_exact_size(Vec2::new(target_width - 32.0, 1.0), Sense::hover());
                                ui.painter().line_segment([sep_r.min, sep_r.max], Stroke::new(1.0_f32, Color32::from_rgb(226, 232, 240)));
                                ui.add_space(12.0);
                                ui.horizontal(|ui| {
                                    ui.add_space(16.0);
                                    ui.label(
                                        RichText::new(text)
                                            .size(11.5 * zoom_level.clamp(0.8, 1.5))
                                            .color(Color32::from_rgb(15, 23, 42)),
                                    );
                                });
                            });
                        });
                    } else {
                        // Clean loading card inside sheet
                        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(sheet_rect), |ui| {
                            ui.centered_and_justified(|ui| {
                                ui.vertical_centered(|ui| {
                                    ui.spinner();
                                    ui.add_space(8.0);
                                    ui.label(
                                        RichText::new(format!("Rendering Page {}...", page.original_page_num))
                                            .size(12.0)
                                            .color(Color32::from_rgb(100, 116, 139)),
                                    );
                                });
                            });
                        });
                    }

                    ui.add_space(vert_pad);
                },
            );
        });

    action
}
