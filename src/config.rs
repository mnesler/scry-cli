use serde::Deserialize;
use std::path::PathBuf;

/// RGB color represented as a 3-element array.
pub type Rgb = [u8; 3];

/// Color configuration for the UI.
#[derive(Debug, Clone, Deserialize)]
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
#[derive(Debug, Clone, Deserialize)]
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
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct WelcomeConfig {
    /// Whether to show the welcome screen at all
    pub enabled: bool,
    /// Whether to use TTE effects (falls back to simple if TTE not installed)
    pub use_tte: bool,
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
            effect: "beams".to_string(),
            beam_row_speed_min: 10,
            beam_row_speed_max: 25,
            beam_column_speed_min: 6,
            beam_column_speed_max: 10,
            beam_delay: 10,
            final_wipe_speed: 1,
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

/// Main application configuration.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub colors: ColorConfig,
    pub behavior: BehaviorConfig,
    pub welcome: WelcomeConfig,
}

impl Config {
    /// Returns the default config file path: ~/.config/chat-cli/config.toml
    pub fn default_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("chat-cli").join("config.toml"))
    }

    /// Load configuration from the default path, falling back to defaults.
    pub fn load() -> Self {
        Self::default_path()
            .and_then(|path| Self::load_from_path(&path).ok())
            .unwrap_or_default()
    }

    /// Load configuration from a specific path.
    pub fn load_from_path(path: &PathBuf) -> anyhow::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
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
