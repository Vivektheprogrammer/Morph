#![allow(dead_code)]

use super::colors;
use eframe::egui::{epaint::Shadow, Frame, Margin, Rounding, Stroke, Vec2};

pub fn shadow_card() -> Shadow {
    Shadow {
        offset: Vec2::new(0.0, 1.0),
        blur: 4.0,
        spread: 0.0,
        color: eframe::egui::Color32::from_black_alpha(40),
    }
}

pub fn shadow_card_selected() -> Shadow {
    Shadow {
        offset: Vec2::new(0.0, 2.0),
        blur: 8.0,
        spread: 0.0,
        color: eframe::egui::Color32::from_rgba_premultiplied(57, 132, 255, 45),
    }
}

pub fn shadow_dropdown() -> Shadow {
    Shadow {
        offset: Vec2::new(0.0, 4.0),
        blur: 14.0,
        spread: 0.0,
        color: eframe::egui::Color32::from_black_alpha(80),
    }
}

pub fn shadow_document() -> Shadow {
    Shadow {
        offset: Vec2::new(0.0, 4.0),
        blur: 20.0,
        spread: 2.0,
        color: eframe::egui::Color32::from_black_alpha(90),
    }
}

pub fn shadow_floating() -> Shadow {
    Shadow {
        offset: Vec2::new(0.0, 3.0),
        blur: 10.0,
        spread: 0.0,
        color: eframe::egui::Color32::from_black_alpha(60),
    }
}

pub fn frame_card(is_selected: bool, is_hovered: bool) -> Frame {
    let bg = if is_selected {
        colors::SURFACE_CARD_SELECTED
    } else if is_hovered {
        colors::SURFACE_HOVER
    } else {
        colors::BG_SECONDARY
    };

    let stroke = if is_selected {
        Stroke::new(1.8_f32, colors::BORDER_SELECTED)
    } else if is_hovered {
        Stroke::new(1.0_f32, colors::BORDER_CARD_HOVER)
    } else {
        Stroke::new(1.0_f32, colors::BORDER_SUBTLE)
    };

    let shadow = if is_selected {
        shadow_card_selected()
    } else {
        shadow_card()
    };

    Frame::none()
        .fill(bg)
        .stroke(stroke)
        .shadow(shadow)
        .rounding(Rounding::same(4.0))
        .inner_margin(Margin::same(4.0))
}

pub fn frame_toolbar() -> Frame {
    Frame::none()
        .fill(colors::BG_SECONDARY)
        .stroke(Stroke::new(1.0_f32, colors::BORDER_SUBTLE))
        .inner_margin(Margin::symmetric(8.0, 4.0))
}

pub fn frame_shelf() -> Frame {
    Frame::none()
        .fill(colors::BG_SECONDARY)
        .stroke(Stroke::new(1.0_f32, colors::BORDER_SUBTLE))
        .inner_margin(Margin::symmetric(8.0, 4.0))
}

pub fn frame_floating_bar() -> Frame {
    Frame::none()
        .fill(colors::SURFACE_ELEVATED)
        .stroke(Stroke::new(1.0_f32, colors::BORDER_SUBTLE))
        .shadow(shadow_floating())
        .rounding(Rounding::same(6.0))
        .inner_margin(Margin::symmetric(8.0, 4.0))
}
