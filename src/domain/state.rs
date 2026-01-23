use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemState {
    Focused,
    Unfocused,
    Paused,
    Resumed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureState {
    Active,
    Paused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusState {
    Focused,
    Unfocused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputTarget {
    ChatBubble,
    SystemTransparent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceStatus {
    Active,
    Paused,
}

#[derive(Error, Debug)]
pub enum StateTransitionError {
    #[error("Invalid transition from {0:?} to {1:?}")]
    InvalidTransition(String, String),
    #[error("State immutable: {0}")]
    Immutable(String),
}
