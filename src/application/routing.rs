use crate::domain::{CaptureState, Command, FocusState};
use crate::infrastructure::hotkey::GlobalHotKeyManager;
use evdev::Key;

pub enum RoutingResult {
    Ignored,
    Dispatch(Key, bool),
    StateChanged(CaptureState, FocusState),
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

    pub fn process(&mut self, key: Key, pressed: bool, enable_focus_toggle: bool) -> RoutingResult {
        if let Some(command) = self.hotkey_manager.process(key, pressed) {
            match command {
                Command::ToggleCapture => {
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
}
