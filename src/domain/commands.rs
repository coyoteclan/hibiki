use super::state::{CaptureState, FocusState};

#[derive(Debug, Clone)]
pub enum Command {
    ToggleCapture,
    ToggleFocus,
    SetCaptureState(CaptureState),
    SetFocusState(FocusState),
}
