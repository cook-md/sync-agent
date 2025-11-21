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

    // Main card frame
    style::card_frame(palette).show(ui, |ui| {
        ui.set_min_width(400.0);

        // Step 1: Authentication
        render_step1_auth(ui, state, palette, &mut response);

        ui.add_space(spacing::MEDIUM);
        render_divider(ui, palette);
        ui.add_space(spacing::MEDIUM);

        // Step 2: Directory Selection
        render_step2_directory(ui, state, palette, &mut response);

        ui.add_space(spacing::MEDIUM);
        render_divider(ui, palette);
        ui.add_space(spacing::MEDIUM);

        // Step 3: Preferences (collapsible)
        render_step3_preferences(ui, state, palette);
    });

    response
}

#[derive(Default)]
pub struct SetupStepsResponse {
    pub login_clicked: bool,
    pub select_directory_clicked: bool,
}

fn render_step1_auth(
    ui: &mut egui::Ui,
    state: &mut WelcomeState,
    palette: &ColorPalette,
    response: &mut SetupStepsResponse,
) {
    ui.horizontal(|ui| {
        // Status indicator
        style::status_indicator(
            ui,
            state.is_step1_complete(),
            state.is_step1_in_progress(),
            palette,
        );

        ui.add_space(spacing::SMALL);

        // Step title
        ui.label(
            egui::RichText::new("Step 1: Connect to Cook.md")
                .size(typography::SECTION_HEADING_SIZE)
                .color(palette.text_primary)
                .strong(),
        );
    });

    ui.add_space(spacing::SMALL);

    // Login button or status
    if state.is_logging_in && !state.is_logged_in() {
        // Show login in progress
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("⟳")
                    .size(sizing::ICON_SIZE_SMALL)
                    .color(palette.brand_orange),
            );
            ui.label(
                egui::RichText::new("Logging in... Check your browser")
                    .size(typography::BODY_REGULAR_SIZE)
                    .color(palette.text_secondary),
            );
        });
    } else if state.is_logged_in() {
        // Show actually logged in status (only if we have a user email)
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("✓")
                    .size(sizing::ICON_SIZE_SMALL)
                    .color(palette.success),
            );
            ui.label(
                egui::RichText::new(format!(
                    "Logged in as {}",
                    state.user_email.as_deref().unwrap_or("user")
                ))
                .size(typography::BODY_REGULAR_SIZE)
                .color(palette.text_secondary),
            );
        });
    } else {
        // Show login button
        if style::primary_button(ui, "Login to Cook.md", true, palette).clicked() {
            response.login_clicked = true;
        }

        // Show error if any
        if let Some(ref error) = state.login_error {
            ui.add_space(spacing::SMALL);
            style::error_message(ui, error, palette);
        }

        // Helper text
        if state.login_error.is_none() {
            ui.add_space(spacing::MICRO);
            ui.label(
                egui::RichText::new("Click to open browser for authentication")
                    .size(typography::CAPTION_SIZE)
                    .color(palette.text_secondary),
            );
        }
    }
}

fn render_step2_directory(
    ui: &mut egui::Ui,
    state: &mut WelcomeState,
    palette: &ColorPalette,
    response: &mut SetupStepsResponse,
) {
    ui.horizontal(|ui| {
        // Status indicator
        style::status_indicator(
            ui,
            state.is_step2_complete(),
            false, // Never in progress for directory
            palette,
        );

        ui.add_space(spacing::SMALL);

        // Step title
        ui.label(
            egui::RichText::new("Step 2: Choose recipes directory")
                .size(typography::SECTION_HEADING_SIZE)
                .color(palette.text_primary)
                .strong(),
        );
    });

    ui.add_space(spacing::SMALL);

    // Directory picker
    ui.horizontal(|ui| {
        // Directory display / input
        let dir_text = state
            .recipes_dir
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "Click to select your recipes folder".to_string());

        let text_color = if state.recipes_dir.is_some() {
            palette.text_primary
        } else {
            palette.text_secondary
        };

        let mut frame = egui::Frame::none()
            .fill(palette.surface)
            .stroke(egui::Stroke::new(
                1.0,
                if state.directory_error.is_some() {
                    palette.error
                } else {
                    palette.border
                },
            ))
            .rounding(sizing::INPUT_ROUNDING)
            .inner_margin(egui::Margin::symmetric(spacing::MEDIUM, spacing::SMALL));

        if state.recipes_dir.is_none() {
            // Make border dashed for empty state
            frame = frame.stroke(egui::Stroke::new(1.0, palette.border));
        }

        frame.show(ui, |ui| {
            ui.set_min_width(280.0);
            ui.set_min_height(sizing::INPUT_HEIGHT - spacing::SMALL * 2.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("📁").size(sizing::ICON_SIZE_SMALL));
                ui.label(
                    egui::RichText::new(dir_text)
                        .size(typography::BODY_REGULAR_SIZE)
                        .color(text_color)
                        .monospace(),
                );
            });
        });

        // Browse button
        if style::icon_button(ui, "📂", palette).clicked() {
            response.select_directory_clicked = true;
        }
    });

    // Show error if any
    if let Some(ref error) = state.directory_error {
        ui.add_space(spacing::SMALL);
        style::error_message(ui, error, palette);
    }

    // Helper text
    if state.directory_error.is_none() {
        ui.add_space(spacing::MICRO);
        ui.label(
            egui::RichText::new("Select the folder where you keep your .cook recipe files")
                .size(typography::CAPTION_SIZE)
                .color(palette.text_secondary),
        );
    }
}

fn render_step3_preferences(ui: &mut egui::Ui, state: &mut WelcomeState, palette: &ColorPalette) {
    // Collapsible header
    let header_response = ui.horizontal(|ui| {
        // Always show circle (no completion for optional step)
        ui.label(
            egui::RichText::new("○")
                .size(sizing::ICON_SIZE_MEDIUM)
                .color(palette.border),
        );

        ui.add_space(spacing::SMALL);

        // Step title
        ui.label(
            egui::RichText::new("Step 3: Preferences (Optional)")
                .size(typography::SECTION_HEADING_SIZE)
                .color(palette.text_primary)
                .strong(),
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Expand/collapse icon
            let icon = if state.preferences_expanded {
                "▲"
            } else {
                "▼"
            };
            ui.label(
                egui::RichText::new(icon)
                    .size(typography::BODY_REGULAR_SIZE)
                    .color(palette.text_secondary),
            );
        });
    });

    // Make the whole header clickable
    if header_response
        .response
        .interact(egui::Sense::click())
        .clicked()
    {
        state.toggle_preferences();
    }

    // Show preferences when expanded
    if state.preferences_expanded {
        ui.add_space(spacing::SMALL);

        // Auto-start checkbox
        ui.horizontal(|ui| {
            ui.add_space(sizing::ICON_SIZE_MEDIUM + spacing::SMALL);
            if ui.checkbox(&mut state.auto_start, "").changed() {
                // Checkbox state already updated
            }
            ui.label(
                egui::RichText::new("Start automatically when I log in")
                    .size(typography::BODY_REGULAR_SIZE)
                    .color(palette.text_primary),
            );
        });

        ui.add_space(spacing::MICRO);

        // Auto-update checkbox
        ui.horizontal(|ui| {
            ui.add_space(sizing::ICON_SIZE_MEDIUM + spacing::SMALL);
            if ui.checkbox(&mut state.auto_update, "").changed() {
                // Checkbox state already updated
            }
            ui.label(
                egui::RichText::new("Download updates automatically")
                    .size(typography::BODY_REGULAR_SIZE)
                    .color(palette.text_primary),
            );
        });
    }
}

fn render_divider(ui: &mut egui::Ui, palette: &ColorPalette) {
    ui.separator();
    // Override separator color
    ui.style_mut()
        .visuals
        .widgets
        .noninteractive
        .bg_stroke
        .color = palette.border;
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
