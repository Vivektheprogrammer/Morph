#![allow(dead_code)]

use eframe::egui::Color32;

// Centralized Motion and Animation Duration Tokens (in seconds)
pub const BUTTON_HOVER_DURATION: f32 = 0.120; // 120ms
pub const BUTTON_PRESS_DURATION: f32 = 0.090; // 90ms
pub const ICON_TRANSITION_DURATION: f32 = 0.150; // 150ms
pub const TAB_SWITCH_DURATION: f32 = 0.180; // 180ms
pub const SIDEBAR_COLLAPSE_DURATION: f32 = 0.220; // 220ms
pub const DROPDOWN_DURATION: f32 = 0.160; // 160ms
pub const TOOLTIP_DURATION: f32 = 0.120; // 120ms
pub const MODAL_DURATION: f32 = 0.180; // 180ms
pub const PANEL_TRANSITION_DURATION: f32 = 0.200; // 200ms
pub const TOAST_DURATION: f32 = 0.180; // 180ms
pub const THUMBNAIL_SELECTION_DURATION: f32 = 0.150; // 150ms

/// Fast, smooth linear interpolation helper
#[inline]
pub fn lerp(start: f32, end: f32, factor: f32) -> f32 {
    start + (end - start) * factor.clamp(0.0, 1.0)
}

/// Interpolates between two Color32 values with a factor [0.0, 1.0]
#[inline]
pub fn lerp_color(c1: Color32, c2: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let r = lerp(c1.r() as f32, c2.r() as f32, t).round() as u8;
    let g = lerp(c1.g() as f32, c2.g() as f32, t).round() as u8;
    let b = lerp(c1.b() as f32, c2.b() as f32, t).round() as u8;
    let a = lerp(c1.a() as f32, c2.a() as f32, t).round() as u8;
    Color32::from_rgba_premultiplied(r, g, b, a)
}

/// Normalized angle lerp avoiding full 360 spin artifacts
#[inline]
pub fn lerp_angle_deg(start: f32, end: f32, factor: f32) -> f32 {
    let diff = (end - start) % 360.0;
    let shortest_diff = ((2.0 * diff) % 360.0) - diff;
    start + shortest_diff * factor.clamp(0.0, 1.0)
}

/// Evaluates a 1D cubic-bezier easing curve given control points (x1, y1) and (x2, y2).
/// Fits standard web / desktop easing curves:
/// - Standard: (0.2, 0.0, 0.0, 1.0)
/// - Enter / Decelerate: (0.0, 0.0, 0.0, 1.0)
/// - Exit / Accelerate: (0.4, 0.0, 1.0, 1.0)
pub fn cubic_bezier(t: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    // Approximate solution via Newton-Raphson
    let mut current_t = t;
    for _ in 0..4 {
        let current_x = 3.0 * (1.0 - current_t) * (1.0 - current_t) * current_t * x1
            + 3.0 * (1.0 - current_t) * current_t * current_t * x2
            + current_t * current_t * current_t;
        let dx = 3.0 * (1.0 - current_t) * (1.0 - current_t) * x1
            + 6.0 * (1.0 - current_t) * current_t * (x2 - x1)
            + 3.0 * current_t * current_t * (1.0 - x2);
        if dx.abs() < 1e-5 {
            break;
        }
        current_t -= (current_x - t) / dx;
        current_t = current_t.clamp(0.0, 1.0);
    }
    // Compute y at current_t
    3.0 * (1.0 - current_t) * (1.0 - current_t) * current_t * y1
        + 3.0 * (1.0 - current_t) * current_t * current_t * y2
        + current_t * current_t * current_t
}

/// Standard cubic-bezier easing: (0.2, 0.0, 0.0, 1.0)
#[inline]
pub fn ease_standard(t: f32) -> f32 {
    cubic_bezier(t, 0.2, 0.0, 0.0, 1.0)
}

/// Enter / Deceleration easing: (0.0, 0.0, 0.0, 1.0)
#[inline]
pub fn ease_enter(t: f32) -> f32 {
    cubic_bezier(t, 0.0, 0.0, 0.0, 1.0)
}

/// Exit / Acceleration easing: (0.4, 0.0, 1.0, 1.0)
#[inline]
pub fn ease_exit(t: f32) -> f32 {
    cubic_bezier(t, 0.4, 0.0, 1.0, 1.0)
}

/// Spring-damping state tracker for smooth organic motion
#[derive(Clone, Copy, Debug)]
pub struct Spring {
    pub value: f32,
    pub target: f32,
    pub velocity: f32,
    pub stiffness: f32,
    pub damping: f32,
}

impl Default for Spring {
    fn default() -> Self {
        Self {
            value: 0.0,
            target: 0.0,
            velocity: 0.0,
            stiffness: 180.0,
            damping: 18.0,
        }
    }
}

impl Spring {
    pub fn new(initial: f32) -> Self {
        Self {
            value: initial,
            target: initial,
            velocity: 0.0,
            stiffness: 180.0,
            damping: 18.0,
        }
    }

    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    /// Advances the spring by dt seconds. Returns true only while actively in motion.
    pub fn update(&mut self, dt: f32) -> bool {
        let dt = dt.clamp(0.001, 0.05);
        let displacement = self.value - self.target;

        // Epsilon snapping: cleanly halt repainting when settled
        if displacement.abs() < 0.005 && self.velocity.abs() < 0.005 {
            self.value = self.target;
            self.velocity = 0.0;
            return false;
        }

        let spring_force = -self.stiffness * displacement;
        let damping_force = -self.damping * self.velocity;
        let acceleration = spring_force + damping_force;

        self.velocity += acceleration * dt;
        self.value += self.velocity * dt;

        true
    }

    pub fn snap_to_target(&mut self) {
        self.value = self.target;
        self.velocity = 0.0;
    }
}
