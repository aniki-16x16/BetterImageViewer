use crate::animation::exp_decay;
use crate::image_loader::{ThumbnailCommand, ThumbnailLoader, ThumbnailResult};
use eframe::egui;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct ThumbnailList {
    is_expanded: bool,
    expand_progress: f32,
    hover_opacity: f32, // For the bottom arrow

    loader: ThumbnailLoader,
    thumbnails: HashMap<PathBuf, egui::TextureHandle>,
    loading_path: Option<PathBuf>,
}

pub enum ThumbnailAction {
    None,
    SelectImage(usize),
}

impl Default for ThumbnailList {
    fn default() -> Self {
        panic!("Cannot use default without context");
    }
}

impl ThumbnailList {
    pub fn new(ctx: &egui::Context) -> Self {
        Self {
            is_expanded: false,
            expand_progress: 0.0,
            hover_opacity: 0.0,
            loader: ThumbnailLoader::new(ctx.clone()),
            thumbnails: HashMap::new(),
            loading_path: None,
        }
    }

    pub fn update_folder(&mut self, folder_images: &[PathBuf], current_index: usize) {
        if folder_images.is_empty() {
            return;
        }

        // Evict if too many
        if self.thumbnails.len() > 200 {
            let mut keys: Vec<_> = self.thumbnails.keys().cloned().collect();
            // Sort by distance from current index. We just do simple string index comparison for now,
            // but we can find the exact index in folder_images.
            keys.sort_by(|a, b| {
                let a_idx = folder_images.iter().position(|p| p == a).unwrap_or(0);
                let b_idx = folder_images.iter().position(|p| p == b).unwrap_or(0);

                let dist_a = (a_idx as isize - current_index as isize).abs();
                let dist_b = (b_idx as isize - current_index as isize).abs();

                dist_b.cmp(&dist_a) // furthest first
            });

            while self.thumbnails.len() > 100 {
                if let Some(k) = keys.first() {
                    self.thumbnails.remove(k);
                }
                keys.remove(0);
            }
        }

        self.try_load_next(folder_images, current_index);
    }

    fn try_load_next(&mut self, folder_images: &[PathBuf], current_index: usize) {
        if self.loading_path.is_none() && !folder_images.is_empty() {
            // Find nearest missing thumbnail
            let mut best_idx = None;
            let mut min_dist = usize::MAX;

            for (i, path) in folder_images.iter().enumerate() {
                if !self.thumbnails.contains_key(path) {
                    let dist = (i as isize - current_index as isize).abs() as usize;
                    if dist < min_dist {
                        min_dist = dist;
                        best_idx = Some(i);
                    }
                }
            }

            if let Some(idx) = best_idx {
                let path = folder_images[idx].clone();
                self.loading_path = Some(path.clone());
                let _ = self.loader.tx.send(ThumbnailCommand::Load(path, 128));
            }
        }
    }

    pub fn process_results(
        &mut self,
        ctx: &egui::Context,
        folder_images: &[PathBuf],
        current_index: usize,
    ) {
        let mut loaded = false;
        while let Ok(result) = self.loader.rx.try_recv() {
            match result {
                ThumbnailResult::Success(path, color_image) => {
                    let texture = ctx.load_texture(
                        format!("thumb_{}", path.to_string_lossy()),
                        color_image,
                        egui::TextureOptions::LINEAR,
                    );
                    self.thumbnails.insert(path.clone(), texture);
                }
                ThumbnailResult::Error(path, _err) => {
                    // For now, we can just track that we attempted and failed.
                    // To prevent infinite loops we could insert a dummy invisible texture or
                    // just let it not find it and not retry if we had a "failed" set.
                    // For simplicity we create an empty 1x1 image as placeholder.
                    let dummy = egui::ColorImage::from_rgba_unmultiplied([1, 1], &[0, 0, 0, 0]);
                    let texture = ctx.load_texture(
                        format!("thumb_err_{}", path.to_string_lossy()),
                        dummy,
                        egui::TextureOptions::LINEAR,
                    );
                    self.thumbnails.insert(path.clone(), texture);
                }
            }
            loaded = true;
        }

        if loaded {
            self.loading_path = None;
            self.try_load_next(folder_images, current_index);
        }
    }

    pub fn display(
        &mut self,
        ui: &mut egui::Ui,
        folder_images: &[PathBuf],
        current_index: usize,
    ) -> ThumbnailAction {
        let mut action = ThumbnailAction::None;

        let ctx = ui.ctx().clone();
        let dt = ctx.input(|i| i.stable_dt).min(0.1);

        // 1. Animations
        let target_expand = if self.is_expanded { 1.0 } else { 0.0 };
        if exp_decay(&mut self.expand_progress, target_expand, dt, 15.0) {
            ctx.request_repaint();
        }

        let screen_rect = ui.clip_rect();
        let panel_height = 150.0;
        let arrow_height = 30.0;
        let arrow_width = 100.0;

        let current_y_offset = screen_rect.max.y
            - (panel_height * self.expand_progress)
            - (arrow_height * (1.0 - self.expand_progress));

        // 2. Control logic
        let arrow_rect = egui::Rect::from_min_size(
            egui::pos2(screen_rect.center().x - arrow_width / 2.0, current_y_offset),
            egui::vec2(arrow_width, arrow_height),
        );

        let panel_rect = egui::Rect::from_min_size(
            egui::pos2(screen_rect.min.x, current_y_offset + arrow_height),
            egui::vec2(screen_rect.width(), panel_height),
        );

        // Arrow hover interaction
        let interact_rect = if self.is_expanded {
            panel_rect.union(arrow_rect)
        } else {
            arrow_rect
        };
        if let Some(pointer) = ctx.input(|i| i.pointer.hover_pos()) {
            let is_hovered = interact_rect.contains(pointer);
            let target_hover = if is_hovered || self.is_expanded {
                1.0
            } else {
                0.3
            };
            if exp_decay(&mut self.hover_opacity, target_hover, dt, 10.0) {
                ctx.request_repaint();
            }

            if arrow_rect.contains(pointer)
                && ctx.input(|i| i.pointer.button_clicked(egui::PointerButton::Primary))
            {
                self.is_expanded = !self.is_expanded;
            }
        } else {
            let target_hover = if self.is_expanded { 1.0 } else { 0.3 };
            if exp_decay(&mut self.hover_opacity, target_hover, dt, 10.0) {
                ctx.request_repaint();
            }
        }

        // 3. Draw Arrow
        let arrow_color =
            egui::Color32::from_rgba_premultiplied(50, 50, 50, (200.0 * self.hover_opacity) as u8);
        ui.painter().rect(
            egui::Rect::from_min_size(
                arrow_rect.min,
                egui::vec2(
                    arrow_rect.width(),
                    arrow_rect.height() + (panel_height * self.expand_progress),
                ),
            ),
            egui::Rounding {
                nw: 10.0,
                ne: 10.0,
                sw: 0.0,
                se: 0.0,
            },
            arrow_color,
            egui::Stroke::NONE,
        );

        // Draw icon on arrow manually (vector shape instead of text characters)
        let center = arrow_rect.center();
        let arrow_color_fg = egui::Color32::from_white_alpha((255.0 * self.hover_opacity) as u8);

        let path = if self.is_expanded {
            vec![
                center + egui::vec2(-6.0, -2.0),
                center + egui::vec2(6.0, -2.0),
                center + egui::vec2(0.0, 4.0),
            ]
        } else {
            vec![
                center + egui::vec2(-6.0, 2.0),
                center + egui::vec2(6.0, 2.0),
                center + egui::vec2(0.0, -4.0),
            ]
        };

        ui.painter().add(egui::Shape::convex_polygon(
            path,
            arrow_color_fg,
            egui::Stroke::NONE,
        ));

        // 4. Draw Panel when expanding
        if self.expand_progress > 0.01 {
            let panel_bg = egui::Color32::from_rgba_premultiplied(
                30,
                30,
                30,
                (220.0 * self.expand_progress) as u8,
            );
            ui.painter().rect(
                panel_rect,
                egui::Rounding::ZERO,
                panel_bg,
                egui::Stroke::NONE,
            );

            let mut child_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(panel_rect.shrink(10.0))
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );

            egui::ScrollArea::horizontal()
                .auto_shrink([false, false])
                .drag_to_scroll(true)
                .show(&mut child_ui, |ui| {
                    if folder_images.is_empty() {
                        ui.label("No images");
                        return;
                    }

                    for (i, path) in folder_images.iter().enumerate() {
                        let is_current = i == current_index;

                        let thrb_width = 100.0;
                        let item_size = egui::vec2(thrb_width, 130.0);
                        let (rect, response) =
                            ui.allocate_exact_size(item_size, egui::Sense::click());

                        if response.clicked() {
                            action = ThumbnailAction::SelectImage(i);
                        }

                        if ui.is_rect_visible(rect) {
                            // Border & Background
                            let bg_color = if response.hovered() {
                                egui::Color32::from_rgba_premultiplied(80, 80, 80, 100)
                            } else {
                                egui::Color32::TRANSPARENT
                            };

                            let stroke = if is_current {
                                egui::Stroke::new(2.0, egui::Color32::LIGHT_BLUE)
                            } else {
                                egui::Stroke::NONE
                            };

                            ui.painter().rect(rect, 5.0, bg_color, stroke);

                            // Image
                            let mut thumb_rect = rect;
                            thumb_rect.min.y += 25.0; // Space for text
                            thumb_rect = thumb_rect.shrink(5.0);

                            if let Some(texture) = self.thumbnails.get(path) {
                                // Keep aspect ratio
                                let aspect = texture.size()[0] as f32 / texture.size()[1] as f32;
                                let mut draw_size = thumb_rect.size();
                                if draw_size.x / draw_size.y > aspect {
                                    draw_size.x = draw_size.y * aspect;
                                } else {
                                    draw_size.y = draw_size.x / aspect;
                                }

                                let center_min = thumb_rect.center() - draw_size / 2.0;
                                let draw_rect = egui::Rect::from_min_size(center_min, draw_size);

                                ui.painter().image(
                                    texture.id(),
                                    draw_rect,
                                    egui::Rect::from_min_max(
                                        egui::pos2(0.0, 0.0),
                                        egui::pos2(1.0, 1.0),
                                    ),
                                    egui::Color32::WHITE,
                                );
                            } else {
                                ui.painter().rect(
                                    thumb_rect,
                                    2.0,
                                    egui::Color32::from_gray(50),
                                    egui::Stroke::NONE,
                                );
                                ui.painter().text(
                                    thumb_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "...",
                                    egui::FontId::proportional(14.0),
                                    egui::Color32::GRAY,
                                );
                            }

                            // File name
                            let file_name_str =
                                path.file_name().unwrap_or_default().to_string_lossy();
                            let mut display_name = file_name_str.to_string();
                            if display_name.chars().count() > 12 {
                                display_name = display_name.chars().take(11).collect();
                                display_name.push('…');
                            }

                            let text_color = if is_current {
                                egui::Color32::LIGHT_BLUE
                            } else {
                                egui::Color32::WHITE
                            };
                            let text_pos = rect.min + egui::vec2(5.0, 5.0);

                            ui.painter().text(
                                text_pos,
                                egui::Align2::LEFT_TOP,
                                display_name,
                                egui::FontId::proportional(12.0),
                                text_color,
                            );
                        }
                    }
                });
        }

        action
    }
}
