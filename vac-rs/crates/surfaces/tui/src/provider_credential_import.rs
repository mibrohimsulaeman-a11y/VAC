#![cfg(test)]

use std::path::Path;

use crate::session_protocol::AuthMode;
use vac_config::types::AuthCredentialsStoreMode;
use vac_login::load_auth_dot_json;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LocalProviderCredentialAuth {
    pub(crate) access_token: String,
    pub(crate) provider_credential_account_id: String,
    pub(crate) provider_credential_plan_type: Option<String>,
}

pub(crate) fn load_provider_credential_import(
    vac_home: &Path,
    auth_credentials_store_mode: AuthCredentialsStoreMode,
    forced_provider_credential_workspace_id: Option<&str>,
) -> Result<LocalProviderCredentialAuth, String> {
    let auth = load_auth_dot_json(vac_home, auth_credentials_store_mode)
        .map_err(|err| format!("failed to load local auth: {err}"))?
        .ok_or_else(|| "no local auth available".to_string())?;
    if matches!(auth.auth_mode, Some(AuthMode::ApiKey)) || auth.vastar_api_key.is_some() {
        return Err("local auth is not a provider credential login".to_string());
    }

    let tokens = auth
        .tokens
        .ok_or_else(|| "local provider credential auth is missing token data".to_string())?;
    let access_token = tokens.access_token;
    let provider_credential_account_id = tokens
        .account_id
        .or_else(|| {
            tokens
                .id_token
                .provider_credential_account_id()
                .map(str::to_string)
        })
        .ok_or_else(|| {
            "local provider credential auth is missing provider_credential account id".to_string()
        })?;
    if let Some(expected_workspace) = forced_provider_credential_workspace_id
        && provider_credential_account_id != expected_workspace
    {
        return Err(format!(
            "local provider credential auth must use workspace {expected_workspace}, but found {provider_credential_account_id:?}"
        ));
    }

    let provider_credential_plan_type = tokens
        .id_token
        .get_provider_credential_plan_type_raw()
        .map(|plan_type| plan_type.to_ascii_lowercase());

    Ok(LocalProviderCredentialAuth {
        access_token,
        provider_credential_account_id,
        provider_credential_plan_type,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::session_protocol::AuthMode;
    use base64::Engine;
    use chrono::Utc;
    use pretty_assertions::assert_eq;
    use serde::Serialize;
    use serde_json::json;
    use tempfile::TempDir;
    use vac_login::AuthDotJson;
    use vac_login::auth::login_with_chatgpt_auth_tokens;
    use vac_login::save_auth;
    use vac_login::token_data::TokenData;

    fn fake_jwt(email: &str, account_id: &str, plan_type: &str) -> String {
        #[derive(Serialize)]
        struct Header {
            alg: &'static str,
            typ: &'static str,
        }

        let header = Header {
            alg: "none",
            typ: "JWT",
        };
        let payload = json!({
            "email": email,
            "https://api.vastar.com/auth": {
                "provider_credential_account_id": account_id,
                "provider_credential_plan_type": plan_type,
            },
        });
        let encode = |bytes: &[u8]| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);
        let header_b64 = encode(&serde_json::to_vec(&header).expect("serialize header"));
        let payload_b64 = encode(&serde_json::to_vec(&payload).expect("serialize payload"));
        let signature_b64 = encode(b"sig");
        format!("{header_b64}.{payload_b64}.{signature_b64}")
    }

    fn write_provider_credential_auth(vac_home: &Path, plan_type: &str) {
        let id_token = fake_jwt("user@example.com", "workspace-1", plan_type);
        let access_token = fake_jwt("user@example.com", "workspace-1", plan_type);
        let auth = AuthDotJson {
            auth_mode: Some(AuthMode::ProviderCredential),
            vastar_api_key: None,
            tokens: Some(TokenData {
                id_token: vac_login::token_data::parse_provider_credential_jwt_claims(&id_token)
                    .expect("id token should parse"),
                access_token,
                refresh_token: "refresh-token".to_string(),
                account_id: Some("workspace-1".to_string()),
            }),
            last_refresh: Some(Utc::now()),
            agent_identity: None,
        };
        save_auth(vac_home, &auth, AuthCredentialsStoreMode::File)
            .expect("provider_credential auth should save");
    }

    #[test]
    fn loads_provider_credential_import_from_managed_auth() {
        let vac_home = TempDir::new().expect("tempdir");
        write_provider_credential_auth(vac_home.path(), "business");

        let auth = load_provider_credential_import(
            vac_home.path(),
            AuthCredentialsStoreMode::File,
            Some("workspace-1"),
        )
        .expect("provider_credential auth should load");

        assert_eq!(auth.provider_credential_account_id, "workspace-1");
        assert_eq!(
            auth.provider_credential_plan_type.as_deref(),
            Some("business")
        );
        assert!(!auth.access_token.is_empty());
    }

    #[test]
    fn rejects_missing_local_auth() {
        let vac_home = TempDir::new().expect("tempdir");

        let err = load_provider_credential_import(
            vac_home.path(),
            AuthCredentialsStoreMode::File,
            /*forced_provider_credential_workspace_id*/ None,
        )
        .expect_err("missing auth should fail");

        assert_eq!(err, "no local auth available");
    }

    #[test]
    fn rejects_api_key_auth() {
        let vac_home = TempDir::new().expect("tempdir");
        save_auth(
            vac_home.path(),
            &AuthDotJson {
                auth_mode: Some(AuthMode::ApiKey),
                vastar_api_key: Some("sk-test".to_string()),
                tokens: None,
                last_refresh: None,
                agent_identity: None,
            },
            AuthCredentialsStoreMode::File,
        )
        .expect("api key auth should save");

        let err = load_provider_credential_import(
            vac_home.path(),
            AuthCredentialsStoreMode::File,
            /*forced_provider_credential_workspace_id*/ None,
        )
        .expect_err("api key auth should fail");

        assert_eq!(err, "local auth is not a provider credential login");
    }

    #[test]
    fn prefers_managed_auth_over_external_ephemeral_tokens() {
        let vac_home = TempDir::new().expect("tempdir");
        write_provider_credential_auth(vac_home.path(), "business");
        login_with_chatgpt_auth_tokens(
            vac_home.path(),
            &fake_jwt("user@example.com", "workspace-2", "enterprise"),
            "workspace-2",
            Some("enterprise"),
        )
        .expect("external auth should save");

        let auth = load_provider_credential_import(
            vac_home.path(),
            AuthCredentialsStoreMode::File,
            Some("workspace-1"),
        )
        .expect("managed auth should win");

        assert_eq!(auth.provider_credential_account_id, "workspace-1");
        assert_eq!(
            auth.provider_credential_plan_type.as_deref(),
            Some("business")
        );
    }

    #[test]
    fn preserves_usage_based_plan_type_wire_name() {
        let vac_home = TempDir::new().expect("tempdir");
        write_provider_credential_auth(vac_home.path(), "self_serve_business_usage_based");

        let auth = load_provider_credential_import(
            vac_home.path(),
            AuthCredentialsStoreMode::File,
            Some("workspace-1"),
        )
        .expect("provider_credential auth should load");

        assert_eq!(
            auth.provider_credential_plan_type.as_deref(),
            Some("self_serve_business_usage_based")
        );
    }
}
