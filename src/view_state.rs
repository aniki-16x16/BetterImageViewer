use eframe::egui;

pub struct ViewState {
    pub zoom: f32,
    pub pan: egui::Vec2,
    pub target_zoom: f32,
    pub target_pan: egui::Vec2,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: egui::Vec2::ZERO,
            target_zoom: 1.0,
            target_pan: egui::Vec2::ZERO,
        }
    }
}

impl ViewState {
    pub fn reset(&mut self) {
        self.zoom = 1.0;
        self.pan = egui::Vec2::ZERO;
        self.target_zoom = 1.0;
        self.target_pan = egui::Vec2::ZERO;
    }

    pub fn process_input(&mut self, ui: &mut egui::Ui) {
        // 0. Handle Double Click to Reset
        if ui.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary)) {
            self.target_zoom = 1.0;
            self.target_pan = egui::Vec2::ZERO;
        }

        // 1. Handle Zoom (Scroll)
        let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);

        if scroll_delta != 0.0 {
            // A typical mouse wheel click is 50 points.
            // We scale the scroll delta to determine how many "steps" to zoom.
            let zoom_steps = scroll_delta / 50.0;
            let zoom_multiplier = 1.2_f32.powf(zoom_steps);

            let pointer_pos = ui
                .input(|i| i.pointer.hover_pos())
                .unwrap_or(ui.clip_rect().center());

            let old_target_zoom = self.target_zoom;
            self.target_zoom *= zoom_multiplier;
            self.target_zoom = self.target_zoom.clamp(0.01, 500.0);

            // Calculate the new target pan so the zoom is centered on the mouse pointer
            let center_screen = ui.clip_rect().center().to_vec2();
            let rel_m = pointer_pos.to_vec2() - center_screen;

            self.target_pan =
                rel_m - (rel_m - self.target_pan) * (self.target_zoom / old_target_zoom);
        }

        // 2. Handle Pan (Mouse Drag)
        let is_dragging = ui.input(|i| {
            i.pointer.button_down(egui::PointerButton::Primary)
                || i.pointer.button_down(egui::PointerButton::Middle)
        });

        if is_dragging {
            let delta = ui.input(|i| i.pointer.delta());
            self.target_pan += delta;
            self.pan += delta; // Instant pan for responsiveness
        }

        // 3. Animate Zoom and Pan
        let dt = ui.input(|i| i.unstable_dt).min(0.1);
        let speed = 15.0;
        let t = 1.0 - (-speed * dt).exp();

        let zoom_diff = (self.zoom - self.target_zoom).abs();
        let pan_diff = (self.pan - self.target_pan).length();

        if zoom_diff > 0.001 || pan_diff > 0.1 {
            self.zoom = self.zoom + (self.target_zoom - self.zoom) * t;
            self.pan = self.pan + (self.target_pan - self.pan) * t;
            ui.ctx().request_repaint(); // Keep repainting until animation finishes
        } else {
            self.zoom = self.target_zoom;
            self.pan = self.target_pan;
        }
    }
}
