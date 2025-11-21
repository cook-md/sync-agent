// Theme and styling for the welcome screen
use eframe::egui;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppTheme {
    Light,
    Dark,
}

impl AppTheme {
    /// Detect system theme using dark-light crate
    pub fn detect() -> Self {
        match dark_light::detect() {
            Ok(dark_light::Mode::Dark) => AppTheme::Dark,
            _ => AppTheme::Light,
        }
    }
}

// Color palette based on the design spec
pub struct ColorPalette {
    // Brand color
    pub brand_orange: egui::Color32,

    // Backgrounds
    pub background: egui::Color32,
    pub surface: egui::Color32,

    // Text
    pub text_primary: egui::Color32,
    pub text_secondary: egui::Color32,

    // Borders and dividers
    pub border: egui::Color32,

    // Semantic colors
    pub success: egui::Color32,
    pub error: egui::Color32,
    #[allow(dead_code)]
    pub warning: egui::Color32,
    #[allow(dead_code)]
    pub info: egui::Color32,
}

impl ColorPalette {
    pub fn light() -> Self {
        Self {
            brand_orange: egui::Color32::from_rgb(255, 107, 53),
            background: egui::Color32::from_rgb(255, 255, 255),
            surface: egui::Color32::from_rgb(248, 249, 250),
            text_primary: egui::Color32::from_rgb(26, 26, 26),
            text_secondary: egui::Color32::from_rgb(108, 117, 125),
            border: egui::Color32::from_rgb(233, 236, 239),
            success: egui::Color32::from_rgb(40, 167, 69),
            error: egui::Color32::from_rgb(220, 53, 69),
            warning: egui::Color32::from_rgb(255, 193, 7),
            info: egui::Color32::from_rgb(0, 123, 255),
        }
    }

    pub fn dark() -> Self {
        Self {
            brand_orange: egui::Color32::from_rgb(255, 107, 53),
            background: egui::Color32::from_rgb(26, 26, 26),
            surface: egui::Color32::from_rgb(45, 45, 45),
            text_primary: egui::Color32::from_rgb(248, 249, 250),
            text_secondary: egui::Color32::from_rgb(173, 181, 189),
            border: egui::Color32::from_rgb(64, 64, 64),
            success: egui::Color32::from_rgb(52, 199, 89),
            error: egui::Color32::from_rgb(255, 69, 58),
            warning: egui::Color32::from_rgb(255, 214, 10),
            info: egui::Color32::from_rgb(0, 123, 255),
        }
    }

    pub fn for_theme(theme: AppTheme) -> Self {
        match theme {
            AppTheme::Light => Self::light(),
            AppTheme::Dark => Self::dark(),
        }
    }
}

// Spacing constants (8-point grid)
pub mod spacing {
    pub const MICRO: f32 = 4.0;
    pub const SMALL: f32 = 8.0;
    pub const MEDIUM: f32 = 16.0;
    pub const LARGE: f32 = 24.0;
    pub const XLARGE: f32 = 32.0;
    #[allow(dead_code)]
    pub const XXLARGE: f32 = 48.0;
}

// Typography constants
pub mod typography {
    pub const HERO_SIZE: f32 = 32.0;
    pub const SECTION_HEADING_SIZE: f32 = 20.0;
    pub const BODY_LARGE_SIZE: f32 = 16.0;
    pub const BODY_REGULAR_SIZE: f32 = 14.0;
    pub const CAPTION_SIZE: f32 = 12.0;
}

// Component sizing
pub mod sizing {
    pub const BUTTON_HEIGHT: f32 = 44.0;
    pub const BUTTON_MIN_WIDTH: f32 = 180.0;
    pub const BUTTON_PADDING_H: f32 = 24.0;
    pub const BUTTON_PADDING_V: f32 = 12.0;
    pub const BUTTON_ROUNDING: f32 = 8.0;

    pub const INPUT_HEIGHT: f32 = 48.0;
    pub const INPUT_ROUNDING: f32 = 8.0;

    pub const ICON_SIZE_SMALL: f32 = 20.0;
    pub const ICON_SIZE_MEDIUM: f32 = 24.0;
    #[allow(dead_code)]
    pub const LOGO_SIZE: f32 = 64.0;
}

/// Configure egui style with our design system
pub fn configure_style(ctx: &egui::Context, theme: AppTheme) {
    let mut style = (*ctx.style()).clone();
    let palette = ColorPalette::for_theme(theme);

    // Window background
    style.visuals.window_fill = palette.background;
    style.visuals.panel_fill = palette.background;

    // Spacing
    style.spacing.button_padding = egui::vec2(sizing::BUTTON_PADDING_H, sizing::BUTTON_PADDING_V);
    style.spacing.item_spacing = egui::vec2(spacing::SMALL, spacing::SMALL);
    style.spacing.window_margin = egui::Margin::same(spacing::XLARGE);

    // Rounding
    style.visuals.widgets.noninteractive.rounding = egui::Rounding::same(sizing::BUTTON_ROUNDING);
    style.visuals.widgets.inactive.rounding = egui::Rounding::same(sizing::BUTTON_ROUNDING);
    style.visuals.widgets.hovered.rounding = egui::Rounding::same(sizing::BUTTON_ROUNDING);
    style.visuals.widgets.active.rounding = egui::Rounding::same(sizing::BUTTON_ROUNDING);

    // Text colors
    style.visuals.override_text_color = Some(palette.text_primary);

    ctx.set_style(style);
}

/// Render a primary button (brand orange, high emphasis)
pub fn primary_button(
    ui: &mut egui::Ui,
    label: &str,
    enabled: bool,
    palette: &ColorPalette,
) -> egui::Response {
    let button = egui::Button::new(
        egui::RichText::new(label)
            .size(typography::BODY_LARGE_SIZE)
            .color(egui::Color32::WHITE),
    )
    .fill(if enabled {
        palette.brand_orange
    } else {
        palette.border
    })
    .min_size(egui::vec2(sizing::BUTTON_MIN_WIDTH, sizing::BUTTON_HEIGHT))
    .rounding(sizing::BUTTON_ROUNDING);

    ui.add_enabled(enabled, button)
}

/// Render a secondary button (outlined, medium emphasis)
#[allow(dead_code)]
pub fn secondary_button(
    ui: &mut egui::Ui,
    label: &str,
    enabled: bool,
    palette: &ColorPalette,
) -> egui::Response {
    let button = egui::Button::new(
        egui::RichText::new(label)
            .size(typography::BODY_LARGE_SIZE)
            .color(palette.text_primary),
    )
    .fill(egui::Color32::TRANSPARENT)
    .stroke(egui::Stroke::new(2.0, palette.border))
    .min_size(egui::vec2(sizing::BUTTON_MIN_WIDTH, sizing::BUTTON_HEIGHT))
    .rounding(sizing::BUTTON_ROUNDING);

    ui.add_enabled(enabled, button)
}

/// Render a small icon button
pub fn icon_button(ui: &mut egui::Ui, icon: &str, palette: &ColorPalette) -> egui::Response {
    let button = egui::Button::new(egui::RichText::new(icon).size(sizing::ICON_SIZE_SMALL))
        .fill(palette.surface)
        .min_size(egui::vec2(sizing::BUTTON_HEIGHT, sizing::BUTTON_HEIGHT))
        .rounding(sizing::BUTTON_ROUNDING);

    ui.add(button)
}

/// Render a status indicator icon
pub fn status_indicator(
    ui: &mut egui::Ui,
    completed: bool,
    in_progress: bool,
    palette: &ColorPalette,
) {
    let (icon, color) = if completed {
        ("✓", palette.success)
    } else if in_progress {
        ("⟳", palette.brand_orange)
    } else {
        ("○", palette.border)
    };

    ui.label(
        egui::RichText::new(icon)
            .size(sizing::ICON_SIZE_MEDIUM)
            .color(color),
    );
}

/// Render an error message box
pub fn error_message(ui: &mut egui::Ui, message: &str, palette: &ColorPalette) {
    egui::Frame::none()
        .fill(palette.error.linear_multiply(0.1))
        .stroke(egui::Stroke::new(1.0, palette.error))
        .rounding(sizing::INPUT_ROUNDING)
        .inner_margin(egui::Margin::same(spacing::MEDIUM))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("⚠️").size(sizing::ICON_SIZE_SMALL));
                ui.label(
                    egui::RichText::new(message)
                        .size(typography::BODY_REGULAR_SIZE)
                        .color(palette.error),
                );
            });
        });
}

/// Render a card/surface container
pub fn card_frame(palette: &ColorPalette) -> egui::Frame {
    egui::Frame::none()
        .fill(palette.surface)
        .stroke(egui::Stroke::new(1.0, palette.border))
        .rounding(sizing::INPUT_ROUNDING)
        .inner_margin(egui::Margin::same(spacing::LARGE))
}
