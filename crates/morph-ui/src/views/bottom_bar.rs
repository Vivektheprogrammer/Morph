use crate::theme::colors;
use eframe::egui::{Align, Frame, Layout, Margin, RichText, Stroke, Ui, Vec2};

pub fn render_bottom_bar(
    ui: &mut Ui,
    status_message: &str,
    is_error: bool,
    total_pages: usize,
    selected_idx: Option<usize>,
    zoom_level: f32,
    is_loading: bool,
) {
    Frame::none()
        .fill(colors::BG_SECONDARY)
        .stroke(Stroke::new(1.0_f32, colors::BORDER_SUBTLE))
        .inner_margin(Margin::symmetric(14.0, 4.0))
        .show(ui, |ui| {
            ui.set_height(20.0);

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(8.0, 0.0);

                // LEFT: Page counter ("Page 3 of 12" or "Ready")
                if total_pages > 0 {
                    let page_text = if let Some(idx) = selected_idx {
                        format!("Page {} of {}", idx + 1, total_pages)
                    } else {
                        format!("{} pages", total_pages)
                    };

                    ui.label(
                        RichText::new(page_text)
                            .size(11.5)
                            .strong()
                            .color(colors::TEXT_PRIMARY),
                    );

                    // Divider
                    let (sep_r, _) = ui.allocate_exact_size(Vec2::new(1.0, 12.0), eframe::egui::Sense::hover());
                    ui.painter().line_segment([sep_r.min, sep_r.max], Stroke::new(1.0_f32, colors::BORDER_SUBTLE));
                }

                // Status indicator pill badge
                if is_loading {
                    ui.spinner();
                    ui.label(
                        RichText::new("Processing...")
                            .size(11.0)
                            .color(colors::TEXT_SECONDARY),
                    );
                } else if !status_message.is_empty() {
                    let dot_color = if is_error {
                        colors::DESTRUCTIVE
                    } else if status_message == "Ready" {
                        colors::TEXT_MUTED
                    } else {
                        colors::ACCENT_EMERALD
                    };

                    Frame::none()
                        .fill(colors::SURFACE_ELEVATED)
                        .stroke(Stroke::new(1.0_f32, colors::BORDER_SUBTLE))
                        .rounding(eframe::egui::Rounding::same(10.0))
                        .inner_margin(Margin::symmetric(8.0, 2.5))
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(5.0, 0.0);
                            ui.horizontal(|ui| {
                                let (dot_rect, _) = ui.allocate_exact_size(Vec2::new(7.0, 7.0), eframe::egui::Sense::hover());
                                ui.painter().circle_filled(dot_rect.center(), 2.8, dot_color);

                                ui.label(
                                    RichText::new(status_message)
                                        .size(11.0)
                                        .color(if is_error {
                                            colors::DESTRUCTIVE
                                        } else {
                                            colors::TEXT_SECONDARY
                                        }),
                                );
                            });
                        });
                }

                // RIGHT: Zoom percentage
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if total_pages > 0 {
                        let zoom_text = format!("{:.0}%", zoom_level * 100.0);
                        ui.label(
                            RichText::new(zoom_text)
                                .size(11.0)
                                .color(colors::TEXT_MUTED),
                        );
                    }
                });
            });
        });
}
