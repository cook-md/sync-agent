// Welcome screen application using egui
use crate::error::Result;
use crate::welcome::components::{render_action_buttons, render_brand_header, render_setup_steps};
use crate::welcome::state::WelcomeState;
use crate::welcome::style::{self, AppTheme, ColorPalette};
use eframe::egui;
use log::info;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Load window icon from assets
fn load_window_icon() -> Option<egui::IconData> {
    let icon_paths = [
        "assets/logo-1024.png",
        "assets/icon-256.png",
        "assets/icon-128.png",
    ];

    for path in &icon_paths {
        if let Ok(image_bytes) = std::fs::read(path) {
            if let Ok(image) = image::load_from_memory(&image_bytes) {
                let rgba = image.to_rgba8();
                let (width, height) = rgba.dimensions();
                return Some(egui::IconData {
                    rgba: rgba.into_raw(),
                    width,
                    height,
                });
            }
        }
    }
    None
}

// Note: We don't open the browser here because we need to start the OAuth callback
// server first (which happens in browser_login()). The browser will be opened
// automatically when the daemon starts and calls browser_login() if login_requested is true.

#[derive(Debug, Clone)]
pub struct WelcomeResult {
    pub login_requested: bool,
    pub recipes_dir: Option<PathBuf>,
    pub auto_start: bool,
    pub auto_update: bool,
}

pub struct WelcomeApp {
    pub state: WelcomeState,
    theme: AppTheme,
    palette: ColorPalette,
    logo_texture: Option<Arc<egui::TextureHandle>>,
    result_ref: Arc<Mutex<WelcomeResult>>,
}

impl Default for WelcomeApp {
    fn default() -> Self {
        Self::new()
    }
}

impl WelcomeApp {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::with_result_ref(Arc::new(Mutex::new(WelcomeResult {
            login_requested: false,
            recipes_dir: None,
            auto_start: true,
            auto_update: true,
        })))
    }

    fn with_result_ref(result_ref: Arc<Mutex<WelcomeResult>>) -> Self {
        let theme = AppTheme::detect();
        let palette = ColorPalette::for_theme(theme);

        Self {
            state: WelcomeState::default(),
            theme,
            palette,
            logo_texture: None, // Will be loaded on first frame
            result_ref,
        }
    }
}

impl eframe::App for WelcomeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Load logo on first frame if not already loaded
        if self.logo_texture.is_none() {
            use crate::welcome::components::load_logo;
            self.logo_texture = load_logo(ctx, self.theme);
        }

        // Configure style for the theme
        style::configure_style(ctx, self.theme);

        // Add horizontal margin to the whole panel
        let frame = egui::Frame::central_panel(&ctx.style())
            .inner_margin(egui::Margin::symmetric(16.0, 0.0)); // 16px horizontal margin

        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    // Center content vertically
                    ui.vertical_centered(|ui| {
                        // Minimal top spacing
                        ui.add_space(style::spacing::SMALL);

                        // Brand header
                        render_brand_header(ui, &self.palette, self.theme, &self.logo_texture);

                        // Setup steps card
                        let steps_response = render_setup_steps(ui, &mut self.state, &self.palette);

                        // Handle login click
                        if steps_response.login_clicked {
                            info!("Login button clicked - starting browser login");
                            self.state.start_browser_login();
                        }

                        // Update state from login status (check for changes from background thread)
                        self.state.update_from_login_status();
                        ctx.request_repaint(); // Keep updating UI while login is in progress

                        // Handle directory selection click
                        if steps_response.select_directory_clicked {
                            use crate::welcome::components::setup_steps::handle_directory_selection;
                            handle_directory_selection(&mut self.state);
                        }

                        // Action buttons
                        if render_action_buttons(ui, &self.state, &self.palette) {
                            // Save the result before closing
                            if let Ok(mut result) = self.result_ref.lock() {
                                // Login already happened, so we only need to pass directory and preferences
                                result.login_requested = false; // Not needed anymore since we logged in
                                result.recipes_dir = self.state.recipes_dir.clone();
                                result.auto_start = self.state.auto_start;
                                result.auto_update = self.state.auto_update;
                            }
                            self.state.request_close();
                        }
                    });
                });
        });

        if self.state.should_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

pub fn show_welcome_screen() -> Result<WelcomeResult> {
    // Load window icon
    let icon = load_window_icon();

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([600.0, 800.0]) // Compact size
        .with_resizable(true)
        .with_decorations(true)
        .with_transparent(false)
        .with_title("Welcome to Cook Sync");

    if let Some(icon_data) = icon {
        viewport = viewport.with_icon(Arc::new(icon_data));
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    // Store result in Arc<Mutex<>> so we can retrieve it after the window closes
    let result_ref = Arc::new(Mutex::new(WelcomeResult {
        login_requested: false,
        recipes_dir: None,
        auto_start: true,
        auto_update: true,
    }));
    let result_clone = result_ref.clone();

    eframe::run_native(
        "Welcome to Cook Sync",
        options,
        Box::new(move |_cc| Ok(Box::new(WelcomeApp::with_result_ref(result_ref)))),
    )
    .map_err(|e| crate::error::SyncError::Other(e.to_string()))?;

    // Extract the result from the Arc<Mutex<>>
    let result = result_clone.lock().unwrap().clone();
    Ok(result)
}
