use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use strum_macros::Display;
use thiserror::Error;
use ts_rs::TS;

pub const LEGACY_CHATGPT_ACCOUNT_ENV: &str = "VAC_ENABLE_LEGACY_CHATGPT_ACCOUNT_BACKEND";

fn legacy_chatgpt_account_enabled_from_env() -> bool {
    std::env::var(LEGACY_CHATGPT_ACCOUNT_ENV)
        .map(|value| matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PlanType {
    Known(KnownPlan),
    Unknown(String),
}

impl PlanType {
    pub fn from_raw_value(raw: &str) -> Self {
        match raw.to_ascii_lowercase().as_str() {
            "free" => Self::Known(KnownPlan::Free),
            "go" => Self::Known(KnownPlan::Go),
            "plus" => Self::Known(KnownPlan::Plus),
            "pro" => Self::Known(KnownPlan::Pro),
            "prolite" => Self::Known(KnownPlan::ProLite),
            "team" => Self::Known(KnownPlan::Team),
            "self_serve_business_usage_based" => {
                Self::Known(KnownPlan::SelfServeBusinessUsageBased)
            }
            "business" => Self::Known(KnownPlan::Business),
            "enterprise_cbp_usage_based" => Self::Known(KnownPlan::EnterpriseCbpUsageBased),
            "enterprise" | "hc" => Self::Known(KnownPlan::Enterprise),
            "education" | "edu" => Self::Known(KnownPlan::Edu),
            _ => Self::Unknown(raw.to_string()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KnownPlan {
    Free,
    Go,
    Plus,
    Pro,
    ProLite,
    Team,
    #[serde(rename = "self_serve_business_usage_based")]
    SelfServeBusinessUsageBased,
    Business,
    #[serde(rename = "enterprise_cbp_usage_based")]
    EnterpriseCbpUsageBased,
    #[serde(alias = "hc")]
    Enterprise,
    #[serde(alias = "education")]
    Edu,
}

impl KnownPlan {
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Free => "Free",
            Self::Go => "Go",
            Self::Plus => "Plus",
            Self::Pro => "Pro",
            Self::ProLite => "Pro Lite",
            Self::Team => "Team",
            Self::SelfServeBusinessUsageBased => "Self Serve Business Usage Based",
            Self::Business => "Business",
            Self::EnterpriseCbpUsageBased => "Enterprise CBP Usage Based",
            Self::Enterprise => "Enterprise",
            Self::Edu => "Edu",
        }
    }

    pub fn raw_value(self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::Go => "go",
            Self::Plus => "plus",
            Self::Pro => "pro",
            Self::ProLite => "prolite",
            Self::Team => "team",
            Self::SelfServeBusinessUsageBased => "self_serve_business_usage_based",
            Self::Business => "business",
            Self::EnterpriseCbpUsageBased => "enterprise_cbp_usage_based",
            Self::Enterprise => "enterprise",
            Self::Edu => "edu",
        }
    }

    pub fn is_workspace_account(self) -> bool {
        matches!(
            self,
            Self::Team
                | Self::SelfServeBusinessUsageBased
                | Self::Business
                | Self::EnterpriseCbpUsageBased
                | Self::Enterprise
                | Self::Edu
        )
    }
}

/// Authentication mode for Vastar-backed providers.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Display, JsonSchema, TS)]
#[serde(rename_all = "lowercase")]
pub enum AuthMode {
    /// Vastar API key provided by the caller and stored by VAC.
    ApiKey,
    /// Bearer token supplied by a provider-neutral credential source.
    Bearer,
    /// Local-only credential resolved by the runtime owner.
    Local,
    /// Provider credential whose concrete account mode is owned by provider config.
    #[serde(rename = "providerCredential")]
    #[ts(rename = "providerCredential")]
    #[strum(serialize = "providerCredential")]
    ProviderCredential,
    /// Legacy ChatGPT OAuth managed by VAC. Kept parseable only behind explicit opt-in gates.
    Chatgpt,
    /// [UNSTABLE] FOR VASTAR INTERNAL USE ONLY - DO NOT USE.
    ///
    /// ChatGPT auth tokens are supplied by an external host app and are only
    /// stored in memory. Token refresh must be handled by the external host app.
    #[serde(rename = "chatgptAuthTokens")]
    #[ts(rename = "chatgptAuthTokens")]
    #[strum(serialize = "chatgptAuthTokens")]
    ChatgptAuthTokens,
    /// Programmatic VAC auth backed by a registered Agent Identity.
    #[serde(rename = "agentIdentity")]
    #[ts(rename = "agentIdentity")]
    #[strum(serialize = "agentIdentity")]
    AgentIdentity,
}

impl AuthMode {
    /// Returns true for legacy ChatGPT account-backed modes that are outside
    /// the default local coding-agent path. These modes remain parseable for
    /// compatibility, but production callers should require an explicit
    /// operator opt-in before using them.
    pub const fn is_legacy_chatgpt_account(self) -> bool {
        matches!(self, Self::Chatgpt | Self::ChatgptAuthTokens)
    }

    /// Fail-closed local-agent gate for legacy ChatGPT account modes.
    pub fn allowed_for_local_coding_agent(self) -> bool {
        !self.is_legacy_chatgpt_account() || legacy_chatgpt_account_enabled_from_env()
    }

    pub fn legacy_chatgpt_account_enabled() -> bool {
        legacy_chatgpt_account_enabled_from_env()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{message}")]
pub struct RefreshTokenFailedError {
    pub reason: RefreshTokenFailedReason,
    pub message: String,
}

impl RefreshTokenFailedError {
    pub fn new(reason: RefreshTokenFailedReason, message: impl Into<String>) -> Self {
        Self {
            reason,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshTokenFailedReason {
    Expired,
    Exhausted,
    Revoked,
    Other,
}

#[cfg(test)]
mod tests {
    use super::KnownPlan;
    use super::PlanType;
    use pretty_assertions::assert_eq;

    #[test]
    fn plan_type_deserializes_raw_aliases() {
        assert_eq!(
            serde_json::from_str::<PlanType>("\"hc\"").expect("hc should deserialize"),
            PlanType::Known(KnownPlan::Enterprise)
        );
        assert_eq!(
            serde_json::from_str::<PlanType>("\"education\"")
                .expect("education should deserialize"),
            PlanType::Known(KnownPlan::Edu)
        );
    }
}
