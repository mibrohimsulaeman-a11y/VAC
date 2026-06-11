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
#[derive(Clone, Debug)]
pub struct VACApiConfig {
    /// API key for authentication
    pub api_key: String,
    /// API endpoint URL (default: https://api.vastar.ai/vac)
    pub api_endpoint: String,
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
