use scry_cli::config::{
    BehaviorConfig, ColorConfig, Config, LlmConfigFile, ThemeConfig, WelcomeConfig,
};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_config_default_has_expected_values() {
    let config = Config::default();

    // Check LLM defaults (Anthropic)
    assert_eq!(config.llm.api_base, "https://api.anthropic.com/v1");
    assert_eq!(config.llm.model, "claude-sonnet-4-5");
    assert_eq!(config.llm.temperature, Some(0.7));
    assert_eq!(config.llm.max_tokens, Some(4096));
    assert!(config.llm.api_key.is_none());

    // Check behavior defaults
    assert_eq!(config.behavior.scroll_page_size, 10);
    assert_eq!(config.behavior.animation_chars_per_frame, 3);
    assert_eq!(config.behavior.animation_frame_ms, 16);
    assert_eq!(config.behavior.idle_poll_ms, 100);

    // Check welcome defaults
    assert!(config.welcome.enabled);
    assert!(config.welcome.use_tte);
    assert!(!config.welcome.fullscreen);
    assert_eq!(config.welcome.effect, "slide");
}

#[test]
fn test_llm_config_file_default() {
    let llm = LlmConfigFile::default();

    assert_eq!(llm.api_base, "https://api.anthropic.com/v1");
    assert!(llm.api_key.is_none());
    assert_eq!(llm.model, "claude-sonnet-4-5");
    assert_eq!(llm.temperature, Some(0.7));
    assert_eq!(llm.max_tokens, Some(4096));
}

#[test]
fn test_color_config_default() {
    let colors = ColorConfig::default();

    assert_eq!(colors.chat_gradient_start, [147, 51, 234]); // Purple
    assert_eq!(colors.chat_gradient_end, [59, 130, 246]); // Blue
    assert_eq!(colors.input_gradient_start, [16, 185, 129]); // Green
    assert_eq!(colors.input_gradient_end, [6, 182, 212]); // Cyan
    assert_eq!(colors.miami_pink, [255, 0, 128]);
    assert_eq!(colors.miami_purple, [138, 43, 226]);
    assert_eq!(colors.miami_cyan, [0, 255, 255]);
    assert_eq!(colors.miami_orange, [255, 140, 0]);
}

#[test]
fn test_behavior_config_default() {
    let behavior = BehaviorConfig::default();

    assert_eq!(behavior.scroll_page_size, 10);
    assert_eq!(behavior.animation_chars_per_frame, 3);
    assert_eq!(behavior.animation_frame_ms, 16);
    assert_eq!(behavior.idle_poll_ms, 100);
}

#[test]
fn test_welcome_config_default() {
    let welcome = WelcomeConfig::default();

    assert!(welcome.enabled);
    assert!(welcome.use_tte);
    assert!(!welcome.fullscreen);
    assert_eq!(welcome.effect, "slide");
    assert_eq!(welcome.beam_row_speed_min, 30);
    assert_eq!(welcome.beam_row_speed_max, 80);
    assert_eq!(welcome.beam_column_speed_min, 20);
    assert_eq!(welcome.beam_column_speed_max, 50);
    assert_eq!(welcome.beam_delay, 12);
    assert_eq!(welcome.final_wipe_speed, 3);
    assert_eq!(welcome.gradient_stops.len(), 3);
    assert_eq!(welcome.final_gradient_stops.len(), 3);
}

#[test]
fn test_theme_config_default() {
    let theme = ThemeConfig::default();

    // Menu colors
    assert_eq!(theme.menu_bg, [25, 25, 35]);
    assert_eq!(theme.menu_shadow, [10, 10, 15]);
    assert_eq!(theme.menu_selected_bg, [60, 60, 80]);
    assert_eq!(theme.menu_selected_fg, [0, 255, 255]); // Cyan
    assert_eq!(theme.menu_unselected_fg, [140, 140, 160]);
    assert_eq!(theme.menu_separator, [60, 60, 80]);
    assert_eq!(theme.menu_border, [80, 80, 100]);
    assert_eq!(theme.menu_input_bg, [50, 50, 60]);

    // Status colors
    assert_eq!(theme.status_ready, [100, 255, 100]);
    assert_eq!(theme.status_streaming, [100, 200, 255]);
    assert_eq!(theme.status_error, [255, 100, 100]);
    assert_eq!(theme.status_not_configured, [255, 100, 100]);

    // Background colors
    assert_eq!(theme.bg_primary, [20, 20, 25]);
    assert_eq!(theme.bg_secondary, [30, 30, 35]);
}

#[test]
fn test_config_save_and_load_roundtrip() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("config.toml");

    // Create a custom config
    let mut config = Config::default();
    config.llm.api_base = "https://custom.api.com/v1".to_string();
    config.llm.model = "custom-model".to_string();
    config.behavior.scroll_page_size = 20;
    config.welcome.effect = "beams".to_string();

    // Save it
    config
        .save_to_path(&config_path)
        .expect("Failed to save config");

    // Verify file exists
    assert!(config_path.exists());

    // Load it back
    let loaded = Config::load_from_path(&config_path).expect("Failed to load config");

    // Verify values match
    assert_eq!(loaded.llm.api_base, "https://custom.api.com/v1");
    assert_eq!(loaded.llm.model, "custom-model");
    assert_eq!(loaded.behavior.scroll_page_size, 20);
    assert_eq!(loaded.welcome.effect, "beams");
}

#[test]
fn test_config_load_from_path_with_valid_toml() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("config.toml");

    let toml_content = r#"
[llm]
api_base = "https://test.api.com/v1"
model = "test-model"
temperature = 0.5

[behavior]
scroll_page_size = 15

[welcome]
enabled = false
effect = "decrypt"
"#;

    fs::write(&config_path, toml_content).expect("Failed to write test config");

    let config = Config::load_from_path(&config_path).expect("Failed to load config");

    assert_eq!(config.llm.api_base, "https://test.api.com/v1");
    assert_eq!(config.llm.model, "test-model");
    assert_eq!(config.llm.temperature, Some(0.5));
    assert_eq!(config.behavior.scroll_page_size, 15);
    assert!(!config.welcome.enabled);
    assert_eq!(config.welcome.effect, "decrypt");
}

#[test]
fn test_config_load_from_path_missing_file() {
    let result = Config::load_from_path("/nonexistent/path/config.toml");
    assert!(result.is_err());
}

#[test]
fn test_color_config_to_tuple() {
    let rgb = [100, 150, 200];
    let tuple = ColorConfig::to_tuple(&rgb);
    assert_eq!(tuple, (100, 150, 200));
}

#[test]
fn test_color_config_chat_gradient() {
    let colors = ColorConfig::default();
    let (start, end) = colors.chat_gradient();

    assert_eq!(start, (147, 51, 234)); // Purple
    assert_eq!(end, (59, 130, 246)); // Blue
}

#[test]
fn test_color_config_input_gradient() {
    let colors = ColorConfig::default();
    let (start, end) = colors.input_gradient();

    assert_eq!(start, (16, 185, 129)); // Green
    assert_eq!(end, (6, 182, 212)); // Cyan
}

#[test]
fn test_color_config_miami_colors() {
    let colors = ColorConfig::default();
    let miami = colors.miami_colors();

    assert_eq!(miami.pink, (255, 0, 128));
    assert_eq!(miami.purple, (138, 43, 226));
    assert_eq!(miami.cyan, (0, 255, 255));
    assert_eq!(miami.orange, (255, 140, 0));
}

#[test]
fn test_theme_config_to_color() {
    use ratatui::style::Color;

    let rgb = [100, 150, 200];
    let color = ThemeConfig::to_color(&rgb);

    assert_eq!(color, Color::Rgb(100, 150, 200));
}

#[test]
fn test_theme_config_color_methods() {
    use ratatui::style::Color;

    let theme = ThemeConfig::default();

    assert_eq!(theme.menu_bg(), Color::Rgb(25, 25, 35));
    assert_eq!(theme.menu_shadow(), Color::Rgb(10, 10, 15));
    assert_eq!(theme.menu_selected_bg(), Color::Rgb(60, 60, 80));
    assert_eq!(theme.menu_unselected_fg(), Color::Rgb(140, 140, 160));
    assert_eq!(theme.menu_separator(), Color::Rgb(60, 60, 80));
    assert_eq!(theme.menu_border(), Color::Rgb(80, 80, 100));
    assert_eq!(theme.menu_input_bg(), Color::Rgb(50, 50, 60));

    assert_eq!(theme.status_ready(), Color::Rgb(100, 255, 100));
    assert_eq!(theme.status_streaming(), Color::Rgb(100, 200, 255));
    assert_eq!(theme.status_error(), Color::Rgb(255, 100, 100));
    assert_eq!(theme.status_not_configured(), Color::Rgb(255, 100, 100));

    assert_eq!(theme.bg_primary(), Color::Rgb(20, 20, 25));
    assert_eq!(theme.bg_secondary(), Color::Rgb(30, 30, 35));
}
