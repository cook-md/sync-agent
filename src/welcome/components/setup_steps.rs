// Setup steps component - authentication, directory, preferences
use crate::welcome::state::WelcomeState;
use crate::welcome::style::{self, sizing, spacing, typography, ColorPalette};
use eframe::egui;
use std::path::PathBuf;

/// Render all setup steps in a card
pub fn render_setup_steps(
    ui: &mut egui::Ui,
    state: &mut WelcomeState,
    palette: &ColorPalette,
) -> SetupStepsResponse {
    let mut response = SetupStepsResponse::default();

    // Main card frame (no border per Figma)
    style::card_frame(palette).show(ui, |ui| {
        // Use available width (respects parent margins)

        // Step 1: Authentication
        render_step1_auth(ui, state, palette, &mut response);

        render_divider(ui, palette);

        // Step 2: Directory Selection
        render_step2_directory(ui, state, palette, &mut response);

        render_divider(ui, palette);

        // Step 3: Preferences
        render_step3_preferences(ui, state, palette);
    });

    response
}

#[derive(Default)]
pub struct SetupStepsResponse {
    pub login_clicked: bool,
    pub select_directory_clicked: bool,
}

/// Render a step header with status indicator and title
fn render_step_header(
    ui: &mut egui::Ui,
    step_num: u8,
    title: &str,
    completed: bool,
    in_progress: bool,
    palette: &ColorPalette,
) {
    ui.horizontal(|ui| {
        // Status indicator (orange circle with checkmark when complete)
        style::status_indicator(ui, completed, in_progress, palette);

        ui.add_space(spacing::SMALL);

        // Step title with colored "Step N:" prefix
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("Step {}:", step_num))
                    .size(typography::SECTION_HEADING_SIZE)
                    .color(palette.brand_orange)
                    .strong(),
            );
            ui.label(
                egui::RichText::new(format!(" {}", title))
                    .size(typography::SECTION_HEADING_SIZE)
                    .color(palette.text_primary)
                    .strong(),
            );
        });
    });
}

fn render_step1_auth(
    ui: &mut egui::Ui,
    state: &mut WelcomeState,
    palette: &ColorPalette,
    response: &mut SetupStepsResponse,
) {
    // Step container with padding
    egui::Frame::none()
        .inner_margin(egui::Margin::same(spacing::MEDIUM))
        .show(ui, |ui| {
            // Header
            render_step_header(
                ui,
                1,
                "Connect to Cook.md",
                state.is_step1_complete(),
                state.is_step1_in_progress(),
                palette,
            );

            ui.add_space(spacing::MEDIUM);

            // Content area
            if state.is_logging_in && !state.is_logged_in() {
                // Login in progress
                render_login_in_progress(ui, palette);
            } else if state.is_logged_in() {
                // Logged in - show email with change link
                render_logged_in_state(ui, state, palette, response);
            } else {
                // Not logged in - show login button
                render_login_button(ui, palette, response);
            }

            // Show error if any
            if let Some(ref error) = state.login_error {
                ui.add_space(spacing::SMALL);
                style::error_message(ui, error, palette);
            }
        });
}

fn render_login_in_progress(ui: &mut egui::Ui, _palette: &ColorPalette) {
    // Calculate centering manually
    let available_width = ui.available_width();
    let box_width = 320.0;
    let left_space = ((available_width - box_width) / 2.0).max(0.0);

    ui.horizontal(|ui| {
        ui.add_space(left_space);
        egui::Frame::none()
            .fill(egui::Color32::WHITE)
            .rounding(sizing::BUTTON_ROUNDING)
            .inner_margin(egui::Margin::symmetric(spacing::XLARGE, spacing::LARGE))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.add_space(spacing::SMALL);
                    ui.label(
                        egui::RichText::new("Logging in... Check your browser")
                            .size(typography::BODY_REGULAR_SIZE)
                            .color(egui::Color32::from_rgba_unmultiplied(22, 22, 29, 204)),
                    );
                });
            });
    });
}

fn render_logged_in_state(
    ui: &mut egui::Ui,
    state: &WelcomeState,
    palette: &ColorPalette,
    _response: &mut SetupStepsResponse,
) {
    // Calculate centering manually
    let available_width = ui.available_width();
    let box_width = 280.0;
    let left_space = ((available_width - box_width) / 2.0).max(0.0);

    ui.horizontal(|ui| {
        ui.add_space(left_space);
        // White background card with email (always white per Figma)
        egui::Frame::none()
            .fill(egui::Color32::WHITE)
            .rounding(sizing::BUTTON_ROUNDING)
            .inner_margin(egui::Margin::symmetric(spacing::XLARGE, spacing::LARGE))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    // Email display (dark text per Figma)
                    ui.label(
                        egui::RichText::new(
                            state.user_email.as_deref().unwrap_or("user@example.com"),
                        )
                        .size(typography::BUTTON_TEXT_SIZE)
                        .color(egui::Color32::from_rgb(22, 22, 29))
                        .strong(),
                    );

                    ui.add_space(spacing::SMALL);

                    // Change e-mail link
                    ui.label(
                        egui::RichText::new("Change e-mail")
                            .size(typography::BODY_REGULAR_SIZE)
                            .color(palette.brand_orange),
                    );
                });
            });
    });
}

fn render_login_button(
    ui: &mut egui::Ui,
    palette: &ColorPalette,
    response: &mut SetupStepsResponse,
) {
    ui.add_space(spacing::MEDIUM);

    // Center the button
    ui.vertical_centered(|ui| {
        if style::primary_button(ui, "Login to Cook.md", true, palette).clicked() {
            response.login_clicked = true;
        }
    });

    ui.add_space(spacing::SMALL);

    // Helper text centered
    ui.vertical_centered(|ui| {
        ui.label(
            egui::RichText::new("Click to open browser for authentication")
                .size(typography::BODY_REGULAR_SIZE)
                .color(palette.text_secondary),
        );
    });
}

fn render_step2_directory(
    ui: &mut egui::Ui,
    state: &mut WelcomeState,
    palette: &ColorPalette,
    response: &mut SetupStepsResponse,
) {
    // Step container with padding
    egui::Frame::none()
        .inner_margin(egui::Margin::same(spacing::MEDIUM))
        .show(ui, |ui| {
            // Header
            render_step_header(
                ui,
                2,
                "Choose recipes directory",
                state.is_step2_complete(),
                false,
                palette,
            );

            ui.add_space(spacing::MEDIUM);

            // Content area
            if state.recipes_dir.is_some() {
                // Directory selected - show path with change link
                render_directory_selected(ui, state, palette, response);
            } else {
                // No directory - show picker button
                render_directory_picker(ui, palette, response);
            }

            // Show error if any
            if let Some(ref error) = state.directory_error {
                ui.add_space(spacing::SMALL);
                style::error_message(ui, error, palette);
            }
        });
}

fn render_directory_picker(
    ui: &mut egui::Ui,
    palette: &ColorPalette,
    response: &mut SetupStepsResponse,
) {
    // Calculate centering manually
    let available_width = ui.available_width();
    let button_width = 320.0; // Approximate button width
    let left_space = ((available_width - button_width) / 2.0).max(0.0);

    // Center the button by adding space on the left
    ui.horizontal(|ui| {
        ui.add_space(left_space);

        // Create a clickable area for the entire button
        let button_id = ui.make_persistent_id("directory_picker_button");
        let desired_size = egui::vec2(button_width, 44.0);
        let (rect, button_response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

        if button_response.clicked() {
            response.select_directory_clicked = true;
        }

        // Draw the button visuals
        if ui.is_rect_visible(rect) {
            let _visuals = ui.style().interact(&button_response);

            // Outer dashed border
            ui.painter().rect_stroke(
                rect,
                sizing::BUTTON_ROUNDING,
                egui::Stroke::new(1.0, palette.border_dashed),
            );

            // Inner white fill
            let inner_rect = rect.shrink(spacing::MICRO);
            ui.painter().rect_filled(
                inner_rect,
                sizing::BUTTON_ROUNDING - 2.0,
                egui::Color32::WHITE,
            );

            // Draw icon and text
            let text_color = egui::Color32::from_rgb(22, 22, 29);
            let icon_pos = inner_rect.left_center() + egui::vec2(spacing::MEDIUM, 0.0);
            ui.painter().text(
                icon_pos,
                egui::Align2::LEFT_CENTER,
                "📁",
                egui::FontId::proportional(sizing::ICON_SIZE_SMALL),
                text_color,
            );

            let text_pos = icon_pos + egui::vec2(sizing::ICON_SIZE_SMALL + spacing::SMALL, 0.0);
            ui.painter().text(
                text_pos,
                egui::Align2::LEFT_CENTER,
                "Click to select your recipes folder",
                egui::FontId::proportional(typography::BUTTON_TEXT_SIZE),
                text_color,
            );
        }

        // Change cursor on hover
        if button_response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        let _ = button_id; // suppress unused warning
    });

    ui.add_space(spacing::SMALL);

    // Helper text - centered
    ui.vertical_centered(|ui| {
        ui.label(
            egui::RichText::new("Select the folder where you keep your .cook recipe files")
                .size(typography::BODY_REGULAR_SIZE)
                .color(palette.text_secondary),
        );
    });
}

fn render_directory_selected(
    ui: &mut egui::Ui,
    state: &WelcomeState,
    palette: &ColorPalette,
    response: &mut SetupStepsResponse,
) {
    // Dashed border with path
    let dir_text = state
        .recipes_dir
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_default();

    // Measure actual text width using layout
    let font_id = egui::FontId::proportional(typography::BUTTON_TEXT_SIZE);
    let text_galley =
        ui.fonts(|f| f.layout_no_wrap(dir_text.clone(), font_id, egui::Color32::BLACK));
    let text_width = text_galley.size().x;

    // Total content width: icon + spacing + text + frame padding
    let content_width =
        sizing::ICON_SIZE_MEDIUM + spacing::SMALL * 3.0 + text_width + spacing::SMALL * 2.0;

    let available = ui.available_width();
    let left_offset = ((available - content_width) / 2.0).max(0.0);

    ui.horizontal(|ui| {
        ui.add_space(left_offset);
        let button_response = egui::Frame::none()
            .stroke(egui::Stroke::new(1.0, palette.border_dashed))
            .rounding(sizing::BUTTON_ROUNDING)
            .inner_margin(egui::Margin::symmetric(spacing::SMALL, spacing::SMALL))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("📁").size(sizing::ICON_SIZE_MEDIUM));
                    ui.add_space(spacing::SMALL);
                    ui.label(
                        egui::RichText::new(&dir_text)
                            .size(typography::BUTTON_TEXT_SIZE)
                            .color(egui::Color32::from_rgb(22, 22, 29)),
                    );
                });
            });

        // Make clickable to change
        if button_response
            .response
            .interact(egui::Sense::click())
            .clicked()
        {
            response.select_directory_clicked = true;
        }
    });

    ui.add_space(spacing::MEDIUM);

    // Change folder link centered
    ui.vertical_centered(|ui| {
        if style::text_link(ui, "Change folder", palette).clicked() {
            response.select_directory_clicked = true;
        }
    });
}

fn render_step3_preferences(ui: &mut egui::Ui, state: &mut WelcomeState, palette: &ColorPalette) {
    // Step container with padding
    egui::Frame::none()
        .inner_margin(egui::Margin::same(spacing::MEDIUM))
        .show(ui, |ui| {
            // Step 3 is optional - show checkmark when both required steps are done
            let step3_complete = state.is_step1_complete() && state.is_step2_complete();

            render_step_header(
                ui,
                3,
                "Preferences (Optional)",
                step3_complete,
                false,
                palette,
            );

            // Always show preferences content - steps can be completed in any order
            ui.add_space(spacing::MEDIUM);
            render_preferences_checkboxes(ui, state, palette);
        });
}

fn render_preferences_checkboxes(
    ui: &mut egui::Ui,
    state: &mut WelcomeState,
    palette: &ColorPalette,
) {
    // Indent checkboxes to align with step content (past the status indicator)
    let indent = sizing::ICON_SIZE_MEDIUM + spacing::SMALL;

    // Auto-start checkbox
    ui.horizontal(|ui| {
        ui.add_space(indent);
        render_custom_checkbox(ui, &mut state.auto_start, palette);
        ui.add_space(spacing::SMALL);
        ui.label(
            egui::RichText::new("Auto-start the app with system")
                .size(typography::BUTTON_TEXT_SIZE)
                .color(palette.text_primary),
        );
    });

    ui.add_space(spacing::SMALL);

    // Auto-update checkbox
    ui.horizontal(|ui| {
        ui.add_space(indent);
        render_custom_checkbox(ui, &mut state.auto_update, palette);
        ui.add_space(spacing::SMALL);
        ui.label(
            egui::RichText::new("Automatically update")
                .size(typography::BUTTON_TEXT_SIZE)
                .color(palette.text_primary),
        );
    });
}

/// Render a custom square checkbox matching Figma design
fn render_custom_checkbox(ui: &mut egui::Ui, checked: &mut bool, palette: &ColorPalette) {
    let size = sizing::ICON_SIZE_MEDIUM;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click());

    if response.clicked() {
        *checked = !*checked;
    }

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let center = rect.center();
        let rounding = 4.0;
        let rect_inset = rect.shrink(2.0);

        if *checked {
            // Orange filled square with white checkmark
            painter.rect_filled(rect_inset, rounding, palette.brand_orange);

            // White checkmark
            let check_color = egui::Color32::WHITE;
            let stroke = egui::Stroke::new(2.0, check_color);

            let check_start = center + egui::vec2(-4.0, 0.0);
            let check_mid = center + egui::vec2(-1.0, 3.0);
            let check_end = center + egui::vec2(5.0, -4.0);

            painter.line_segment([check_start, check_mid], stroke);
            painter.line_segment([check_mid, check_end], stroke);
        } else {
            // Empty square with rounded corners (unchecked state)
            painter.rect_stroke(
                rect_inset,
                rounding,
                egui::Stroke::new(1.5, palette.border_dashed),
            );
        }
    }
}

fn render_divider(ui: &mut egui::Ui, palette: &ColorPalette) {
    let rect = ui.available_rect_before_wrap();
    let painter = ui.painter();

    let y = rect.top();
    let start = egui::pos2(rect.left(), y);
    let end = egui::pos2(rect.right(), y);

    painter.line_segment([start, end], egui::Stroke::new(1.0, palette.border));

    // Reserve the space
    ui.allocate_space(egui::vec2(rect.width(), 1.0));
}

/// Handle directory selection dialog
pub fn handle_directory_selection(state: &mut WelcomeState) {
    if let Some(dir) = rfd::FileDialog::new()
        .set_title("Select Recipes Directory")
        .pick_folder()
    {
        // Validate directory
        if validate_directory(&dir) {
            state.set_recipes_dir(dir);
        } else {
            state.set_directory_error(
                "Directory is not writable. Please choose another location.".to_string(),
            );
        }
    }
}

/// Validate that directory is writable
fn validate_directory(dir: &PathBuf) -> bool {
    // Check if directory is writable
    match std::fs::metadata(dir) {
        Ok(metadata) => {
            // Check if it's a directory and writable
            if !metadata.is_dir() {
                return false;
            }

            // Try to create a temporary file to test write permissions
            let test_file = dir.join(".cook_sync_test");
            match std::fs::write(&test_file, b"test") {
                Ok(_) => {
                    // Clean up test file
                    let _ = std::fs::remove_file(&test_file);
                    true
                }
                Err(_) => false,
            }
        }
        Err(_) => false,
    }
}
