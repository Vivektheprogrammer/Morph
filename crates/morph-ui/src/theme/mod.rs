pub mod colors;
pub mod elevation;

use eframe::egui::{self, Color32, Stroke};

pub fn apply_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();

    visuals.panel_fill = colors::BG_SECONDARY;
    visuals.window_fill = colors::BG_APP;
    visuals.selection.bg_fill = colors::ACCENT;
    visuals.selection.stroke = Stroke::new(1.0_f32, colors::ACCENT_HOVER);
    visuals.override_text_color = Some(colors::TEXT_PRIMARY);

    visuals.widgets.noninteractive.bg_fill = colors::BG_SECONDARY;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, colors::BORDER_SUBTLE);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, colors::TEXT_SECONDARY);

    // Clean, modern Windows Fluent 4px rounding
    visuals.widgets.inactive.rounding = egui::Rounding::same(4.0);
    visuals.widgets.hovered.rounding = egui::Rounding::same(4.0);
    visuals.widgets.active.rounding = egui::Rounding::same(4.0);

    visuals.widgets.inactive.bg_fill = Color32::TRANSPARENT;
    visuals.widgets.inactive.bg_stroke = Stroke::NONE;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, colors::TEXT_PRIMARY);

    visuals.widgets.hovered.bg_fill = colors::SURFACE_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, colors::BORDER_SUBTLE);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);

    visuals.widgets.active.bg_fill = colors::SURFACE_ACTIVE;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, colors::ACCENT);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);

    visuals.window_rounding = egui::Rounding::same(8.0);
    visuals.window_stroke = Stroke::new(1.0_f32, colors::BORDER_SUBTLE);

    ctx.set_visuals(visuals);
}
