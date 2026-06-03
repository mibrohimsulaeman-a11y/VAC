use anyhow::Context;
use anyhow::Result;
use base64::Engine;
use core_test_support::skip_if_no_network;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use std::ffi::OsString;
use tempfile::TempDir;
use vac_config::types::AuthCredentialsStoreMode;
use vac_login::AuthDotJson;
use vac_login::AuthManager;
use vac_login::CLIENT_ID;
use vac_login::REVOKE_TOKEN_URL_OVERRIDE_ENV_VAR;
use vac_login::logout_with_revoke;
use vac_login::save_auth;
use vac_login::token_data::IdTokenInfo;
use vac_login::token_data::TokenData;
use vac_runtime_protocol::AuthMode;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;
use wiremock::matchers::method;
use wiremock::matchers::path;

const ACCESS_TOKEN: &str = "access-token";
const REFRESH_TOKEN: &str = "refresh-token";

#[serial_test::serial(logout_revoke)]
#[tokio::test]
async fn logout_with_revoke_revokes_refresh_token_then_removes_auth() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/revoke"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "message": "success"
        })))
        .expect(1)
        .mount(&server)
        .await;
    let _env_guard = EnvGuard::set(
        REVOKE_TOKEN_URL_OVERRIDE_ENV_VAR,
        format!("{}/oauth/revoke", server.uri()),
    );

    let vac_home = TempDir::new()?;
    save_auth(
        vac_home.path(),
        &chatgpt_auth(),
        AuthCredentialsStoreMode::File,
    )?;

    let removed = logout_with_revoke(vac_home.path(), AuthCredentialsStoreMode::File).await?;

    assert!(removed);
    assert!(!vac_home.path().join("auth.json").exists());

    let requests = server
        .received_requests()
        .await
        .context("failed to fetch revoke requests")?;
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0]
            .body_json::<Value>()
            .context("revoke request should be JSON")?,
        json!({
            "token": REFRESH_TOKEN,
            "token_type_hint": "refresh_token",
            "client_id": CLIENT_ID,
        })
    );
    server.verify().await;
    Ok(())
}

#[serial_test::serial(logout_revoke)]
#[tokio::test]
async fn logout_with_revoke_removes_auth_when_revoke_fails() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/revoke"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({
            "error": {
                "message": "revoke failed"
            }
        })))
        .expect(1)
        .mount(&server)
        .await;
    let _env_guard = EnvGuard::set(
        REVOKE_TOKEN_URL_OVERRIDE_ENV_VAR,
        format!("{}/oauth/revoke", server.uri()),
    );

    let vac_home = TempDir::new()?;
    save_auth(
        vac_home.path(),
        &chatgpt_auth(),
        AuthCredentialsStoreMode::File,
    )?;

    let removed = logout_with_revoke(vac_home.path(), AuthCredentialsStoreMode::File).await?;

    assert!(removed);
    assert!(!vac_home.path().join("auth.json").exists());

    server.verify().await;
    Ok(())
}

#[serial_test::serial(logout_revoke)]
#[tokio::test]
async fn auth_manager_logout_with_revoke_uses_cached_auth() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/revoke"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "message": "success"
        })))
        .expect(1)
        .mount(&server)
        .await;
    let _env_guard = EnvGuard::set(
        REVOKE_TOKEN_URL_OVERRIDE_ENV_VAR,
        format!("{}/oauth/revoke", server.uri()),
    );

    let vac_home = TempDir::new()?;
    save_auth(
        vac_home.path(),
        &chatgpt_auth_with_refresh_token(REFRESH_TOKEN),
        AuthCredentialsStoreMode::File,
    )?;
    let manager = AuthManager::new(
        vac_home.path().to_path_buf(),
        /*enable_vac_api_key_env*/ false,
        AuthCredentialsStoreMode::File,
        /*chatgpt_base_url*/ None,
    )
    .await;
    save_auth(
        vac_home.path(),
        &chatgpt_auth_with_refresh_token("newer-disk-refresh-token"),
        AuthCredentialsStoreMode::File,
    )?;

    let removed = manager.logout_with_revoke().await?;

    assert!(removed);
    assert!(manager.auth_cached().is_none());
    assert!(!vac_home.path().join("auth.json").exists());

    let requests = server
        .received_requests()
        .await
        .context("failed to fetch revoke requests")?;
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0]
            .body_json::<Value>()
            .context("revoke request should be JSON")?,
        json!({
            "token": REFRESH_TOKEN,
            "token_type_hint": "refresh_token",
            "client_id": CLIENT_ID,
        })
    );
    server.verify().await;
    Ok(())
}

fn chatgpt_auth() -> AuthDotJson {
    chatgpt_auth_with_refresh_token(REFRESH_TOKEN)
}

fn chatgpt_auth_with_refresh_token(refresh_token: &str) -> AuthDotJson {
    AuthDotJson {
        auth_mode: Some(AuthMode::ProviderCredential),
        vastar_api_key: None,
        tokens: Some(TokenData {
            id_token: IdTokenInfo {
                raw_jwt: minimal_jwt(),
                ..Default::default()
            },
            access_token: ACCESS_TOKEN.to_string(),
            refresh_token: refresh_token.to_string(),
            account_id: Some("account-id".to_string()),
        }),
        last_refresh: None,
        agent_identity: None,
    }
}

fn minimal_jwt() -> String {
    let b64 = |b: &[u8]| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b);
    let header_b64 = b64(br#"{"alg":"none"}"#);
    let payload_b64 = b64(br#"{"sub":"user-123"}"#);
    let signature_b64 = b64(b"sig");
    format!("{header_b64}.{payload_b64}.{signature_b64}")
}

struct EnvGuard {
    key: &'static str,
    original: Option<OsString>,
}

impl EnvGuard {
    fn set(key: &'static str, value: String) -> Self {
        let original = std::env::var_os(key);
        // SAFETY: these tests execute serially, so updating the process environment is safe.
        unsafe {
            std::env::set_var(key, &value);
        }
        Self { key, original }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        // SAFETY: the guard restores the original environment value before other tests run.
        unsafe {
            match &self.original {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }
}
