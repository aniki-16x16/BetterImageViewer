use eframe::egui;

/// Applies a smooth exponential decay easing.
///
/// `current` is the current value.
/// `target` is the target value.
/// `dt` is the time delta since last frame.
/// `speed` is the easing speed multiplier.
///
/// Returns true if the value is still animating (requires repaint).
pub fn exp_decay(current: &mut f32, target: f32, dt: f32, speed: f32) -> bool {
    let t = 1.0 - (-speed * dt).exp();
    let diff = (*current - target).abs();
    if diff > 0.001 {
        *current = *current + (target - *current) * t;
        true
    } else {
        *current = target;
        false
    }
}

pub fn exp_decay_vec2(current: &mut egui::Vec2, target: egui::Vec2, dt: f32, speed: f32) -> bool {
    let t = 1.0 - (-speed * dt).exp();
    let diff = (*current - target).length();
    if diff > 0.1 {
        *current = *current + (target - *current) * t;
        true
    } else {
        *current = target;
        false
    }
}
