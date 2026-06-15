//! VAC API Client
//!
//! Provides access to VAC's non-inference APIs:
//! - Sessions and checkpoints (via new `/v1/sessions` endpoints)
//! - MCP tool calls (memory, docs search, Slack)
//! - Billing and account
//! - Rulebooks

mod client;
mod knowledge;
mod models;
pub mod storage;

pub use client::VACApiClient;
pub use knowledge::KnowledgeApiError;
pub use models::*;

/// Configuration for VACApiClient
#[derive(Clone)]
pub struct VACApiConfig {
    /// API key for authentication
    pub api_key: String,
    /// API endpoint URL (default: https://api.vastar.ai/vac)
    pub api_endpoint: String,
}

impl std::fmt::Debug for VACApiConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VACApiConfig")
            .field("api_key", &"[REDACTED]")
            .field("api_endpoint", &self.api_endpoint)
            .finish()
    }
}

impl VACApiConfig {
    /// Create new config with API key
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            api_endpoint: "https://api.vastar.ai/vac".to_string(),
        }
    }

    /// Set API endpoint
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.api_endpoint = endpoint.into();
        self
    }
}

impl Default for VACApiConfig {
    fn default() -> Self {
        Self::new(std::env::var("VAC_API_KEY").unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vac_api_config_debug_redacts_api_key() {
        let raw = "vac-api-debug-raw";
        let config = VACApiConfig::new(raw).with_endpoint("https://example.invalid/vac");
        let rendered = format!("{config:?}");

        assert!(rendered.contains("[REDACTED]"));
        assert!(rendered.contains("https://example.invalid/vac"));
        assert!(!rendered.contains(raw));
    }
}
