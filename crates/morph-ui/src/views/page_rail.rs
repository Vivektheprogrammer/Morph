use crate::icons;
use crate::motion::{self, THUMBNAIL_SELECTION_DURATION};
use crate::state::PageItem;
use crate::theme::colors;
use crate::theme::elevation;
use eframe::egui::{Align2, Color32, FontId, Frame, Layout, Margin, Pos2, Rect, RichText, Rounding, Sense, Stroke, Ui, Vec2};
use std::collections::HashMap;
use std::path::PathBuf;

pub struct PageRailAction {
    pub selected: Option<usize>,
    pub rotate_cw: Option<usize>,
    pub rotate_ccw: Option<usize>,
    pub flip_180: Option<usize>,
    pub duplicate: Option<usize>,
    pub move_up: Option<usize>,
    pub move_down: Option<usize>,
    pub reorder: Option<(usize, usize)>,
    pub delete: Option<usize>,
    pub toggle_sidebar: bool,
    pub request_textures: Vec<(usize, PathBuf)>,
}

pub fn render_page_rail(
    ui: &mut Ui,
    pages: &[PageItem],
    selected_idx: Option<usize>,
    thumb_textures: &HashMap<usize, eframe::egui::TextureHandle>,
    preview_textures: &HashMap<usize, eframe::egui::TextureHandle>,
) -> PageRailAction {
    let mut action = PageRailAction {
        selected: None,
        rotate_cw: None,
        rotate_ccw: None,
        flip_180: None,
        duplicate: None,
        move_up: None,
        move_down: None,
        reorder: None,
        delete: None,
        toggle_sidebar: false,
        request_textures: Vec::new(),
    };

    // Sidebar Header: "PAGES", Count, and Collapse button
    Frame::none()
        .fill(colors::BG_SECONDARY)
        .inner_margin(Margin {
            left: 12.0,
            right: 8.0,
            top: 7.0,
            bottom: 7.0,
        })
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("PAGES")
                        .size(11.0)
                        .strong()
                        .color(colors::TEXT_SECONDARY),
                );

                if !pages.is_empty() {
                    ui.label(
                        RichText::new(format!("({})", pages.len()))
                            .size(11.0)
                            .color(colors::TEXT_MUTED),
                    );
                }

                ui.with_layout(Layout::right_to_left(eframe::egui::Align::Center), |ui| {
                    let collapse_resp = Frame::none()
                        .rounding(Rounding::same(3.0))
                        .inner_margin(Margin::same(3.0))
                        .show(ui, |ui| {
                            icons::icon_chevron_left(ui, 12.0, colors::TEXT_SECONDARY);
                        })
                        .response
                        .interact(Sense::click());

                    if collapse_resp.on_hover_text("Collapse sidebar (Ctrl+B)").clicked() {
                        action.toggle_sidebar = true;
                    }
                });
            });
        });

    let (sep_rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
    ui.painter().line_segment(
        [sep_rect.min, Pos2::new(sep_rect.max.x, sep_rect.min.y)],
        Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );

    ui.add_space(4.0);

    // Track dragging across cards using egui memory
    let drag_id = ui.make_persistent_id("page_rail_dragged_idx");
    let mut current_drag: Option<usize> = ui.memory(|m| m.data.get_temp(drag_id));

    // Virtualized ScrollArea: each row is 134px high (compact, sleek Fluent card)
    let card_height = 134.0_f32;
    let available_h = ui.available_height().max(100.0);

    eframe::egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .max_height(available_h)
        .show_rows(ui, card_height, pages.len(), |ui, row_range| {
            ui.spacing_mut().item_spacing.y = 6.0;

            for idx in row_range {
                let page = &pages[idx];
                let is_selected = selected_idx == Some(idx);
                let card_id = ui.make_persistent_id(format!("page_card_{}_{}", page.id, idx));

                let (card_rect, card_resp) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width() - 4.0, card_height),
                    Sense::click_and_drag(),
                );

                let is_hovered = card_resp.hovered();

                // Smooth selection & hover transition
                let sel_t = ui.ctx().animate_bool_with_time(card_id.with("sel"), is_selected, THUMBNAIL_SELECTION_DURATION);
                let sel_factor = motion::ease_standard(sel_t);

                // Card surface background & border
                let card_bg = if is_selected {
                    colors::SURFACE_CARD_SELECTED
                } else if is_hovered {
                    colors::SURFACE_HOVER
                } else {
                    colors::BG_SECONDARY
                };

                let card_stroke = if is_selected {
                    Stroke::new(1.5_f32, colors::ACCENT)
                } else if is_hovered {
                    Stroke::new(1.0_f32, colors::BORDER_MEDIUM)
                } else {
                    Stroke::new(1.0_f32, colors::BORDER_SUBTLE)
                };

                ui.painter().rect_filled(card_rect, Rounding::same(6.0), card_bg);
                ui.painter().rect_stroke(card_rect, Rounding::same(6.0), card_stroke);

                // Thumbnail sheet inside card
                let max_thumb_size = Vec2::new(104.0, 88.0);
                let slot_center = Pos2::new(card_rect.center().x, card_rect.min.y + 52.0);

                if !thumb_textures.contains_key(&page.id) && !preview_textures.contains_key(&page.id) {
                    if let Some(path) = page.thumbnail_path.as_ref().or(page.highres_path.as_ref()) {
                        action.request_textures.push((page.id, path.to_path_buf()));
                    }
                }

                let tex_opt = thumb_textures.get(&page.id).or_else(|| preview_textures.get(&page.id));

                let raw_aspect = if let Some(tex) = tex_opt {
                    let s = tex.size_vec2();
                    if s.y > 0.0 { s.x / s.y } else { 0.707 }
                } else {
                    0.707
                };

                let deg_int = ((page.rotation_degrees % 360) + 360) % 360;
                let is_rotated_sideways = deg_int == 90 || deg_int == 270;
                let aspect = if is_rotated_sideways {
                    1.0 / raw_aspect
                } else {
                    raw_aspect
                };

                let (thumb_w, thumb_h) = if aspect > (max_thumb_size.x / max_thumb_size.y) {
                    (max_thumb_size.x, max_thumb_size.x / aspect)
                } else {
                    (max_thumb_size.y * aspect, max_thumb_size.y)
                };

                let sheet_rect = Rect::from_center_size(slot_center, Vec2::new(thumb_w, thumb_h));

                // 1. Drop shadow behind the miniature sheet
                ui.painter().add(elevation::shadow_card().as_shape(sheet_rect, 1.0));

                // 2. White sheet background
                ui.painter().rect_filled(sheet_rect, 1.0, colors::DOCUMENT_SHEET);
                ui.painter().rect_stroke(
                    sheet_rect,
                    1.0,
                    Stroke::new(1.0_f32, Color32::from_black_alpha(40)),
                );

                // 3. Render thumbnail image onto sheet with GPU rotation (0 flicker)
                if let Some(tex) = tex_opt {
                    if page.rotation_degrees != 0 {
                        let rad = (page.rotation_degrees as f32).to_radians();
                        let unrotated_size = if is_rotated_sideways {
                            Vec2::new(thumb_h, thumb_w)
                        } else {
                            Vec2::new(thumb_w, thumb_h)
                        };
                        let img = eframe::egui::Image::new(tex)
                            .fit_to_exact_size(unrotated_size)
                            .rounding(1.0)
                            .rotate(rad, Vec2::splat(0.5));
                        let unrotated_rect = Rect::from_center_size(sheet_rect.center(), unrotated_size);
                        ui.allocate_new_ui(eframe::egui::UiBuilder::new().max_rect(unrotated_rect), |ui| {
                            ui.add(img);
                        });
                    } else {
                        ui.painter().image(
                            tex.id(),
                            sheet_rect,
                            Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    }
                } else {
                    // Minimal placeholder
                    ui.painter().text(
                        sheet_rect.center(),
                        Align2::CENTER_CENTER,
                        format!("{}", idx + 1),
                        eframe::egui::FontId::proportional(13.0),
                        Color32::from_rgb(148, 163, 184),
                    );
                }

                // 4. Rotation badge (shown only when rotated > 0°)
                if page.rotation_degrees != 0 {
                    let badge_rect = Rect::from_min_size(
                        Pos2::new(sheet_rect.max.x - 26.0, sheet_rect.min.y + 2.0),
                        Vec2::new(24.0, 13.0),
                    );
                    ui.painter().rect_filled(badge_rect, 2.0, colors::SURFACE_ELEVATED);
                    ui.painter().rect_stroke(badge_rect, 2.0, Stroke::new(1.0_f32, colors::ACCENT));
                    ui.painter().text(
                        badge_rect.center(),
                        Align2::CENTER_CENTER,
                        format!("{}°", page.rotation_degrees),
                        eframe::egui::FontId::proportional(8.5),
                        colors::ACCENT,
                    );
                }

                // 5. Page Number Badge below thumbnail
                let page_label = format!("{}", idx + 1);
                let label_color = motion::lerp_color(colors::TEXT_SECONDARY, colors::ACCENT, sel_factor);
                let label_pos = Pos2::new(card_rect.center().x, card_rect.max.y - 14.0);

                ui.painter().text(
                    label_pos,
                    Align2::CENTER_CENTER,
                    page_label,
                    FontId::proportional(11.5),
                    label_color,
                );

                // Click to select
                if card_resp.clicked() {
                    action.selected = Some(idx);
                }

                // Drag-and-drop reordering handling
                if card_resp.drag_started() {
                    current_drag = Some(idx);
                }

                if let Some(dragged_from) = current_drag {
                    if dragged_from != idx && card_rect.contains(ui.input(|i| i.pointer.hover_pos().unwrap_or(Pos2::ZERO))) {
                        // Draw drag drop insertion indicator line
                        let indicator_y = if dragged_from < idx { card_rect.max.y } else { card_rect.min.y };
                        ui.painter().line_segment(
                            [Pos2::new(card_rect.min.x, indicator_y), Pos2::new(card_rect.max.x, indicator_y)],
                            Stroke::new(2.0_f32, colors::ACCENT),
                        );
                        if ui.input(|i| i.pointer.any_released()) {
                            action.reorder = Some((dragged_from, idx));
                            current_drag = None;
                        }
                    }
                }

                // Context Menu on Right Click (Acrobat style)
                card_resp.context_menu(|ui| {
                    ui.set_min_width(170.0);

                    if ui.button("Rotate Clockwise 90°").clicked() {
                        action.rotate_cw = Some(idx);
                        ui.close_menu();
                    }
                    if ui.button("Rotate Counter-Clockwise 90°").clicked() {
                        action.rotate_ccw = Some(idx);
                        ui.close_menu();
                    }
                    if ui.button("Flip 180°").clicked() {
                        action.flip_180 = Some(idx);
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("Duplicate Page").clicked() {
                        action.duplicate = Some(idx);
                        ui.close_menu();
                    }

                    if idx > 0 {
                        if ui.button("Move Up").clicked() {
                            action.move_up = Some(idx);
                            ui.close_menu();
                        }
                    }

                    if idx + 1 < pages.len() {
                        if ui.button("Move Down").clicked() {
                            action.move_down = Some(idx);
                            ui.close_menu();
                        }
                    }

                    ui.separator();

                    let del_btn = eframe::egui::Button::new(
                        RichText::new("Delete Page")
                            .color(colors::DESTRUCTIVE),
                    );
                    if ui.add(del_btn).clicked() {
                        action.delete = Some(idx);
                        ui.close_menu();
                    }
                });
            }
        });

    if ui.input(|i| i.pointer.any_released()) {
        current_drag = None;
    }
    ui.memory_mut(|m| m.data.insert_temp(drag_id, current_drag));

    action
}
