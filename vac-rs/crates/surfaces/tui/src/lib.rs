#![cfg_attr(not(feature = "full-tui"), allow(dead_code))]

#[cfg(feature = "full-tui")]
include!("full_tui_runtime.rs");

#[cfg(not(feature = "full-tui"))]
mod cli;
#[cfg(not(feature = "full-tui"))]
mod token_usage;
#[cfg(not(feature = "full-tui"))]
mod update_action;

#[cfg(not(feature = "full-tui"))]
pub use cli::Cli;
#[cfg(not(feature = "full-tui"))]
pub use token_usage::TokenUsage;
#[cfg(not(feature = "full-tui"))]
pub use update_action::UpdateAction;

#[cfg(not(feature = "full-tui"))]
use vac_arg0::Arg0DispatchPaths;
#[cfg(not(feature = "full-tui"))]
use vac_config::LoaderOverrides;
#[cfg(not(feature = "full-tui"))]
use vac_protocol::ThreadId;

#[cfg(not(feature = "full-tui"))]
#[derive(Debug, Clone)]
pub struct AppExitInfo {
    pub token_usage: TokenUsage,
    pub thread_id: Option<ThreadId>,
    pub thread_name: Option<String>,
    pub update_action: Option<UpdateAction>,
    pub exit_reason: ExitReason,
}

#[cfg(not(feature = "full-tui"))]
impl AppExitInfo {
    pub fn fatal(message: impl Into<String>) -> Self {
        Self {
            token_usage: TokenUsage::default(),
            thread_id: None,
            thread_name: None,
            update_action: None,
            exit_reason: ExitReason::Fatal(message.into()),
        }
    }
}

#[cfg(not(feature = "full-tui"))]
#[derive(Debug, Clone)]
pub enum ExitReason {
    UserRequested,
    Fatal(String),
}

/// Default-off local surface entry point used by static/cargo validation.
///
/// The full historical TUI implementation is preserved behind the explicit
/// `full-tui` feature. The default path stays provider-neutral and avoids
/// compiling legacy cloud/realtime UI seams during local control-plane
/// validation, while the CLI still gets a typed, fail-closed interactive
/// surface contract instead of reaching into removed app-server paths.
/// EXPLICIT_AUTH_STATE_NO_SILENT_SKIP: the local facade keeps authentication
/// state explicit and fail-closed; it never silently falls back to cloud login.
#[cfg(not(feature = "full-tui"))]
pub const EXPLICIT_AUTH_STATE_NO_SILENT_SKIP: &str = "explicit_auth_state_no_silent_skip";

#[cfg(not(feature = "full-tui"))]
pub async fn run_main(
    _cli: Cli,
    _arg0_paths: Arg0DispatchPaths,
    _loader_overrides: LoaderOverrides,
) -> std::io::Result<AppExitInfo> {
    Ok(AppExitInfo::fatal(
        "interactive TUI requires the explicit full-tui feature in this zero-residual local build",
    ))
}
