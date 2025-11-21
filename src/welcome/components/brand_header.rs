// Brand header component - logo and welcome text
use crate::welcome::style::{spacing, typography, AppTheme, ColorPalette};
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
        ui.add_space(spacing::XLARGE);

        // Display the logo if loaded, otherwise show placeholder
        if let Some(texture) = logo_texture {
            ui.image(egui::ImageSource::Texture(egui::load::SizedTexture {
                id: texture.id(),
                size: egui::vec2(64.0, 64.0),
            }));
        } else {
            // Fallback to emoji if logo fails to load
            ui.label(egui::RichText::new("🍳").size(64.0));
        }

        ui.add_space(spacing::MEDIUM);

        // Main heading
        ui.label(
            egui::RichText::new("Welcome to Cook Sync")
                .size(typography::HERO_SIZE)
                .color(palette.text_primary)
                .strong(),
        );

        ui.add_space(spacing::SMALL);

        // Subheading / value proposition
        ui.label(
            egui::RichText::new("Keep your recipes synced across all your devices")
                .size(typography::BODY_LARGE_SIZE)
                .color(palette.text_secondary),
        );

        ui.add_space(spacing::XLARGE);
    });
}
