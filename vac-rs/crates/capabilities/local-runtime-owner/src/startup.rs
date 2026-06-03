//! Startup seam for the local runtime owner.
//!
//! This builds retained resources directly from resolved runtime inputs and
//! produces app-server-free bootstrap data for the default TUI product path.
//! Prompt submission, event streaming, request dispatch, server-request
//! resolution, and config/import surfaces are now owned by the local runtime
//! owner child plans; any app-server compatibility is non-default quarantine
//! material outside this startup seam.

use std::fmt;
use std::sync::Arc;

use vac_analytics::AnalyticsEventsClient;
use vac_core::ThreadManager;
use vac_core::config::Config;
use vac_core::thread_store_from_config;
use vac_exec_server::EnvironmentManager;
use vac_login::AuthManager;
use vac_login::VACAuth;
use vac_models_manager::manager::RefreshStrategy;
use vac_protocol::account::PlanType;
use vac_protocol::protocol::SessionSource;
use vac_protocol::vastar_models::ModelPreset;

use crate::LocalRuntimeOwnerSession;
use crate::RuntimeRetainedResources;

pub const DEFAULT_PATH_APP_SERVER_FALLBACKS: &[&str] = &[];

pub const OWNER_NATIVE_DEFAULT_SURFACES: &[&str] = &[
    "retained resource startup",
    "TUI bootstrap projection",
    "prompt submit / turn execution",
    "typed TUI request dispatch",
    "live event stream projection",
    "server-request resolve/reject registry",
    "config reload/import command surface",
];

#[derive(Clone)]
pub struct RuntimeStartupInput {
    pub config: Arc<Config>,
    pub environment_manager: Arc<EnvironmentManager>,
    pub session_source: SessionSource,
    pub enable_vac_api_key_env: bool,
}

impl RuntimeStartupInput {
    #[must_use]
    pub fn new(
        config: Arc<Config>,
        environment_manager: Arc<EnvironmentManager>,
        session_source: SessionSource,
        enable_vac_api_key_env: bool,
    ) -> Self {
        Self {
            config,
            environment_manager,
            session_source,
            enable_vac_api_key_env,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalRuntimeAuthMode {
    ApiKey,
    ProviderCredential,
    AgentIdentity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LocalRuntimeAccountDisplay {
    ApiKey,
    ChatGpt {
        email: Option<String>,
        plan: Option<String>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalRuntimeFeedbackAudience {
    External,
    InternalTester,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LocalRuntimeBootstrap {
    pub account_email: Option<String>,
    pub auth_mode: Option<LocalRuntimeAuthMode>,
    pub status_account_display: Option<LocalRuntimeAccountDisplay>,
    pub plan_type: Option<PlanType>,
    pub requires_vastar_auth: bool,
    pub default_model: String,
    pub feedback_audience: LocalRuntimeFeedbackAudience,
    pub has_chatgpt_account: bool,
    pub available_models: Vec<ModelPreset>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeStartupError {
    NoModelsAvailable,
    MissingChatGptAccountDetails,
}

impl fmt::Display for RuntimeStartupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoModelsAvailable => {
                f.write_str("model list returned no models for local runtime bootstrap")
            }
            Self::MissingChatGptAccountDetails => {
                f.write_str("email and plan type are required for chatgpt authentication")
            }
        }
    }
}

impl std::error::Error for RuntimeStartupError {}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LocalRuntimeOwner;

impl LocalRuntimeOwner {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    pub async fn start(
        &self,
        input: RuntimeStartupInput,
    ) -> Result<LocalRuntimeOwnerSession, RuntimeStartupError> {
        let retained = self.build_retained_resources(input).await;
        let bootstrap = self.build_bootstrap(&retained).await?;
        Ok(LocalRuntimeOwnerSession::new(retained, bootstrap))
    }

    pub async fn build_retained_resources(
        &self,
        input: RuntimeStartupInput,
    ) -> RuntimeRetainedResources {
        let auth_manager =
            AuthManager::shared_from_config(input.config.as_ref(), input.enable_vac_api_key_env)
                .await;
        let analytics_events_client =
            analytics_events_client_from_config(Arc::clone(&auth_manager), input.config.as_ref());
        let thread_store = thread_store_from_config(input.config.as_ref());
        let thread_manager = Arc::new(ThreadManager::new(
            input.config.as_ref(),
            Arc::clone(&auth_manager),
            input.session_source,
            Arc::clone(&input.environment_manager),
            Some(analytics_events_client.clone()),
            Arc::clone(&thread_store),
        ));

        RuntimeRetainedResources::new(
            input.config,
            auth_manager,
            thread_manager,
            thread_store,
            input.environment_manager,
            analytics_events_client,
        )
    }

    pub async fn build_bootstrap(
        &self,
        retained: &RuntimeRetainedResources,
    ) -> Result<LocalRuntimeBootstrap, RuntimeStartupError> {
        let account = account_from_auth_manager(&retained.auth_manager())?;
        let available_models = retained
            .thread_manager()
            .list_models(RefreshStrategy::OnlineIfUncached)
            .await;
        let config = retained.config();
        let default_model = config
            .model
            .clone()
            .or_else(|| {
                available_models
                    .iter()
                    .find(|model| model.is_default)
                    .map(|model| model.model.clone())
            })
            .or_else(|| available_models.first().map(|model| model.model.clone()))
            .ok_or(RuntimeStartupError::NoModelsAvailable)?;

        let requires_vastar_auth = config.model_provider.requires_vastar_auth;
        let feedback_audience = match account.account_email.as_deref() {
            Some(email) if email.ends_with("@vastar.com") => {
                LocalRuntimeFeedbackAudience::InternalTester
            }
            _ => LocalRuntimeFeedbackAudience::External,
        };
        let has_chatgpt_account = matches!(
            account.auth_mode,
            Some(LocalRuntimeAuthMode::AgentIdentity | LocalRuntimeAuthMode::ProviderCredential)
        );

        Ok(LocalRuntimeBootstrap {
            account_email: account.account_email,
            auth_mode: account.auth_mode,
            status_account_display: account.status_account_display,
            plan_type: account.plan_type,
            requires_vastar_auth,
            default_model,
            feedback_audience,
            has_chatgpt_account,
            available_models,
        })
    }
}

#[derive(Debug)]
struct LocalRuntimeAccount {
    account_email: Option<String>,
    auth_mode: Option<LocalRuntimeAuthMode>,
    status_account_display: Option<LocalRuntimeAccountDisplay>,
    plan_type: Option<PlanType>,
}

fn analytics_events_client_from_config(
    auth_manager: Arc<AuthManager>,
    config: &Config,
) -> AnalyticsEventsClient {
    AnalyticsEventsClient::new(
        auth_manager,
        config.chatgpt_base_url.trim_end_matches('/').to_string(),
        config.analytics_enabled,
    )
}

fn account_from_auth_manager(
    auth_manager: &Arc<AuthManager>,
) -> Result<LocalRuntimeAccount, RuntimeStartupError> {
    let auth = auth_manager.auth_cached();
    let Some(auth) = auth.as_ref() else {
        return Ok(LocalRuntimeAccount {
            account_email: None,
            auth_mode: None,
            status_account_display: None,
            plan_type: None,
        });
    };

    match auth {
        VACAuth::ApiKey(_) => Ok(LocalRuntimeAccount {
            account_email: None,
            auth_mode: Some(LocalRuntimeAuthMode::ApiKey),
            status_account_display: Some(LocalRuntimeAccountDisplay::ApiKey),
            plan_type: None,
        }),
        VACAuth::Chatgpt(_) | VACAuth::ChatgptAuthTokens(_)
            if !vac_protocol::auth::AuthMode::legacy_chatgpt_account_enabled() =>
        {
            tracing::warn!(
                "legacy ChatGPT account local-runtime startup is disabled; treating account as unauthenticated local provider"
            );
            Ok(LocalRuntimeAccount {
                account_email: None,
                auth_mode: None,
                status_account_display: None,
                plan_type: None,
            })
        }
        VACAuth::Chatgpt(_) | VACAuth::ChatgptAuthTokens(_) | VACAuth::AgentIdentity(_) => {
            let email = auth
                .get_account_email()
                .ok_or(RuntimeStartupError::MissingChatGptAccountDetails)?;
            let plan_type = auth
                .account_plan_type()
                .ok_or(RuntimeStartupError::MissingChatGptAccountDetails)?;
            Ok(LocalRuntimeAccount {
                account_email: Some(email.clone()),
                auth_mode: Some(match auth {
                    VACAuth::Chatgpt(_) => LocalRuntimeAuthMode::ProviderCredential,
                    VACAuth::ChatgptAuthTokens(_) => LocalRuntimeAuthMode::ProviderCredential,
                    VACAuth::AgentIdentity(_) => LocalRuntimeAuthMode::AgentIdentity,
                    VACAuth::ApiKey(_) => unreachable!("handled above"),
                }),
                status_account_display: Some(LocalRuntimeAccountDisplay::ChatGpt {
                    email: Some(email),
                    plan: Some(plan_type_display_name(plan_type)),
                }),
                plan_type: Some(plan_type),
            })
        }
    }
}

fn plan_type_display_name(plan_type: PlanType) -> String {
    if plan_type.is_team_like() {
        "Business".to_string()
    } else if plan_type.is_business_like() {
        "Enterprise".to_string()
    } else if plan_type == PlanType::ProLite {
        "Pro Lite".to_string()
    } else {
        title_case(format!("{plan_type:?}").as_str())
    }
}

fn title_case(s: &str) -> String {
    if s.is_empty() {
        return String::new();
    }
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let rest = chars.as_str().to_ascii_lowercase();
    first.to_uppercase().collect::<String>() + &rest
}
