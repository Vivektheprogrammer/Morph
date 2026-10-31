#![allow(dead_code)]

use eframe::egui::{Color32, Pos2, Stroke, Ui, Vec2};
use std::f32::consts::PI;

pub fn icon_rotate_cw(ui: &mut Ui, size: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), eframe::egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let center = rect.center();
    let radius = size * 0.38;
    let stroke = Stroke::new(1.6_f32, color);

    let points: Vec<Pos2> = (0..=24)
        .map(|i| {
            let angle = -PI * 0.5 + (i as f32 / 24.0) * (PI * 1.5);
            Pos2::new(center.x + radius * angle.cos(), center.y + radius * angle.sin())
        })
        .collect();

    for pair in points.windows(2) {
        painter.line_segment([pair[0], pair[1]], stroke);
    }

    let end_p = *points.last().unwrap();
    let arrow_size = size * 0.22;
    painter.line_segment([end_p, Pos2::new(end_p.x - arrow_size * 0.8, end_p.y - arrow_size)], stroke);
    painter.line_segment([end_p, Pos2::new(end_p.x + arrow_size * 0.8, end_p.y - arrow_size)], stroke);
}

pub fn icon_trash(ui: &mut Ui, size: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), eframe::egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let stroke = Stroke::new(1.4_f32, color);

    let w = size * 0.55;
    let h = size * 0.60;
    let cx = rect.center().x;
    let top_y = rect.center().y - h * 0.45;

    painter.line_segment([Pos2::new(cx - w * 0.6, top_y), Pos2::new(cx + w * 0.6, top_y)], stroke);
    painter.line_segment([Pos2::new(cx - w * 0.25, top_y - h * 0.15), Pos2::new(cx + w * 0.25, top_y - h * 0.15)], stroke);

    let body_top = top_y + h * 0.12;
    let body_bot = body_top + h * 0.80;
    painter.line_segment([Pos2::new(cx - w * 0.45, body_top), Pos2::new(cx - w * 0.38, body_bot)], stroke);
    painter.line_segment([Pos2::new(cx + w * 0.45, body_top), Pos2::new(cx + w * 0.38, body_bot)], stroke);
    painter.line_segment([Pos2::new(cx - w * 0.38, body_bot), Pos2::new(cx + w * 0.38, body_bot)], stroke);

    let inner_top = body_top + h * 0.15;
    let inner_bot = body_bot - h * 0.15;
    painter.line_segment([Pos2::new(cx - w * 0.15, inner_top), Pos2::new(cx - w * 0.15, inner_bot)], stroke);
    painter.line_segment([Pos2::new(cx + w * 0.15, inner_top), Pos2::new(cx + w * 0.15, inner_bot)], stroke);
}

pub fn icon_undo(ui: &mut Ui, size: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), eframe::egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let stroke = Stroke::new(1.5_f32, color);
    let c = rect.center();
    let r = size * 0.36;

    let points: Vec<Pos2> = (0..=16)
        .map(|i| {
            let t = i as f32 / 16.0;
            let angle = -PI * 0.2 + t * (PI * 1.0);
            Pos2::new(c.x + r * angle.cos(), c.y + r * angle.sin())
        })
        .collect();

    for pair in points.windows(2) {
        painter.line_segment([pair[0], pair[1]], stroke);
    }

    let tip = Pos2::new(c.x - r * 0.8, c.y - r * 0.2);
    let a_size = size * 0.22;
    painter.line_segment([tip, Pos2::new(tip.x + a_size, tip.y - a_size * 0.8)], stroke);
    painter.line_segment([tip, Pos2::new(tip.x + a_size, tip.y + a_size * 0.8)], stroke);
}

pub fn icon_redo(ui: &mut Ui, size: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), eframe::egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let stroke = Stroke::new(1.5_f32, color);
    let c = rect.center();
    let r = size * 0.36;

    let points: Vec<Pos2> = (0..=16)
        .map(|i| {
            let t = i as f32 / 16.0;
            let angle = PI * 1.2 - t * (PI * 1.0);
            Pos2::new(c.x + r * angle.cos(), c.y + r * angle.sin())
        })
        .collect();

    for pair in points.windows(2) {
        painter.line_segment([pair[0], pair[1]], stroke);
    }

    let tip = Pos2::new(c.x + r * 0.8, c.y - r * 0.2);
    let a_size = size * 0.22;
    painter.line_segment([tip, Pos2::new(tip.x - a_size, tip.y - a_size * 0.8)], stroke);
    painter.line_segment([tip, Pos2::new(tip.x - a_size, tip.y + a_size * 0.8)], stroke);
}

pub fn icon_plus(ui: &mut Ui, size: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), eframe::egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let stroke = Stroke::new(1.6_f32, color);
    let c = rect.center();
    let arm = size * 0.35;

    painter.line_segment([Pos2::new(c.x - arm, c.y), Pos2::new(c.x + arm, c.y)], stroke);
    painter.line_segment([Pos2::new(c.x, c.y - arm), Pos2::new(c.x, c.y + arm)], stroke);
}

pub fn icon_minus(ui: &mut Ui, size: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), eframe::egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let stroke = Stroke::new(1.6_f32, color);
    let c = rect.center();
    let arm = size * 0.35;

    painter.line_segment([Pos2::new(c.x - arm, c.y), Pos2::new(c.x + arm, c.y)], stroke);
}

pub fn icon_chevron_up(ui: &mut Ui, size: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), eframe::egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let stroke = Stroke::new(1.6_f32, color);
    let c = rect.center();
    let w = size * 0.30;
    let h = size * 0.18;

    painter.line_segment([Pos2::new(c.x - w, c.y + h), Pos2::new(c.x, c.y - h)], stroke);
    painter.line_segment([Pos2::new(c.x, c.y - h), Pos2::new(c.x + w, c.y + h)], stroke);
}

pub fn icon_chevron_down(ui: &mut Ui, size: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), eframe::egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let stroke = Stroke::new(1.6_f32, color);
    let c = rect.center();
    let w = size * 0.30;
    let h = size * 0.18;

    painter.line_segment([Pos2::new(c.x - w, c.y - h), Pos2::new(c.x, c.y + h)], stroke);
    painter.line_segment([Pos2::new(c.x, c.y + h), Pos2::new(c.x + w, c.y - h)], stroke);
}

pub fn icon_chevron_left(ui: &mut Ui, size: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), eframe::egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let stroke = Stroke::new(1.6_f32, color);
    let c = rect.center();
    let w = size * 0.18;
    let h = size * 0.30;

    painter.line_segment([Pos2::new(c.x + w, c.y - h), Pos2::new(c.x - w, c.y)], stroke);
    painter.line_segment([Pos2::new(c.x - w, c.y), Pos2::new(c.x + w, c.y + h)], stroke);
}

pub fn icon_chevron_right(ui: &mut Ui, size: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), eframe::egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let stroke = Stroke::new(1.6_f32, color);
    let c = rect.center();
    let w = size * 0.18;
    let h = size * 0.30;

    painter.line_segment([Pos2::new(c.x - w, c.y - h), Pos2::new(c.x + w, c.y)], stroke);
    painter.line_segment([Pos2::new(c.x + w, c.y), Pos2::new(c.x - w, c.y + h)], stroke);
}

pub fn paint_icon_cross(painter: &eframe::egui::Painter, center: Pos2, size: f32, color: Color32) {
    let stroke = Stroke::new(1.4_f32, color);
    let d = size * 0.32;
    painter.line_segment([Pos2::new(center.x - d, center.y - d), Pos2::new(center.x + d, center.y + d)], stroke);
    painter.line_segment([Pos2::new(center.x + d, center.y - d), Pos2::new(center.x - d, center.y + d)], stroke);
}

pub fn icon_cross(ui: &mut Ui, size: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), eframe::egui::Sense::hover());
    let painter = ui.painter_at(rect);
    paint_icon_cross(&painter, rect.center(), size, color);
}

pub fn icon_check(ui: &mut Ui, size: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), eframe::egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let stroke = Stroke::new(1.8_f32, color);
    let c = rect.center();
    let s = size * 0.35;

    let p1 = Pos2::new(c.x - s, c.y);
    let p2 = Pos2::new(c.x - s * 0.2, c.y + s * 0.7);
    let p3 = Pos2::new(c.x + s * 0.9, c.y - s * 0.6);

    painter.line_segment([p1, p2], stroke);
    painter.line_segment([p2, p3], stroke);
}

pub fn icon_drag_grip(ui: &mut Ui, size: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), eframe::egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let c = rect.center();
    let r = 1.3_f32;
    let dx = size * 0.16;
    let dy = size * 0.22;

    for x in [-dx, dx] {
        for y in [-dy, 0.0, dy] {
            painter.circle_filled(Pos2::new(c.x + x, c.y + y), r, color);
        }
    }
}

pub fn icon_export(ui: &mut Ui, size: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), eframe::egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let stroke = Stroke::new(1.6_f32, color);
    let c = rect.center();
    let w = size * 0.36;
    let h = size * 0.36;

    painter.line_segment([Pos2::new(c.x - w, c.y), Pos2::new(c.x - w, c.y + h)], stroke);
    painter.line_segment([Pos2::new(c.x - w, c.y + h), Pos2::new(c.x + w, c.y + h)], stroke);
    painter.line_segment([Pos2::new(c.x + w, c.y + h), Pos2::new(c.x + w, c.y)], stroke);

    painter.line_segment([Pos2::new(c.x, c.y + h * 0.6), Pos2::new(c.x, c.y - h * 0.8)], stroke);
    let tip = Pos2::new(c.x, c.y - h * 0.8);
    let a_w = size * 0.18;
    painter.line_segment([tip, Pos2::new(tip.x - a_w, tip.y + a_w)], stroke);
    painter.line_segment([tip, Pos2::new(tip.x + a_w, tip.y + a_w)], stroke);
}

pub fn icon_fit_page(ui: &mut Ui, size: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), eframe::egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let stroke = Stroke::new(1.4_f32, color);
    let c = rect.center();
    let s = size * 0.36;
    let corner = s * 0.5;

    painter.line_segment([Pos2::new(c.x - s, c.y - s + corner), Pos2::new(c.x - s, c.y - s)], stroke);
    painter.line_segment([Pos2::new(c.x - s, c.y - s), Pos2::new(c.x - s + corner, c.y - s)], stroke);
    painter.line_segment([Pos2::new(c.x + s - corner, c.y - s), Pos2::new(c.x + s, c.y - s)], stroke);
    painter.line_segment([Pos2::new(c.x + s, c.y - s), Pos2::new(c.x + s, c.y - s + corner)], stroke);
    painter.line_segment([Pos2::new(c.x - s, c.y + s - corner), Pos2::new(c.x - s, c.y + s)], stroke);
    painter.line_segment([Pos2::new(c.x - s, c.y + s), Pos2::new(c.x - s + corner, c.y + s)], stroke);
    painter.line_segment([Pos2::new(c.x + s - corner, c.y + s), Pos2::new(c.x + s, c.y + s)], stroke);
    painter.line_segment([Pos2::new(c.x + s, c.y + s), Pos2::new(c.x + s, c.y + s - corner)], stroke);
}

pub fn icon_rotate_ccw(ui: &mut Ui, size: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), eframe::egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let center = rect.center();
    let radius = size * 0.38;
    let stroke = Stroke::new(1.6_f32, color);

    let points: Vec<Pos2> = (0..=24)
        .map(|i| {
            let angle = -PI * 0.5 - (i as f32 / 24.0) * (PI * 1.5);
            Pos2::new(center.x + radius * angle.cos(), center.y + radius * angle.sin())
        })
        .collect();

    for pair in points.windows(2) {
        painter.line_segment([pair[0], pair[1]], stroke);
    }

    let end_p = *points.last().unwrap();
    let arrow_size = size * 0.22;
    painter.line_segment([end_p, Pos2::new(end_p.x - arrow_size * 0.8, end_p.y - arrow_size)], stroke);
    painter.line_segment([end_p, Pos2::new(end_p.x + arrow_size * 0.8, end_p.y - arrow_size)], stroke);
}

pub fn icon_flip(ui: &mut Ui, size: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), eframe::egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let stroke = Stroke::new(1.4_f32, color);
    let c = rect.center();
    let half_w = size * 0.34;
    let half_h = size * 0.26;

    // Center divider
    let divider_stroke = Stroke::new(1.0_f32, color.gamma_multiply(0.4));
    painter.line_segment([Pos2::new(c.x, c.y - half_h * 1.2), Pos2::new(c.x, c.y + half_h * 1.2)], divider_stroke);

    // Left triangle
    painter.line_segment([Pos2::new(c.x - 2.0, c.y - half_h), Pos2::new(c.x - half_w, c.y)], stroke);
    painter.line_segment([Pos2::new(c.x - half_w, c.y), Pos2::new(c.x - 2.0, c.y + half_h)], stroke);
    painter.line_segment([Pos2::new(c.x - 2.0, c.y + half_h), Pos2::new(c.x - 2.0, c.y - half_h)], stroke);

    // Right triangle (outline or filled subtle)
    painter.line_segment([Pos2::new(c.x + 2.0, c.y - half_h), Pos2::new(c.x + half_w, c.y)], stroke);
    painter.line_segment([Pos2::new(c.x + half_w, c.y), Pos2::new(c.x + 2.0, c.y + half_h)], stroke);
    painter.line_segment([Pos2::new(c.x + 2.0, c.y + half_h), Pos2::new(c.x + 2.0, c.y - half_h)], stroke);
}

pub fn icon_duplicate(ui: &mut Ui, size: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), eframe::egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let stroke = Stroke::new(1.4_f32, color);
    let c = rect.center();
    let w = size * 0.40;
    let h = size * 0.48;

    // Back card (top-left offset)
    let b_min = Pos2::new(c.x - w * 0.65, c.y - h * 0.65);
    let b_max = Pos2::new(c.x + w * 0.35, c.y + h * 0.35);
    painter.line_segment([b_min, Pos2::new(b_max.x, b_min.y)], stroke);
    painter.line_segment([b_min, Pos2::new(b_min.x, b_max.y)], stroke);

    // Front card
    let f_min = Pos2::new(c.x - w * 0.35, c.y - h * 0.35);
    let f_max = Pos2::new(c.x + w * 0.65, c.y + h * 0.65);
    painter.rect_stroke(
        eframe::egui::Rect::from_min_max(f_min, f_max),
        1.5,
        stroke,
    );
}

pub fn icon_sidebar_toggle(ui: &mut Ui, size: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), eframe::egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let stroke = Stroke::new(1.4_f32, color);
    let c = rect.center();
    let w = size * 0.65;
    let h = size * 0.55;
    let outer = eframe::egui::Rect::from_center_size(c, Vec2::new(w, h));

    painter.rect_stroke(outer, 2.0, stroke);
    let div_x = outer.min.x + w * 0.35;
    painter.line_segment([Pos2::new(div_x, outer.min.y), Pos2::new(div_x, outer.max.y)], stroke);
}

pub fn icon_fit_width(ui: &mut Ui, size: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), eframe::egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let stroke = Stroke::new(1.5_f32, color);
    let c = rect.center();
    let w = size * 0.40;
    let h = size * 0.30;

    // Left and right vertical boundaries
    painter.line_segment([Pos2::new(c.x - w, c.y - h), Pos2::new(c.x - w, c.y + h)], stroke);
    painter.line_segment([Pos2::new(c.x + w, c.y - h), Pos2::new(c.x + w, c.y + h)], stroke);

    // Center horizontal bar with arrows
    let line_y = c.y;
    painter.line_segment([Pos2::new(c.x - w + 2.0, line_y), Pos2::new(c.x + w - 2.0, line_y)], stroke);

    // Arrow heads
    let a = size * 0.12;
    painter.line_segment([Pos2::new(c.x - w + 2.0, line_y), Pos2::new(c.x - w + 2.0 + a, line_y - a)], stroke);
    painter.line_segment([Pos2::new(c.x - w + 2.0, line_y), Pos2::new(c.x - w + 2.0 + a, line_y + a)], stroke);

    painter.line_segment([Pos2::new(c.x + w - 2.0, line_y), Pos2::new(c.x + w - 2.0 - a, line_y - a)], stroke);
    painter.line_segment([Pos2::new(c.x + w - 2.0, line_y), Pos2::new(c.x + w - 2.0 - a, line_y + a)], stroke);
}

pub fn paint_icon_pdf(painter: &eframe::egui::Painter, center: Pos2, size: f32, color: Color32) {
    let stroke = Stroke::new(1.4_f32, color);
    let w = size * 0.42;
    let h = size * 0.52;
    let fold = size * 0.14;

    let min = Pos2::new(center.x - w * 0.5, center.y - h * 0.5);
    let max = Pos2::new(center.x + w * 0.5, center.y + h * 0.5);

    // Page outline with folded corner
    painter.line_segment([min, Pos2::new(max.x - fold, min.y)], stroke);
    painter.line_segment([Pos2::new(max.x - fold, min.y), Pos2::new(max.x, min.y + fold)], stroke);
    painter.line_segment([Pos2::new(max.x, min.y + fold), max], stroke);
    painter.line_segment([max, Pos2::new(min.x, max.y)], stroke);
    painter.line_segment([Pos2::new(min.x, max.y), min], stroke);

    // Fold flap
    painter.line_segment([Pos2::new(max.x - fold, min.y), Pos2::new(max.x - fold, min.y + fold)], stroke);
    painter.line_segment([Pos2::new(max.x - fold, min.y + fold), Pos2::new(max.x, min.y + fold)], stroke);

    // Interior lines
    let l_stroke = Stroke::new(1.0_f32, color.gamma_multiply(0.6));
    let line_start_x = min.x + w * 0.22;
    let line_end_x = max.x - w * 0.22;
    painter.line_segment([Pos2::new(line_start_x, center.y), Pos2::new(line_end_x, center.y)], l_stroke);
    painter.line_segment([Pos2::new(line_start_x, center.y + h * 0.20), Pos2::new(line_end_x, center.y + h * 0.20)], l_stroke);
}

pub fn icon_pdf(ui: &mut Ui, size: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), eframe::egui::Sense::hover());
    let painter = ui.painter_at(rect);
    paint_icon_pdf(&painter, rect.center(), size, color);
}
