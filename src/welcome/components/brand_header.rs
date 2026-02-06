// Brand header component - logo and welcome text
use crate::welcome::style::{sizing, spacing, typography, AppTheme, ColorPalette};
use eframe::egui;
use std::sync::Arc;

/// Render the brand header with logo and welcome text
pub fn render_brand_header(
    ui: &mut egui::Ui,
    palette: &ColorPalette,
    _theme: AppTheme,
    logo_texture: &Option<Arc<egui::TextureHandle>>,
) {
    ui.vertical_centered(|ui| {
        ui.add_space(spacing::MEDIUM);

        // Logo container - fixed size square (Figma: 113x113px, 30px rounding, #F3F1ED bg)
        let logo_bg = egui::Color32::from_rgb(243, 241, 237);
        let container_size = sizing::LOGO_SIZE;
        let inner_logo_size = container_size * 0.55; // Logo takes ~55% of container

        // Allocate exact size for the logo container
        let (rect, _response) = ui.allocate_exact_size(
            egui::vec2(container_size, container_size),
            egui::Sense::hover(),
        );

        if ui.is_rect_visible(rect) {
            // Draw rounded background
            ui.painter()
                .rect_filled(rect, sizing::LOGO_ROUNDING, logo_bg);

            // Draw logo centered in the container
            if let Some(texture) = logo_texture {
                let logo_rect = egui::Rect::from_center_size(
                    rect.center(),
                    egui::vec2(inner_logo_size, inner_logo_size),
                );
                // Use TRANSPARENT tint to show original image colors
                ui.painter().image(
                    texture.id(),
                    logo_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 255), // No tint
                );
            } else {
                // Fallback - draw centered text
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "🍳",
                    egui::FontId::proportional(inner_logo_size),
                    palette.text_primary,
                );
            }
        }

        ui.add_space(spacing::MEDIUM);

        // Main heading (Figma: 34px)
        ui.label(
            egui::RichText::new("Welcome to Cook Sync")
                .size(typography::HERO_SIZE)
                .color(palette.text_primary)
                .strong(),
        );

        ui.add_space(spacing::SMALL);

        // Subheading / value proposition (Figma: 15px)
        ui.label(
            egui::RichText::new("Keep your recipes synced across all your devices")
                .size(typography::BODY_REGULAR_SIZE)
                .color(palette.text_secondary),
        );

        ui.add_space(spacing::MEDIUM);
    });
}
