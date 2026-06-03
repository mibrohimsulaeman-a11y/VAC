//! Provider-specific HTTP compatibility seams.
//!
//! Generic HTTP/SSE/websocket transport stays in `vac-provider-http`. First-party or
//! cloud-account-specific behavior must live behind explicit features and must not be
//! reachable from the default local coding-agent path.

#[cfg(feature = "provider-chatgpt")]
pub mod chatgpt;

#[cfg(feature = "provider-realtime")]
pub mod realtime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderFeatureStatus {
    pub provider_chatgpt_default_enabled: bool,
    pub provider_realtime_default_enabled: bool,
    pub telemetry_default_enabled: bool,
    pub feedback_default_enabled: bool,
}

pub const DEFAULT_PROVIDER_FEATURE_STATUS: ProviderFeatureStatus = ProviderFeatureStatus {
    provider_chatgpt_default_enabled: false,
    provider_realtime_default_enabled: false,
    telemetry_default_enabled: false,
    feedback_default_enabled: false,
};
