//! Legacy ChatGPT provider compatibility.
//!
//! This module is compiled only with `provider-chatgpt`. It intentionally exposes no
//! hardcoded endpoint; callers must provide provider config/JWKS/endpoint values.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatgptProviderConfig {
    pub base_url: String,
    pub jwks_url: Option<String>,
}

impl ChatgptProviderConfig {
    pub fn new(base_url: impl Into<String>, jwks_url: Option<String>) -> Self {
        Self {
            base_url: base_url.into(),
            jwks_url,
        }
    }
}
