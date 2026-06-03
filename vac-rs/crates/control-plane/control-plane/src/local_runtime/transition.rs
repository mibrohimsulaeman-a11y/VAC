use std::fmt;

/// Error returned when a runtime FSM transition is rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidTransition {
    pub kind: &'static str,
    pub from: &'static str,
    pub to: &'static str,
}

impl InvalidTransition {
    pub const fn new(kind: &'static str, from: &'static str, to: &'static str) -> Self {
        Self { kind, from, to }
    }
}

impl fmt::Display for InvalidTransition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid {} transition: {} -> {}",
            self.kind, self.from, self.to
        )
    }
}

impl std::error::Error for InvalidTransition {}
