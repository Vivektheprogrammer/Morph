use crate::theme::colors;
use eframe::egui::{self, Color32, Rect, RichText, Rounding, Stroke, Vec2};

pub fn render_dropzone_overlay(ctx: &egui::Context, is_hovering: bool) {
    if !is_hovering {
        return;
    }

    let time = ctx.input(|i| i.time);
    let pulse = ((time * 3.5).sin() * 0.5 + 0.5) as f32;

    let screen_rect = ctx.screen_rect();
    let pad = 16.0;
    let inner_rect = screen_rect.shrink(pad);

    egui::Area::new(egui::Id::new("dropzone_overlay"))
        .fixed_pos(screen_rect.min)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            let painter = ui.painter_at(screen_rect);
            painter.rect_filled(
                screen_rect,
                0.0,
                Color32::from_rgba_premultiplied(10, 14, 22, 210),
            );

            let glow_alpha = (140.0 + 100.0 * pulse) as u8;
            let border_color = Color32::from_rgba_premultiplied(59, 130, 246, glow_alpha);
            let stroke = Stroke::new(2.5_f32, border_color);

            painter.rect_stroke(inner_rect, Rounding::same(12.0), stroke);

            let card_w = 340.0;
            let card_h = 130.0;
            let card_rect = Rect::from_center_size(inner_rect.center(), Vec2::new(card_w, card_h));

            painter.rect_filled(card_rect, Rounding::same(10.0), Color32::from_rgb(18, 24, 36));
            painter.rect_stroke(card_rect, Rounding::same(10.0), Stroke::new(1.5_f32, colors::ACCENT_LIGHT));

            let mut card_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(card_rect)
                    .layout(egui::Layout::top_down(egui::Align::Center)),
            );

            card_ui.add_space(14.0);
            card_ui.label(RichText::new("📥").size(32.0));
            card_ui.add_space(4.0);
            card_ui.heading(
                RichText::new("Drop Files to Insert")
                    .size(16.0)
                    .strong()
                    .color(Color32::WHITE),
            );
            card_ui.add_space(2.0);
            card_ui.label(
                RichText::new("PDFs and Images will be added seamlessly")
                    .size(11.5)
                    .color(colors::TEXT_SECONDARY),
            );
        });

    ctx.request_repaint();
}
