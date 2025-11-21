// Components module
pub mod action_buttons;
pub mod brand_header;
pub mod setup_steps;

pub use action_buttons::render_action_buttons;
pub use brand_header::render_brand_header;
pub use setup_steps::render_setup_steps;

use crate::welcome::style::AppTheme;
use eframe::egui;
use std::sync::Arc;

/// Load the Cook logo based on the current theme
pub fn load_logo(ctx: &egui::Context, theme: AppTheme) -> Option<Arc<egui::TextureHandle>> {
    let logo_path = match theme {
        AppTheme::Dark => "assets/logo-white-64.png",
        AppTheme::Light => "assets/logo-black-64.png",
    };

    // Try to load from the assets directory relative to the executable
    let full_path = std::env::current_exe()
        .ok()
        .and_then(|exe_path| {
            exe_path
                .parent()
                .map(|parent| parent.join("../..").join(logo_path))
        })
        .or_else(|| Some(std::path::PathBuf::from(logo_path)));

    if let Some(path) = full_path {
        if let Ok(image_bytes) = std::fs::read(&path) {
            if let Ok(image) = image::load_from_memory(&image_bytes) {
                let rgba_image = image.to_rgba8();
                let size = [rgba_image.width() as usize, rgba_image.height() as usize];
                let pixels = rgba_image.as_flat_samples();

                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());

                let texture =
                    ctx.load_texture("cook-logo", color_image, egui::TextureOptions::default());

                return Some(Arc::new(texture));
            }
        }
    }

    None
}
