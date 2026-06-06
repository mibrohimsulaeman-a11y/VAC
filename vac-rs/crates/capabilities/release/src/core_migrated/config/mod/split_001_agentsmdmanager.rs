use crate::agents_md::AgentsMdManager;
use crate::config::edit::ConfigEdit;
use crate::config::edit::ConfigEditsBuilder;
use crate::path_utils::normalize_for_native_workdir;
use crate::unified_exec::DEFAULT_MAX_BACKGROUND_TERMINAL_TIMEOUT_MS;
use crate::unified_exec::MIN_EMPTY_YIELD_TIME_MS;
use crate::windows_sandbox::WindowsSandboxLevelExt;
use crate::windows_sandbox::resolve_windows_sandbox_mode;
use crate::windows_sandbox::resolve_windows_sandbox_private_desktop;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::io::ErrorKind;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use vac_config::ConfigLayerSource;
use vac_config::ConfigLayerStack;
use vac_config::ConfigLayerStackOrdering;
use vac_config::ConfigRequirements;
use vac_config::ConfigRequirementsToml;
use vac_config::ConstrainedWithSource;
use vac_config::FeatureRequirementsToml;
use vac_config::LoaderOverrides;
use vac_config::McpServerIdentity;
use vac_config::McpServerRequirement;
use vac_config::PluginRequirementsToml;
use vac_config::ResidencyRequirement;
use vac_config::SandboxModeRequirement;
use vac_config::Sourced;
use vac_config::ThreadConfigLoader;
use vac_config::config_toml::ConfigLockfileToml;
use vac_config::config_toml::ConfigToml;
use vac_config::config_toml::DEFAULT_PROJECT_DOC_MAX_BYTES;
use vac_config::config_toml::ProjectConfig;
use vac_config::config_toml::ThreadStoreToml;
use vac_config::config_toml::validate_model_providers;
use vac_config::loader::load_config_layers_state;
use vac_config::loader::project_trust_key;
use vac_config::profile_toml::ConfigProfile;
use vac_config::sandbox_mode_requirement_for_permission_profile;
use vac_config::types::ApprovalsReviewer;
use vac_config::types::AuthCredentialsStoreMode;
use vac_config::types::DEFAULT_OTEL_ENVIRONMENT;
use vac_config::types::History;
use vac_config::types::McpServerConfig;
use vac_config::types::McpServerDisabledReason;
use vac_config::types::McpServerTransportConfig;
use vac_config::types::MemoriesConfig;
use vac_config::types::ModelAvailabilityNuxConfig;
use vac_config::types::Notice;
use vac_config::types::OAuthCredentialsStoreMode;
use vac_config::types::OtelConfig;
use vac_config::types::OtelConfigToml;
use vac_config::types::OtelExporterKind;
use vac_config::types::ToolSuggestConfig;
use vac_config::types::ToolSuggestDisabledTool;
use vac_config::types::ToolSuggestDiscoverable;
use vac_config::types::TuiKeymap;
use vac_config::types::TuiNotificationSettings;
use vac_config::types::UriBasedFileOpener;
use vac_config::types::WindowsSandboxModeToml;
use vac_core_plugins::PluginsConfigInput;
use vac_exec_server::ExecutorFileSystem;
use vac_exec_server::LOCAL_FS;
use vac_features::AppsMcpPathOverrideConfigToml;
use vac_features::Feature;
use vac_features::FeatureConfigSource;
use vac_features::FeatureOverrides;
use vac_features::FeatureToml;
use vac_features::Features;
use vac_features::FeaturesToml;
use vac_features::MultiAgentV2ConfigToml;
use vac_git_utils::resolve_root_git_project_for_trust;
use vac_login::AuthManagerConfig;
use vac_mcp::McpConfig;
use vac_memories_read::memory_root;
use vac_model_provider_info::LEGACY_OLLAMA_CHAT_PROVIDER_ID;
use vac_model_provider_info::ModelProviderInfo;
use vac_model_provider_info::OLLAMA_CHAT_PROVIDER_REMOVED_ERROR;
use vac_model_provider_info::built_in_model_providers;
use vac_model_provider_info::merge_configured_model_providers;
use vac_models_manager::ModelsManagerConfig;
use vac_protocol::config_types::AltScreenMode;
use vac_protocol::config_types::ForcedLoginMethod;
use vac_protocol::config_types::Personality;
use vac_protocol::config_types::ReasoningSummary;
use vac_protocol::config_types::SandboxMode;
use vac_protocol::config_types::ServiceTier;
use vac_protocol::config_types::ShellEnvironmentPolicy;
use vac_protocol::config_types::TrustLevel;
use vac_protocol::config_types::Verbosity;
use vac_protocol::config_types::WebSearchConfig;
use vac_protocol::config_types::WebSearchMode;
use vac_protocol::config_types::WindowsSandboxLevel;
use vac_protocol::models::ActivePermissionProfile;
use vac_protocol::models::ActivePermissionProfileModification;
use vac_protocol::models::PermissionProfile;
use vac_protocol::models::SandboxEnforcement;
use vac_protocol::permissions::FileSystemSandboxPolicy;
use vac_protocol::permissions::NetworkSandboxPolicy;
use vac_protocol::protocol::AskForApproval;
use vac_protocol::protocol::SandboxPolicy;
use vac_protocol::vastar_models::ModelsResponse;
use vac_protocol::vastar_models::ReasoningEffort;
use vac_utils_absolute_path::AbsolutePathBuf;
use vac_utils_absolute_path::AbsolutePathBufGuard;

use crate::config::permissions::BUILT_IN_WORKSPACE_PROFILE;
use crate::config::permissions::builtin_permission_profile;
use crate::config::permissions::compile_permission_profile_selection;
use crate::config::permissions::default_builtin_permission_profile_name;
use crate::config::permissions::get_readable_roots_required_for_vac_runtime;
use crate::config::permissions::network_proxy_config_for_profile_selection;
use crate::config::permissions::validate_user_permission_profile_names;
use crate::config_lock::config_without_lock_controls;
use crate::config_lock::lock_layer_from_config;
use crate::config_lock::read_config_lock_from_path;
use toml::Value as TomlValue;
use toml_edit::DocumentMut;
use vac_network_proxy::NetworkProxyConfig;

pub(crate) mod agent_roles;
pub mod edit;
mod managed_features;
mod network_proxy_spec;
mod permissions;
#[cfg(test)]
mod schema;
pub use managed_features::ManagedFeatures;
pub use network_proxy_spec::NetworkProxySpec;
pub use network_proxy_spec::StartedNetworkProxy;
pub(crate) use permissions::resolve_permission_profile;
pub use vac_config::Constrained;
pub use vac_config::ConstraintError;
pub use vac_config::ConstraintResult;
pub use vac_network_proxy::NetworkProxyAuditMetadata;
use vac_sandboxing::compatibility_sandbox_policy_for_permission_profile;
pub use vac_sandboxing::system_bwrap_warning;

const DEFAULT_IGNORE_LARGE_UNTRACKED_DIRS: i64 = 200;
const DEFAULT_IGNORE_LARGE_UNTRACKED_FILES: i64 = 10 * 1024 * 1024;

/// Compatibility-only config retained so legacy `ghost_snapshot` settings
/// continue to load even though snapshots are no longer produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GhostSnapshotConfig {
    pub ignore_large_untracked_files: Option<i64>,
    pub ignore_large_untracked_dirs: Option<i64>,
    pub disable_warnings: bool,
}

impl Default for GhostSnapshotConfig {
    fn default() -> Self {
        Self {
            ignore_large_untracked_files: Some(DEFAULT_IGNORE_LARGE_UNTRACKED_FILES),
            ignore_large_untracked_dirs: Some(DEFAULT_IGNORE_LARGE_UNTRACKED_DIRS),
            disable_warnings: false,
        }
    }
}

/// Maximum number of bytes of the documentation that will be embedded. Larger
/// files are *silently truncated* to this size so we do not take up too much of
/// the context window.
pub(crate) const AGENTS_MD_MAX_BYTES: usize = DEFAULT_PROJECT_DOC_MAX_BYTES; // 32 KiB
pub(crate) const DEFAULT_AGENT_MAX_THREADS: Option<usize> = Some(6);
pub(crate) const DEFAULT_MULTI_AGENT_V2_MAX_CONCURRENT_THREADS_PER_SESSION: usize = 4;
pub(crate) const DEFAULT_MULTI_AGENT_V2_MIN_WAIT_TIMEOUT_MS: i64 = 10_000;
pub(crate) const MAX_MULTI_AGENT_V2_WAIT_TIMEOUT_MS: i64 = 3600 * 1000;
pub(crate) const DEFAULT_AGENT_MAX_DEPTH: i32 = 1;
pub(crate) const DEFAULT_AGENT_JOB_MAX_RUNTIME_SECONDS: Option<u64> = None;
const LOCAL_DEV_BUILD_VERSION: &str = "0.0.0";

pub const CONFIG_TOML_FILE: &str = "config.toml";

fn resolve_sqlite_home_env(resolved_cwd: &Path) -> Option<PathBuf> {
    let raw = std::env::var(vac_state::SQLITE_HOME_ENV).ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let path = PathBuf::from(trimmed);
    if path.is_absolute() {
        Some(path)
    } else {
        Some(resolved_cwd.join(path))
    }
}

fn resolve_cli_auth_credentials_store_mode(
    configured: AuthCredentialsStoreMode,
    package_version: &str,
) -> AuthCredentialsStoreMode {
    match (package_version, configured) {
        (
            LOCAL_DEV_BUILD_VERSION,
            AuthCredentialsStoreMode::Keyring | AuthCredentialsStoreMode::Auto,
        ) => AuthCredentialsStoreMode::File,
        (_, mode) => mode,
    }
}

fn resolve_mcp_oauth_credentials_store_mode(
    configured: OAuthCredentialsStoreMode,
    package_version: &str,
) -> OAuthCredentialsStoreMode {
    match (package_version, configured) {
        (
            LOCAL_DEV_BUILD_VERSION,
            OAuthCredentialsStoreMode::Keyring | OAuthCredentialsStoreMode::Auto,
        ) => OAuthCredentialsStoreMode::File,
        (_, mode) => mode,
    }
}

#[cfg(test)]
pub(crate) async fn test_config() -> Config {
    let vac_home = tempfile::tempdir().expect("create temp dir");
    Config::load_from_base_config_with_overrides(
        ConfigToml::default(),
        ConfigOverrides::default(),
        AbsolutePathBuf::from_absolute_path(vac_home.path()).expect("temp dir should resolve"),
    )
    .await
    .expect("load default test config")
}

/// Application configuration loaded from disk and merged with overrides.
#[derive(Debug, Clone, PartialEq)]
pub struct Permissions {
    /// Approval policy for executing commands.
    pub approval_policy: Constrained<AskForApproval>,
    /// Canonical effective runtime permissions after config requirements and
    /// runtime readable-root additions have been applied.
    pub permission_profile: Constrained<PermissionProfile>,
    /// Named or implicit built-in profile selected by config, rather than an
    /// ad-hoc override.
    pub active_permission_profile: Option<ActivePermissionProfile>,
    /// Effective network configuration applied to all spawned processes.
    pub network: Option<NetworkProxySpec>,
    /// Whether the model may request a login shell for shell-based tools.
    /// Default to `true`
    ///
    /// If `true`, the model may request a login shell (`login = true`), and
    /// omitting `login` defaults to using a login shell.
    /// If `false`, the model can never use a login shell: `login = true`
    /// requests are rejected, and omitting `login` defaults to a non-login
    /// shell.
    pub allow_login_shell: bool,
    /// Policy used to build process environments for shell/unified exec.
    pub shell_environment_policy: ShellEnvironmentPolicy,
    /// Effective Windows sandbox mode derived from `[windows].sandbox` or
    /// legacy feature keys.
    pub windows_sandbox_mode: Option<WindowsSandboxModeToml>,
    /// Whether the final Windows sandboxed child should run on a private desktop.
    pub windows_sandbox_private_desktop: bool,
}

impl Permissions {
    /// Effective runtime permissions after config requirements and runtime
    /// readable-root additions have been applied.
    pub fn permission_profile(&self) -> PermissionProfile {
        self.permission_profile.get().clone()
    }

    /// Named profile selected by config, if the current profile has one.
    pub fn active_permission_profile(&self) -> Option<ActivePermissionProfile> {
        self.active_permission_profile.clone()
    }

    /// Effective filesystem sandbox policy derived from the canonical profile.
    pub fn file_system_sandbox_policy(&self) -> FileSystemSandboxPolicy {
        self.permission_profile.get().file_system_sandbox_policy()
    }

    /// Effective network sandbox policy derived from the canonical profile.
    pub fn network_sandbox_policy(&self) -> NetworkSandboxPolicy {
        self.permission_profile.get().network_sandbox_policy()
    }

    /// Legacy compatibility projection derived from the canonical profile.
    pub fn legacy_sandbox_policy(&self, cwd: &Path) -> SandboxPolicy {
        let permission_profile = self.permission_profile.get();
        let file_system_sandbox_policy = permission_profile.file_system_sandbox_policy();
        compatibility_sandbox_policy_for_permission_profile(
            permission_profile,
            &file_system_sandbox_policy,
            permission_profile.network_sandbox_policy(),
            cwd,
        )
    }

    /// Check whether a legacy sandbox policy can be applied to this permission
    /// set after projecting it into the canonical permission profile.
    pub fn can_set_legacy_sandbox_policy(
        &self,
        sandbox_policy: &SandboxPolicy,
        cwd: &Path,
    ) -> ConstraintResult<()> {
        let file_system_sandbox_policy =
            FileSystemSandboxPolicy::from_legacy_sandbox_policy_for_cwd(sandbox_policy, cwd);
        let network_sandbox_policy = NetworkSandboxPolicy::from(sandbox_policy);
        let permission_profile = PermissionProfile::from_runtime_permissions_with_enforcement(
            SandboxEnforcement::from_legacy_sandbox_policy(sandbox_policy),
            &file_system_sandbox_policy,
            network_sandbox_policy,
        );
        self.permission_profile.can_set(&permission_profile)
    }

    /// Replace permissions from a legacy sandbox policy and keep every
    /// permission projection in sync.
    pub fn set_legacy_sandbox_policy(
        &mut self,
        sandbox_policy: SandboxPolicy,
        cwd: &Path,
    ) -> ConstraintResult<()> {
        self.can_set_legacy_sandbox_policy(&sandbox_policy, cwd)?;
        let file_system_sandbox_policy =
            FileSystemSandboxPolicy::from_legacy_sandbox_policy_for_cwd(&sandbox_policy, cwd);
        let network_sandbox_policy = NetworkSandboxPolicy::from(&sandbox_policy);
        let permission_profile = PermissionProfile::from_runtime_permissions_with_enforcement(
            SandboxEnforcement::from_legacy_sandbox_policy(&sandbox_policy),
            &file_system_sandbox_policy,
            network_sandbox_policy,
        );

        self.permission_profile.set(permission_profile)?;
        self.active_permission_profile = None;
        Ok(())
    }

    /// Replace permissions from the canonical profile.
    pub fn set_permission_profile(
        &mut self,
        permission_profile: PermissionProfile,
    ) -> ConstraintResult<()> {
        self.set_permission_profile_with_active_profile(
            permission_profile,
            /*active_permission_profile*/ None,
        )
    }

    /// Replace permissions from the canonical profile and record the named
    /// source profile, if one is known.
    pub fn set_permission_profile_with_active_profile(
        &mut self,
        permission_profile: PermissionProfile,
        active_permission_profile: Option<ActivePermissionProfile>,
    ) -> ConstraintResult<()> {
        self.permission_profile.can_set(&permission_profile)?;

        self.permission_profile.set(permission_profile)?;
        self.active_permission_profile = active_permission_profile;
        Ok(())
    }
}

// A profile override only inherits the selected profile's proxy/allowlist config
// when VAC is still responsible for the network policy. `Disabled` means no
// outer sandbox, so starting the managed proxy would narrow the override.
fn profile_allows_configured_network_proxy(permission_profile: &PermissionProfile) -> bool {
    match permission_profile {
        PermissionProfile::Managed { network, .. } | PermissionProfile::External { network } => {
            network.is_enabled()
        }
        PermissionProfile::Disabled => false,
    }
}

/// Configured thread persistence backend.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ThreadStoreConfig {
    /// Persist threads locally using rollout JSONL files and sqlite metadata.
    #[default]
    Local,
    /// Persist threads through the remote thread-store service.
    Remote { endpoint: String },
    /// In-memory thread store for test and debug configurations.
    InMemory { id: String },
}

/// Application configuration loaded from disk and merged with overrides.
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    /// Provenance for how this [`Config`] was derived (merged layers + enforced
    /// requirements).
    pub config_layer_stack: ConfigLayerStack,

    /// Warnings collected during config load that should be shown on startup.
    pub startup_warnings: Vec<String>,

    /// Optional override of model selection.
    pub model: Option<String>,

    /// Effective service tier preference for new turns (`fast` or `flex`).
    pub service_tier: Option<ServiceTier>,

    /// Model used specifically for review sessions.
    pub review_model: Option<String>,

    /// Size of the context window for the model, in tokens.
    pub model_context_window: Option<i64>,

    /// Token usage threshold triggering auto-compaction of conversation history.
    pub model_auto_compact_token_limit: Option<i64>,

    /// Key into the model_providers map that specifies which provider to use.
    pub model_provider_id: String,

    /// Info needed to make an API request to the model.
    pub model_provider: ModelProviderInfo,

    /// Optionally specify the personality of the model
    pub personality: Option<Personality>,

    /// Effective permission configuration for shell tool execution.
    pub permissions: Permissions,

    /// Configures who approval requests are routed to for review once they have
    /// been escalated. This does not disable separate safety checks such as
    /// ARC.
    pub approvals_reviewer: ApprovalsReviewer,

    /// enforce_residency means web traffic cannot be routed outside of a
    /// particular geography. HTTP clients should direct their requests
    /// using backend-specific headers or URLs to enforce this.
    pub enforce_residency: Constrained<Option<ResidencyRequirement>>,

    /// When `true`, `AgentReasoning` events emitted by the backend will be
    /// suppressed from the frontend output. This can reduce visual noise when
    /// users are only interested in the final agent responses.
    pub hide_agent_reasoning: bool,

    /// When set to `true`, `AgentReasoningRawContentEvent` events will be shown in the UI/output.
    /// Defaults to `false`.
    pub show_raw_agent_reasoning: bool,

    /// User-provided instructions from AGENTS.md.
    pub user_instructions: Option<String>,

    /// Base instructions override.
    pub base_instructions: Option<String>,

    /// Developer instructions override injected as a separate message.
    pub developer_instructions: Option<String>,

    /// Guardian-specific policy config override from requirements.toml or config.toml.
    /// This is inserted into the fixed guardian prompt template under the
    /// `# Policy Configuration` section rather than replacing the whole
    /// guardian developer prompt.
    pub guardian_policy_config: Option<String>,

    /// Whether to inject the `<permissions instructions>` developer block.
    pub include_permissions_instructions: bool,

    /// Whether to inject the `<apps_instructions>` developer block.
    pub include_apps_instructions: bool,

    /// Whether to inject the `<skills_instructions>` developer block.
    pub include_skill_instructions: bool,

    /// Whether to inject the `<environment_context>` user block.
    pub include_environment_context: bool,

    /// Compact prompt override.
    pub compact_prompt: Option<String>,

    /// Optional commit attribution text for commit message co-author trailers.
    ///
    /// - `None`: use default attribution (`VAC <noreply@vastar.com>`)
    /// - `Some("")` or whitespace-only: disable commit attribution
    /// - `Some("...")`: use the provided attribution text verbatim
    pub commit_attribution: Option<String>,

    /// Deprecated optional external notifier command. When set, VAC will spawn this
    /// program after each completed *turn* (i.e. when the agent finishes
    /// processing a user submission). The value must be the full command
    /// broken into argv tokens **without** the trailing JSON argument - VAC
    /// appends one extra argument containing a JSON payload describing the
    /// event.
    ///
    /// Example `~/.vac/config.toml` snippet:
    ///
    /// ```toml
    /// notify = ["notify-send", "VAC"]
    /// ```
    ///
    /// which will be invoked as:
    ///
    /// ```shell
    /// notify-send VAC '{"type":"agent-turn-complete","turn-id":"12345"}'
    /// ```
    ///
    /// If unset the feature is disabled. Use lifecycle hooks for new automation.
    pub notify: Option<Vec<String>>,

    /// TUI notification settings, including enabled events, delivery method, and focus condition.
    pub tui_notifications: TuiNotificationSettings,

    /// Enable ASCII animations and shimmer effects in the TUI.
    pub animations: bool,

    /// Show startup tooltips in the TUI welcome screen.
    pub show_tooltips: bool,

    /// Persisted startup availability NUX state for model tooltips.
    pub model_availability_nux: ModelAvailabilityNuxConfig,

    /// Start the composer in Vim mode (`Normal`) by default.
    pub tui_vim_mode_default: bool,

    /// Start the TUI in the specified collaboration mode (plan/default).

    /// Controls whether the TUI uses the terminal's alternate screen buffer.
    ///
    /// This is the same `tui.alternate_screen` value from `config.toml`.
    /// - `auto` (default): Disable alternate screen in Zellij, enable elsewhere.
    /// - `always`: Always use alternate screen (original behavior).
    /// - `never`: Never use alternate screen (inline mode, preserves scrollback).
    pub tui_alternate_screen: AltScreenMode,
    /// Ordered list of status line item identifiers for the TUI.
    ///
    /// When unset, the TUI defaults to: `model-with-reasoning` and `current-dir`.
    pub tui_status_line: Option<Vec<String>>,

    /// Whether to color status line items with colors from the active syntax theme.
    pub tui_status_line_use_colors: bool,

    /// Ordered list of terminal title item identifiers for the TUI.
    ///
    /// When unset, the TUI defaults to: `activity` and `project`.
    /// The `activity` item spins while working and shows an action-required
    /// message when blocked on the user.
    pub tui_terminal_title: Option<Vec<String>>,

    /// Syntax highlighting theme override (kebab-case name).
    pub tui_theme: Option<String>,

    /// Terminal resize-reflow tuning knobs.
    pub terminal_resize_reflow: TerminalResizeReflowConfig,

    /// Keybinding overrides for the TUI.
    ///
    /// Precedence is:
    ///
    /// 1. context table (`tui.keymap.chat`, `tui.keymap.composer`, etc.)
    /// 2. `tui.keymap.global`
    /// 3. built-in defaults
    pub tui_keymap: TuiKeymap,

    /// The absolute directory that should be treated as the current working
    /// directory for the session. All relative paths inside the business-logic
    /// layer are resolved against this path.
    pub cwd: AbsolutePathBuf,

    /// Preferred store for CLI auth credentials.
    /// file (default): Use a file in the VAC home directory.
    /// keyring: Use an OS-specific keyring service.
    /// auto: Use the OS-specific keyring service if available, otherwise use a file.
    pub cli_auth_credentials_store_mode: AuthCredentialsStoreMode,

    /// Definition for MCP servers that VAC can reach out to for tool calls.
    pub mcp_servers: Constrained<HashMap<String, McpServerConfig>>,

    /// Preferred store for MCP OAuth credentials.
    /// keyring: Use an OS-specific keyring service.
    ///          Credentials stored in the keyring will only be readable by VAC unless the user explicitly grants access via OS-level keyring access.
    ///          https://github.com/vastar/vac/blob/main/vac-rs/rmcp-client/src/oauth.rs#L2
    /// file: VAC_HOME/.credentials.json
    ///       This file will be readable to VAC and other applications running as the same user.
    /// auto (default): keyring if available, otherwise file.
    pub mcp_oauth_credentials_store_mode: OAuthCredentialsStoreMode,

    /// Optional fixed port to use for the local HTTP callback server used during MCP OAuth login.
    ///
    /// When unset, VAC will bind to an ephemeral port chosen by the OS.
    pub mcp_oauth_callback_port: Option<u16>,

    /// Optional redirect URI to use during MCP OAuth login.
    ///
    /// When set, this URI is used in the OAuth authorization request instead
    /// of the local listener address. The local callback listener still binds
    /// to 127.0.0.1 (using `mcp_oauth_callback_port` when provided).
    pub mcp_oauth_callback_url: Option<String>,

    /// Combined provider map (defaults plus user-defined providers).
    pub model_providers: HashMap<String, ModelProviderInfo>,

    /// Maximum number of bytes to include from an AGENTS.md project doc file.
    pub project_doc_max_bytes: usize,

    /// Additional filenames to try when looking for project-level docs.
    pub project_doc_fallback_filenames: Vec<String>,

    /// Token budget applied when storing tool/function outputs in the context manager.
    pub tool_output_token_limit: Option<usize>,

    /// Maximum number of agent threads that can be open concurrently.
    pub agent_max_threads: Option<usize>,
    /// Maximum runtime in seconds for agent job workers before they are failed.
    pub agent_job_max_runtime_seconds: Option<u64>,

    /// Whether to record a model-visible message when an agent turn is interrupted.
    pub agent_interrupt_message_enabled: bool,

    /// Maximum nesting depth allowed for spawned agent threads.
    pub agent_max_depth: i32,

    /// User-defined role declarations keyed by role name.
    pub agent_roles: BTreeMap<String, AgentRoleConfig>,

    /// Memories subsystem settings.
    pub memories: MemoriesConfig,

    /// Directory containing all VAC state (defaults to `~/.vac` but can be
    /// overridden by the `VAC_HOME` environment variable).
    pub vac_home: AbsolutePathBuf,

    /// Directory where VAC stores the SQLite state DB.
    pub sqlite_home: PathBuf,

    /// Directory where VAC writes log files (defaults to `$VAC_HOME/log`).
    pub log_dir: PathBuf,

    /// Directory where VAC writes effective session config lock files.
    pub config_lock_export_dir: Option<AbsolutePathBuf>,

    /// Whether config lock replay ignores VAC version drift between the
    /// lock metadata and the regenerated lock.
    pub config_lock_allow_vac_version_mismatch: bool,

    /// Whether config lock creation saves values resolved from the model
    /// catalog/session configuration.
    pub config_lock_save_fields_resolved_from_model_catalog: bool,

    /// Effective config lock used for strict replay validation.
    pub config_lock_toml: Option<Arc<ConfigLockfileToml>>,

    /// Settings that govern if and what will be written to `~/.vac/history.jsonl`.
    pub history: History,

    /// When true, session is not persisted on disk. Default to `false`
    pub ephemeral: bool,

    /// Optional URI-based file opener. If set, citations to files in the model
    /// output will be hyperlinked using the specified URI scheme.
    pub file_opener: UriBasedFileOpener,

    /// Path to the current VAC executable. This cannot be set in the config
    /// file: it must be set in code via [`ConfigOverrides`].
    pub vac_self_exe: Option<PathBuf>,

    /// Path to the `vac-linux-sandbox` executable. This must be set if
    /// [`vac_sandboxing::SandboxType::LinuxSeccomp`] is used. Note that this
    /// cannot be set in the config file: it must be set in code via
    /// [`ConfigOverrides`].
    ///
    /// When this program is invoked, arg0 will be set to `vac-linux-sandbox`.
    pub vac_linux_sandbox_exe: Option<PathBuf>,

    /// Path to the `vac-execve-wrapper` executable used for shell
    /// escalation. This cannot be set in the config file: it must be set in
    /// code via [`ConfigOverrides`].
    pub main_execve_wrapper_exe: Option<PathBuf>,

    /// Optional absolute path to patched zsh used by zsh-exec-bridge-backed shell execution.
    pub zsh_path: Option<PathBuf>,

    /// Value to use for `reasoning.effort` when making a request using the
    /// Responses API.
    pub model_reasoning_effort: Option<ReasoningEffort>,
    /// Optional Plan-mode-specific reasoning effort override used by the TUI.
    ///
    /// When unset, Plan mode uses the built-in Plan preset default (currently
    /// `medium`). When explicitly set (including `none`), this overrides the
    /// Plan preset. The `none` value means "no reasoning" (not "inherit the
    /// global default").
    pub plan_mode_reasoning_effort: Option<ReasoningEffort>,

    /// Optional value to use for `reasoning.summary` when making a request
    /// using the Responses API. When unset, the model catalog default is used.
    pub model_reasoning_summary: Option<ReasoningSummary>,

    /// Optional override to force-enable reasoning summaries for the configured model.
    pub model_supports_reasoning_summaries: Option<bool>,

    /// Optional full model catalog loaded from `model_catalog_json`.
    /// When set, this replaces the bundled catalog for the current process.
    pub model_catalog: Option<ModelsResponse>,

    /// Optional verbosity control for GPT-5 models (Responses API `text.verbosity`).
    pub model_verbosity: Option<Verbosity>,

    /// Base URL for requests to ChatGPT (as opposed to the Vastar API).
    pub chatgpt_base_url: String,

    /// Optional path override for the built-in apps MCP server.
    pub apps_mcp_path_override: Option<String>,

    /// Experimental / do not use. When set, app-server fetches thread-scoped
    /// config from a remote service at this endpoint.
    pub experimental_thread_config_endpoint: Option<String>,

    /// Experimental / do not use. Selects the thread persistence backend.
    pub experimental_thread_store: ThreadStoreConfig,
    /// When set, restricts ChatGPT login to a specific workspace identifier.
    pub forced_chatgpt_workspace_id: Option<String>,

    /// When set, restricts the login mechanism users may use.
    pub forced_login_method: Option<ForcedLoginMethod>,

    /// Include the `apply_patch` tool for models that benefit from invoking
    /// file edits as a structured tool call. When unset, this falls back to the
    /// model info's default preference.
    pub include_apply_patch_tool: bool,

    /// Explicit or feature-derived web search mode.
    pub web_search_mode: Constrained<WebSearchMode>,

    /// Additional parameters for the web search tool when it is enabled.
    pub web_search_config: Option<WebSearchConfig>,

    /// If set to `true`, used only the experimental unified exec tool.
    pub use_experimental_unified_exec_tool: bool,

    /// Maximum poll window for background terminal output (`write_stdin`), in milliseconds.
    /// Default: `300000` (5 minutes).
    pub background_terminal_max_timeout: u64,

    /// Compatibility-only settings retained for legacy `ghost_snapshot`
    /// config loading.
    pub ghost_snapshot: GhostSnapshotConfig,

    /// Settings specific to the task-path-based multi-agent tool surface.
    pub multi_agent_v2: MultiAgentV2Config,

    /// Centralized feature flags; source of truth for feature gating.
    pub features: ManagedFeatures,

    /// When `true`, suppress warnings about unstable (under development) features.
    pub suppress_unstable_features_warning: bool,

    /// The active profile name used to derive this `Config` (if any).
    pub active_profile: Option<String>,

    /// The currently active project config, resolved by checking if cwd:
    /// is (1) part of a git repo, (2) a git worktree, or (3) just using the cwd
    pub active_project: ProjectConfig,

    /// Tracks whether the Windows onboarding screen has been acknowledged.
    pub windows_wsl_setup_acknowledged: bool,

    /// Collection of various notices we show the user
    pub notices: Notice,

    /// When `true`, checks for VAC updates on startup and surfaces update prompts.
    /// Set to `false` only if your VAC updates are centrally managed.
    /// Defaults to `true`.
    pub check_for_update_on_startup: bool,

    /// When true, disables burst-paste detection for typed input entirely.
    /// All characters are inserted as they are received, and no buffering
    /// or placeholder replacement will occur for fast keypress bursts.
    pub disable_paste_burst: bool,

    /// When `false`, disables analytics across VAC product surfaces in this machine.
    /// Voluntarily left as Optional because the default value might depend on the client.
    pub analytics_enabled: Option<bool>,

    /// When `false`, disables feedback collection across VAC product surfaces.
    /// Defaults to `true`.
    pub feedback_enabled: bool,

    /// Configured discoverable tools for tool suggestions.
    pub tool_suggest: ToolSuggestConfig,

    /// OTEL configuration (exporter type, endpoint, headers, etc.).
    pub otel: vac_config::types::OtelConfig,
}

