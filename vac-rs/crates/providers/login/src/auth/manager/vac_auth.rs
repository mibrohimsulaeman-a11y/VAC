use super::*;

impl VACAuth {
    pub(super) async fn from_auth_dot_json(
        vac_home: &Path,
        auth_dot_json: AuthDotJson,
        auth_credentials_store_mode: AuthCredentialsStoreMode,
        chatgpt_base_url: Option<&str>,
    ) -> std::io::Result<Self> {
        let auth_mode = auth_dot_json.resolved_mode();
        let client = create_client();
        if auth_mode == ApiCredentialMode::ApiKey {
            let Some(api_key) = auth_dot_json.vastar_api_key.as_deref() else {
                return Err(std::io::Error::other("API key auth is missing a key."));
            };
            return Ok(Self::from_api_key(api_key));
        }
        if auth_mode == ApiCredentialMode::AgentIdentity {
            let Some(agent_identity) = auth_dot_json.agent_identity else {
                return Err(std::io::Error::other(
                    "agent identity auth is missing an agent identity token.",
                ));
            };
            return Self::from_agent_identity_jwt(&agent_identity, chatgpt_base_url).await;
        }

        let storage_mode = auth_dot_json.storage_mode(auth_credentials_store_mode);
        let state = ChatgptAuthState {
            auth_dot_json: Arc::new(Mutex::new(Some(auth_dot_json))),
            client,
        };

        match auth_mode {
            ApiCredentialMode::ProviderCredential | ApiCredentialMode::Chatgpt => {
                if auth_mode.is_legacy_chatgpt_account()
                    && !ApiCredentialMode::legacy_chatgpt_account_enabled()
                {
                    return Err(std::io::Error::other(
                        "legacy ChatGPT account auth is disabled; use providerCredential or enable the explicit legacy provider feature",
                    ));
                }
                let storage = create_auth_storage(vac_home.to_path_buf(), storage_mode);
                Ok(Self::Chatgpt(ChatgptAuth { state, storage }))
            }
            ApiCredentialMode::ChatgptAuthTokens => {
                if !ApiCredentialMode::legacy_chatgpt_account_enabled() {
                    return Err(std::io::Error::other(
                        "legacy ChatGPT token auth is disabled; use providerCredential bearer credentials",
                    ));
                }
                Ok(Self::ChatgptAuthTokens(ChatgptAuthTokens { state }))
            }
            ApiCredentialMode::Bearer | ApiCredentialMode::Local => Err(std::io::Error::other(
                "bearer/local auth modes must be materialized by the provider credential resolver before login storage is loaded",
            )),
            ApiCredentialMode::ApiKey => unreachable!("api key mode is handled above"),
            ApiCredentialMode::AgentIdentity => {
                unreachable!("agent identity mode is handled above")
            }
        }
    }

    pub async fn from_auth_storage(
        vac_home: &Path,
        auth_credentials_store_mode: AuthCredentialsStoreMode,
        chatgpt_base_url: Option<&str>,
    ) -> std::io::Result<Option<Self>> {
        load_auth(
            vac_home,
            /*enable_vac_api_key_env*/ false,
            auth_credentials_store_mode,
            chatgpt_base_url,
        )
        .await
    }

    pub async fn from_agent_identity_jwt(
        jwt: &str,
        chatgpt_base_url: Option<&str>,
    ) -> std::io::Result<Self> {
        let base_url = require_agent_identity_provider_url(chatgpt_base_url)?;
        let record = verified_agent_identity_record(jwt, &base_url).await?;
        Ok(Self::AgentIdentity(AgentIdentityAuth::load(record).await?))
    }

    pub fn auth_mode(&self) -> CredentialMode {
        match self {
            Self::ApiKey(_) => CredentialMode::ApiKey,
            Self::Chatgpt(_) | Self::ChatgptAuthTokens(_) => CredentialMode::ProviderCredential,
            Self::AgentIdentity(_) => CredentialMode::AgentIdentity,
        }
    }

    pub fn api_auth_mode(&self) -> ApiCredentialMode {
        match self {
            Self::ApiKey(_) => ApiCredentialMode::ApiKey,
            Self::Chatgpt(_) => ApiCredentialMode::ProviderCredential,
            Self::ChatgptAuthTokens(_) => ApiCredentialMode::ProviderCredential,
            Self::AgentIdentity(_) => ApiCredentialMode::AgentIdentity,
        }
    }

    pub fn is_api_key_auth(&self) -> bool {
        self.auth_mode() == CredentialMode::ApiKey
    }

    pub fn is_chatgpt_auth(&self) -> bool {
        matches!(self, Self::Chatgpt(_) | Self::ChatgptAuthTokens(_))
    }

    pub fn uses_vac_backend(&self) -> bool {
        matches!(
            self,
            Self::Chatgpt(_) | Self::ChatgptAuthTokens(_) | Self::AgentIdentity(_)
        )
    }

    pub fn is_external_chatgpt_tokens(&self) -> bool {
        matches!(self, Self::ChatgptAuthTokens(_))
    }

    /// Returns `None` if `auth_mode() != CredentialMode::ApiKey`.
    pub fn api_key(&self) -> Option<&str> {
        match self {
            Self::ApiKey(auth) => Some(auth.api_key.as_str()),
            Self::Chatgpt(_) | Self::ChatgptAuthTokens(_) | Self::AgentIdentity(_) => None,
        }
    }

    /// Returns `Err` if token-backed ChatGPT auth is unavailable.
    pub fn get_token_data(&self) -> Result<TokenData, std::io::Error> {
        let auth_dot_json: Option<AuthDotJson> = self.get_current_auth_json();
        match auth_dot_json {
            Some(AuthDotJson {
                tokens: Some(tokens),
                last_refresh: Some(_),
                ..
            }) => Ok(tokens),
            _ => Err(std::io::Error::other("Token data is not available.")),
        }
    }

    /// Returns the token string used for bearer authentication.
    pub fn get_token(&self) -> Result<String, std::io::Error> {
        match self {
            Self::ApiKey(auth) => Ok(auth.api_key.clone()),
            Self::Chatgpt(_) | Self::ChatgptAuthTokens(_) => {
                let access_token = self.get_token_data()?.access_token;
                Ok(access_token)
            }
            Self::AgentIdentity(_) => Err(std::io::Error::other(
                "agent identity auth does not expose a bearer token",
            )),
        }
    }

    /// Returns `None` if VAC backend auth does not expose an account id.
    pub fn get_account_id(&self) -> Option<String> {
        match self {
            Self::AgentIdentity(auth) => Some(auth.account_id().to_string()),
            _ => self.get_current_token_data().and_then(|t| t.account_id),
        }
    }

    /// Returns false if VAC backend auth omits the FedRAMP claim.
    pub fn is_fedramp_account(&self) -> bool {
        match self {
            Self::AgentIdentity(auth) => auth.is_fedramp_account(),
            _ => self
                .get_current_token_data()
                .is_some_and(|t| t.id_token.is_fedramp_account()),
        }
    }

    /// Returns `None` if VAC backend auth does not expose an account email.
    pub fn get_account_email(&self) -> Option<String> {
        match self {
            Self::AgentIdentity(auth) => Some(auth.email().to_string()),
            _ => self.get_current_token_data().and_then(|t| t.id_token.email),
        }
    }

    /// Returns `None` if VAC backend auth does not expose a ChatGPT user id.
    pub fn get_chatgpt_user_id(&self) -> Option<String> {
        match self {
            Self::AgentIdentity(auth) => Some(auth.chatgpt_user_id().to_string()),
            _ => self
                .get_current_token_data()
                .and_then(|t| t.id_token.chatgpt_user_id),
        }
    }

    /// Account-facing plan classification derived from the current auth.
    /// Returns a high-level `AccountPlanType` (e.g., Free/Plus/Pro/Team/…)
    /// for UI or product decisions based on the user's subscription.
    pub fn account_plan_type(&self) -> Option<AccountPlanType> {
        if let Self::AgentIdentity(auth) = self {
            return Some(auth.plan_type());
        }

        self.get_current_token_data().map(|t| {
            t.id_token
                .chatgpt_plan_type
                .map(AccountPlanType::from)
                .unwrap_or(AccountPlanType::Unknown)
        })
    }

    pub fn is_workspace_account(&self) -> bool {
        self.account_plan_type()
            .is_some_and(AccountPlanType::is_workspace_account)
    }

    /// Returns `None` if token-backed ChatGPT auth is unavailable.
    pub(super) fn get_current_auth_json(&self) -> Option<AuthDotJson> {
        let state = match self {
            Self::Chatgpt(auth) => &auth.state,
            Self::ChatgptAuthTokens(auth) => &auth.state,
            Self::ApiKey(_) | Self::AgentIdentity(_) => return None,
        };
        auth_dot_json_snapshot(state)
    }

    /// Returns `None` if token-backed ChatGPT auth is unavailable.
    pub(super) fn get_current_token_data(&self) -> Option<TokenData> {
        self.get_current_auth_json().and_then(|t| t.tokens)
    }

    /// Consider this private to integration tests.
    pub fn create_dummy_chatgpt_auth_for_testing() -> Self {
        let auth_dot_json = AuthDotJson {
            auth_mode: Some(ApiCredentialMode::ProviderCredential),
            vastar_api_key: None,
            tokens: Some(TokenData {
                id_token: Default::default(),
                access_token: "Access Token".to_string(),
                refresh_token: "test".to_string(),
                account_id: Some("account_id".to_string()),
            }),
            last_refresh: Some(Utc::now()),
            agent_identity: None,
        };

        let client = create_client();
        let state = ChatgptAuthState {
            auth_dot_json: Arc::new(Mutex::new(Some(auth_dot_json))),
            client,
        };
        let dummy_auth_id = NEXT_DUMMY_AUTH_ID.fetch_add(1, Ordering::Relaxed);
        let storage = create_auth_storage(
            PathBuf::from(format!("dummy-chatgpt-auth-{dummy_auth_id}")),
            AuthCredentialsStoreMode::Ephemeral,
        );
        Self::Chatgpt(ChatgptAuth { state, storage })
    }

    pub fn from_api_key(api_key: &str) -> Self {
        Self::ApiKey(ApiKeyAuth {
            api_key: api_key.to_owned(),
        })
    }
}
