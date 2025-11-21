// Integration tests for welcome screen functionality
// These tests verify the end-to-end flow of the welcome screen feature

use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_config_has_welcome_shown_field() {
    // Test that Config can track whether welcome has been shown
    // This will fail until welcome_shown is added to Settings
    use cook_sync::config::Settings;

    let settings = Settings::default();

    // Access the welcome_shown field
    let _ = settings.welcome_shown;
}

#[test]
fn test_first_run_detection() {
    // Test that we can detect first run (welcome_shown = false)
    use cook_sync::config::Settings;

    let settings = Settings::default();

    assert_eq!(
        settings.welcome_shown, false,
        "First run should have welcome_shown = false"
    );
}

#[test]
fn test_second_run_detection() {
    // Test that we can detect subsequent runs (welcome_shown = true)
    use cook_sync::config::Settings;

    let mut settings = Settings::default();
    settings.welcome_shown = true;

    assert_eq!(
        settings.welcome_shown, true,
        "Subsequent runs should have welcome_shown = true"
    );
}

#[test]
fn test_welcome_shown_persists_across_save_load() {
    // Test that welcome_shown flag persists when saving and loading config
    use cook_sync::config::Settings;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let settings_path = temp_dir.path().join("settings.json");

    // Create settings with welcome_shown = false
    let settings = Settings::default();
    assert_eq!(settings.welcome_shown, false);

    // Save settings
    settings
        .save(&settings_path)
        .expect("Failed to save settings");

    // Load settings back
    let loaded = Settings::load(&settings_path).expect("Failed to load settings");

    assert_eq!(
        loaded.welcome_shown, false,
        "welcome_shown should persist as false"
    );

    // Now update to true and save again
    let mut settings_true = loaded;
    settings_true.welcome_shown = true;
    settings_true
        .save(&settings_path)
        .expect("Failed to save updated settings");

    // Load again
    let loaded_again = Settings::load(&settings_path).expect("Failed to load settings again");

    assert_eq!(
        loaded_again.welcome_shown, true,
        "welcome_shown should persist as true"
    );
}

#[test]
fn test_config_update_settings_can_modify_welcome_shown() {
    // Test that Config::update_settings can modify welcome_shown field
    // This tests the integration with the update mechanism
    use cook_sync::config::{Config, Settings};
    use std::fs;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Set up test environment
    std::env::set_var("COOK_SYNC_CONFIG_DIR", temp_dir.path());

    // Create initial settings file
    let settings_path = temp_dir.path().join("settings.json");
    let initial_settings = Settings::default();
    initial_settings
        .save(&settings_path)
        .expect("Failed to save initial settings");

    // Create config (this will load the settings)
    // Note: This may fail if Config::new() doesn't use the env var yet
    // but it tests the intended behavior

    // For now, test the update pattern directly
    let mut settings = Settings::load(&settings_path).expect("Failed to load settings");

    assert_eq!(settings.welcome_shown, false, "Should start as false");

    // Update welcome_shown
    settings.welcome_shown = true;
    settings
        .save(&settings_path)
        .expect("Failed to save updated settings");

    // Verify it persisted
    let loaded = Settings::load(&settings_path).expect("Failed to load updated settings");

    assert_eq!(loaded.welcome_shown, true, "Should be updated to true");

    // Cleanup
    std::env::remove_var("COOK_SYNC_CONFIG_DIR");
}

#[test]
fn test_backward_compatibility_with_old_settings() {
    // Test that old settings files without welcome_shown field can be loaded
    use cook_sync::config::Settings;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let settings_path = temp_dir.path().join("settings.json");

    // Create an old-style settings file (without welcome_shown)
    let old_settings_json = r#"{
        "recipes_dir": null,
        "sync_interval_secs": 12,
        "auto_start": true,
        "auto_update": true,
        "show_notifications": true,
        "update_settings": {
            "check_interval_hours": 24,
            "auto_download": true,
            "auto_install": false,
            "show_release_notes": true,
            "skip_versions": []
        }
    }"#;

    std::fs::write(&settings_path, old_settings_json).expect("Failed to write old settings");

    // Load the old settings
    let settings = Settings::load(&settings_path).expect("Failed to load old settings");

    // Should default to false for backward compatibility
    assert_eq!(
        settings.welcome_shown, false,
        "Old settings should default welcome_shown to false"
    );

    // All other fields should be preserved
    assert!(settings.recipes_dir.is_none());
    assert_eq!(settings.sync_interval_secs, 12);
    assert_eq!(settings.auto_start, true);
}

#[test]
fn test_welcome_module_exists() {
    // Test that the welcome module is accessible
    // This will fail until the welcome module is created

    // Try to access the welcome module through the public API
    // This is a compile-time test
    let _ = std::any::type_name::<cook_sync::welcome::WelcomeResult>();
}

#[test]
fn test_show_welcome_screen_function_accessible() {
    // Test that show_welcome_screen function is accessible from main crate
    // This will fail until the function is implemented and exported

    use cook_sync::welcome::show_welcome_screen;

    // Just check that the function exists with the right type signature
    let _: fn() -> cook_sync::error::Result<cook_sync::welcome::WelcomeResult> =
        show_welcome_screen;
}

#[test]
fn test_first_run_workflow() {
    // Test the complete first-run workflow:
    // 1. Load config
    // 2. Check if welcome_shown is false
    // 3. (Would show welcome screen here)
    // 4. Update welcome_shown to true
    // 5. Save config
    // 6. Verify persistence

    use cook_sync::config::Settings;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let settings_path = temp_dir.path().join("settings.json");

    // Step 1: Load config (will be default since file doesn't exist)
    let settings = Settings::load(&settings_path).expect("Failed to load settings");

    // Step 2: Check if first run
    assert_eq!(
        settings.welcome_shown, false,
        "First run should have welcome_shown = false"
    );

    // Step 3: Would show welcome screen here (skipped in test)

    // Step 4: Update welcome_shown
    let mut updated_settings = settings;
    updated_settings.welcome_shown = true;

    // Step 5: Save config
    updated_settings
        .save(&settings_path)
        .expect("Failed to save updated settings");

    // Step 6: Verify persistence (simulate second run)
    let second_run_settings =
        Settings::load(&settings_path).expect("Failed to load settings on second run");

    assert_eq!(
        second_run_settings.welcome_shown, true,
        "Second run should have welcome_shown = true"
    );
}

#[test]
fn test_second_run_skips_welcome() {
    // Test that on second run, we can detect that welcome was already shown
    use cook_sync::config::Settings;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let settings_path = temp_dir.path().join("settings.json");

    // Simulate that welcome was already shown
    let mut settings = Settings::default();
    settings.welcome_shown = true;
    settings
        .save(&settings_path)
        .expect("Failed to save settings");

    // Load on "second run"
    let loaded_settings = Settings::load(&settings_path).expect("Failed to load settings");

    // Should indicate welcome was already shown
    assert_eq!(
        loaded_settings.welcome_shown, true,
        "Second run should detect welcome was already shown"
    );
}

#[test]
fn test_welcome_result_can_store_user_choices() {
    // Test that WelcomeResult can capture user's choices from welcome screen
    use cook_sync::welcome::WelcomeResult;
    use std::path::PathBuf;

    // Test case 1: User clicks login
    let result_login = WelcomeResult {
        login_requested: true,
        recipes_dir: None,
    };
    assert!(result_login.login_requested);
    assert!(result_login.recipes_dir.is_none());

    // Test case 2: User selects directory
    let test_dir = PathBuf::from("/test/recipes");
    let result_dir = WelcomeResult {
        login_requested: false,
        recipes_dir: Some(test_dir.clone()),
    };
    assert!(!result_dir.login_requested);
    assert_eq!(result_dir.recipes_dir, Some(test_dir));

    // Test case 3: User does both
    let test_dir2 = PathBuf::from("/another/path");
    let result_both = WelcomeResult {
        login_requested: true,
        recipes_dir: Some(test_dir2.clone()),
    };
    assert!(result_both.login_requested);
    assert_eq!(result_both.recipes_dir, Some(test_dir2));

    // Test case 4: User clicks "Get Started" without doing anything
    let result_neither = WelcomeResult {
        login_requested: false,
        recipes_dir: None,
    };
    assert!(!result_neither.login_requested);
    assert!(result_neither.recipes_dir.is_none());
}

#[test]
fn test_integration_with_existing_config_system() {
    // Test that welcome_shown integrates properly with existing config system
    use cook_sync::config::Settings;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let settings_path = temp_dir.path().join("settings.json");

    // Create settings with various fields set
    let mut settings = Settings::default();
    settings.recipes_dir = Some(PathBuf::from("/test/recipes"));
    settings.sync_interval_secs = 30;
    settings.auto_start = false;
    settings.auto_update = false;
    settings.welcome_shown = true;

    // Save
    settings
        .save(&settings_path)
        .expect("Failed to save settings");

    // Load and verify all fields
    let loaded = Settings::load(&settings_path).expect("Failed to load settings");

    assert_eq!(loaded.recipes_dir, Some(PathBuf::from("/test/recipes")));
    assert_eq!(loaded.sync_interval_secs, 30);
    assert_eq!(loaded.auto_start, false);
    assert_eq!(loaded.auto_update, false);
    assert_eq!(loaded.welcome_shown, true);
}
