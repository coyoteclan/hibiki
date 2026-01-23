use crate::domain::config::DisplayMode;
use crate::domain::{CaptureState, Command, FocusState};
use crate::infrastructure::hotkey::GlobalHotKeyManager;
use evdev::Key;

pub enum RoutingResult {
    Ignored,
    Dispatch(Key, bool),
    StateChanged(CaptureState, FocusState),
    SwitchMode(DisplayMode),
}

pub struct RoutingEngine {
    hotkey_manager: GlobalHotKeyManager,
    capture_state: CaptureState,
    focus_state: FocusState,
}

impl Default for RoutingEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RoutingEngine {
    pub fn new() -> Self {
        Self {
            hotkey_manager: GlobalHotKeyManager::new(),
            capture_state: CaptureState::Active,
            focus_state: FocusState::Focused,
        }
    }

    pub fn toggle_capture(&mut self) -> CaptureState {
        self.capture_state = match self.capture_state {
            CaptureState::Active => CaptureState::Paused,
            CaptureState::Paused => CaptureState::Active,
        };
        self.capture_state
    }

    pub fn toggle_focus(&mut self) -> FocusState {
        self.focus_state = match self.focus_state {
            FocusState::Focused => FocusState::Unfocused,
            FocusState::Unfocused => FocusState::Focused,
        };
        self.focus_state
    }

    pub fn process(
        &mut self,
        key: Key,
        pressed: bool,
        enable_focus_toggle: bool,
        current_mode: DisplayMode,
    ) -> RoutingResult {
        if let Some(command) = self.hotkey_manager.process(key, pressed) {
            tracing::info!("Hotkey detected: {:?} in mode {:?}", command, current_mode);
            match command {
                Command::ActivateKeystrokeMode => {
                    if current_mode != DisplayMode::Keystroke {
                        return RoutingResult::SwitchMode(DisplayMode::Keystroke);
                    }
                    return RoutingResult::Ignored;
                }
                Command::ActivateBubbleMode => {
                    if current_mode != DisplayMode::Bubble {
                        return RoutingResult::SwitchMode(DisplayMode::Bubble);
                    }
                    return RoutingResult::Ignored;
                }
                Command::TogglePause => {
                    self.capture_state = match self.capture_state {
                        CaptureState::Active => CaptureState::Paused,
                        CaptureState::Paused => CaptureState::Active,
                    };
                    return RoutingResult::StateChanged(self.capture_state, self.focus_state);
                }
                Command::ToggleFocus => {
                    if enable_focus_toggle {
                        self.focus_state = match self.focus_state {
                            FocusState::Focused => FocusState::Unfocused,
                            FocusState::Unfocused => FocusState::Focused,
                        };
                        return RoutingResult::StateChanged(self.capture_state, self.focus_state);
                    }
                    return RoutingResult::Dispatch(key, pressed);
                }
                _ => {}
            }
        }

        if self.capture_state == CaptureState::Paused {
            return RoutingResult::Ignored;
        }

        if self.focus_state == FocusState::Unfocused {
            return RoutingResult::Ignored;
        }

        RoutingResult::Dispatch(key, pressed)
    }

    pub fn get_states(&self) -> (CaptureState, FocusState) {
        (self.capture_state, self.focus_state)
    }

    pub fn update_config(&mut self, capture_hotkey: &str, focus_hotkey: &str) {
        self.hotkey_manager.set_capture_hotkey(capture_hotkey);
        self.hotkey_manager.set_focus_hotkey(focus_hotkey);
    }

    pub fn reset_hotkey_state(&mut self) {
        self.hotkey_manager.reset();
    }

    pub fn reset_mode_state(&mut self) {
        self.capture_state = CaptureState::Active;
        self.focus_state = FocusState::Focused;
    }
}
