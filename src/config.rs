use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// RGB color represented as a 3-element array.
pub type Rgb = [u8; 3];

/// LLM configuration for API access.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct LlmConfigFile {
    /// API base URL
    pub api_base: String,
    /// API key (can also be set via ANTHROPIC_API_KEY env var)
    pub api_key: Option<String>,
    /// Model name
    pub model: String,
    /// Temperature for generation
    pub temperature: Option<f32>,
    /// Max tokens for generation
    pub max_tokens: Option<u32>,
}

impl Default for LlmConfigFile {
    fn default() -> Self {
        Self {
            api_base: "https://api.anthropic.com/v1".to_string(),
            api_key: None,
            model: "claude-sonnet-4-5".to_string(),
            temperature: Some(0.7),
            max_tokens: Some(4096),
        }
    }
}

/// Color configuration for the UI.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct ColorConfig {
    /// Chat area gradient start color (Purple by default)
    pub chat_gradient_start: Rgb,
    /// Chat area gradient end color (Blue by default)
    pub chat_gradient_end: Rgb,
    /// Input area gradient start color (Green by default)
    pub input_gradient_start: Rgb,
    /// Input area gradient end color (Cyan by default)
    pub input_gradient_end: Rgb,
    /// Miami pink for banner
    pub miami_pink: Rgb,
    /// Miami purple for banner
    pub miami_purple: Rgb,
    /// Miami cyan for banner
    pub miami_cyan: Rgb,
    /// Miami orange for banner
    pub miami_orange: Rgb,
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self {
            chat_gradient_start: [147, 51, 234],  // Purple
            chat_gradient_end: [59, 130, 246],    // Blue
            input_gradient_start: [16, 185, 129], // Green
            input_gradient_end: [6, 182, 212],    // Cyan
            miami_pink: [255, 0, 128],            // Hot pink
            miami_purple: [138, 43, 226],         // Blue violet
            miami_cyan: [0, 255, 255],            // Cyan
            miami_orange: [255, 140, 0],          // Dark orange
        }
    }
}

/// Behavior configuration for the UI.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct BehaviorConfig {
    /// Number of messages to scroll with Page Up/Down
    pub scroll_page_size: usize,
    /// Characters revealed per animation frame
    pub animation_chars_per_frame: usize,
    /// Animation frame duration in milliseconds
    pub animation_frame_ms: u64,
    /// Idle polling interval in milliseconds
    pub idle_poll_ms: u64,
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        Self {
            scroll_page_size: 10,
            animation_chars_per_frame: 3,
            animation_frame_ms: 16,  // ~60 FPS
            idle_poll_ms: 100,
        }
    }
}

/// TTE (Terminal Text Effects) welcome screen configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct WelcomeConfig {
    /// Whether to show the welcome screen at all
    pub enabled: bool,
    /// Whether to use TTE effects (falls back to simple if TTE not installed)
    pub use_tte: bool,
    /// Use fullscreen canvas (fills entire terminal window)
    pub fullscreen: bool,
    /// TTE effect to use: "beams", "decrypt", "rain", "slide", "waves", etc.
    pub effect: String,
    /// Row beam speed range (min-max), lower = slower
    pub beam_row_speed_min: u32,
    pub beam_row_speed_max: u32,
    /// Column beam speed range (min-max), lower = slower
    pub beam_column_speed_min: u32,
    pub beam_column_speed_max: u32,
    /// Delay between beam groups
    pub beam_delay: u32,
    /// Final wipe animation speed (lower = slower)
    pub final_wipe_speed: u32,
    /// Gradient colors for the beam effect (hex without #)
    pub gradient_stops: Vec<String>,
    /// Final gradient colors (hex without #)
    pub final_gradient_stops: Vec<String>,
}

impl Default for WelcomeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            use_tte: true,
            fullscreen: false,
            effect: "slide".to_string(),
            beam_row_speed_min: 30,
            beam_row_speed_max: 80,
            beam_column_speed_min: 20,
            beam_column_speed_max: 50,
            beam_delay: 12,
            final_wipe_speed: 3,
            gradient_stops: vec![
                "ff0080".to_string(), // Hot pink
                "00ffff".to_string(), // Cyan
                "8a2be2".to_string(), // Purple
            ],
            final_gradient_stops: vec![
                "8a2be2".to_string(), // Purple
                "00ffff".to_string(), // Cyan
                "ff0080".to_string(), // Hot pink
            ],
        }
    }
}

/// Theme configuration for UI elements.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct ThemeConfig {
    // Menu colors
    /// Menu background color
    pub menu_bg: Rgb,
    /// Menu shadow color
    pub menu_shadow: Rgb,
    /// Selected menu item background
    pub menu_selected_bg: Rgb,
    /// Selected menu item foreground
    pub menu_selected_fg: Rgb,
    /// Unselected menu item foreground
    pub menu_unselected_fg: Rgb,
    /// Menu separator line color
    pub menu_separator: Rgb,
    /// Menu border color
    pub menu_border: Rgb,
    /// Menu input field background
    pub menu_input_bg: Rgb,

    // Status indicator colors
    /// Status: Ready
    pub status_ready: Rgb,
    /// Status: Streaming
    pub status_streaming: Rgb,
    /// Status: Error
    pub status_error: Rgb,
    /// Status: Not configured
    pub status_not_configured: Rgb,

    // General UI colors
    /// Main background color
    pub bg_primary: Rgb,
    /// Secondary background color
    pub bg_secondary: Rgb,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            // Menu colors
            menu_bg: [25, 25, 35],
            menu_shadow: [10, 10, 15],
            menu_selected_bg: [60, 60, 80],
            menu_selected_fg: [0, 255, 255],  // Cyan
            menu_unselected_fg: [140, 140, 160],
            menu_separator: [60, 60, 80],
            menu_border: [80, 80, 100],
            menu_input_bg: [50, 50, 60],

            // Status colors
            status_ready: [100, 255, 100],
            status_streaming: [100, 200, 255],
            status_error: [255, 100, 100],
            status_not_configured: [255, 100, 100],

            // General UI colors
            bg_primary: [20, 20, 25],
            bg_secondary: [30, 30, 35],
        }
    }
}

/// Main application configuration.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct Config {
    pub colors: ColorConfig,
    pub behavior: BehaviorConfig,
    pub welcome: WelcomeConfig,
    pub llm: LlmConfigFile,
    pub theme: ThemeConfig,
}

impl Config {
    /// Returns the default config file path: ~/.config/scry-cli/config.toml
    pub fn default_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("scry-cli").join("config.toml"))
    }

    /// Load configuration from the default path, falling back to defaults.
    pub fn load() -> Self {
        Self::default_path()
            .and_then(|path| Self::load_from_path(&path).ok())
            .unwrap_or_default()
    }

    /// Load configuration from a specific path.
    pub fn load_from_path(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }

    /// Save configuration to the default path.
    pub fn save(&self) -> anyhow::Result<()> {
        if let Some(path) = Self::default_path() {
            self.save_to_path(&path)
        } else {
            Err(anyhow::anyhow!("Could not determine config directory"))
        }
    }

    /// Save configuration to a specific path.
    pub fn save_to_path(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let path = path.as_ref();
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let contents = toml::to_string_pretty(self)?;
        std::fs::write(path, contents)?;
        Ok(())
    }
}

impl ColorConfig {
    /// Convert an RGB array to a tuple for compatibility with existing code.
    pub fn to_tuple(rgb: &Rgb) -> (u8, u8, u8) {
        (rgb[0], rgb[1], rgb[2])
    }

    /// Get chat gradient colors as tuples.
    pub fn chat_gradient(&self) -> ((u8, u8, u8), (u8, u8, u8)) {
        (
            Self::to_tuple(&self.chat_gradient_start),
            Self::to_tuple(&self.chat_gradient_end),
        )
    }

    /// Get input gradient colors as tuples.
    pub fn input_gradient(&self) -> ((u8, u8, u8), (u8, u8, u8)) {
        (
            Self::to_tuple(&self.input_gradient_start),
            Self::to_tuple(&self.input_gradient_end),
        )
    }

    /// Get Miami colors as tuples.
    pub fn miami_colors(&self) -> MiamiColors {
        MiamiColors {
            pink: Self::to_tuple(&self.miami_pink),
            purple: Self::to_tuple(&self.miami_purple),
            cyan: Self::to_tuple(&self.miami_cyan),
            orange: Self::to_tuple(&self.miami_orange),
        }
    }
}

/// Miami gradient colors as tuples for easy access.
#[derive(Debug, Clone, Copy)]
pub struct MiamiColors {
    pub pink: (u8, u8, u8),
    pub purple: (u8, u8, u8),
    pub cyan: (u8, u8, u8),
    pub orange: (u8, u8, u8),
}

impl ThemeConfig {
    /// Convert an RGB array to a ratatui Color.
    pub fn to_color(rgb: &Rgb) -> ratatui::style::Color {
        ratatui::style::Color::Rgb(rgb[0], rgb[1], rgb[2])
    }

    /// Get menu background color.
    pub fn menu_bg(&self) -> ratatui::style::Color {
        Self::to_color(&self.menu_bg)
    }

    /// Get menu shadow color.
    pub fn menu_shadow(&self) -> ratatui::style::Color {
        Self::to_color(&self.menu_shadow)
    }

    /// Get selected menu item background color.
    pub fn menu_selected_bg(&self) -> ratatui::style::Color {
        Self::to_color(&self.menu_selected_bg)
    }

    /// Get selected menu item foreground color.
    #[allow(dead_code)]
    pub fn menu_selected_fg(&self) -> ratatui::style::Color {
        Self::to_color(&self.menu_selected_fg)
    }

    /// Get unselected menu item foreground color.
    pub fn menu_unselected_fg(&self) -> ratatui::style::Color {
        Self::to_color(&self.menu_unselected_fg)
    }

    /// Get menu separator color.
    pub fn menu_separator(&self) -> ratatui::style::Color {
        Self::to_color(&self.menu_separator)
    }

    /// Get menu border color.
    pub fn menu_border(&self) -> ratatui::style::Color {
        Self::to_color(&self.menu_border)
    }

    /// Get menu input background color.
    pub fn menu_input_bg(&self) -> ratatui::style::Color {
        Self::to_color(&self.menu_input_bg)
    }

    /// Get status ready color.
    pub fn status_ready(&self) -> ratatui::style::Color {
        Self::to_color(&self.status_ready)
    }

    /// Get status streaming color.
    pub fn status_streaming(&self) -> ratatui::style::Color {
        Self::to_color(&self.status_streaming)
    }

    /// Get status error color.
    pub fn status_error(&self) -> ratatui::style::Color {
        Self::to_color(&self.status_error)
    }

    /// Get status not configured color.
    pub fn status_not_configured(&self) -> ratatui::style::Color {
        Self::to_color(&self.status_not_configured)
    }

    /// Get primary background color.
    pub fn bg_primary(&self) -> ratatui::style::Color {
        Self::to_color(&self.bg_primary)
    }

    /// Get secondary background color.
    pub fn bg_secondary(&self) -> ratatui::style::Color {
        Self::to_color(&self.bg_secondary)
    }
}
