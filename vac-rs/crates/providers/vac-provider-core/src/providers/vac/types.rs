//! VAC-specific types

use serde::{Deserialize, Serialize};

/// Configuration for VAC inference provider
///
/// Note: This is distinct from `vac_remote_service::VacCloudConfig` which is used
/// for the VAC API client (sessions, billing, etc.).
#[derive(Debug, Clone)]
pub struct VacProviderConfig {
    /// API key
    pub api_key: String,
    /// Base URL (default: https://api.vastar.ai/vac)
    pub base_url: String,
    /// User-Agent header (e.g., "VAC/1.0.0")
    pub user_agent: Option<String>,
}

impl VacProviderConfig {
    /// Create new config with API key
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: "https://api.vastar.ai/vac".to_string(),
            user_agent: None,
        }
    }

    /// Set base URL
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Set User-Agent header
    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }
}

impl Default for VacProviderConfig {
    fn default() -> Self {
        Self {
            api_key: std::env::var("VAC_API_KEY").unwrap_or_else(|_| String::new()),
            base_url: "https://api.vastar.ai/vac".to_string(),
            user_agent: None,
        }
    }
}

/// VAC chat completion response
#[derive(Debug, Deserialize)]
pub struct VACResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<VACChoice>,
    pub usage: VACUsage,
}

/// VAC choice
#[derive(Debug, Deserialize)]
pub struct VACChoice {
    pub message: VACMessage,
    pub finish_reason: Option<String>,
}

/// VAC message
#[derive(Debug, Serialize, Deserialize)]
pub struct VACMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<VACToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// VAC tool call
#[derive(Debug, Serialize, Deserialize)]
pub struct VACToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub function: VACFunctionCall,
}

/// VAC function call
#[derive(Debug, Serialize, Deserialize)]
pub struct VACFunctionCall {
    pub name: String,
    pub arguments: String,
}

/// VAC usage statistics
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct VACUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens_details: Option<VACPromptTokensDetails>,
}

/// VAC prompt token details (includes Anthropic cache fields)
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct VACPromptTokensDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_write_input_tokens: Option<u32>,
}

/// Response from VAC `/v1/models` endpoint
#[derive(Debug, Deserialize)]
pub struct VACModelsResponse {
    pub models: Vec<crate::types::Model>,
}
