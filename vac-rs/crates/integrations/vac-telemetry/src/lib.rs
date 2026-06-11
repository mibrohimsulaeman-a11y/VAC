//! Telemetry boundary. VAC core must be useful with telemetry disabled.

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TelemetryMode {
    #[default]
    Disabled,
    LocalOnly,
    RemoteOptIn,
}
