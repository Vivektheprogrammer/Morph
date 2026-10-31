use crate::components::button::MorphButton;
use crate::icons;
use crate::theme::colors;
use eframe::egui::{self, Align, Frame, Layout, Margin, RichText, Sense, Stroke, Ui, Vec2};

pub struct HeaderAction {
    pub undo: bool,
    pub redo: bool,
    pub export: bool,
}

pub fn render_header(
    ui: &mut Ui,
    logo_texture: Option<&egui::TextureHandle>,
    active_doc_name: Option<&str>,
    can_undo: bool,
    can_redo: bool,
    pages_count: usize,
) -> HeaderAction {
    let mut action = HeaderAction {
        undo: false,
        redo: false,
        export: false,
    };

    Frame::none()
        .fill(colors::BG_SECONDARY)
        .stroke(Stroke::new(1.0_f32, colors::BORDER_SUBTLE))
        .inner_margin(Margin::symmetric(14.0, 5.0))
        .show(ui, |ui| {
            ui.set_height(34.0);
            let header_width = ui.available_width();

            // 1. RIGHT: Always visible, right-aligned (Export PDF + Page Count)
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let export_enabled = pages_count > 0;

                if MorphButton::primary("hdr_export_btn", "Export PDF")
                    .icon_fn(icons::icon_export)
                    .enabled(export_enabled)
                    .tooltip("Export active document as PDF")
                    .shortcut("Ctrl+S")
                    .show(ui)
                    .clicked()
                {
                    action.export = true;
                }

                if pages_count > 0 {
                    ui.add_space(8.0);
                    let count_text = if pages_count == 1 {
                        "1 page".to_string()
                    } else {
                        format!("{} pages", pages_count)
                    };
                    ui.label(
                        RichText::new(count_text)
                            .size(11.5)
                            .color(colors::TEXT_SECONDARY),
                    );
                }

                // 2. LEFT & CENTER: Layout in the remaining space
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    if let Some(logo) = logo_texture {
                        ui.image((logo.id(), Vec2::new(20.0, 20.0)));
                        ui.add_space(6.0);
                    }

                    ui.label(
                        RichText::new("Morph Editor")
                            .size(13.5)
                            .strong()
                            .color(colors::TEXT_PRIMARY),
                    );

                    if let Some(name) = active_doc_name {
                        ui.add_space(8.0);
                        let (sep_rect, _) = ui.allocate_exact_size(Vec2::new(1.0, 14.0), Sense::hover());
                        ui.painter().line_segment(
                            [sep_rect.min, sep_rect.max],
                            Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
                        );
                        ui.add_space(8.0);

                        // Responsive middle-truncation based on header width
                        let max_len = if header_width < 1300.0 { 24 } else { 42 };
                        let display_name = if name.chars().count() > max_len {
                            let prefix: String = name.chars().take(max_len / 2).collect();
                            let suffix: String = name.chars().skip(name.chars().count().saturating_sub((max_len / 2) - 3)).collect();
                            format!("{}...{}", prefix, suffix)
                        } else {
                            name.to_string()
                        };

                        ui.label(
                            RichText::new(display_name)
                                .size(12.0)
                                .color(colors::TEXT_SECONDARY),
                        );
                    }

                    // 3. CENTER: Undo & Redo (centered in available gap)
                    let available_center_gap = ui.available_width();
                    let center_pad = ((available_center_gap - 150.0) * 0.45).max(12.0);
                    ui.add_space(center_pad);

                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

                        if MorphButton::icon_text("hdr_undo", icons::icon_undo, "Undo")
                            .enabled(can_undo)
                            .tooltip("Undo last action")
                            .shortcut("Ctrl+Z")
                            .show(ui)
                            .clicked()
                        {
                            action.undo = true;
                        }

                        if MorphButton::icon_text("hdr_redo", icons::icon_redo, "Redo")
                            .enabled(can_redo)
                            .tooltip("Redo last action")
                            .shortcut("Ctrl+Y")
                            .show(ui)
                            .clicked()
                        {
                            action.redo = true;
                        }
                    });
                });
            });
        });

    action
}
