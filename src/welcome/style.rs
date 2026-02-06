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

// Color palette based on the Figma design spec
pub struct ColorPalette {
    // Brand color
    pub brand_orange: egui::Color32,

    // Backgrounds
    pub background: egui::Color32,
    pub surface: egui::Color32,
    #[allow(dead_code)]
    pub surface_secondary: egui::Color32, // For completed step content areas

    // Text
    pub text_primary: egui::Color32,
    pub text_secondary: egui::Color32,

    // Borders and dividers
    pub border: egui::Color32,
    pub border_dashed: egui::Color32, // For dashed border buttons

    // Button backgrounds
    pub button_disabled_bg: egui::Color32,

    // Semantic colors
    #[allow(dead_code)]
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
            // Brand orange from Figma: #E15A29
            brand_orange: egui::Color32::from_rgb(225, 90, 41),
            background: egui::Color32::from_rgb(255, 255, 255),
            // Card background from Figma: #F3F3F2
            surface: egui::Color32::from_rgb(243, 243, 242),
            // White background for completed step content
            surface_secondary: egui::Color32::from_rgb(255, 255, 255),
            // Text primary from Figma: #16161D
            text_primary: egui::Color32::from_rgb(22, 22, 29),
            // Text secondary from Figma: rgba(22,22,29,0.8)
            text_secondary: egui::Color32::from_rgba_unmultiplied(22, 22, 29, 204),
            // Divider color (light gray)
            border: egui::Color32::from_rgb(233, 236, 239),
            // Dashed border color from Figma: #AE9890
            border_dashed: egui::Color32::from_rgb(174, 152, 144),
            // Disabled button background from Figma: #F9F8F6
            button_disabled_bg: egui::Color32::from_rgb(249, 248, 246),
            success: egui::Color32::from_rgb(40, 167, 69),
            error: egui::Color32::from_rgb(220, 53, 69),
            warning: egui::Color32::from_rgb(255, 193, 7),
            info: egui::Color32::from_rgb(0, 123, 255),
        }
    }

    pub fn dark() -> Self {
        // For welcome screen, use same colors as light mode per Figma design
        // The Figma design is light-themed, so we keep the same appearance
        Self {
            brand_orange: egui::Color32::from_rgb(225, 90, 41),
            background: egui::Color32::from_rgb(255, 255, 255),
            // Card background from Figma: #F3F3F2
            surface: egui::Color32::from_rgb(243, 243, 242),
            surface_secondary: egui::Color32::from_rgb(255, 255, 255),
            text_primary: egui::Color32::from_rgb(22, 22, 29),
            text_secondary: egui::Color32::from_rgba_unmultiplied(22, 22, 29, 204),
            border: egui::Color32::from_rgb(233, 236, 239),
            border_dashed: egui::Color32::from_rgb(174, 152, 144),
            button_disabled_bg: egui::Color32::from_rgb(249, 248, 246),
            success: egui::Color32::from_rgb(40, 167, 69),
            error: egui::Color32::from_rgb(220, 53, 69),
            warning: egui::Color32::from_rgb(255, 193, 7),
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

// Typography constants - reduced sizes for compact layout
pub mod typography {
    // Hero title
    pub const HERO_SIZE: f32 = 28.0;
    // Step headings
    pub const SECTION_HEADING_SIZE: f32 = 18.0;
    // Button text
    pub const BUTTON_TEXT_SIZE: f32 = 15.0;
    // Body text
    pub const BODY_LARGE_SIZE: f32 = 14.0;
    pub const BODY_REGULAR_SIZE: f32 = 13.0;
    // Caption
    #[allow(dead_code)]
    pub const CAPTION_SIZE: f32 = 11.0;
}

// Component sizing - updated from Figma design
pub mod sizing {
    pub const BUTTON_HEIGHT: f32 = 48.0;
    pub const BUTTON_MIN_WIDTH: f32 = 180.0;
    pub const BUTTON_PADDING_H: f32 = 24.0;
    pub const BUTTON_PADDING_V: f32 = 12.0;
    // Button rounding from Figma: 12px
    pub const BUTTON_ROUNDING: f32 = 12.0;

    #[allow(dead_code)]
    pub const INPUT_HEIGHT: f32 = 48.0;
    pub const INPUT_ROUNDING: f32 = 12.0;

    // Card rounding
    pub const CARD_ROUNDING: f32 = 12.0;

    pub const ICON_SIZE_SMALL: f32 = 18.0;
    pub const ICON_SIZE_MEDIUM: f32 = 20.0;
    // Logo size - reduced for compact layout
    pub const LOGO_SIZE: f32 = 80.0;
    pub const LOGO_ROUNDING: f32 = 20.0;
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
    let fill_color = if enabled {
        palette.brand_orange
    } else {
        palette.border
    };

    // Create button with proper sizing
    let button = egui::Button::new(
        egui::RichText::new(label)
            .size(typography::BUTTON_TEXT_SIZE)
            .color(egui::Color32::WHITE)
            .strong(),
    )
    .fill(fill_color)
    .rounding(sizing::BUTTON_ROUNDING);

    ui.add(button)
}

/// Render a disabled action button (Figma: #F9F8F6 bg with orange text)
pub fn action_button_disabled(
    ui: &mut egui::Ui,
    label: &str,
    palette: &ColorPalette,
) -> egui::Response {
    // Use Frame to ensure correct background color
    let mut response = None;
    egui::Frame::none()
        .fill(palette.button_disabled_bg)
        .rounding(sizing::BUTTON_ROUNDING)
        .inner_margin(egui::Margin::symmetric(
            sizing::BUTTON_PADDING_H,
            sizing::BUTTON_PADDING_V,
        ))
        .show(ui, |ui| {
            let label_response = ui.add(
                egui::Label::new(
                    egui::RichText::new(label)
                        .size(typography::BUTTON_TEXT_SIZE)
                        .color(palette.brand_orange),
                )
                .sense(egui::Sense::hover()),
            );
            response = Some(label_response);
        });

    response.unwrap()
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
#[allow(dead_code)]
pub fn icon_button(ui: &mut egui::Ui, icon: &str, palette: &ColorPalette) -> egui::Response {
    let button = egui::Button::new(egui::RichText::new(icon).size(sizing::ICON_SIZE_SMALL))
        .fill(palette.surface)
        .min_size(egui::vec2(sizing::BUTTON_HEIGHT, sizing::BUTTON_HEIGHT))
        .rounding(sizing::BUTTON_ROUNDING);

    ui.add(button)
}

/// Render a status indicator icon (Figma: orange filled circle with white checkmark)
pub fn status_indicator(
    ui: &mut egui::Ui,
    completed: bool,
    in_progress: bool,
    palette: &ColorPalette,
) {
    let size = sizing::ICON_SIZE_MEDIUM;
    let (rect, _response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let center = rect.center();
        let radius = size / 2.0;

        if completed || in_progress {
            // Orange filled circle
            painter.circle_filled(center, radius, palette.brand_orange);

            if completed {
                // White checkmark inside
                let check_color = egui::Color32::WHITE;
                let stroke = egui::Stroke::new(2.0, check_color);

                // Draw checkmark path
                let check_start = center + egui::vec2(-4.0, 0.0);
                let check_mid = center + egui::vec2(-1.0, 3.0);
                let check_end = center + egui::vec2(5.0, -4.0);

                painter.line_segment([check_start, check_mid], stroke);
                painter.line_segment([check_mid, check_end], stroke);
            } else {
                // Spinning indicator (just show filled circle during progress)
            }
        } else {
            // Empty orange circle outline
            painter.circle_stroke(
                center,
                radius - 1.0,
                egui::Stroke::new(2.0, palette.brand_orange),
            );
        }
    }
}

/// Render an error message box (centered)
pub fn error_message(ui: &mut egui::Ui, message: &str, palette: &ColorPalette) {
    // Estimate content width for centering
    let estimated_width =
        message.len() as f32 * 7.0 + sizing::ICON_SIZE_SMALL + spacing::MEDIUM * 3.0;
    let available = ui.available_width();
    let left_offset = ((available - estimated_width) / 2.0).max(0.0);

    ui.horizontal(|ui| {
        ui.add_space(left_offset);
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
    });
}

/// Render a card/surface container (no border per Figma design)
pub fn card_frame(palette: &ColorPalette) -> egui::Frame {
    egui::Frame::none()
        .fill(palette.surface)
        .rounding(sizing::CARD_ROUNDING)
        .inner_margin(egui::Margin::same(0.0)) // Steps have their own padding
}

/// Render a text link in brand orange
pub fn text_link(ui: &mut egui::Ui, text: &str, palette: &ColorPalette) -> egui::Response {
    let label = egui::Label::new(
        egui::RichText::new(text)
            .size(typography::BODY_REGULAR_SIZE)
            .color(palette.brand_orange),
    )
    .sense(egui::Sense::click());

    let response = ui.add(label);

    // Add underline on hover
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    response
}
