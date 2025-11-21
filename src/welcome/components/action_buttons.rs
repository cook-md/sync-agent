// Action buttons component - Get Started button
use crate::welcome::state::WelcomeState;
use crate::welcome::style::{self, spacing, ColorPalette};
use eframe::egui;

/// Render the action buttons at the bottom
pub fn render_action_buttons(
    ui: &mut egui::Ui,
    state: &WelcomeState,
    palette: &ColorPalette,
) -> bool {
    let mut clicked = false;

    ui.add_space(spacing::LARGE);

    ui.vertical_centered(|ui| {
        // Get Started button
        let button_label = if state.can_proceed() {
            "Get Started"
        } else {
            "Complete required steps first"
        };

        let button_response = style::primary_button(ui, button_label, state.can_proceed(), palette);

        if button_response.clicked() {
            clicked = true;
        }

        // Show tooltip if not ready
        if !state.can_proceed() {
            button_response.on_hover_text("Please complete Steps 1 and 2 to continue");
        }
    });

    clicked
}
