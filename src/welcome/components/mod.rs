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
pub fn load_logo(ctx: &egui::Context, _theme: AppTheme) -> Option<Arc<egui::TextureHandle>> {
    // Always use black logo since welcome screen uses light colors (per Figma design)
    let logo_filename = "logo-black-64.png";

    // Try multiple paths to find the logo
    let possible_paths = [
        // Development: relative to crate root
        format!("assets/{}", logo_filename),
        // Installed: relative to executable
        std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|p| p.join("assets").join(logo_filename)))
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default(),
        // macOS bundle: inside .app
        std::env::current_exe()
            .ok()
            .and_then(|exe| {
                exe.parent()
                    .and_then(|p| p.parent())
                    .map(|p| p.join("Resources/assets").join(logo_filename))
            })
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default(),
    ];

    for path in &possible_paths {
        if path.is_empty() {
            continue;
        }
        if let Ok(image_bytes) = std::fs::read(path) {
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
