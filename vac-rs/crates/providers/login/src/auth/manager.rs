use async_trait::async_trait;
use chrono::Utc;
use reqwest::StatusCode;
use serde::Deserialize;
use serde::Serialize;
#[cfg(test)]
use serial_test::serial;
use std::env;
use std::fmt::Debug;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use tokio::sync::Semaphore;

use vac_agent_identity::decode_agent_identity_jwt;
use vac_agent_identity::fetch_agent_identity_jwks;
use vac_protocol::auth::AuthMode as CredentialMode;
use vac_protocol::auth::AuthMode as ApiCredentialMode;
use vac_protocol::config_types::ForcedLoginMethod;
use vac_protocol::config_types::ModelProviderAuthInfo;

use super::external_bearer::BearerTokenRefresher;
use super::revoke::revoke_auth_tokens;
pub use crate::auth::agent_identity::AgentIdentityAuth;
pub use crate::auth::storage::AgentIdentityAuthRecord;
pub use crate::auth::storage::AuthDotJson;
use crate::auth::storage::AuthStorageBackend;
use crate::auth::storage::create_auth_storage;
use crate::auth::util::try_parse_error_message;
use crate::default_client::build_reqwest_client;
use crate::default_client::create_client;
use crate::token_data::TokenData;
use crate::token_data::parse_chatgpt_jwt_claims;
use crate::token_data::parse_jwt_expiration;
use serde_json::Value;
use thiserror::Error;
use vac_client::VACHttpClient;
use vac_config::types::AuthCredentialsStoreMode;
use vac_protocol::account::PlanType as AccountPlanType;
use vac_protocol::auth::PlanType as InternalPlanType;
use vac_protocol::auth::RefreshTokenFailedError;
use vac_protocol::auth::RefreshTokenFailedReason;

/// Authentication mechanism used by the current user.
#[derive(Debug, Clone)]
pub enum VACAuth {
    ApiKey(ApiKeyAuth),
    Chatgpt(ChatgptAuth),
    ChatgptAuthTokens(ChatgptAuthTokens),
    AgentIdentity(AgentIdentityAuth),
}

impl PartialEq for VACAuth {
    fn eq(&self, other: &Self) -> bool {
        self.api_auth_mode() == other.api_auth_mode()
    }
}

#[derive(Debug, Clone)]
pub struct ApiKeyAuth {
    api_key: String,
}

#[derive(Debug, Clone)]
pub struct ChatgptAuth {
    state: ChatgptAuthState,
    storage: Arc<dyn AuthStorageBackend>,
}

#[derive(Debug, Clone)]
pub struct ChatgptAuthTokens {
    state: ChatgptAuthState,
}

#[derive(Debug, Clone)]
struct ChatgptAuthState {
    auth_dot_json: Arc<Mutex<Option<AuthDotJson>>>,
    client: VACHttpClient,
}

fn auth_dot_json_snapshot(state: &ChatgptAuthState) -> Option<AuthDotJson> {
    match state.auth_dot_json.lock() {
        Ok(auth_dot_json) => auth_dot_json.clone(),
        Err(poisoned) => {
            tracing::warn!("recovering poisoned ChatGPT auth state lock");
            poisoned.into_inner().clone()
        }
    }
}

const TOKEN_REFRESH_INTERVAL: i64 = 8;

const REFRESH_TOKEN_EXPIRED_MESSAGE: &str = "Your access token could not be refreshed because your refresh token has expired. Please log out and sign in again.";
const REFRESH_TOKEN_REUSED_MESSAGE: &str = "Your access token could not be refreshed because your refresh token was already used. Please log out and sign in again.";
const REFRESH_TOKEN_INVALIDATED_MESSAGE: &str = "Your access token could not be refreshed because your refresh token was revoked. Please log out and sign in again.";
const REFRESH_TOKEN_UNKNOWN_MESSAGE: &str =
    "Your access token could not be refreshed. Please log out and sign in again.";
const REFRESH_TOKEN_ACCOUNT_MISMATCH_MESSAGE: &str = "Your access token could not be refreshed because you have since logged out or signed in to another account. Please sign in again.";
const AGENT_IDENTITY_PROVIDER_URL_REQUIRED: &str = "agent identity JWKS provider URL is required; configure identity.jwks.url/provider_base_url when identity verification is required";
const REFRESH_TOKEN_URL: &str = "https://auth.vastar.com/oauth/token";
pub(super) const REVOKE_TOKEN_URL: &str = "https://auth.vastar.com/oauth/revoke";
pub const REFRESH_TOKEN_URL_OVERRIDE_ENV_VAR: &str = "VAC_REFRESH_TOKEN_URL_OVERRIDE";
pub const REVOKE_TOKEN_URL_OVERRIDE_ENV_VAR: &str = "VAC_REVOKE_TOKEN_URL_OVERRIDE";
static NEXT_DUMMY_AUTH_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Error)]
pub enum RefreshTokenError {
    #[error("{0}")]
    Permanent(#[from] RefreshTokenFailedError),
    #[error(transparent)]
    Transient(#[from] std::io::Error),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalAuthTokens {
    pub access_token: String,
    pub chatgpt_metadata: Option<ExternalAuthChatgptMetadata>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalAuthChatgptMetadata {
    pub account_id: String,
    pub plan_type: Option<String>,
}

impl ExternalAuthTokens {
    pub fn access_token_only(access_token: impl Into<String>) -> Self {
        Self {
            access_token: access_token.into(),
            chatgpt_metadata: None,
        }
    }

    pub fn chatgpt(
        access_token: impl Into<String>,
        chatgpt_account_id: impl Into<String>,
        chatgpt_plan_type: Option<String>,
    ) -> Self {
        Self {
            access_token: access_token.into(),
            chatgpt_metadata: Some(ExternalAuthChatgptMetadata {
                account_id: chatgpt_account_id.into(),
                plan_type: chatgpt_plan_type,
            }),
        }
    }

    pub fn chatgpt_metadata(&self) -> Option<&ExternalAuthChatgptMetadata> {
        self.chatgpt_metadata.as_ref()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExternalAuthRefreshReason {
    Unauthorized,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalAuthRefreshContext {
    pub reason: ExternalAuthRefreshReason,
    pub previous_account_id: Option<String>,
}

#[async_trait]
/// Pluggable auth provider used by `AuthManager` for externally managed auth flows.
///
/// Implementations may either resolve auth eagerly via `resolve()` or provide refreshed
/// credentials on demand via `refresh()`.
pub trait ExternalAuth: Send + Sync {
    /// Indicates which top-level auth mode this external provider supplies.
    fn auth_mode(&self) -> CredentialMode;

    /// Returns cached or immediately available auth, if this provider can resolve it synchronously
    /// from the caller's perspective.
    async fn resolve(&self) -> std::io::Result<Option<ExternalAuthTokens>> {
        Ok(None)
    }

    /// Refreshes auth in response to a manager-driven refresh attempt.
    async fn refresh(
        &self,
        context: ExternalAuthRefreshContext,
    ) -> std::io::Result<ExternalAuthTokens>;
}

impl RefreshTokenError {
    pub fn failed_reason(&self) -> Option<RefreshTokenFailedReason> {
        match self {
            Self::Permanent(error) => Some(error.reason),
            Self::Transient(_) => None,
        }
    }
}

impl From<RefreshTokenError> for std::io::Error {
    fn from(err: RefreshTokenError) -> Self {
        match err {
            RefreshTokenError::Permanent(failed) => std::io::Error::other(failed),
            RefreshTokenError::Transient(inner) => inner,
        }
    }
}

mod auth_manager;
mod recovery;
mod vac_auth;


impl ChatgptAuth {
    fn current_auth_json(&self) -> Option<AuthDotJson> {
        auth_dot_json_snapshot(&self.state)
    }

    fn current_token_data(&self) -> Option<TokenData> {
        self.current_auth_json().and_then(|auth| auth.tokens)
    }

    fn storage(&self) -> &Arc<dyn AuthStorageBackend> {
        &self.storage
    }

    fn client(&self) -> &VACHttpClient {
        &self.state.client
    }
}

pub const VASTAR_API_KEY_ENV_VAR: &str = "VASTAR_API_KEY";
pub const VAC_API_KEY_ENV_VAR: &str = "VAC_API_KEY";
pub const VAC_AGENT_IDENTITY_ENV_VAR: &str = "VAC_AGENT_IDENTITY";

pub fn read_vastar_api_key_from_env() -> Option<String> {
    env::var(VASTAR_API_KEY_ENV_VAR)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn read_vac_api_key_from_env() -> Option<String> {
    env::var(VAC_API_KEY_ENV_VAR)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn read_vac_agent_identity_from_env() -> Option<String> {
    env::var(VAC_AGENT_IDENTITY_ENV_VAR)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn require_agent_identity_provider_url(provider_base_url: Option<&str>) -> std::io::Result<String> {
    let Some(base_url) = provider_base_url
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Err(std::io::Error::other(AGENT_IDENTITY_PROVIDER_URL_REQUIRED));
    };
    Ok(base_url.trim_end_matches('/').to_string())
}

async fn verified_agent_identity_record(
    jwt: &str,
    chatgpt_base_url: &str,
) -> std::io::Result<AgentIdentityAuthRecord> {
    AgentIdentityAuthRecord::from_agent_identity_jwt(jwt)?;
    let jwks = fetch_agent_identity_jwks(&build_reqwest_client(), chatgpt_base_url)
        .await
        .map_err(std::io::Error::other)?;
    let claims = decode_agent_identity_jwt(jwt, Some(&jwks)).map_err(std::io::Error::other)?;
    Ok(claims.into())
}

/// Delete the auth.json file inside `vac_home` if it exists. Returns `Ok(true)`
/// if a file was removed, `Ok(false)` if no auth file was present.
pub fn logout(
    vac_home: &Path,
    auth_credentials_store_mode: AuthCredentialsStoreMode,
) -> std::io::Result<bool> {
    let storage = create_auth_storage(vac_home.to_path_buf(), auth_credentials_store_mode);
    storage.delete()
}

pub async fn logout_with_revoke(
    vac_home: &Path,
    auth_credentials_store_mode: AuthCredentialsStoreMode,
) -> std::io::Result<bool> {
    AuthManager::new(
        vac_home.to_path_buf(),
        /*enable_vac_api_key_env*/ false,
        auth_credentials_store_mode,
        /*chatgpt_base_url*/ None,
    )
    .await
    .logout_with_revoke()
    .await
}

/// Writes an `auth.json` that contains only the API key.
pub fn login_with_api_key(
    vac_home: &Path,
    api_key: &str,
    auth_credentials_store_mode: AuthCredentialsStoreMode,
) -> std::io::Result<()> {
    let auth_dot_json = AuthDotJson {
        auth_mode: Some(ApiCredentialMode::ApiKey),
        vastar_api_key: Some(api_key.to_string()),
        tokens: None,
        last_refresh: None,
        agent_identity: None,
    };
    save_auth(vac_home, &auth_dot_json, auth_credentials_store_mode)
}

/// Writes an `auth.json` that contains only the Agent Identity token.
pub async fn login_with_agent_identity(
    vac_home: &Path,
    agent_identity: &str,
    auth_credentials_store_mode: AuthCredentialsStoreMode,
    chatgpt_base_url: Option<&str>,
) -> std::io::Result<()> {
    let base_url = require_agent_identity_provider_url(chatgpt_base_url)?;
    verified_agent_identity_record(agent_identity, &base_url).await?;
    let auth_dot_json = AuthDotJson {
        auth_mode: Some(ApiCredentialMode::AgentIdentity),
        vastar_api_key: None,
        tokens: None,
        last_refresh: None,
        agent_identity: Some(agent_identity.to_string()),
    };
    save_auth(vac_home, &auth_dot_json, auth_credentials_store_mode)
}

/// Writes an in-memory auth payload for externally managed ChatGPT tokens.
pub fn login_with_chatgpt_auth_tokens(
    vac_home: &Path,
    access_token: &str,
    chatgpt_account_id: &str,
    chatgpt_plan_type: Option<&str>,
) -> std::io::Result<()> {
    let auth_dot_json = AuthDotJson::from_external_access_token(
        access_token,
        chatgpt_account_id,
        chatgpt_plan_type,
    )?;
    save_auth(
        vac_home,
        &auth_dot_json,
        AuthCredentialsStoreMode::Ephemeral,
    )
}

/// Persist the provided auth payload using the specified backend.
pub fn save_auth(
    vac_home: &Path,
    auth: &AuthDotJson,
    auth_credentials_store_mode: AuthCredentialsStoreMode,
) -> std::io::Result<()> {
    let storage = create_auth_storage(vac_home.to_path_buf(), auth_credentials_store_mode);
    storage.save(auth)
}

/// Load CLI auth data using the configured credential store backend.
/// Returns `None` when no credentials are stored. This function is
/// provided only for tests. Production code should not directly load
/// from the auth.json storage. It should use the AuthManager abstraction
/// instead.
pub fn load_auth_dot_json(
    vac_home: &Path,
    auth_credentials_store_mode: AuthCredentialsStoreMode,
) -> std::io::Result<Option<AuthDotJson>> {
    let storage = create_auth_storage(vac_home.to_path_buf(), auth_credentials_store_mode);
    storage.load()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthConfig {
    pub vac_home: PathBuf,
    pub auth_credentials_store_mode: AuthCredentialsStoreMode,
    pub forced_login_method: Option<ForcedLoginMethod>,
    pub forced_chatgpt_workspace_id: Option<String>,
    pub chatgpt_base_url: Option<String>,
}

pub async fn enforce_login_restrictions(config: &AuthConfig) -> std::io::Result<()> {
    let Some(auth) = load_auth(
        &config.vac_home,
        /*enable_vac_api_key_env*/ true,
        config.auth_credentials_store_mode,
        config.chatgpt_base_url.as_deref(),
    )
    .await?
    else {
        return Ok(());
    };

    if let Some(required_method) = config.forced_login_method {
        let method_violation = match (required_method, auth.auth_mode()) {
            (ForcedLoginMethod::Api, CredentialMode::ApiKey) => None,
            (
                ForcedLoginMethod::Chatgpt,
                CredentialMode::ProviderCredential
                | CredentialMode::AgentIdentity
                | CredentialMode::Bearer
                | CredentialMode::Local,
            ) => None,
            (
                ForcedLoginMethod::Chatgpt,
                CredentialMode::Chatgpt | CredentialMode::ChatgptAuthTokens,
            ) if CredentialMode::legacy_chatgpt_account_enabled() => None,
            (ForcedLoginMethod::Api, CredentialMode::ProviderCredential | CredentialMode::AgentIdentity)
            | (ForcedLoginMethod::Api, CredentialMode::Chatgpt | CredentialMode::ChatgptAuthTokens) => Some(
                "API key login is required, but provider credential auth is currently being used. Logging out."
                    .to_string(),
            ),
            (ForcedLoginMethod::Api, CredentialMode::Bearer | CredentialMode::Local) => None,
            (ForcedLoginMethod::Chatgpt, CredentialMode::ApiKey) => Some(
                "Provider credential login is required, but an API key is currently being used. Logging out."
                    .to_string(),
            ),
            (ForcedLoginMethod::Chatgpt, CredentialMode::Chatgpt | CredentialMode::ChatgptAuthTokens) => Some(
                "Legacy ChatGPT account auth is disabled. Logging out.".to_string(),
            ),
        };

        if let Some(message) = method_violation {
            return logout_with_message(
                &config.vac_home,
                message,
                config.auth_credentials_store_mode,
            );
        }
    }

    if let Some(expected_account_id) = config.forced_chatgpt_workspace_id.as_deref() {
        // workspace is the external identifier for account id.
        let chatgpt_account_id = match auth {
            VACAuth::ApiKey(_) => return Ok(()),
            VACAuth::AgentIdentity(_) => auth.get_account_id(),
            VACAuth::Chatgpt(_) | VACAuth::ChatgptAuthTokens(_) => {
                let token_data = match auth.get_token_data() {
                    Ok(data) => data,
                    Err(err) => {
                        return logout_with_message(
                            &config.vac_home,
                            format!(
                                "Failed to load ChatGPT credentials while enforcing workspace restrictions: {err}. Logging out."
                            ),
                            config.auth_credentials_store_mode,
                        );
                    }
                };
                token_data.id_token.chatgpt_account_id
            }
        };
        if chatgpt_account_id.as_deref() != Some(expected_account_id) {
            let message = match chatgpt_account_id {
                Some(actual) => format!(
                    "Login is restricted to workspace {expected_account_id}, but current credentials belong to {actual}. Logging out."
                ),
                None => format!(
                    "Login is restricted to workspace {expected_account_id}, but current credentials lack a workspace identifier. Logging out."
                ),
            };
            return logout_with_message(
                &config.vac_home,
                message,
                config.auth_credentials_store_mode,
            );
        }
    }

    Ok(())
}

fn logout_with_message(
    vac_home: &Path,
    message: String,
    auth_credentials_store_mode: AuthCredentialsStoreMode,
) -> std::io::Result<()> {
    // External auth tokens live in the ephemeral store, but persistent auth may still exist
    // from earlier logins. Clear both so a forced logout truly removes all active auth.
    let removal_result = logout_all_stores(vac_home, auth_credentials_store_mode);
    let error_message = match removal_result {
        Ok(_) => message,
        Err(err) => format!("{message}. Failed to remove auth.json: {err}"),
    };
    Err(std::io::Error::other(error_message))
}

fn logout_all_stores(
    vac_home: &Path,
    auth_credentials_store_mode: AuthCredentialsStoreMode,
) -> std::io::Result<bool> {
    if auth_credentials_store_mode == AuthCredentialsStoreMode::Ephemeral {
        return logout(vac_home, AuthCredentialsStoreMode::Ephemeral);
    }
    let removed_ephemeral = logout(vac_home, AuthCredentialsStoreMode::Ephemeral)?;
    let removed_managed = logout(vac_home, auth_credentials_store_mode)?;
    Ok(removed_ephemeral || removed_managed)
}

async fn load_auth(
    vac_home: &Path,
    enable_vac_api_key_env: bool,
    auth_credentials_store_mode: AuthCredentialsStoreMode,
    chatgpt_base_url: Option<&str>,
) -> std::io::Result<Option<VACAuth>> {
    // API key via env var takes precedence over any other auth method.
    if enable_vac_api_key_env && let Some(api_key) = read_vac_api_key_from_env() {
        return Ok(Some(VACAuth::from_api_key(api_key.as_str())));
    }

    // External ChatGPT auth tokens live in the in-memory (ephemeral) store. Always check this
    // first so external auth takes precedence over any persisted credentials.
    let ephemeral_storage =
        create_auth_storage(vac_home.to_path_buf(), AuthCredentialsStoreMode::Ephemeral);
    if let Some(auth_dot_json) = ephemeral_storage.load()? {
        let auth = VACAuth::from_auth_dot_json(
            vac_home,
            auth_dot_json,
            AuthCredentialsStoreMode::Ephemeral,
            chatgpt_base_url,
        )
        .await?;
        return Ok(Some(auth));
    }

    // If the caller explicitly requested ephemeral auth, there is no persisted fallback.
    if auth_credentials_store_mode == AuthCredentialsStoreMode::Ephemeral {
        return Ok(None);
    }

    if let Some(agent_identity) = read_vac_agent_identity_from_env() {
        return VACAuth::from_agent_identity_jwt(&agent_identity, chatgpt_base_url)
            .await
            .map(Some);
    }

    // Fall back to the configured persistent store (file/keyring/auto) for managed auth.
    let storage = create_auth_storage(vac_home.to_path_buf(), auth_credentials_store_mode);
    let auth_dot_json = match storage.load()? {
        Some(auth) => auth,
        None => return Ok(None),
    };

    let auth = VACAuth::from_auth_dot_json(
        vac_home,
        auth_dot_json,
        auth_credentials_store_mode,
        chatgpt_base_url,
    )
    .await?;
    Ok(Some(auth))
}

// Persist refreshed tokens into auth storage and update last_refresh.
fn persist_tokens(
    storage: &Arc<dyn AuthStorageBackend>,
    id_token: Option<String>,
    access_token: Option<String>,
    refresh_token: Option<String>,
) -> std::io::Result<AuthDotJson> {
    let mut auth_dot_json = storage
        .load()?
        .ok_or(std::io::Error::other("Token data is not available."))?;

    let tokens = auth_dot_json.tokens.get_or_insert_with(TokenData::default);
    if let Some(id_token) = id_token {
        tokens.id_token = parse_chatgpt_jwt_claims(&id_token).map_err(std::io::Error::other)?;
    }
    if let Some(access_token) = access_token {
        tokens.access_token = access_token;
    }
    if let Some(refresh_token) = refresh_token {
        tokens.refresh_token = refresh_token;
    }
    auth_dot_json.last_refresh = Some(Utc::now());
    storage.save(&auth_dot_json)?;
    Ok(auth_dot_json)
}

// Requests refreshed ChatGPT OAuth tokens from the auth service using a refresh token.
// The caller is responsible for persisting any returned tokens.
async fn request_chatgpt_token_refresh(
    refresh_token: String,
    client: &VACHttpClient,
) -> Result<RefreshResponse, RefreshTokenError> {
    let refresh_request = RefreshRequest {
        client_id: CLIENT_ID,
        grant_type: "refresh_token",
        refresh_token,
    };

    let endpoint = refresh_token_endpoint();

    // Use shared client factory to include standard headers
    let response = client
        .post(endpoint.as_str())
        .header("Content-Type", "application/json")
        .json(&refresh_request)
        .send()
        .await
        .map_err(|err| RefreshTokenError::Transient(std::io::Error::other(err)))?;

    let status = response.status();
    if status.is_success() {
        let refresh_response = response
            .json::<RefreshResponse>()
            .await
            .map_err(|err| RefreshTokenError::Transient(std::io::Error::other(err)))?;
        Ok(refresh_response)
    } else {
        let body = response.text().await.unwrap_or_default();
        tracing::error!("Failed to refresh token: {status}: {body}");
        if status == StatusCode::UNAUTHORIZED {
            let failed = classify_refresh_token_failure(&body);
            Err(RefreshTokenError::Permanent(failed))
        } else {
            let message = try_parse_error_message(&body);
            Err(RefreshTokenError::Transient(std::io::Error::other(
                format!("Failed to refresh token: {status}: {message}"),
            )))
        }
    }
}

fn classify_refresh_token_failure(body: &str) -> RefreshTokenFailedError {
    let code = extract_refresh_token_error_code(body);

    let normalized_code = code.as_deref().map(str::to_ascii_lowercase);
    let reason = match normalized_code.as_deref() {
        Some("refresh_token_expired") => RefreshTokenFailedReason::Expired,
        Some("refresh_token_reused") => RefreshTokenFailedReason::Exhausted,
        Some("refresh_token_invalidated") => RefreshTokenFailedReason::Revoked,
        _ => RefreshTokenFailedReason::Other,
    };

    if reason == RefreshTokenFailedReason::Other {
        tracing::warn!(
            backend_code = normalized_code.as_deref(),
            backend_body = body,
            "Encountered unknown 401 response while refreshing token"
        );
    }

    let message = match reason {
        RefreshTokenFailedReason::Expired => REFRESH_TOKEN_EXPIRED_MESSAGE.to_string(),
        RefreshTokenFailedReason::Exhausted => REFRESH_TOKEN_REUSED_MESSAGE.to_string(),
        RefreshTokenFailedReason::Revoked => REFRESH_TOKEN_INVALIDATED_MESSAGE.to_string(),
        RefreshTokenFailedReason::Other => REFRESH_TOKEN_UNKNOWN_MESSAGE.to_string(),
    };

    RefreshTokenFailedError::new(reason, message)
}

fn extract_refresh_token_error_code(body: &str) -> Option<String> {
    if body.trim().is_empty() {
        return None;
    }

    let Value::Object(map) = serde_json::from_str::<Value>(body).ok()? else {
        return None;
    };

    if let Some(error_value) = map.get("error") {
        match error_value {
            Value::Object(obj) => {
                if let Some(code) = obj.get("code").and_then(Value::as_str) {
                    return Some(code.to_string());
                }
            }
            Value::String(code) => {
                return Some(code.to_string());
            }
            _ => {}
        }
    }

    map.get("code").and_then(Value::as_str).map(str::to_string)
}

#[derive(Serialize)]
struct RefreshRequest {
    client_id: &'static str,
    grant_type: &'static str,
    refresh_token: String,
}

#[derive(Deserialize, Clone)]
struct RefreshResponse {
    id_token: Option<String>,
    access_token: Option<String>,
    refresh_token: Option<String>,
}

// Shared constant for token refresh (client id used for oauth token refresh flow)
pub const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";

fn refresh_token_endpoint() -> String {
    std::env::var(REFRESH_TOKEN_URL_OVERRIDE_ENV_VAR)
        .unwrap_or_else(|_| REFRESH_TOKEN_URL.to_string())
}

impl AuthDotJson {
    fn from_external_tokens(external: &ExternalAuthTokens) -> std::io::Result<Self> {
        let Some(chatgpt_metadata) = external.chatgpt_metadata() else {
            return Err(std::io::Error::other(
                "external auth tokens are missing ChatGPT metadata",
            ));
        };
        let mut token_info =
            parse_chatgpt_jwt_claims(&external.access_token).map_err(std::io::Error::other)?;
        token_info.chatgpt_account_id = Some(chatgpt_metadata.account_id.clone());
        token_info.chatgpt_plan_type = chatgpt_metadata
            .plan_type
            .as_deref()
            .map(InternalPlanType::from_raw_value)
            .or(token_info.chatgpt_plan_type)
            .or(Some(InternalPlanType::Unknown("unknown".to_string())));
        let tokens = TokenData {
            id_token: token_info,
            access_token: external.access_token.clone(),
            refresh_token: String::new(),
            account_id: Some(chatgpt_metadata.account_id.clone()),
        };

        Ok(Self {
            auth_mode: Some(ApiCredentialMode::ProviderCredential),
            vastar_api_key: None,
            tokens: Some(tokens),
            last_refresh: Some(Utc::now()),
            agent_identity: None,
        })
    }

    fn from_external_access_token(
        access_token: &str,
        chatgpt_account_id: &str,
        chatgpt_plan_type: Option<&str>,
    ) -> std::io::Result<Self> {
        let external = ExternalAuthTokens::chatgpt(
            access_token,
            chatgpt_account_id,
            chatgpt_plan_type.map(str::to_string),
        );
        Self::from_external_tokens(&external)
    }

    fn resolved_mode(&self) -> ApiCredentialMode {
        if let Some(mode) = self.auth_mode {
            return mode;
        }
        if self.vastar_api_key.is_some() {
            return ApiCredentialMode::ApiKey;
        }
        ApiCredentialMode::ProviderCredential
    }

    fn storage_mode(
        &self,
        auth_credentials_store_mode: AuthCredentialsStoreMode,
    ) -> AuthCredentialsStoreMode {
        if matches!(
            self.resolved_mode(),
            ApiCredentialMode::ProviderCredential | ApiCredentialMode::ChatgptAuthTokens
        ) {
            AuthCredentialsStoreMode::Ephemeral
        } else {
            auth_credentials_store_mode
        }
    }
}

/// Internal cached auth state.
#[derive(Clone)]
struct CachedAuth {
    auth: Option<VACAuth>,
    /// Permanent refresh failure cached for the current auth snapshot so
    /// later refresh attempts for the same credentials fail fast without network.
    permanent_refresh_failure: Option<AuthScopedRefreshFailure>,
}

#[derive(Clone)]
struct AuthScopedRefreshFailure {
    auth: VACAuth,
    error: RefreshTokenFailedError,
}

impl Debug for CachedAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CachedAuth")
            .field("auth_mode", &self.auth.as_ref().map(VACAuth::api_auth_mode))
            .field(
                "permanent_refresh_failure",
                &self
                    .permanent_refresh_failure
                    .as_ref()
                    .map(|failure| failure.error.reason),
            )
            .finish()
    }
}

enum UnauthorizedRecoveryStep {
    Reload,
    RefreshToken,
    ExternalRefresh,
    Done,
}

enum ReloadOutcome {
    /// Reload was performed and the cached auth changed
    ReloadedChanged,
    /// Reload was performed and the cached auth remained the same
    ReloadedNoChange,
    /// Reload was skipped (missing or mismatched account id)
    Skipped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UnauthorizedRecoveryMode {
    Managed,
    External,
}

// UnauthorizedRecovery is a state machine that handles an attempt to refresh the authentication when requests
// to API fail with 401 status code.
// The client calls next() every time it encounters a 401 error, one time per retry.
// For API key based authentication, we don't do anything and let the error bubble to the user.
//
// For ChatGPT based authentication, we:
// 1. Attempt to reload the auth data from disk. We only reload if the account id matches the one the current process is running as.
// 2. Attempt to refresh the token using OAuth token refresh flow.
// If after both steps the server still responds with 401 we let the error bubble to the user.
//
// For external auth sources, UnauthorizedRecovery retries once.
//
// - External ChatGPT auth tokens (`chatgptAuthTokens`) are refreshed by asking
//   the parent app for new tokens through the configured
//   `ExternalAuth`, persisting them in the ephemeral auth store, and
//   reloading the cached auth snapshot.
// - External bearer auth sources for custom model providers rerun the provider
//   auth command without touching disk.
pub struct UnauthorizedRecovery {
    manager: Arc<AuthManager>,
    step: UnauthorizedRecoveryStep,
    expected_account_id: Option<String>,
    mode: UnauthorizedRecoveryMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnauthorizedRecoveryStepResult {
    auth_state_changed: Option<bool>,
}

impl UnauthorizedRecoveryStepResult {
    pub fn auth_state_changed(&self) -> Option<bool> {
        self.auth_state_changed
    }
}


/// Central manager providing a single source of truth for auth.json derived
/// authentication data. It loads once (or on preference change) and then
/// hands out cloned `VACAuth` values so the rest of the program has a
/// consistent snapshot.
///
/// External modifications to `auth.json` will NOT be observed until
/// `reload()` is called explicitly. This matches the design goal of avoiding
/// different parts of the program seeing inconsistent auth data mid‑run.
pub struct AuthManager {
    vac_home: PathBuf,
    inner: RwLock<CachedAuth>,
    enable_vac_api_key_env: bool,
    auth_credentials_store_mode: AuthCredentialsStoreMode,
    forced_chatgpt_workspace_id: RwLock<Option<String>>,
    chatgpt_base_url: Option<String>,
    refresh_lock: Semaphore,
    external_auth: RwLock<Option<Arc<dyn ExternalAuth>>>,
}

/// Configuration view required to construct a shared [`AuthManager`].
///
/// Implementations should return the auth-related config values for the
/// already-resolved runtime configuration. The primary implementation is
/// `vac_core::config::Config`, but this trait keeps `vac-login` independent
/// from `vac-core`.
pub trait AuthManagerConfig {
    /// Returns the VAC home directory used for auth storage.
    fn vac_home(&self) -> PathBuf;

    /// Returns the CLI auth credential storage mode for auth loading.
    fn cli_auth_credentials_store_mode(&self) -> AuthCredentialsStoreMode;

    /// Returns the workspace ID that ChatGPT auth should be restricted to, if any.
    fn forced_chatgpt_workspace_id(&self) -> Option<String>;

    /// Returns the ChatGPT backend base URL used for first-party backend authorization.
    fn chatgpt_base_url(&self) -> String;
}

impl Debug for AuthManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthManager")
            .field("vac_home", &self.vac_home)
            .field("inner", &self.inner)
            .field("enable_vac_api_key_env", &self.enable_vac_api_key_env)
            .field(
                "auth_credentials_store_mode",
                &self.auth_credentials_store_mode,
            )
            .field(
                "forced_chatgpt_workspace_id",
                &self.forced_chatgpt_workspace_id,
            )
            .field("chatgpt_base_url", &self.chatgpt_base_url)
            .field("has_external_auth", &self.has_external_auth())
            .finish_non_exhaustive()
    }
}


#[cfg(test)]
#[path = "auth_tests.rs"]
mod tests;
