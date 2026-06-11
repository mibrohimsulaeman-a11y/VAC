//! Autopilot is a runtime job producer, not a separate product server.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutopilotStatus {
    pub running: bool,
    pub mode: String,
}

impl Default for AutopilotStatus {
    fn default() -> Self {
        Self {
            running: false,
            mode: "monitor-only".to_string(),
        }
    }
}
