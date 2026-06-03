//! Fail-closed guards for legacy ChatGPT/cloud-account surfaces.
//!
//! VAC local coding-agent builds keep API-key/provider auth, MCP, connectors,
//! sandbox exec, and TUI surfaces. Legacy ChatGPT-account backend features are
//! intentionally disabled unless a future feature gate reintroduces them with a
//! bounded, provider-owned implementation.

pub(crate) const LEGACY_CLOUD_ACCOUNT_DISABLED_REASON: &str =
    "legacy ChatGPT-account backend integration is disabled in the local coding-agent build";

pub(crate) fn disabled_feature_message(feature: &str) -> String {
    format!("{feature}: {LEGACY_CLOUD_ACCOUNT_DISABLED_REASON}")
}
