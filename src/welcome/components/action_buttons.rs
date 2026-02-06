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

    ui.add_space(spacing::MEDIUM);

    ui.vertical_centered(|ui| {
        if state.can_proceed() {
            // Enabled "Get Started" button
            let button_response = style::primary_button(ui, "Get Started", true, palette);
            if button_response.clicked() {
                clicked = true;
            }
        } else {
            // Disabled button with Figma styling (#F9F8F6 bg, orange text)
            let button_response =
                style::action_button_disabled(ui, "Complete required steps first", palette);
            button_response.on_hover_text("Please complete Steps 1 and 2 to continue");
        }
    });

    ui.add_space(spacing::MEDIUM);

    clicked
}
