use axum::{
    extract::State,
    http::{Request, StatusCode, header::AUTHORIZATION},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub auth_token: Option<String>,
    pub no_auth: bool,
}

impl AuthConfig {
    pub fn disabled() -> Self {
        Self {
            auth_token: None,
            no_auth: true,
        }
    }

    pub fn token(auth_token: impl Into<String>) -> Self {
        Self {
            auth_token: Some(auth_token.into()),
            no_auth: false,
        }
    }

    fn should_bypass(&self) -> bool {
        self.no_auth
    }

    fn is_authorized(&self, request: &Request<axum::body::Body>) -> bool {
        if self.should_bypass() {
            return true;
        }

        let Some(expected_token) = self.auth_token.as_deref().filter(|token| !token.is_empty())
        else {
            return false;
        };

        request
            .headers()
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .is_some_and(|provided| {
                constant_time_eq(provided.as_bytes(), expected_token.as_bytes())
            })
    }
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }

    let mut diff = 0u8;
    for (left_byte, right_byte) in left.iter().zip(right.iter()) {
        diff |= left_byte ^ right_byte;
    }
    diff == 0
}

#[derive(Debug, Serialize)]
struct AuthErrorBody {
    error: String,
    code: String,
}

pub async fn require_bearer(
    State(config): State<AuthConfig>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    if config.is_authorized(&request) {
        return next.run(request).await;
    }

    let body = AuthErrorBody {
        error: "Unauthorized".to_string(),
        code: "unauthorized".to_string(),
    };

    (StatusCode::UNAUTHORIZED, axum::Json(body)).into_response()
}

#[cfg(test)]
mod tests {
    use super::{AUTHORIZATION, AuthConfig};
    use axum::body::Body;
    use axum::http::Request;

    fn request_with_auth(value: &str) -> Request<Body> {
        Request::builder()
            .header(AUTHORIZATION, value)
            .body(Body::empty())
            .expect("test request builds")
    }

    #[test]
    fn missing_token_denies_when_auth_is_not_disabled() {
        let config = AuthConfig {
            auth_token: None,
            no_auth: false,
        };
        let request = request_with_auth("Bearer secret");

        assert!(!config.is_authorized(&request));
    }

    #[test]
    fn empty_token_denies_when_auth_is_not_disabled() {
        let config = AuthConfig::token("");
        let request = request_with_auth("Bearer ");

        assert!(!config.is_authorized(&request));
    }

    #[test]
    fn explicit_disabled_auth_bypasses_bearer_check() {
        let config = AuthConfig::disabled();
        let request = Request::builder()
            .body(Body::empty())
            .expect("test request builds");

        assert!(config.is_authorized(&request));
    }

    #[test]
    fn bearer_token_must_match_expected_token() {
        let config = AuthConfig::token("secret");
        let valid_request = request_with_auth("Bearer secret");
        let invalid_request = request_with_auth("Bearer wrong");

        assert!(config.is_authorized(&valid_request));
        assert!(!config.is_authorized(&invalid_request));
    }
}
