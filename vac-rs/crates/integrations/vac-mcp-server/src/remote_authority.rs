use rmcp::model::{CallToolResult, Content};
use std::fmt::Display;
use vac_foundation::remote_connection::{PathLocation, RemoteConnectionInfo};

#[derive(Debug, Clone)]
pub(crate) struct RemotePathAuthority {
    pub(crate) connection: RemoteConnectionInfo,
    pub(crate) remote_path: String,
}

impl RemotePathAuthority {
    fn apply_credential_overrides(
        &mut self,
        password: Option<String>,
        private_key_path: Option<String>,
    ) {
        if let Some(password) = password {
            self.connection.password = Some(password);
        }
        if let Some(private_key_path) = private_key_path {
            self.connection.private_key_path = Some(private_key_path);
        }
    }
}

/// Validate and normalize a remote connection string.
///
/// Enforces the same structure expected by `RemoteConnectionInfo::parse_connection_string()`:
/// exactly one `@`, non-empty username, non-empty hostname, and optional port that parses as u16.
/// Returns the trimmed string on success, or a structured `CallToolResult` error.
pub(crate) fn validate_remote_connection(raw: &str) -> Result<String, CallToolResult> {
    let trimmed = raw.trim().to_string();

    let make_err = |detail: &str| invalid_remote_connection_error(&trimmed, detail);

    let (username, host_port) = trimmed
        .split_once('@')
        .ok_or_else(|| make_err("Missing '@'"))?;

    if username.is_empty() {
        return Err(make_err("Username is empty"));
    }

    // Reject multiple '@' — the host_port portion must not contain another '@'
    if host_port.contains('@') {
        return Err(make_err("Contains multiple '@' characters"));
    }

    let (hostname, port_str) = if let Some((h, p)) = host_port.split_once(':') {
        (h, Some(p))
    } else {
        (host_port, None)
    };

    if hostname.is_empty() {
        return Err(make_err("Hostname is empty"));
    }

    if let Some(port) = port_str
        && port.parse::<u16>().is_err()
    {
        return Err(make_err(&format!("Invalid port '{port}'")));
    }

    Ok(trimmed)
}

pub(crate) fn resolve_remote_path_authority(
    path: &str,
    password: Option<String>,
    private_key_path: Option<String>,
) -> Result<RemotePathAuthority, CallToolResult> {
    let mut authority = parse_remote_path_authority(path)?;
    authority.apply_credential_overrides(password, private_key_path);
    Ok(authority)
}

pub(crate) fn parse_remote_path_authority(
    path: &str,
) -> Result<RemotePathAuthority, CallToolResult> {
    let path_location = PathLocation::parse(path).map_err(invalid_path_error)?;

    match path_location {
        PathLocation::Remote { connection, path } => Ok(RemotePathAuthority {
            connection,
            remote_path: path,
        }),
        PathLocation::Local(_) => Err(not_remote_error()),
    }
}

#[must_use]
pub(crate) fn is_remote_path(path: &str) -> bool {
    PathLocation::parse(path)
        .map(|loc| loc.is_remote())
        .unwrap_or(false)
}

pub(crate) fn remote_connection_error(error: impl Display) -> CallToolResult {
    CallToolResult::error(vec![
        Content::text("REMOTE_CONNECTION_ERROR"),
        Content::text(format!("Failed to connect to remote host: {error}")),
    ])
}

fn invalid_remote_connection_error(trimmed: &str, detail: &str) -> CallToolResult {
    CallToolResult::error(vec![
        Content::text("INVALID_REMOTE_CONNECTION"),
        Content::text(format!(
            "Invalid remote connection string '{}'. {}. Expected format: user@host or user@host:port",
            trimmed, detail
        )),
    ])
}

fn invalid_path_error(error: impl Display) -> CallToolResult {
    CallToolResult::error(vec![
        Content::text("INVALID_PATH"),
        Content::text(format!("Failed to parse path: {error}")),
    ])
}

fn not_remote_error() -> CallToolResult {
    CallToolResult::error(vec![
        Content::text("NOT_REMOTE"),
        Content::text("This helper is for remote connections only"),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call_tool_error_text(error: CallToolResult) -> String {
        format!("{error:?}")
    }

    #[test]
    fn validate_remote_rejects_empty() {
        let err = validate_remote_connection("").unwrap_err();
        assert!(call_tool_error_text(err).contains("INVALID_REMOTE_CONNECTION"));
    }

    #[test]
    fn validate_remote_rejects_whitespace_only() {
        let err = validate_remote_connection("   ").unwrap_err();
        assert!(call_tool_error_text(err).contains("INVALID_REMOTE_CONNECTION"));
    }

    #[test]
    fn validate_remote_rejects_missing_at() {
        let err = validate_remote_connection("hostname").unwrap_err();
        assert!(call_tool_error_text(err).contains("INVALID_REMOTE_CONNECTION"));
    }

    #[test]
    fn validate_remote_accepts_user_at_host() {
        let result = validate_remote_connection("user@host");
        assert_eq!(result.unwrap(), "user@host");
    }

    #[test]
    fn validate_remote_accepts_user_at_host_port() {
        let result = validate_remote_connection("user@host:2222");
        assert_eq!(result.unwrap(), "user@host:2222");
    }

    #[test]
    fn validate_remote_trims_whitespace() {
        let result = validate_remote_connection("  user@host  ");
        assert_eq!(result.unwrap(), "user@host");
    }

    #[test]
    fn validate_remote_rejects_empty_username() {
        let err = validate_remote_connection("@host").unwrap_err();
        let text = call_tool_error_text(err);
        assert!(text.contains("INVALID_REMOTE_CONNECTION"));
        assert!(text.contains("Username is empty"));
    }

    #[test]
    fn validate_remote_rejects_empty_hostname() {
        let err = validate_remote_connection("user@").unwrap_err();
        let text = call_tool_error_text(err);
        assert!(text.contains("INVALID_REMOTE_CONNECTION"));
        assert!(text.contains("Hostname is empty"));
    }

    #[test]
    fn validate_remote_rejects_multiple_at() {
        let err = validate_remote_connection("user@@host").unwrap_err();
        let text = call_tool_error_text(err);
        assert!(text.contains("INVALID_REMOTE_CONNECTION"));
        assert!(text.contains("multiple '@'"));
    }

    #[test]
    fn validate_remote_rejects_invalid_port() {
        let err = validate_remote_connection("user@host:abc").unwrap_err();
        let text = call_tool_error_text(err);
        assert!(text.contains("INVALID_REMOTE_CONNECTION"));
        assert!(text.contains("Invalid port"));
    }

    #[test]
    fn validate_remote_rejects_port_out_of_range() {
        let err = validate_remote_connection("user@host:99999").unwrap_err();
        let text = call_tool_error_text(err);
        assert!(text.contains("INVALID_REMOTE_CONNECTION"));
        assert!(text.contains("Invalid port"));
    }

    #[test]
    fn validate_remote_accepts_port_22() {
        assert_eq!(
            validate_remote_connection("user@host:22").unwrap(),
            "user@host:22"
        );
    }

    #[test]
    fn scp_style_remote_absolute_path_is_recognized() {
        assert!(is_remote_path("user@host:/etc/config"));

        let authority = parse_remote_path_authority("user@host:/etc/config").unwrap();
        assert_eq!(authority.connection.connection_string, "user@host");
        assert_eq!(authority.remote_path, "/etc/config");
    }

    #[test]
    fn ssh_scheme_remote_path_is_recognized() {
        assert!(is_remote_path("ssh://user@host/var/www/app/config.php"));

        let authority =
            parse_remote_path_authority("ssh://user@host/var/www/app/config.php").unwrap();
        assert_eq!(authority.connection.connection_string, "user@host");
        assert_eq!(authority.remote_path, "/var/www/app/config.php");
    }

    #[test]
    fn local_path_returns_not_remote_from_remote_authority_parser() {
        assert!(!is_remote_path("/tmp/local.txt"));

        let err = parse_remote_path_authority("/tmp/local.txt").unwrap_err();
        let text = call_tool_error_text(err);
        assert!(text.contains("NOT_REMOTE"));
        assert!(text.contains("remote connections only"));
    }

    #[test]
    fn scp_style_relative_remote_path_is_not_remote() {
        assert!(!is_remote_path("user@host:relative/path.txt"));

        let err = parse_remote_path_authority("user@host:relative/path.txt").unwrap_err();
        assert!(call_tool_error_text(err).contains("NOT_REMOTE"));
    }

    #[test]
    fn resolve_remote_path_authority_applies_credential_overrides() {
        let authority = resolve_remote_path_authority(
            "user@host:/etc/config",
            Some("override-password".to_string()),
            Some("/tmp/test-key".to_string()),
        )
        .unwrap();

        assert_eq!(
            authority.connection.password.as_deref(),
            Some("override-password")
        );
        assert_eq!(
            authority.connection.private_key_path.as_deref(),
            Some("/tmp/test-key")
        );
    }
}
