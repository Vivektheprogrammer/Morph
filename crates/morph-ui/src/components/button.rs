#![allow(dead_code)]

use crate::motion::{self, BUTTON_HOVER_DURATION, BUTTON_PRESS_DURATION};
use crate::theme::colors;
use eframe::egui::{Align2, Color32, FontId, Pos2, Rect, Response, Rounding, Sense, Stroke, Ui, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    IconOnly,
    Destructive,
    Ghost,
}

pub struct MorphButton<'a> {
    id_source: &'a str,
    text: Option<&'a str>,
    icon_fn: Option<Box<dyn FnOnce(&mut Ui, f32, Color32) + 'a>>,
    variant: ButtonVariant,
    tooltip: Option<&'a str>,
    shortcut: Option<&'a str>,
    enabled: bool,
    active: bool,
    min_width: f32,
    height: f32,
    icon_size: f32,
}

impl<'a> MorphButton<'a> {
    pub fn new(id_source: &'a str) -> Self {
        Self {
            id_source,
            text: None,
            icon_fn: None,
            variant: ButtonVariant::Secondary,
            tooltip: None,
            shortcut: None,
            enabled: true,
            active: false,
            min_width: 32.0,
            height: 32.0,
            icon_size: 14.0,
        }
    }

    pub fn primary(id_source: &'a str, text: &'a str) -> Self {
        let mut btn = Self::new(id_source);
        btn.variant = ButtonVariant::Primary;
        btn.text = Some(text);
        btn.min_width = 84.0;
        btn
    }

    pub fn secondary(id_source: &'a str, text: &'a str) -> Self {
        let mut btn = Self::new(id_source);
        btn.variant = ButtonVariant::Secondary;
        btn.text = Some(text);
        btn.min_width = 72.0;
        btn
    }

    pub fn icon(
        id_source: &'a str,
        icon_fn: impl FnOnce(&mut Ui, f32, Color32) + 'a,
    ) -> Self {
        let mut btn = Self::new(id_source);
        btn.variant = ButtonVariant::IconOnly;
        btn.icon_fn = Some(Box::new(icon_fn));
        btn.min_width = 32.0;
        btn
    }

    pub fn icon_text(
        id_source: &'a str,
        icon_fn: impl FnOnce(&mut Ui, f32, Color32) + 'a,
        text: &'a str,
    ) -> Self {
        let mut btn = Self::new(id_source);
        btn.variant = ButtonVariant::Secondary;
        btn.icon_fn = Some(Box::new(icon_fn));
        btn.text = Some(text);
        btn.min_width = 76.0;
        btn
    }

    pub fn destructive(
        id_source: &'a str,
        icon_fn: impl FnOnce(&mut Ui, f32, Color32) + 'a,
    ) -> Self {
        let mut btn = Self::new(id_source);
        btn.variant = ButtonVariant::Destructive;
        btn.icon_fn = Some(Box::new(icon_fn));
        btn.min_width = 32.0;
        btn
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn icon_fn(mut self, icon_fn: impl FnOnce(&mut Ui, f32, Color32) + 'a) -> Self {
        self.icon_fn = Some(Box::new(icon_fn));
        self
    }

    pub fn text(mut self, text: &'a str) -> Self {
        self.text = Some(text);
        self
    }

    pub fn tooltip(mut self, tooltip: &'a str) -> Self {
        self.tooltip = Some(tooltip);
        self
    }

    pub fn shortcut(mut self, shortcut: &'a str) -> Self {
        self.shortcut = Some(shortcut);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn min_width(mut self, min_width: f32) -> Self {
        self.min_width = min_width;
        self
    }

    pub fn icon_size(mut self, icon_size: f32) -> Self {
        self.icon_size = icon_size;
        self
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let id = ui.make_persistent_id(self.id_source);

        // Calculate needed content width
        let text_width = if let Some(t) = self.text {
            let font_id = FontId::proportional(11.5);
            ui.fonts(|f| f.layout_no_wrap(t.to_string(), font_id, Color32::WHITE).size().x)
        } else {
            0.0
        };

        let mut content_width = 0.0;
        if self.icon_fn.is_some() {
            content_width += self.icon_size;
        }
        if self.text.is_some() {
            if self.icon_fn.is_some() {
                content_width += 6.0;
            }
            content_width += text_width;
        }

        let padding_x = if self.text.is_some() { 12.0 } else { 8.0 };
        let desired_width = (content_width + padding_x * 2.0).max(self.min_width);
        let desired_size = Vec2::new(desired_width, self.height);

        let (rect, mut response) = ui.allocate_exact_size(
            desired_size,
            if self.enabled { Sense::click() } else { Sense::hover() },
        );

        // Smooth animations using standardized motion tokens
        let is_hovered = response.hovered() && self.enabled;
        let is_pressed = response.is_pointer_button_down_on() && self.enabled;

        let hover_raw = ui.ctx().animate_bool_with_time(id.with("hover"), is_hovered, BUTTON_HOVER_DURATION);
        let press_raw = ui.ctx().animate_bool_with_time(id.with("press"), is_pressed, BUTTON_PRESS_DURATION);

        let hover_t = motion::ease_standard(hover_raw);
        let press_t = motion::ease_exit(press_raw);

        // Base colors per variant
        let (base_bg, hover_bg, press_bg, base_border, hover_border, base_fg, hover_fg) = match self.variant {
            ButtonVariant::Primary => (
                colors::ACCENT,
                colors::ACCENT_HOVER,
                Color32::from_rgb(43, 114, 230),
                colors::ACCENT,
                colors::ACCENT_LIGHT,
                colors::TEXT_ON_ACCENT,
                colors::TEXT_ON_ACCENT,
            ),
            ButtonVariant::Secondary => (
                colors::SURFACE_ELEVATED,
                Color32::from_rgb(40, 45, 55),
                Color32::from_rgb(27, 30, 38),
                colors::BORDER_SUBTLE,
                colors::BORDER_MEDIUM,
                colors::TEXT_PRIMARY,
                colors::TEXT_PRIMARY,
            ),
            ButtonVariant::IconOnly => {
                if self.active {
                    (
                        colors::SURFACE_ELEVATED,
                        Color32::from_rgb(40, 45, 55),
                        Color32::from_rgb(27, 30, 38),
                        colors::BORDER_SUBTLE,
                        colors::BORDER_MEDIUM,
                        colors::ACCENT,
                        colors::ACCENT_HOVER,
                    )
                } else {
                    (
                        Color32::TRANSPARENT,
                        colors::SURFACE_HOVER,
                        colors::SURFACE_ACTIVE,
                        Color32::TRANSPARENT,
                        colors::BORDER_SUBTLE,
                        colors::TEXT_SECONDARY,
                        colors::TEXT_PRIMARY,
                    )
                }
            }
            ButtonVariant::Destructive => (
                Color32::TRANSPARENT,
                Color32::from_rgba_premultiplied(239, 68, 68, 30),
                Color32::from_rgba_premultiplied(239, 68, 68, 55),
                Color32::TRANSPARENT,
                Color32::from_rgba_premultiplied(239, 68, 68, 70),
                colors::TEXT_SECONDARY,
                colors::DESTRUCTIVE,
            ),
            ButtonVariant::Ghost => (
                Color32::TRANSPARENT,
                colors::SURFACE_HOVER,
                colors::SURFACE_ACTIVE,
                Color32::TRANSPARENT,
                colors::BORDER_SUBTLE,
                colors::TEXT_SECONDARY,
                colors::TEXT_PRIMARY,
            ),
        };

        // Determine animated rendering colors
        let (bg_color, border_color, fg_color) = if !self.enabled {
            (
                if self.variant == ButtonVariant::Primary { Color32::from_rgb(24, 27, 34) } else { Color32::TRANSPARENT },
                if self.variant == ButtonVariant::Primary { colors::BORDER_SUBTLE } else { Color32::TRANSPARENT },
                colors::TEXT_MUTED,
            )
        } else {
            let bg = motion::lerp_color(base_bg, hover_bg, hover_t);
            let final_bg = motion::lerp_color(bg, press_bg, press_t);

            let border = motion::lerp_color(base_border, hover_border, hover_t);
            let fg = motion::lerp_color(base_fg, hover_fg, hover_t);
            (final_bg, border, fg)
        };

        // 1. Paint button surface background
        let rounding = Rounding::same(4.0);
        ui.painter().rect_filled(rect, rounding, bg_color);

        // 2. Paint border stroke
        if border_color != Color32::TRANSPARENT {
            ui.painter().rect_stroke(rect, rounding, Stroke::new(1.0_f32, border_color));
        }

        // 3. Paint subtle keyboard focus ring if focused
        if response.has_focus() {
            let focus_rect = rect.expand(1.5);
            ui.painter().rect_stroke(focus_rect, Rounding::same(5.0), Stroke::new(1.5_f32, colors::ACCENT));
        }

        // 4. Render content (icon + label) perfectly centered inside button rect
        let content_start_x = rect.center().x - (content_width * 0.5);
        let center_y = rect.center().y;

        let mut current_x = content_start_x;

        if let Some(icon) = self.icon_fn {
            let icon_rect = Rect::from_center_size(
                Pos2::new(current_x + (self.icon_size * 0.5), center_y),
                Vec2::splat(self.icon_size),
            );
            let mut icon_ui = ui.new_child(
                eframe::egui::UiBuilder::new()
                    .max_rect(icon_rect)
                    .layout(eframe::egui::Layout::centered_and_justified(eframe::egui::Direction::LeftToRight)),
            );
            icon(&mut icon_ui, self.icon_size, fg_color);
            current_x += self.icon_size + 6.0;
        }

        if let Some(t) = self.text {
            let text_pos = Pos2::new(current_x + (text_width * 0.5), center_y);
            ui.painter().text(
                text_pos,
                Align2::CENTER_CENTER,
                t,
                FontId::proportional(11.5),
                fg_color,
            );
        }

        // 5. Attach tooltip with optional shortcut
        if let Some(tip) = self.tooltip {
            let full_tip = if let Some(sc) = self.shortcut {
                format!("{} ({})", tip, sc)
            } else {
                tip.to_string()
            };
            response = response.on_hover_text(full_tip);
        }

        response
    }
}
