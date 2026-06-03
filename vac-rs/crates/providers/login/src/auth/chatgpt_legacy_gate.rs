//! Legacy ChatGPT account auth is default-off for VAC local coding-agent mode.

pub const PROVIDER_CHATGPT_FEATURE: &str = "provider-chatgpt";
pub const ENABLE_LEGACY_CHATGPT_ACCOUNT_BACKEND: &str = "VAC_ENABLE_LEGACY_CHATGPT_ACCOUNT_BACKEND";

pub fn legacy_chatgpt_account_enabled() -> bool {
    std::env::var(ENABLE_LEGACY_CHATGPT_ACCOUNT_BACKEND).as_deref() == Ok("1")
}
