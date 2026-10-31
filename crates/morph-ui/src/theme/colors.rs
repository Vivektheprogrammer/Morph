#![allow(dead_code)]

use eframe::egui::Color32;

// Professional Windows Desktop Palette (Acrobat / Office / Fluent dark)
pub const BG_APP: Color32 = Color32::from_rgb(17, 19, 24);         // #111318 Application canvas
pub const BG_SECONDARY: Color32 = Color32::from_rgb(24, 27, 34);   // #181B22 Toolbars & sidebar
pub const SURFACE_ELEVATED: Color32 = Color32::from_rgb(32, 36, 45);// #20242D Active tabs & cards
pub const SURFACE_HOVER: Color32 = Color32::from_rgb(40, 45, 57);   // #282D39 Subtle hover
pub const SURFACE_ACTIVE: Color32 = Color32::from_rgb(48, 54, 68);  // Pressed surface
pub const SURFACE_CARD_SELECTED: Color32 = Color32::from_rgb(26, 36, 56); // Selected thumbnail

// Accents (Morph / Windows Fluent Accent)
pub const ACCENT: Color32 = Color32::from_rgb(57, 132, 255);       // #3984FF Primary blue
pub const ACCENT_HOVER: Color32 = Color32::from_rgb(82, 148, 255); // #5294FF
pub const ACCENT_MUTED: Color32 = Color32::from_rgba_premultiplied(57, 132, 255, 40);

// Destructive (Muted Red)
pub const DESTRUCTIVE: Color32 = Color32::from_rgb(239, 68, 68);    // #EF4444
pub const DESTRUCTIVE_BG: Color32 = Color32::from_rgb(55, 20, 24);  // Subtle red background
pub const DESTRUCTIVE_BORDER: Color32 = Color32::from_rgb(127, 29, 29);
pub const DESTRUCTIVE_HOVER: Color32 = Color32::from_rgb(220, 38, 38);

// Typography
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(243, 244, 246);  // #F3F4F6
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(146, 153, 167);// #9299A7
pub const TEXT_MUTED: Color32 = Color32::from_rgb(105, 112, 126);   // Inactive/shortcuts
pub const TEXT_ON_ACCENT: Color32 = Color32::WHITE;

// Borders & Dividers
pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(48, 53, 65);   // #303541 Clean 1px border
pub const BORDER_LIGHT: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 18);
pub const BORDER_SELECTED: Color32 = Color32::from_rgb(57, 132, 255);

// Document Canvas
pub const CANVAS_WORKSPACE: Color32 = Color32::from_rgb(15, 17, 22);
pub const DOCUMENT_SHEET: Color32 = Color32::WHITE;

// Aliases for compatibility
pub const BG_CANVAS: Color32 = BG_APP;
pub const SURFACE_BASE: Color32 = BG_SECONDARY;
pub const SURFACE_CARD: Color32 = BG_SECONDARY;
pub const ACCENT_PRIMARY: Color32 = ACCENT;
pub const ACCENT_LIGHT: Color32 = ACCENT_HOVER;
pub const ACCENT_CYAN: Color32 = ACCENT;
pub const ACCENT_AMBER: Color32 = Color32::from_rgb(245, 158, 11);
pub const ACCENT_EMERALD: Color32 = Color32::from_rgb(34, 197, 94);
pub const ACCENT_CRIMSON: Color32 = DESTRUCTIVE;
pub const BORDER_MEDIUM: Color32 = BORDER_SUBTLE;
pub const BORDER_CARD: Color32 = BORDER_SUBTLE;
pub const BORDER_CARD_HOVER: Color32 = Color32::from_rgb(70, 78, 96);
pub const BADGE_PDF_BG: Color32 = Color32::from_rgb(45, 20, 24);
pub const BADGE_PDF_TXT: Color32 = Color32::from_rgb(248, 113, 113);
pub const BADGE_IMG_BG: Color32 = Color32::from_rgb(16, 38, 28);
pub const BADGE_IMG_TXT: Color32 = Color32::from_rgb(52, 211, 153);
pub const BADGE_ROT_BG: Color32 = Color32::from_rgb(48, 32, 14);
pub const BADGE_ROT_TXT: Color32 = Color32::from_rgb(251, 191, 36);
