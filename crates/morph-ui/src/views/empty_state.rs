use crate::theme::colors;
use crate::theme::elevation;
use eframe::egui::{self, Color32, Frame, Margin, RichText, Rounding, Stroke, Ui, Vec2};
use std::path::PathBuf;

pub fn render_empty_state(
    ui: &mut Ui,
    logo_texture: Option<&egui::TextureHandle>,
) -> Option<Vec<PathBuf>> {
    let mut picked_files = None;

    ui.vertical_centered(|ui| {
        ui.add_space(ui.available_height() * 0.18);

        // Centered Hero Card
        let card_frame = Frame::none()
            .fill(colors::SURFACE_BASE)
            .stroke(Stroke::new(1.0_f32, colors::BORDER_MEDIUM))
            .shadow(elevation::shadow_floating())
            .rounding(Rounding::same(12.0))
            .inner_margin(Margin::same(28.0));

        card_frame.show(ui, |ui| {
            ui.set_max_width(460.0);
            ui.vertical_centered(|ui| {
                if let Some(logo) = logo_texture {
                    ui.image((logo.id(), Vec2::new(72.0, 72.0)));
                    ui.add_space(14.0);
                }

                ui.heading(
                    RichText::new("Drop Documents & Images Here")
                        .strong()
                        .size(20.0)
                        .color(colors::TEXT_PRIMARY),
                );
                ui.add_space(6.0);

                ui.label(
                    RichText::new("Arrange, rotate, reorder, or merge multiple files into a clean PDF.")
                        .size(13.0)
                        .color(colors::TEXT_SECONDARY),
                );
                ui.add_space(16.0);

                // Supported Format Pills
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(6.0, 0.0);
                    let formats = [
                        ("PDF", colors::BADGE_PDF_BG, colors::BADGE_PDF_TXT),
                        ("PNG", colors::BADGE_IMG_BG, colors::BADGE_IMG_TXT),
                        ("JPG", colors::BADGE_IMG_BG, colors::BADGE_IMG_TXT),
                        ("WEBP", colors::BADGE_IMG_BG, colors::BADGE_IMG_TXT),
                        ("BMP", colors::BADGE_IMG_BG, colors::BADGE_IMG_TXT),
                    ];
                    for (name, bg, fg) in formats {
                        Frame::none()
                            .fill(bg)
                            .rounding(Rounding::same(4.0))
                            .inner_margin(Margin::symmetric(6.0, 2.5))
                            .show(ui, |ui| {
                                ui.label(RichText::new(name).size(10.0).strong().color(fg));
                            });
                    }
                });

                ui.add_space(22.0);

                let add_btn = egui::Button::new(
                    RichText::new("Browse Files...")
                        .strong()
                        .size(13.5)
                        .color(Color32::WHITE),
                )
                .fill(colors::ACCENT_PRIMARY)
                .stroke(Stroke::new(1.0_f32, colors::ACCENT_LIGHT))
                .rounding(Rounding::same(6.0))
                .min_size(Vec2::new(170.0, 36.0));

                if ui.add(add_btn).on_hover_text("Open file chooser dialog").clicked() {
                    if let Some(files) = rfd::FileDialog::new()
                        .add_filter("Supported Files", &["pdf", "png", "jpg", "jpeg", "webp", "bmp"])
                        .pick_files()
                    {
                        picked_files = Some(files);
                    }
                }
            });
        });
    });

    picked_files
}
