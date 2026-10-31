#![allow(dead_code)]

use crate::theme::elevation;
use eframe::egui::{self, Color32, Frame, Margin, Pos2, RichText, Rounding, Stroke};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Success,
    Error,
}

#[derive(Clone)]
pub struct Toast {
    pub message: String,
    pub kind: ToastKind,
    pub created_at: f64,
    pub duration: f64,
}

pub struct ToastManager {
    toasts: Vec<Toast>,
}

impl Default for ToastManager {
    fn default() -> Self {
        Self { toasts: Vec::new() }
    }
}

impl ToastManager {
    pub fn push(&mut self, message: impl Into<String>, kind: ToastKind, now: f64) {
        self.toasts.push(Toast {
            message: message.into(),
            kind,
            created_at: now,
            duration: 3.2,
        });
        if self.toasts.len() > 3 {
            self.toasts.remove(0);
        }
    }

    pub fn render(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        self.toasts.retain(|t| (now - t.created_at) < t.duration);

        if self.toasts.is_empty() {
            return;
        }

        let screen_rect = ctx.screen_rect();
        let margin_bottom = 44.0;
        let toast_width = 320.0;
        let mut y_offset = screen_rect.max.y - margin_bottom;

        for (idx, toast) in self.toasts.iter().enumerate().rev() {
            let age = now - toast.created_at;
            let time_left = toast.duration - age;

            let alpha = if age < 0.15 {
                (age / 0.15) as f32
            } else if time_left < 0.3 {
                (time_left / 0.3) as f32
            } else {
                1.0
            }
            .clamp(0.0, 1.0);

            let slide_y = (1.0 - alpha) * 12.0;

            let card_bg = Color32::from_rgba_unmultiplied(24, 27, 35, (245.0 * alpha) as u8);
            let card_stroke = Color32::from_rgba_unmultiplied(54, 60, 74, (210.0 * alpha) as u8);

            let toast_pos = Pos2::new(
                screen_rect.max.x - toast_width - 24.0,
                y_offset - 42.0 + slide_y,
            );

            egui::Area::new(egui::Id::new(format!("toast_area_{}", idx)))
                .fixed_pos(toast_pos)
                .order(egui::Order::Tooltip)
                .show(ctx, |ui| {
                    Frame::none()
                        .fill(card_bg)
                        .stroke(Stroke::new(1.0_f32, card_stroke))
                        .shadow(elevation::shadow_floating())
                        .rounding(Rounding::same(8.0))
                        .inner_margin(Margin::symmetric(12.0, 9.0))
                        .show(ui, |ui| {
                            ui.set_width(toast_width - 24.0);
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = egui::Vec2::new(8.0, 0.0);
                                draw_toast_icon(ui, toast.kind, alpha);
                                ui.label(
                                    RichText::new(&toast.message)
                                        .size(11.5)
                                        .color(Color32::from_rgba_unmultiplied(241, 245, 249, (255.0 * alpha) as u8)),
                                );
                            });
                        });
                });

            y_offset -= 46.0;
        }

        ctx.request_repaint();
    }
}

fn draw_toast_icon(ui: &mut egui::Ui, kind: ToastKind, alpha: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::Vec2::splat(18.0), egui::Sense::hover());
    let center = rect.center();

    let (bg_col, fg_col) = match kind {
        ToastKind::Success => (
            Color32::from_rgba_unmultiplied(16, 185, 129, (38.0 * alpha) as u8),
            Color32::from_rgba_unmultiplied(52, 211, 153, (255.0 * alpha) as u8),
        ),
        ToastKind::Error => (
            Color32::from_rgba_unmultiplied(239, 68, 68, (38.0 * alpha) as u8),
            Color32::from_rgba_unmultiplied(248, 113, 113, (255.0 * alpha) as u8),
        ),
        ToastKind::Info => (
            Color32::from_rgba_unmultiplied(59, 130, 246, (38.0 * alpha) as u8),
            Color32::from_rgba_unmultiplied(96, 165, 250, (255.0 * alpha) as u8),
        ),
    };

    // Soft circular badge
    ui.painter().circle_filled(center, 9.0, bg_col);

    let stroke = Stroke::new(1.6_f32, fg_col);
    match kind {
        ToastKind::Success => {
            let p1 = Pos2::new(center.x - 4.0, center.y);
            let p2 = Pos2::new(center.x - 1.0, center.y + 3.2);
            let p3 = Pos2::new(center.x + 4.5, center.y - 3.2);
            ui.painter().line_segment([p1, p2], stroke);
            ui.painter().line_segment([p2, p3], stroke);
        }
        ToastKind::Error => {
            let d = 3.2_f32;
            ui.painter().line_segment([Pos2::new(center.x - d, center.y - d), Pos2::new(center.x + d, center.y + d)], stroke);
            ui.painter().line_segment([Pos2::new(center.x + d, center.y - d), Pos2::new(center.x - d, center.y + d)], stroke);
        }
        ToastKind::Info => {
            ui.painter().circle_filled(center, 2.5, fg_col);
        }
    }
}
