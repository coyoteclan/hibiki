use gtk4_layer_shell::Edge;
use serde::{Deserialize, Serialize};

const DEFAULT_DISPLAY_TIMEOUT_MS: u64 = 2000;
const DEFAULT_BUBBLE_TIMEOUT_MS: u64 = 10000;
const DEFAULT_MAX_KEYS: usize = 5;
const DEFAULT_MARGIN: i32 = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DisplayMode {
    #[default]
    Keystroke,
    Bubble,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Position {
    TopLeft,
    TopCenter,
    TopRight,
    MiddleLeft,
    Center,
    MiddleRight,
    BottomLeft,
    #[default]
    BottomCenter,
    BottomRight,
}

impl Position {
    #[must_use]
    pub fn layer_shell_edges(self) -> Vec<(Edge, bool)> {
        match self {
            Position::TopLeft => vec![
                (Edge::Top, true),
                (Edge::Left, true),
                (Edge::Bottom, false),
                (Edge::Right, false),
            ],
            Position::TopCenter => vec![
                (Edge::Top, true),
                (Edge::Left, false),
                (Edge::Bottom, false),
                (Edge::Right, false),
            ],
            Position::TopRight => vec![
                (Edge::Top, true),
                (Edge::Left, false),
                (Edge::Bottom, false),
                (Edge::Right, true),
            ],
            Position::MiddleLeft => vec![
                (Edge::Top, false),
                (Edge::Left, true),
                (Edge::Bottom, false),
                (Edge::Right, false),
            ],
            Position::Center => vec![
                (Edge::Top, false),
                (Edge::Left, false),
                (Edge::Bottom, false),
                (Edge::Right, false),
            ],
            Position::MiddleRight => vec![
                (Edge::Top, false),
                (Edge::Left, false),
                (Edge::Bottom, false),
                (Edge::Right, true),
            ],
            Position::BottomLeft => vec![
                (Edge::Top, false),
                (Edge::Left, true),
                (Edge::Bottom, true),
                (Edge::Right, false),
            ],
            Position::BottomCenter => vec![
                (Edge::Top, false),
                (Edge::Left, false),
                (Edge::Bottom, true),
                (Edge::Right, false),
            ],
            Position::BottomRight => vec![
                (Edge::Top, false),
                (Edge::Left, false),
                (Edge::Bottom, true),
                (Edge::Right, true),
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
#[allow(clippy::struct_excessive_bools)]
pub struct KeystrokeConfig {
    pub display_mode: DisplayMode,

    pub position: Position,

    pub display_timeout_ms: u64,

    pub bubble_timeout_ms: u64,

    pub max_keys: usize,

    pub margin: i32,

    pub show_modifiers: bool,

    pub all_keyboards: bool,

    pub font_scale: f64,

    pub opacity: f64,

    pub keystroke_font_size: f64,

    pub bubble_font_size: f64,

    pub bubble_font_family: String,

    pub bubble_color: String,

    pub bubble_sound_enabled: bool,

    pub bubble_position: Position,

    pub bubble_draggable: bool,

    pub bubble_hotkey: String,

    pub keystroke_theme: String,

    pub keystroke_draggable: bool,

    pub keystroke_hotkey: String,

    pub pause_hotkey: String,

    pub toggle_focus_hotkey: String,

    pub auto_detect_layout: bool,

    pub keyboard_layout: Option<String>,
}

impl Default for KeystrokeConfig {
    fn default() -> Self {
        Self {
            display_mode: DisplayMode::Keystroke,
            position: Position::BottomCenter,
            display_timeout_ms: DEFAULT_DISPLAY_TIMEOUT_MS,
            bubble_timeout_ms: DEFAULT_BUBBLE_TIMEOUT_MS,
            max_keys: DEFAULT_MAX_KEYS,
            margin: DEFAULT_MARGIN,
            show_modifiers: true,
            all_keyboards: true,
            font_scale: 1.0,
            opacity: 0.9,
            keystroke_font_size: 1.2,
            bubble_font_size: 1.0,
            bubble_font_family: "Sans".to_string(),
            bubble_color: "#3584e4".to_string(),
            bubble_sound_enabled: false,
            bubble_position: Position::TopRight,
            bubble_draggable: false,
            bubble_hotkey: "<Control>b".to_string(),
            keystroke_theme: "system".to_string(),
            keystroke_draggable: false,
            keystroke_hotkey: "<Control>p".to_string(),
            pause_hotkey: "<Control>p".to_string(),
            toggle_focus_hotkey: "<Control>b".to_string(),
            auto_detect_layout: true,
            keyboard_layout: None,
        }
    }
}
