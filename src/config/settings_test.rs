use super::Settings;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_settings_default_has_welcome_shown_false() {
    // Test that new Settings have welcome_shown set to false by default
    let settings = Settings::default();

    assert!(
        !settings.welcome_shown,
        "Default settings should have welcome_shown set to false"
    );
}

#[test]
fn test_settings_serialization_includes_welcome_shown() {
    // Test that welcome_shown field is serialized correctly
    let settings = Settings {
        welcome_shown: true,
        ..Default::default()
    };

    let json = serde_json::to_string(&settings).expect("Failed to serialize settings");

    assert!(
        json.contains("\"welcome_shown\":true"),
        "Serialized JSON should contain welcome_shown field: {}",
        json
    );
}

#[test]
fn test_settings_deserialization_with_welcome_shown_field() {
    // Test that settings can be deserialized with welcome_shown field present
    let json = r#"{
        "recipes_dir": null,
        "sync_interval_secs": 12,
        "auto_start": true,
        "auto_update": true,
        "show_notifications": true,
        "welcome_shown": true,
        "update_settings": {
            "check_interval_hours": 24,
            "auto_download": true,
            "auto_install": false,
            "show_release_notes": true,
            "skip_versions": []
        }
    }"#;

    let settings: Settings = serde_json::from_str(json).expect("Failed to deserialize settings");

    assert!(
        settings.welcome_shown,
        "Deserialized settings should preserve welcome_shown value"
    );
}

#[test]
fn test_settings_deserialization_without_welcome_shown_field_backward_compat() {
    // Test backward compatibility: settings without welcome_shown field should default to false
    let json = r#"{
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

    let settings: Settings =
        serde_json::from_str(json).expect("Failed to deserialize old settings format");

    assert!(
        !settings.welcome_shown,
        "Settings without welcome_shown field should default to false for backward compatibility"
    );
}

#[test]
fn test_settings_save_and_load_preserves_welcome_shown() {
    // Test that save/load cycle preserves welcome_shown field
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let settings_path = temp_dir.path().join("settings.json");

    // Create settings with welcome_shown = true
    let settings = Settings {
        welcome_shown: true,
        recipes_dir: Some(PathBuf::from("/test/recipes")),
        ..Default::default()
    };

    // Save settings
    settings
        .save(&settings_path)
        .expect("Failed to save settings");

    // Load settings back
    let loaded_settings = Settings::load(&settings_path).expect("Failed to load settings");

    assert!(
        loaded_settings.welcome_shown,
        "Loaded settings should preserve welcome_shown = true"
    );
    assert_eq!(
        loaded_settings.recipes_dir,
        Some(PathBuf::from("/test/recipes")),
        "Other settings should also be preserved"
    );
}

#[test]
fn test_settings_load_nonexistent_file_defaults_to_false() {
    // Test that loading from non-existent file creates default settings with welcome_shown = false
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let nonexistent_path = temp_dir.path().join("nonexistent.json");

    let settings = Settings::load(&nonexistent_path).expect("Failed to load settings");

    assert!(
        !settings.welcome_shown,
        "Settings loaded from non-existent file should default welcome_shown to false"
    );
}

#[test]
fn test_settings_welcome_shown_can_be_set_to_true() {
    // Test that welcome_shown can be updated from false to true
    let mut settings = Settings::default();

    assert!(!settings.welcome_shown, "Should start as false");

    settings.welcome_shown = true;

    assert!(settings.welcome_shown, "Should be updatable to true");
}

#[test]
fn test_settings_round_trip_with_welcome_shown_false() {
    // Test that welcome_shown = false survives serialization round-trip
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let settings_path = temp_dir.path().join("settings.json");

    let settings = Settings::default(); // welcome_shown defaults to false

    settings
        .save(&settings_path)
        .expect("Failed to save settings");
    let loaded_settings = Settings::load(&settings_path).expect("Failed to load settings");

    assert!(
        !loaded_settings.welcome_shown,
        "welcome_shown = false should survive round-trip"
    );
}

#[test]
fn test_settings_json_structure_with_welcome_shown() {
    // Test that the JSON structure is correct with welcome_shown field
    let settings = Settings {
        welcome_shown: true,
        recipes_dir: Some(PathBuf::from("/test/path")),
        ..Default::default()
    };

    let json = serde_json::to_value(&settings).expect("Failed to serialize to JSON value");

    assert!(
        json.get("welcome_shown").is_some(),
        "JSON should contain welcome_shown field"
    );
    assert_eq!(
        json.get("welcome_shown").and_then(|v| v.as_bool()),
        Some(true),
        "welcome_shown should be a boolean with value true"
    );
}

#[test]
fn test_settings_multiple_updates_to_welcome_shown() {
    // Test that welcome_shown can be toggled multiple times
    let mut settings = Settings::default();

    assert!(!settings.welcome_shown);

    settings.welcome_shown = true;
    assert!(settings.welcome_shown);

    settings.welcome_shown = false;
    assert!(!settings.welcome_shown);

    settings.welcome_shown = true;
    assert!(settings.welcome_shown);
}
