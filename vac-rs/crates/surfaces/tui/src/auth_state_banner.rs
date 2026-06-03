// User-visible auth/runtime state banner for local coding-agent mode.

use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AuthStateBanner {
    Authenticated,
    LocalMode,
    AuthUnavailable,
    ProviderDisabled,
}

impl AuthStateBanner {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Authenticated => "Authenticated",
            Self::LocalMode => "Local mode",
            Self::AuthUnavailable => "Auth unavailable",
            Self::ProviderDisabled => "Provider disabled",
        }
    }

    pub(crate) fn guidance(self) -> &'static str {
        match self {
            Self::Authenticated => "Provider authentication is active.",
            Self::LocalMode => "VAC is running in local provider-neutral mode.",
            Self::AuthUnavailable => "Auth UI could not attach to the app-server request handle; provider setup can continue from /status or config.",
            Self::ProviderDisabled => "The selected provider is disabled by feature/config; choose another provider or enable it explicitly.",
        }
    }

    pub(crate) fn status_section(self) -> String {
        format!("auth_state: {}\nauth_guidance: {}\n", self.label(), self.guidance())
    }
}

pub(crate) fn write_auth_state_runtime_event(root: &Path, state: AuthStateBanner, reason: &str) {
    let path = root.join(".vac/registry/runtime/auth-state.yaml");
    if let Some(parent) = path.parent() { let _ = fs::create_dir_all(parent); }
    let body = format!(
        "schema_version: 1\nkind: runtime.auth_state\nid: runtime.auth_state\nstate: {}\nreason: {}\nguidance: {}\n",
        state.label(),
        reason.replace('\n', " "),
        state.guidance().replace('\n', " "),
    );
    let _ = fs::write(path, body);
}
