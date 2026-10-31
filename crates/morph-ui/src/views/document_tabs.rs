use crate::icons;
use crate::motion::{self, TAB_SWITCH_DURATION};
use crate::state::DocumentState;
use crate::theme::colors;
use eframe::egui::{Align2, Color32, FontId, Frame, Margin, Pos2, Rect, Rounding, Sense, Stroke, Ui, Vec2};

pub struct DocumentTabsAction {
    pub open_new_document: bool,
    pub switch_to_document: Option<usize>,
    pub close_document: Option<usize>,
}

pub fn render_document_tabs(
    ui: &mut Ui,
    documents: &[DocumentState],
    active_doc_id: Option<usize>,
) -> DocumentTabsAction {
    let mut action = DocumentTabsAction {
        open_new_document: false,
        switch_to_document: None,
        close_document: None,
    };

    if documents.is_empty() {
        return action;
    }

    Frame::none()
        .fill(colors::BG_SECONDARY)
        .stroke(Stroke::new(1.0_f32, colors::BORDER_SUBTLE))
        .inner_margin(Margin {
            left: 10.0,
            right: 10.0,
            top: 4.0,
            bottom: 0.0,
        })
        .show(ui, |ui| {
            ui.set_height(30.0);

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(3.0, 0.0);

                eframe::egui::ScrollArea::horizontal()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(3.0, 0.0);

                            for doc in documents {
                                let is_active = active_doc_id == Some(doc.id);
                                let doc_id_key = ui.make_persistent_id(format!("tab_{}", doc.id));

                                // Intelligent middle-truncation for filenames
                                let fname = &doc.file_name;
                                let char_count = fname.chars().count();
                                let display_name = if char_count > 22 {
                                    let ext = doc.file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
                                    let prefix: String = fname.chars().take(12).collect();
                                    let suffix: String = if !ext.is_empty() {
                                        let stem_end: String = fname.chars().skip(char_count.saturating_sub(6)).collect();
                                        stem_end
                                    } else {
                                        fname.chars().skip(char_count.saturating_sub(5)).collect()
                                    };
                                    format!("{}...{}", prefix, suffix)
                                } else {
                                    fname.clone()
                                };

                                // Calculate tab width: icon (12) + gap (6) + text + gap (6) + close button (18) + margins (16)
                                let font_id = FontId::proportional(11.5);
                                let text_width = ui.fonts(|f| {
                                    f.layout_no_wrap(display_name.clone(), font_id.clone(), Color32::WHITE).size().x
                                });
                                let tab_width = (text_width + 58.0).clamp(110.0, 210.0);
                                let tab_height = 27.0;

                                let (tab_rect, tab_resp) = ui.allocate_exact_size(
                                    Vec2::new(tab_width, tab_height),
                                    Sense::click(),
                                );

                                let tab_hovered = tab_resp.hovered();
                                let close_rect = Rect::from_center_size(
                                    Pos2::new(tab_rect.max.x - 14.0, tab_rect.center().y),
                                    Vec2::splat(18.0),
                                );

                                let pointer_pos = ui.input(|i| i.pointer.interact_pos().or(i.pointer.hover_pos()));
                                let close_hovered = pointer_pos.map_or(false, |p| close_rect.contains(p));

                                // Tab background animation
                                let active_t = ui.ctx().animate_bool_with_time(
                                    doc_id_key.with("active"),
                                    is_active,
                                    TAB_SWITCH_DURATION,
                                );
                                let active_factor = motion::ease_standard(active_t);

                                let tab_bg = if is_active {
                                    colors::SURFACE_ELEVATED
                                } else if tab_hovered {
                                    colors::SURFACE_HOVER
                                } else {
                                    colors::BG_SECONDARY
                                };

                                let tab_border = if is_active {
                                    colors::BORDER_SUBTLE
                                } else if tab_hovered {
                                    colors::BORDER_SUBTLE
                                } else {
                                    Color32::TRANSPARENT
                                };

                                // 1. Paint tab surface
                                let tab_rounding = Rounding {
                                    nw: 4.0,
                                    ne: 4.0,
                                    sw: 0.0,
                                    se: 0.0,
                                };
                                ui.painter().rect_filled(tab_rect, tab_rounding, tab_bg);
                                if tab_border != Color32::TRANSPARENT {
                                    ui.painter().rect_stroke(tab_rect, tab_rounding, Stroke::new(1.0_f32, tab_border));
                                }

                                // 2. Paint Document Icon
                                let icon_x = tab_rect.min.x + 10.0;
                                let icon_center_y = tab_rect.center().y;
                                let icon_color = if is_active {
                                    colors::ACCENT
                                } else {
                                    colors::TEXT_MUTED
                                };

                                icons::paint_icon_pdf(ui.painter(), Pos2::new(icon_x + 6.0, icon_center_y), 12.0, icon_color);

                                // 3. Paint Label
                                let text_color = if is_active {
                                    colors::TEXT_PRIMARY
                                } else if tab_hovered {
                                    colors::TEXT_PRIMARY
                                } else {
                                    colors::TEXT_SECONDARY
                                };

                                let text_start_x = icon_x + 18.0;
                                let text_pos = Pos2::new(text_start_x + (text_width * 0.5), icon_center_y);
                                ui.painter().text(
                                    text_pos,
                                    Align2::CENTER_CENTER,
                                    display_name,
                                    font_id,
                                    text_color,
                                );

                                // 4. Paint Close Button
                                if close_hovered {
                                    ui.painter().rect_filled(
                                        close_rect,
                                        Rounding::same(3.0),
                                        Color32::from_rgb(45, 52, 64),
                                    );
                                }

                                let close_icon_color = if close_hovered {
                                    colors::TEXT_PRIMARY
                                } else if is_active || tab_hovered {
                                    colors::TEXT_SECONDARY
                                } else {
                                    Color32::TRANSPARENT
                                };

                                if close_icon_color != Color32::TRANSPARENT {
                                    icons::paint_icon_cross(ui.painter(), close_rect.center(), 8.5, close_icon_color);
                                }

                                // 5. Paint Bottom Accent Indicator for active tab
                                if active_factor > 0.01 {
                                    let line_y = tab_rect.max.y;
                                    let line_min = Pos2::new(tab_rect.min.x, line_y);
                                    let line_max = Pos2::new(tab_rect.max.x, line_y);
                                    let accent_stroke = Stroke::new(
                                        2.0_f32 * active_factor,
                                        motion::lerp_color(Color32::TRANSPARENT, colors::ACCENT, active_factor),
                                    );
                                    ui.painter().line_segment([line_min, line_max], accent_stroke);
                                }

                                // 6. Handle click events cleanly and independently
                                if tab_resp.clicked() {
                                    if close_hovered {
                                        action.close_document = Some(doc.id);
                                    } else {
                                        action.switch_to_document = Some(doc.id);
                                    }
                                }

                                if tab_resp.middle_clicked() {
                                    action.close_document = Some(doc.id);
                                }

                                tab_resp.on_hover_text(format!(
                                    "{}\nPages: {}\nMiddle-click or click ✕ to close",
                                    doc.file_name,
                                    doc.pages.len()
                                ));
                            }

                            // Clean "+" Open Document button
                            ui.add_space(2.0);
                            let plus_size = Vec2::new(26.0, 26.0);
                            let (plus_rect, plus_resp) = ui.allocate_exact_size(plus_size, Sense::click());
                            let plus_hovered = plus_resp.hovered();

                            let plus_bg = if plus_hovered {
                                colors::SURFACE_HOVER
                            } else {
                                colors::SURFACE_ELEVATED
                            };
                            let plus_border = if plus_hovered {
                                colors::BORDER_MEDIUM
                            } else {
                                colors::BORDER_SUBTLE
                            };

                            ui.painter().rect_filled(plus_rect, Rounding::same(4.0), plus_bg);
                            ui.painter().rect_stroke(plus_rect, Rounding::same(4.0), Stroke::new(1.0_f32, plus_border));

                            let plus_center = plus_rect.center();
                            let plus_stroke = Stroke::new(1.3_f32, if plus_hovered { colors::TEXT_PRIMARY } else { colors::TEXT_SECONDARY });
                            let plus_arm = 4.5_f32;
                            ui.painter().line_segment([Pos2::new(plus_center.x - plus_arm, plus_center.y), Pos2::new(plus_center.x + plus_arm, plus_center.y)], plus_stroke);
                            ui.painter().line_segment([Pos2::new(plus_center.x, plus_center.y - plus_arm), Pos2::new(plus_center.x, plus_center.y + plus_arm)], plus_stroke);

                            if plus_resp.on_hover_text("Open Document in new tab (Ctrl+O)").clicked() {
                                action.open_new_document = true;
                            }
                        });
                    });
            });
        });

    action
}
