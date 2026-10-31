use crate::components::button::MorphButton;
use crate::icons;
use crate::state::InsertPosition;
use crate::theme::colors;
use eframe::egui::{self, Align, Align2, Color32, FontId, Frame, Layout, Margin, Pos2, Rounding, Sense, Stroke, Ui, Vec2};

pub struct ToolbarAction {
    pub rotate_left: bool,
    pub rotate_right: bool,
    pub flip_180: bool,
    pub duplicate_page: bool,
    pub delete_page: bool,
    pub zoom_delta: f32,
    pub reset_zoom: bool,
    pub fit_page: bool,
    pub fit_width: bool,
    pub insert_pages: bool,
    pub rotate_all: bool,
    pub insert_pos_changed: Option<InsertPosition>,
    pub toggle_sidebar: bool,
}

pub fn render_toolbar(
    ui: &mut Ui,
    has_page_selected: bool,
    total_pages: usize,
    zoom_level: f32,
    insert_position: InsertPosition,
    sidebar_collapsed: bool,
) -> ToolbarAction {
    let mut action = ToolbarAction {
        rotate_left: false,
        rotate_right: false,
        flip_180: false,
        duplicate_page: false,
        delete_page: false,
        zoom_delta: 0.0,
        reset_zoom: false,
        fit_page: false,
        fit_width: false,
        insert_pages: false,
        rotate_all: false,
        insert_pos_changed: None,
        toggle_sidebar: false,
    };

    Frame::none()
        .fill(colors::BG_SECONDARY)
        .stroke(Stroke::new(1.0_f32, colors::BORDER_SUBTLE))
        .inner_margin(Margin::symmetric(10.0, 3.0))
        .show(ui, |ui| {
            ui.set_height(34.0);

            // Right side: Insert mode selector
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

                // Right side: Styled Fluent Insert Mode Dropdown Pill
                if let Some(new_pos) = render_insert_menu(ui, insert_position) {
                    action.insert_pos_changed = Some(new_pos);
                }

                // Left side: Sidebar Toggle, Page Operations, View Controls, Document Operations
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

                    // GROUP 1: SIDEBAR TOGGLE
                    let sb_tooltip = if sidebar_collapsed {
                        "Show Pages Sidebar"
                    } else {
                        "Hide Pages Sidebar"
                    };

                    if MorphButton::icon("tb_sb", icons::icon_sidebar_toggle)
                        .active(!sidebar_collapsed)
                        .tooltip(sb_tooltip)
                        .shortcut("Ctrl+B")
                        .show(ui)
                        .clicked()
                    {
                        action.toggle_sidebar = true;
                    }

                    render_divider(ui);

                    // GROUP 2: PAGE OPERATIONS
                    let page_ops_enabled = has_page_selected && total_pages > 0;

                    if MorphButton::icon("tb_rot_ccw", icons::icon_rotate_ccw)
                        .enabled(page_ops_enabled)
                        .tooltip("Rotate Page Counter-Clockwise 90°")
                        .shortcut("Shift+R")
                        .show(ui)
                        .clicked()
                    {
                        action.rotate_left = true;
                    }

                    if MorphButton::icon("tb_rot_cw", icons::icon_rotate_cw)
                        .enabled(page_ops_enabled)
                        .tooltip("Rotate Page Clockwise 90°")
                        .shortcut("R")
                        .show(ui)
                        .clicked()
                    {
                        action.rotate_right = true;
                    }

                    if MorphButton::icon("tb_flip", icons::icon_flip)
                        .enabled(page_ops_enabled)
                        .tooltip("Flip Page Upside Down (180°)")
                        .show(ui)
                        .clicked()
                    {
                        action.flip_180 = true;
                    }

                    if MorphButton::icon("tb_dup", icons::icon_duplicate)
                        .enabled(page_ops_enabled)
                        .tooltip("Duplicate Selected Page")
                        .shortcut("Ctrl+D")
                        .show(ui)
                        .clicked()
                    {
                        action.duplicate_page = true;
                    }

                    if MorphButton::destructive("tb_del", icons::icon_trash)
                        .enabled(page_ops_enabled)
                        .tooltip("Delete Selected Page")
                        .shortcut("Del")
                        .show(ui)
                        .clicked()
                    {
                        action.delete_page = true;
                    }

                    render_divider(ui);

                    // GROUP 3: VIEW CONTROLS
                    let view_enabled = total_pages > 0;

                    if MorphButton::icon("tb_zoom_out", icons::icon_minus)
                        .enabled(view_enabled)
                        .tooltip("Zoom Out")
                        .shortcut("Ctrl+-")
                        .show(ui)
                        .clicked()
                    {
                        action.zoom_delta = -0.15;
                    }

                    let zoom_pct = format!("{:.0}%", zoom_level * 100.0);
                    if MorphButton::secondary("tb_zoom_pill", &zoom_pct)
                        .enabled(view_enabled)
                        .min_width(52.0)
                        .tooltip("Click to reset zoom to 100%")
                        .show(ui)
                        .clicked()
                    {
                        action.reset_zoom = true;
                    }

                    if MorphButton::icon("tb_zoom_in", icons::icon_plus)
                        .enabled(view_enabled)
                        .tooltip("Zoom In")
                        .shortcut("Ctrl++")
                        .show(ui)
                        .clicked()
                    {
                        action.zoom_delta = 0.15;
                    }

                    if MorphButton::icon("tb_fit_page", icons::icon_fit_page)
                        .enabled(view_enabled)
                        .tooltip("Fit Page to Window Height")
                        .show(ui)
                        .clicked()
                    {
                        action.fit_page = true;
                    }

                    if MorphButton::icon("tb_fit_width", icons::icon_fit_width)
                        .enabled(view_enabled)
                        .tooltip("Fit Page to Window Width")
                        .show(ui)
                        .clicked()
                    {
                        action.fit_width = true;
                    }

                    render_divider(ui);

                    // GROUP 4: DOCUMENT OPERATIONS
                    if MorphButton::icon_text("tb_insert_pages", icons::icon_plus, "Insert Pages")
                        .enabled(view_enabled)
                        .tooltip("Insert pages into active document")
                        .show(ui)
                        .clicked()
                    {
                        action.insert_pages = true;
                    }

                    if view_enabled {
                        if MorphButton::icon("tb_rot_all", icons::icon_rotate_cw)
                            .tooltip("Rotate All Pages 90° Clockwise")
                            .show(ui)
                            .clicked()
                        {
                            action.rotate_all = true;
                        }
                    }
                });
            });
        });

    action
}

fn render_divider(ui: &mut Ui) {
    ui.add_space(2.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(1.0, 18.0), Sense::hover());
    ui.painter().line_segment(
        [rect.min, rect.max],
        Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    ui.add_space(2.0);
}

fn render_insert_menu(ui: &mut Ui, current_pos: InsertPosition) -> Option<InsertPosition> {
    let mut selected_pos = None;

    let pos_label = match current_pos {
        InsertPosition::Start => "Insert: Start",
        InsertPosition::AfterSelected => "Insert: After Sel",
        InsertPosition::End => "Insert: End",
    };

    let popup_id = ui.make_persistent_id("insert_pos_popup");
    let is_open = ui.memory(|m| m.is_popup_open(popup_id));

    let btn_size = Vec2::new(125.0, 30.0);
    let (rect, resp) = ui.allocate_exact_size(btn_size, Sense::click());
    if resp.clicked() {
        ui.memory_mut(|m| m.toggle_popup(popup_id));
    }

    let is_hovered = resp.hovered();

    let bg_color = if is_open {
        colors::SURFACE_ACTIVE
    } else if is_hovered {
        colors::SURFACE_HOVER
    } else {
        colors::SURFACE_ELEVATED
    };

    let border_color = if is_open {
        colors::ACCENT
    } else if is_hovered {
        colors::BORDER_CARD_HOVER
    } else {
        colors::BORDER_SUBTLE
    };

    // Draw elevated pill background
    ui.painter().rect_filled(rect, Rounding::same(6.0), bg_color);
    ui.painter().rect_stroke(rect, Rounding::same(6.0), Stroke::new(1.0_f32, border_color));

    // Draw placement indicator dot/icon on left
    let icon_center = Pos2::new(rect.min.x + 14.0, rect.center().y);
    ui.painter().circle_filled(icon_center, 3.0, colors::ACCENT);

    // Draw text label
    let text_pos = Pos2::new(rect.min.x + 24.0, rect.center().y);
    ui.painter().text(
        text_pos,
        Align2::LEFT_CENTER,
        pos_label,
        FontId::proportional(11.5),
        colors::TEXT_PRIMARY,
    );

    // Draw vector chevron down on right
    let chevron_center = Pos2::new(rect.max.x - 14.0, rect.center().y);
    let chev_color = if is_hovered || is_open {
        colors::TEXT_PRIMARY
    } else {
        colors::TEXT_MUTED
    };
    let chev_stroke = Stroke::new(1.4_f32, chev_color);
    if is_open {
        // Chevron pointing up
        ui.painter().line_segment(
            [Pos2::new(chevron_center.x - 4.0, chevron_center.y + 2.0), Pos2::new(chevron_center.x, chevron_center.y - 2.0)],
            chev_stroke,
        );
        ui.painter().line_segment(
            [Pos2::new(chevron_center.x, chevron_center.y - 2.0), Pos2::new(chevron_center.x + 4.0, chevron_center.y + 2.0)],
            chev_stroke,
        );
    } else {
        // Chevron pointing down
        ui.painter().line_segment(
            [Pos2::new(chevron_center.x - 4.0, chevron_center.y - 2.0), Pos2::new(chevron_center.x, chevron_center.y + 2.0)],
            chev_stroke,
        );
        ui.painter().line_segment(
            [Pos2::new(chevron_center.x, chevron_center.y + 2.0), Pos2::new(chevron_center.x + 4.0, chevron_center.y - 2.0)],
            chev_stroke,
        );
    }

    egui::popup_below_widget(ui, popup_id, &resp, egui::PopupCloseBehavior::CloseOnClick, |ui| {
        ui.set_min_width(220.0);
        ui.spacing_mut().item_spacing = Vec2::new(0.0, 2.0);

        let options = [
            (
                InsertPosition::End,
                "At End (Append)",
                "Append new pages to document end",
            ),
            (
                InsertPosition::AfterSelected,
                "After Selected Page",
                "Insert directly after the current page",
            ),
            (
                InsertPosition::Start,
                "At Start (Prepend)",
                "Insert pages at the beginning",
            ),
        ];

        for (pos, label, subtitle) in options {
            let is_cur = pos == current_pos;
            if render_dropdown_item(ui, label, subtitle, is_cur) {
                selected_pos = Some(pos);
                ui.memory_mut(|m| m.close_popup());
            }
        }
    });

    selected_pos
}

fn render_dropdown_item(ui: &mut Ui, label: &str, subtitle: &str, is_selected: bool) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 36.0), Sense::click());
    let is_hovered = resp.hovered();

    let bg = if is_hovered {
        colors::SURFACE_HOVER
    } else if is_selected {
        colors::SURFACE_ELEVATED
    } else {
        Color32::TRANSPARENT
    };

    if bg != Color32::TRANSPARENT {
        ui.painter().rect_filled(rect, Rounding::same(4.0), bg);
    }

    // Active indicator checkmark or dot
    if is_selected {
        let dot_pos = Pos2::new(rect.min.x + 10.0, rect.center().y);
        ui.painter().circle_filled(dot_pos, 3.0, colors::ACCENT);
    }

    let text_x = rect.min.x + 20.0;
    ui.painter().text(
        Pos2::new(text_x, rect.min.y + 6.0),
        Align2::LEFT_TOP,
        label,
        FontId::proportional(11.5),
        if is_selected { colors::TEXT_PRIMARY } else { colors::TEXT_SECONDARY },
    );

    ui.painter().text(
        Pos2::new(text_x, rect.min.y + 20.0),
        Align2::LEFT_TOP,
        subtitle,
        FontId::proportional(9.5),
        colors::TEXT_MUTED,
    );

    resp.clicked()
}
