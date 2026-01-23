use crate::input::keymap::KeyDisplay;

#[derive(Debug, Clone)]
pub enum CommandEvent {
    TextInput(KeyDisplay),
    ControlHotkey(ControlCommand),
}

#[derive(Debug, Clone)]
pub enum ControlCommand {
    ToggleGlobalCapture,
    ToggleInputFocus,
}
