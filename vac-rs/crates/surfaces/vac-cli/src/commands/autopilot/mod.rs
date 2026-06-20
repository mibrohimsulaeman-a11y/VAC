use std::collections::{BTreeSet, HashMap, HashSet};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use chrono::Utc;
use clap::{Args, Subcommand, ValueEnum};
use croner::Cron;
use serde::{Deserialize, Serialize};

use std::sync::Arc;

use vac_foundation::utils::normalize_optional_string;
use vac_remote_service::AgentProvider;

use crate::{
    config::{AppConfig, profile_resolver::resolve_profile_run_overrides},
    onboarding::{OnboardingMode, run_onboarding},
    utils::server_context::{load_remote_skills_context, startup_project_dir},
};

mod presenter;
mod probes;
mod service;

use self::probes::{
    AutopilotProbeContext, ProbeMode, RealProbeEnvironment, run_autopilot_probes,
    summarize as summarize_probe_results,
};
pub(crate) use self::service::{
    AUTOPILOT_SYSTEMD_SERVICE, Platform, autopilot_log_dir, autopilot_service_active,
    autopilot_service_installed, autopilot_service_path, detect_platform,
    install_autopilot_service, is_autopilot_running, start_autopilot_service,
    stop_autopilot_service, uninstall_autopilot_service,
};

const DEFAULT_SYSTEM_PROMPT: &str = include_str!("../../prompts/system_prompt.v1.md");

#[derive(Args, PartialEq, Debug, Clone)]
pub struct StartArgs {
    /// Bind address for embedded server runtime
    #[arg(long, default_value = "127.0.0.1:4096")]
    pub bind: String,

    /// Show generated auth token in stdout (local dev only)
    #[arg(long, default_value_t = false)]
    pub show_token: bool,

    /// Disable auth checks for protected routes (loopback-only local dev mode)
    #[arg(long, default_value_t = false)]
    pub no_auth: bool,

    /// Override default model for server runs (provider/model or model id)
    #[arg(long)]
    pub model: Option<String>,

    /// Auto-approve all tools (CI/headless only)
    #[arg(long, default_value_t = false)]
    pub auto_approve_all: bool,

    /// Run in foreground instead of delegating to OS service
    #[arg(long, default_value_t = false)]
    pub foreground: bool,

    /// Do not prompt; require env vars / pre-existing config for setup
    #[arg(long, default_value_t = false)]
    pub non_interactive: bool,

    /// Overwrite existing config (re-initialize from scratch)
    #[arg(long, default_value_t = false)]
    pub force: bool,
}

#[derive(Args, PartialEq, Debug, Clone)]
pub struct StopArgs {
    /// Also remove installed OS service definition
    #[arg(long, default_value_t = false)]
    pub uninstall: bool,
}

#[derive(Subcommand, PartialEq, Debug, Clone)]
pub enum AutopilotCommands {
    /// Start autopilot and install as system service (runs setup on first use)
    #[command(name = "up")]
    Up {
        #[command(flatten)]
        args: StartArgs,

        /// Internal flag used by service units to avoid recursive delegation
        #[arg(long, hide = true, default_value_t = false)]
        from_service: bool,
    },

    /// Stop autopilot and remove system service
    #[command(name = "down")]
    Down {
        #[command(flatten)]
        args: StopArgs,
    },

    /// Show health, uptime, schedule/channel metadata, and recent activity
    Status {
        /// Emit machine-readable JSON output
        #[arg(long, default_value_t = false)]
        json: bool,

        /// Include recent schedule runs (count)
        #[arg(long)]
        recent_runs: Option<u32>,
    },

    /// Stream autopilot logs
    Logs {
        /// Follow log output
        #[arg(short = 'f', long)]
        follow: bool,

        /// Number of lines to show initially
        #[arg(short = 'n', long)]
        lines: Option<u32>,

        /// Filter logs by component
        #[arg(short = 'c', long, value_parser = ["scheduler", "server", "gateway"])]
        component: Option<String>,
    },

    /// Restart autopilot (reload config)
    Restart,

    /// Manage scheduled tasks
    #[command(subcommand)]
    Schedule(AutopilotScheduleCommands),

    /// Manage messaging channels (Slack, Telegram, Discord)
    #[command(subcommand)]
    Channel(AutopilotChannelCommands),

    /// Run preflight checks for autopilot setup/runtime
    Doctor,
}

#[derive(Subcommand, PartialEq, Debug, Clone)]
pub enum AutopilotScheduleCommands {
    /// List all schedules
    List,

    /// Add a schedule
    Add {
        /// Schedule name
        name: String,

        /// Cron expression
        #[arg(long)]
        cron: String,

        /// Prompt to run on trigger
        #[arg(long)]
        prompt: String,

        /// Check script path
        #[arg(long)]
        check: Option<String>,

        /// When to trigger after check
        #[arg(long, default_value_t = ScheduleTriggerOn::Failure)]
        trigger_on: ScheduleTriggerOn,

        // /// Working directory for this schedule
        // #[arg(long)]
        // workdir: Option<String>,
        /// Max agent steps
        #[arg(long, default_value_t = 50)]
        max_steps: u32,

        /// Report results to this channel
        #[arg(long)]
        channel: Option<String>,

        /// Profile from config.toml used for this schedule's sessions
        #[arg(long)]
        profile: Option<String>,

        /// Require approval before acting
        #[arg(long, default_value_t = false)]
        pause_on_approval: bool,

        /// Run agent tool calls inside a sandboxed warden container
        #[arg(long, default_value_t = false)]
        sandbox: bool,

        /// Enable immediately
        #[arg(long, default_value_t = true)]
        enabled: bool,
    },

    /// Remove a schedule
    Remove { name: String },

    /// Enable a schedule
    Enable { name: String },

    /// Disable a schedule
    Disable { name: String },

    /// Show run history for a schedule
    History {
        /// Schedule name
        name: String,

        /// Number of rows to show
        #[arg(long, default_value_t = 20, value_parser = clap::value_parser!(u32).range(1..=1000))]
        limit: u32,
    },

    /// Manually trigger a schedule now
    Trigger {
        /// Schedule name
        name: String,

        /// Preview what would happen without actually triggering
        #[arg(long)]
        dry_run: bool,
    },

    /// Show details of a specific run
    Show {
        /// Run ID
        id: i64,
    },

    /// Clean up stale runs and optionally prune old history
    Clean {
        /// Also prune runs older than this many days
        #[arg(long)]
        older_than_days: Option<u32>,
    },
}

#[derive(Subcommand, PartialEq, Debug, Clone)]
pub enum AutopilotChannelCommands {
    /// List all channels
    List,

    /// Add a channel
    #[command(
        after_long_help = "HOW TO GET TOKENS:\n\n  Slack (requires both --bot-token and --app-token):\n\n    RECOMMENDED: Use the app manifest for quick setup:\n    1. Go to https://api.slack.com/apps → Create New App → From an app manifest\n    2. Paste the manifest from: local source path: vac-rs/crates/integrations/vac-messaging-gateway/src/channels/slack-manifest.yaml\n    3. Basic Information → App-Level Tokens → generate token with connections:write scope (xapp-...)\n    4. Install to Workspace → copy Bot User OAuth Token (xoxb-...)\n\n    Manual setup (if you already have an app):\n    1. Create app at https://api.slack.com/apps\n    2. Enable Socket Mode → generate app-level token (xapp-...) with connections:write scope\n    3. OAuth & Permissions → add Bot Token Scopes:\n       app_mentions:read, channels:history, channels:read, chat:write,\n       groups:history, groups:read, im:history, im:read,\n       mpim:history, mpim:read, reactions:read, reactions:write\n    4. Event Subscriptions → subscribe to bot events:\n       message.channels, message.groups, message.im, app_mention\n    5. Interactivity & Shortcuts → enable\n    6. Install to Workspace → copy Bot User OAuth Token (xoxb-...)\n\n  Telegram:\n    1. Message @BotFather on Telegram\n    2. Send /newbot → choose name and username (must end in 'bot')\n    3. Copy the bot token (format: 123456789:ABCdef...)\n\n  Discord:\n    1. Create app at https://discord.com/developers/applications\n    2. Bot tab → copy the bot token\n    3. OAuth2 → enable bot scope and required permissions\n\n  Optional default notification target:\n    --target sets [notifications].channel/chat_id for watch alerts\n    Example: --target \"#engineering\" (Slack)\n"
    )]
    Add {
        /// Channel type (slack, telegram, discord)
        #[arg(value_enum)]
        channel_type: ChannelType,

        /// Bot token (Telegram bot token, Discord bot token)
        #[arg(long)]
        token: Option<String>,

        /// Slack bot token (xoxb-...)
        #[arg(long)]
        bot_token: Option<String>,

        /// Slack app token (xapp-...)
        #[arg(long)]
        app_token: Option<String>,

        /// Default notification target (Slack channel, Telegram chat_id, Discord channel_id)
        #[arg(long)]
        target: Option<String>,

        /// Profile from config.toml used for sessions started from this channel
        #[arg(long)]
        profile: Option<String>,
    },

    /// Remove a channel
    Remove {
        /// Channel type (slack, telegram, discord)
        #[arg(value_enum)]
        channel_type: ChannelType,
    },

    /// Test channel connectivity
    Test,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum, Default)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleTriggerOn {
    Success,
    #[default]
    Failure,
    Any,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum ChannelType {
    Slack,
    Telegram,
    Discord,
    Whatsapp,
    Webhook,
}

impl AutopilotCommands {
    pub async fn run(self, mut config: AppConfig) -> Result<(), String> {
        match self {
            AutopilotCommands::Up { args, from_service } => {
                start_autopilot(
                    &mut config,
                    StartOptions {
                        bind: args.bind,
                        show_token: args.show_token,
                        no_auth: args.no_auth,
                        model: args.model,
                        auto_approve_all: args.auto_approve_all,
                        foreground: args.foreground,
                        from_service,
                        non_interactive: args.non_interactive,
                        force: args.force,
                        sandbox_mode: vac_broker::SandboxMode::default(),
                    },
                )
                .await
            }
            AutopilotCommands::Down { args: _ } => stop_autopilot().await,
            AutopilotCommands::Status { json, recent_runs } => {
                status_autopilot(&config, json, recent_runs).await
            }
            AutopilotCommands::Logs {
                follow,
                lines,
                component,
            } => logs_autopilot(follow, lines, component).await,
            AutopilotCommands::Restart => restart_autopilot().await,
            AutopilotCommands::Schedule(command) => run_schedule_command(command, &config).await,
            AutopilotCommands::Channel(command) => run_channel_command(command, &config).await,
            AutopilotCommands::Doctor => doctor_autopilot(&config).await,
        }
    }
}

#[derive(Debug, Clone)]
struct StartOptions {
    bind: String,
    show_token: bool,
    no_auth: bool,
    model: Option<String>,
    auto_approve_all: bool,
    foreground: bool,
    from_service: bool,
    non_interactive: bool,
    force: bool,
    sandbox_mode: vac_broker::SandboxMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AutopilotConfigFile {
    #[serde(default)]
    server: AutopilotServerConfig,
    #[serde(default)]
    schedules: Vec<AutopilotScheduleConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AutopilotServerConfig {
    #[serde(default = "default_server_listen")]
    listen: String,
    #[serde(default)]
    show_token: bool,
    #[serde(default)]
    no_auth: bool,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    auto_approve_all: bool,
    /// Controls sandbox container lifecycle: "persistent" (default) spawns once
    /// at startup and reuses it; "ephemeral" spawns a new container per session.
    #[serde(default)]
    sandbox_mode: vac_broker::SandboxMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AutopilotScheduleConfig {
    name: String,
    cron: String,
    prompt: String,
    #[serde(default)]
    check: Option<String>,
    #[serde(default)]
    trigger_on: ScheduleTriggerOn,
    // #[serde(default)]
    // workdir: Option<String>,
    #[serde(default = "default_schedule_max_steps")]
    max_steps: u32,
    #[serde(default)]
    channel: Option<String>,
    #[serde(default)]
    profile: Option<String>,
    #[serde(default)]
    pause_on_approval: bool,
    #[serde(default)]
    sandbox: bool,
    #[serde(default = "default_enabled")]
    enabled: bool,
}

impl Default for AutopilotServerConfig {
    fn default() -> Self {
        Self {
            listen: default_server_listen(),
            show_token: false,
            no_auth: false,
            model: None,
            auto_approve_all: false,
            sandbox_mode: vac_broker::SandboxMode::default(),
        }
    }
}

impl AutopilotConfigFile {
    fn path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".vac")
            .join("autopilot.toml")
    }

    fn load_or_default() -> Result<Self, String> {
        let path = Self::path();
        if !path.exists() {
            return Ok(Self::default());
        }

        Self::load_from_path(&path)
    }

    async fn load_or_default_async() -> Result<Self, String> {
        tokio::task::spawn_blocking(Self::load_or_default)
            .await
            .map_err(|e| format!("Failed to join config load task: {}", e))?
    }

    fn load_from_path(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read autopilot config {}: {}", path.display(), e))?;

        let config: Self = toml::from_str(&content)
            .map_err(|e| format!("Failed to parse autopilot config {}: {}", path.display(), e))?;

        Ok(config)
    }

    fn save(&self) -> Result<PathBuf, String> {
        let path = Self::path();
        self.save_to_path(&path)?;
        Ok(path)
    }

    fn save_to_path(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create autopilot config dir: {}", e))?;
        }

        let mut root = load_toml_root_table(path)?;

        {
            let server = ensure_toml_table(&mut root, "server");
            server.insert(
                "listen".to_string(),
                toml::Value::String(self.server.listen.clone()),
            );
            server.insert(
                "show_token".to_string(),
                toml::Value::Boolean(self.server.show_token),
            );
            server.insert(
                "no_auth".to_string(),
                toml::Value::Boolean(self.server.no_auth),
            );
            match &self.server.model {
                Some(model) => {
                    server.insert("model".to_string(), toml::Value::String(model.clone()));
                }
                None => {
                    server.remove("model");
                }
            }
            server.insert(
                "auto_approve_all".to_string(),
                toml::Value::Boolean(self.server.auto_approve_all),
            );
        }

        root.insert(
            "schedules".to_string(),
            toml::Value::try_from(&self.schedules)
                .map_err(|e| format!("Failed to serialize schedules: {}", e))?,
        );

        write_toml_root_table(path, root)
    }

    fn find_schedule(&self, name: &str) -> Option<&AutopilotScheduleConfig> {
        self.schedules.iter().find(|schedule| schedule.name == name)
    }

    fn find_schedule_mut(&mut self, name: &str) -> Option<&mut AutopilotScheduleConfig> {
        self.schedules
            .iter_mut()
            .find(|schedule| schedule.name == name)
    }
}

impl std::fmt::Display for ScheduleTriggerOn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScheduleTriggerOn::Success => write!(f, "success"),
            ScheduleTriggerOn::Failure => write!(f, "failure"),
            ScheduleTriggerOn::Any => write!(f, "any"),
        }
    }
}

impl std::fmt::Display for ChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChannelType::Slack => write!(f, "slack"),
            ChannelType::Telegram => write!(f, "telegram"),
            ChannelType::Discord => write!(f, "discord"),
            ChannelType::Whatsapp => write!(f, "whatsapp"),
            ChannelType::Webhook => write!(f, "webhook"),
        }
    }
}

fn default_server_listen() -> String {
    "127.0.0.1:4096".to_string()
}

fn default_enabled() -> bool {
    true
}

fn default_schedule_max_steps() -> u32 {
    50
}

fn load_toml_root_table(path: &Path) -> Result<toml::value::Table, String> {
    if !path.exists() {
        return Ok(toml::value::Table::new());
    }

    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read autopilot config {}: {}", path.display(), e))?;

    let value: toml::Value = toml::from_str(&content)
        .map_err(|e| format!("Failed to parse autopilot config {}: {}", path.display(), e))?;

    match value {
        toml::Value::Table(table) => Ok(table),
        _ => Err(format!(
            "Failed to parse autopilot config {}: top-level TOML value must be a table",
            path.display()
        )),
    }
}

fn ensure_toml_table<'a>(
    table: &'a mut toml::value::Table,
    key: &str,
) -> &'a mut toml::value::Table {
    if !matches!(table.get(key), Some(toml::Value::Table(_))) {
        table.insert(
            key.to_string(),
            toml::Value::Table(toml::value::Table::new()),
        );
    }

    match table.get_mut(key) {
        Some(toml::Value::Table(subtable)) => subtable,
        _ => unreachable!("table key was just initialized"),
    }
}

fn write_toml_root_table(path: &Path, root: toml::value::Table) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create autopilot config dir: {}", e))?;
    }

    let content = toml::to_string_pretty(&toml::Value::Table(root))
        .map_err(|e| format!("Failed to serialize autopilot config: {}", e))?;

    std::fs::write(path, content)
        .map_err(|e| format!("Failed to write autopilot config {}: {}", path.display(), e))
}

impl StartOptions {
    fn with_server_config(mut self, server: &AutopilotServerConfig) -> Self {
        self.bind = server.listen.clone();
        self.show_token = server.show_token;
        self.no_auth = server.no_auth;
        self.model = server.model.clone();
        self.auto_approve_all = server.auto_approve_all;
        self.sandbox_mode = server.sandbox_mode.clone();
        self
    }

    fn has_runtime_overrides(&self) -> bool {
        self.bind != "127.0.0.1:4096"
            || self.show_token
            || self.no_auth
            || self.model.is_some()
            || self.auto_approve_all
    }
}

impl AutopilotServerConfig {
    fn from_start_options(options: &StartOptions) -> Self {
        // Load existing config to preserve fields that are only set via config
        // file (not CLI flags). CLI flags at their default value should NOT
        // overwrite explicitly-set config values.
        let existing = AutopilotConfigFile::load_or_default()
            .map(|c| c.server)
            .unwrap_or_default();
        Self {
            listen: options.bind.clone(),
            show_token: options.show_token,
            no_auth: options.no_auth,
            model: options.model.clone(),
            // Preserve auto_approve_all from config when CLI flag is at default (false).
            // Only override if the user explicitly passed --auto-approve-all.
            auto_approve_all: options.auto_approve_all || existing.auto_approve_all,
            sandbox_mode: existing.sandbox_mode,
        }
    }
}

#[derive(Debug, Serialize)]
struct AutopilotStatusJson {
    command: &'static str,
    ok: bool,
    profile: String,
    config_path: String,
    server_config: AutopilotServerConfig,
    server_allowed_tool_count: usize,
    service: ServiceStatusJson,
    server: EndpointStatusJson,
    gateway: EndpointStatusJson,
    sandbox: SandboxStatusJson,
    scheduler: SchedulerStatusJson,
    schedules: Vec<AutopilotScheduleStatusJson>,
    channels: Vec<AutopilotChannelStatusJson>,
}

#[derive(Debug, Serialize)]
struct ServiceStatusJson {
    installed: bool,
    active: bool,
    path: String,
}

#[derive(Debug, Serialize)]
struct EndpointStatusJson {
    expected_enabled: bool,
    reachable: bool,
    url: String,
}

#[derive(Debug, Serialize)]
struct SandboxStatusJson {
    mode: String,
    healthy: Option<bool>,
    consecutive_ok: Option<u64>,
    consecutive_failures: Option<u64>,
    last_ok: Option<String>,
    last_error: Option<String>,
}

#[derive(Debug, Serialize)]
struct SchedulerStatusJson {
    expected_enabled: bool,
    config_path: String,
    config_valid: bool,
    trigger_count: usize,
    running: bool,
    pid: Option<i64>,
    stale_pid: bool,
    db_path: Option<String>,
    error: Option<String>,
    recent_runs: Vec<ScheduleRunSummaryJson>,
}

#[derive(Debug, Serialize)]
struct ScheduleRunSummaryJson {
    id: i64,
    schedule_name: String,
    status: String,
    started_at: String,
    finished_at: Option<String>,
    error_message: Option<String>,
}

#[derive(Debug, Serialize)]
struct AutopilotScheduleStatusJson {
    name: String,
    cron: String,
    enabled: bool,
    sandbox: bool,
    next_run: Option<String>,
}

#[derive(Debug, Serialize)]
struct AutopilotChannelStatusJson {
    name: String,
    channel_type: String,
    target: String,
    enabled: bool,
    alerts_only: bool,
}

#[cfg(target_os = "linux")]
fn detect_host_user_mapping() -> vac_broker::SandboxUserMapping {
    let uid = read_unix_id_value("-u");
    let gid = read_unix_id_value("-g");

    match (uid, gid) {
        // Refuse to map root into the sandbox — it would weaken the
        // isolation boundary.  Fall back to the image's built-in user.
        (Some(0), _) | (_, Some(0)) => vac_broker::SandboxUserMapping::ImageDefault,
        (Some(uid), Some(gid)) => vac_broker::SandboxUserMapping::HostUser { uid, gid },
        _ => vac_broker::SandboxUserMapping::ImageDefault,
    }
}

#[cfg(not(target_os = "linux"))]
fn detect_host_user_mapping() -> vac_broker::SandboxUserMapping {
    // On macOS, Docker Desktop handles file ownership transparently via its VM
    // layer, so user mapping is not needed and would cause permission errors
    // inside the container.
    vac_broker::SandboxUserMapping::ImageDefault
}

fn sandbox_user_mapping_for_mode(
    _sandbox_mode: &vac_broker::SandboxMode,
) -> vac_broker::SandboxUserMapping {
    // On Linux, always map to the host user so bind-mounted files (local.db,
    // cloud CLI caches, etc.) are writable.  The container entrypoint script
    // patches /etc/passwd when the runtime UID differs from the image UID
    // (1000), preserving a valid user identity, home directory, and group
    // memberships.
    //
    // On macOS, Docker Desktop handles file ownership transparently via its
    // VM layer, so no mapping is needed for either sandbox mode.
    detect_host_user_mapping()
}

#[cfg(target_os = "linux")]
fn read_unix_id_value(flag: &str) -> Option<u32> {
    let output = std::process::Command::new("id").arg(flag).output().ok()?;

    if !output.status.success() {
        return None;
    }

    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<u32>()
        .ok()
}

mod runtime_start;
use runtime_start::start_autopilot;

fn resolve_server_tool_policy(
    allowed_tools: Option<&Vec<String>>,
    auto_approve_tools: Option<&Vec<String>>,
    auto_approve_all: bool,
) -> vac_broker::ToolApprovalPolicy {
    if auto_approve_all {
        return vac_broker::ToolApprovalPolicy::All;
    }

    let mut resolved_allowed_tools: Vec<String> = match allowed_tools {
        Some(tools) if tools.is_empty() => {
            return vac_broker::ToolApprovalPolicy::All;
        }
        Some(tools) => tools.clone(),
        None => vac_broker::SAFE_AUTOPILOT_TOOLS
            .iter()
            .map(|name| (*name).to_string())
            .collect(),
    };

    resolved_allowed_tools.retain(|tool| !tool.trim().is_empty());

    let mut policy = vac_broker::ToolApprovalPolicy::Custom {
        rules: std::collections::HashMap::new(),
        default: vac_broker::ToolApprovalAction::Ask,
    }
    .with_overrides(resolved_allowed_tools.into_iter().filter_map(|tool| {
        let trimmed = tool.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some((
                vac_broker::strip_tool_prefix(&trimmed).to_string(),
                vac_broker::ToolApprovalAction::Approve,
            ))
        }
    }));

    if let Some(overrides) = auto_approve_tools {
        policy = policy.with_overrides(overrides.iter().filter_map(|tool| {
            let trimmed = tool.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some((
                    vac_broker::strip_tool_prefix(trimmed).to_string(),
                    vac_broker::ToolApprovalAction::Approve,
                ))
            }
        }));
    }

    policy
}

#[derive(Debug, Clone)]
struct ProfileRunOverrideResolver {
    config_path: String,
}

impl ProfileRunOverrideResolver {
    fn new(config_path: String) -> Self {
        Self { config_path }
    }
}

impl vac_messaging_gateway::dispatcher::RunOverrideResolver for ProfileRunOverrideResolver {
    fn resolve_run_overrides(
        &self,
        profile_name: &str,
    ) -> Option<vac_messaging_gateway::client::RunOverrides> {
        let resolved =
            resolve_profile_run_overrides(profile_name, Some(self.config_path.as_str()))?;

        let overrides = vac_messaging_gateway::client::RunOverrides {
            model: resolved.model,
            auto_approve: resolved
                .auto_approve
                .map(vac_messaging_gateway::client::AutoApproveOverride::AllowList),
            system_prompt: resolved.system_prompt,
            max_turns: resolved.max_turns,
        };

        if overrides.is_empty() {
            None
        } else {
            Some(overrides)
        }
    }
}

fn gateway_channel_profiles_with_default(
    channels: &vac_messaging_gateway::config::ChannelConfigs,
    default_profile: &str,
) -> HashMap<String, String> {
    let mut profiles = channels.profiles_map();
    let default_profile = normalize_optional_string(Some(default_profile.to_string()))
        .unwrap_or_else(|| "default".to_string());

    if channels.telegram.is_some() {
        profiles
            .entry("telegram".to_string())
            .or_insert_with(|| default_profile.clone());
    }

    if channels.discord.is_some() {
        profiles
            .entry("discord".to_string())
            .or_insert_with(|| default_profile.clone());
    }

    if channels.slack.is_some() {
        profiles
            .entry("slack".to_string())
            .or_insert_with(|| default_profile.clone());
    }

    profiles
}

fn approved_tools_from_policy(policy: &vac_broker::ToolApprovalPolicy) -> Vec<String> {
    match policy {
        vac_broker::ToolApprovalPolicy::Custom { rules, .. } => rules
            .iter()
            .filter_map(|(name, action)| {
                if *action == vac_broker::ToolApprovalAction::Approve {
                    let trimmed = name.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                } else {
                    None
                }
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        _ => Vec::new(),
    }
}

fn mcp_allowed_tools_from_policy(
    policy: &vac_broker::ToolApprovalPolicy,
    configured_allowed_tools: Option<&Vec<String>>,
) -> Option<Vec<String>> {
    match policy {
        vac_broker::ToolApprovalPolicy::Custom { default, .. } => {
            if *default == vac_broker::ToolApprovalAction::Ask {
                // Ask-by-default policies require full tool visibility so gateway/user
                // can approve unlisted tools at runtime.
                configured_allowed_tools.cloned()
            } else {
                let approved = approved_tools_from_policy(policy);
                Some(expand_mcp_allowed_tools(&approved))
            }
        }
        vac_broker::ToolApprovalPolicy::All | vac_broker::ToolApprovalPolicy::None => {
            configured_allowed_tools.cloned()
        }
    }
}

fn expand_mcp_allowed_tools(tools: &[String]) -> Vec<String> {
    let mut normalized = BTreeSet::new();

    for tool in tools {
        let trimmed = tool.trim();
        if trimmed.is_empty() {
            continue;
        }

        normalized.insert(trimmed.to_string());
        if !trimmed.starts_with("vac__") {
            normalized.insert(format!("vac__{trimmed}"));
        }
    }

    normalized.into_iter().collect()
}

fn apply_gateway_policy_from_resolved_tools(
    gateway_cfg: &mut vac_messaging_gateway::GatewayConfig,
    policy: &vac_broker::ToolApprovalPolicy,
) {
    match policy {
        vac_broker::ToolApprovalPolicy::All => {
            gateway_cfg.gateway.approval_mode = vac_messaging_gateway::ApprovalMode::AllowAll;
            gateway_cfg.gateway.approval_allowlist.clear();
        }
        vac_broker::ToolApprovalPolicy::Custom { .. } => {
            gateway_cfg.gateway.approval_mode = vac_messaging_gateway::ApprovalMode::Allowlist;
            gateway_cfg.gateway.approval_allowlist =
                expand_gateway_approval_allowlist(&approved_tools_from_policy(policy));
        }
        vac_broker::ToolApprovalPolicy::None => {}
    }
}

fn describe_tool_policy(policy: &vac_broker::ToolApprovalPolicy) -> String {
    match policy {
        vac_broker::ToolApprovalPolicy::All => "all tools (auto-approve all)".to_string(),
        vac_broker::ToolApprovalPolicy::Custom { .. } => {
            let count = approved_tools_from_policy(policy).len();
            format!("{count} tools allowed")
        }
        vac_broker::ToolApprovalPolicy::None => "no tools approved".to_string(),
    }
}

fn expand_gateway_approval_allowlist(tools: &[String]) -> Vec<String> {
    let mut normalized = std::collections::BTreeSet::new();
    for tool in tools {
        let trimmed = tool.trim();
        if trimmed.is_empty() {
            continue;
        }
        normalized.insert(trimmed.to_string());
        if !trimmed.starts_with("vac__") {
            normalized.insert(format!("vac__{trimmed}"));
        }
    }
    normalized.into_iter().collect()
}

async fn stop_autopilot() -> Result<(), String> {
    let mut stopped = false;

    // First, gracefully stop the process via PID file. This triggers the
    // shutdown handler which tears down warden + Docker containers cleanly.
    // We do this BEFORE uninstalling the service to avoid launchctl/systemd
    // force-killing the process tree before cleanup finishes.
    if let Some(pid) = is_autopilot_running() {
        #[cfg(unix)]
        {
            // Send SIGTERM to the autopilot process only (not the process group).
            // The autopilot's graceful shutdown handler will SIGTERM warden, which
            // in turn cleans up its Docker containers before exiting. Killing the
            // whole process group would race warden's cleanup.
            let _ = std::process::Command::new("kill")
                .arg("-TERM")
                .arg(pid.to_string())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
        }
        #[cfg(windows)]
        {
            let _ = std::process::Command::new("taskkill")
                .args(["/PID", &pid.to_string()])
                .status();
        }

        // Wait for the process to exit — give enough time for the graceful
        // shutdown handler to tear down warden + Docker containers (~10s).
        for _ in 0..50 {
            if !crate::commands::watch::is_process_running(pid) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }

        // If still running after SIGTERM, force kill the process group
        if crate::commands::watch::is_process_running(pid) {
            #[cfg(unix)]
            {
                let _ = std::process::Command::new("kill")
                    .arg("-9")
                    .arg(format!("-{}", pid))
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status();
                let _ = std::process::Command::new("kill")
                    .arg("-9")
                    .arg(pid.to_string())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status();
            }
            #[cfg(windows)]
            {
                let _ = std::process::Command::new("taskkill")
                    .args(["/PID", &pid.to_string(), "/T", "/F"])
                    .status();
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        // Clean up PID file if process is gone
        if !crate::commands::watch::is_process_running(pid) {
            let config = crate::commands::watch::ScheduleConfig::load_default().ok();
            if let Some(config) = config {
                let pid_file = config
                    .db_path()
                    .parent()
                    .unwrap_or(std::path::Path::new("."))
                    .join("autopilot.pid");
                let _ = std::fs::remove_file(&pid_file);
            }
        }

        stopped = true;
    }

    // Now that the process has exited cleanly, uninstall the system service.
    // This is safe because the process is already gone — launchctl/systemd
    // won't force-kill anything.
    if autopilot_service_installed() {
        // stop_autopilot_service is a no-op if the process is already gone,
        // but call it for completeness (systemd may need it).
        let _ = stop_autopilot_service();
        uninstall_autopilot_service()?;
        if !stopped {
            stopped = true;
        }
        println!("✓ Autopilot service stopped and uninstalled");
    } else if stopped {
        println!("✓ Autopilot process stopped");
    }

    if !stopped {
        println!("Autopilot is not running.");
    }

    Ok(())
}

async fn restart_autopilot() -> Result<(), String> {
    // 1. Validate the autopilot config (server + schedules)
    println!("Validating autopilot configuration...");
    let autopilot_config = AutopilotConfigFile::load_or_default()?;

    for schedule in &autopilot_config.schedules {
        validate_schedule(schedule)?;
    }
    let config_path = AutopilotConfigFile::path();
    let channel_count = gateway_channel_count(config_path.as_path())?;

    println!(
        "  ✓ {} schedule(s), {} channel(s), server listen={}",
        autopilot_config.schedules.len(),
        channel_count,
        autopilot_config.server.listen,
    );

    // 2. Validate the watch/scheduler config (cron parsing, check scripts, db/log paths)
    match crate::commands::watch::ScheduleConfig::load_default() {
        Ok(config) => {
            println!(
                "  ✓ Scheduler config valid ({} schedules)",
                config.schedules.len()
            );
        }
        Err(e) => {
            return Err(format!(
                "Scheduler configuration error: {}\nFix {} and try again.",
                e,
                AutopilotConfigFile::path().display(),
            ));
        }
    }

    // 3. Restart: service path or foreground PID
    if autopilot_service_installed() {
        println!("\nRestarting autopilot service...");
        stop_autopilot_service()?;
        // Wait for the old process to fully exit before starting the new one.
        // launchctl stop is async — the process may still be shutting down.
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        start_autopilot_service()?;
        println!("✓ Autopilot service restarted (scheduler, server, gateway)");
    } else if let Some(pid) = is_autopilot_running() {
        println!("\nAutopilot is running in foreground mode (PID {}).", pid);
        println!("Stop it with Ctrl-C or `vac autopilot down`, then start again with `vac up`.");
        println!("Configuration has been validated and will take effect on next start.");
    } else {
        return Err("Autopilot is not running. Start it with `vac up`.".to_string());
    }

    Ok(())
}

async fn run_schedule_command(
    command: AutopilotScheduleCommands,
    config: &AppConfig,
) -> Result<(), String> {
    match command {
        AutopilotScheduleCommands::List => list_schedules().await,
        AutopilotScheduleCommands::Add {
            name,
            cron,
            prompt,
            check,
            trigger_on,
            // workdir,
            max_steps,
            channel,
            profile,
            pause_on_approval,
            sandbox,
            enabled,
        } => {
            let mut autopilot_config = AutopilotConfigFile::load_or_default_async().await?;
            let profile = normalize_optional_string(profile);
            if let Some(profile_name) = profile.as_deref() {
                validate_profile_reference(profile_name, config)?;
            }

            let schedule = AutopilotScheduleConfig {
                name: name.clone(),
                cron,
                prompt,
                check,
                trigger_on,
                // workdir,
                max_steps,
                channel,
                profile,
                pause_on_approval,
                sandbox,
                enabled,
            };
            let check_path = schedule.check.clone();
            add_schedule_in_config(&mut autopilot_config, schedule)?;
            autopilot_config.save()?;

            let signaled = signal_scheduler_reload().await;
            print_schedule_mutation_feedback(&name, "added", signaled);
            if let Some(path) = check_path.as_deref()
                && uses_home_tilde_prefix(path)
            {
                eprintln!(
                    "Note: check path '{}' uses '~'. It expands to the HOME of the user running autopilot; for systemd/launchd/containers, prefer an absolute path.",
                    path
                );
            }
            Ok(())
        }
        AutopilotScheduleCommands::Remove { name } => {
            let mut config = AutopilotConfigFile::load_or_default_async().await?;
            remove_schedule_in_config(&mut config, &name)?;
            config.save()?;

            let signaled = signal_scheduler_reload().await;
            print_schedule_mutation_feedback(&name, "removed", signaled);
            Ok(())
        }
        AutopilotScheduleCommands::Enable { name } => {
            let mut config = AutopilotConfigFile::load_or_default_async().await?;
            set_schedule_enabled_in_config(&mut config, &name, true)?;
            config.save()?;

            let signaled = signal_scheduler_reload().await;
            print_schedule_mutation_feedback(&name, "enabled", signaled);
            Ok(())
        }
        AutopilotScheduleCommands::Disable { name } => {
            let mut config = AutopilotConfigFile::load_or_default_async().await?;
            set_schedule_enabled_in_config(&mut config, &name, false)?;
            config.save()?;

            let signaled = signal_scheduler_reload().await;
            print_schedule_mutation_feedback(&name, "disabled", signaled);
            Ok(())
        }
        AutopilotScheduleCommands::Trigger { name, dry_run } => {
            // Validate the schedule exists in config
            let config = AutopilotConfigFile::load_or_default_async().await?;
            if config.find_schedule(&name).is_none() {
                return Err(format!("Schedule '{}' not found", name));
            }
            // Delegate to the watch module's fire_schedule
            match crate::commands::watch::commands::schedule::fire_schedule(&name, dry_run).await {
                Ok(()) => Ok(()),
                Err(e) if e.contains("not found") || e.contains("not running") => Err(format!(
                    "Cannot trigger '{}': autopilot is not running. Start it with: vac up",
                    name
                )),
                Err(e) => Err(e),
            }
        }
        AutopilotScheduleCommands::History { name, limit } => {
            crate::commands::watch::commands::history::show_history(Some(&name), Some(limit)).await
        }
        AutopilotScheduleCommands::Show { id } => {
            crate::commands::watch::commands::history::show_run(id).await
        }
        AutopilotScheduleCommands::Clean { older_than_days } => {
            let config = crate::commands::watch::ScheduleConfig::load_default()
                .map_err(|e| format!("Failed to load watch config: {}", e))?;
            let db_path = config.db_path();
            let db_path_str = db_path
                .to_str()
                .ok_or_else(|| "Invalid database path".to_string())?;
            let db = crate::commands::watch::ScheduleDb::new(db_path_str)
                .await
                .map_err(|e| format!("Failed to open database: {}", e))?;

            // Clean stale running runs
            let cleaned = db
                .clean_stale_runs()
                .await
                .map_err(|e| format!("Failed to clean stale runs: {}", e))?;
            if cleaned > 0 {
                println!(
                    "✓ Marked {} stale run{} as failed",
                    cleaned,
                    if cleaned == 1 { "" } else { "s" }
                );
            } else {
                println!("No stale runs found");
            }

            // Optionally prune old history
            if let Some(days) = older_than_days {
                let pruned = db
                    .prune_runs(days)
                    .await
                    .map_err(|e| format!("Failed to prune runs: {}", e))?;
                if pruned > 0 {
                    println!(
                        "✓ Pruned {} run{} older than {} day{}",
                        pruned,
                        if pruned == 1 { "" } else { "s" },
                        days,
                        if days == 1 { "" } else { "s" }
                    );
                } else {
                    println!("No runs older than {} days to prune", days);
                }
            }

            Ok(())
        }
    }
}

fn require_non_empty_token(token: String, error_message: &str) -> Result<String, String> {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return Err(error_message.to_string());
    }
    Ok(trimmed.to_string())
}

fn validate_profile_reference(profile_name: &str, config: &AppConfig) -> Result<(), String> {
    let profile_name = profile_name.trim();
    if profile_name.is_empty() {
        return Err("Profile name cannot be empty".to_string());
    }

    if profile_name == "all" {
        return Err("Profile name 'all' is reserved and cannot be selected directly".to_string());
    }

    let config_file = AppConfig::load_config_file(Path::new(&config.config_path))
        .map_err(|error| format!("Failed to load config.toml: {error}"))?;

    if config_file.profiles.contains_key(profile_name) {
        return Ok(());
    }

    let mut available: Vec<String> = config_file
        .profiles
        .keys()
        .filter(|name| name.as_str() != "all")
        .cloned()
        .collect();
    available.sort();

    if available.is_empty() {
        return Err(format!(
            "Profile '{}' not found in config.toml",
            profile_name
        ));
    }

    Err(format!(
        "Profile '{}' not found in config.toml. Available: {}",
        profile_name,
        available.join(", ")
    ))
}

fn add_channel_with_optional_target(
    config_path: &Path,
    channel_type: ChannelType,
    token: Option<String>,
    bot_token: Option<String>,
    app_token: Option<String>,
    target: Option<String>,
    profile: Option<String>,
) -> Result<Option<String>, String> {
    let had_target = target.is_some();
    let normalized_target = target
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    if had_target && normalized_target.is_none() {
        return Err("Target cannot be empty".to_string());
    }

    let normalized_profile = normalize_optional_string(profile);

    let mut root = load_toml_root_table(config_path)?;

    {
        let channels = ensure_toml_table(&mut root, "channels");

        match &channel_type {
            ChannelType::Telegram => {
                let raw_token = token.or_else(|| std::env::var("TELEGRAM_BOT_TOKEN").ok()).ok_or(
                    "Telegram token required. Use --token or set TELEGRAM_BOT_TOKEN\n\n  To get a token: message @BotFather on Telegram → /newbot → copy the bot token",
                )?;
                let tok = require_non_empty_token(
                    raw_token,
                    "Telegram token cannot be empty. Use --token or set TELEGRAM_BOT_TOKEN",
                )?;

                let mut telegram = toml::value::Table::new();
                telegram.insert("token".to_string(), toml::Value::String(tok));
                telegram.insert("require_mention".to_string(), toml::Value::Boolean(false));
                if let Some(profile_name) = normalized_profile.as_ref() {
                    telegram.insert(
                        "profile".to_string(),
                        toml::Value::String(profile_name.clone()),
                    );
                }
                channels.insert("telegram".to_string(), toml::Value::Table(telegram));
            }
            ChannelType::Discord => {
                let raw_token = token.or_else(|| std::env::var("DISCORD_BOT_TOKEN").ok()).ok_or(
                    "Discord token required. Use --token or set DISCORD_BOT_TOKEN\n\n  To get a token: https://discord.com/developers/applications → create app → Bot tab → copy token",
                )?;
                let tok = require_non_empty_token(
                    raw_token,
                    "Discord token cannot be empty. Use --token or set DISCORD_BOT_TOKEN",
                )?;

                let mut discord = toml::value::Table::new();
                discord.insert("token".to_string(), toml::Value::String(tok));
                discord.insert("guilds".to_string(), toml::Value::Array(Vec::new()));
                if let Some(profile_name) = normalized_profile.as_ref() {
                    discord.insert(
                        "profile".to_string(),
                        toml::Value::String(profile_name.clone()),
                    );
                }
                channels.insert("discord".to_string(), toml::Value::Table(discord));
            }
            ChannelType::Slack => {
                let raw_bot = bot_token.or_else(|| std::env::var("SLACK_BOT_TOKEN").ok()).ok_or(
                    "Slack bot token required. Use --bot-token or set SLACK_BOT_TOKEN\n\n  To get tokens: https://api.slack.com/apps → create app → enable Socket Mode\n  → OAuth & Permissions → Install to Workspace → copy Bot User OAuth Token (xoxb-...)",
                )?;
                let raw_app = app_token.or_else(|| std::env::var("SLACK_APP_TOKEN").ok()).ok_or(
                    "Slack app token required. Use --app-token or set SLACK_APP_TOKEN\n\n  To get tokens: https://api.slack.com/apps → your app → Socket Mode\n  → generate app-level token with connections:write scope (xapp-...)",
                )?;
                let bot = require_non_empty_token(
                    raw_bot,
                    "Slack bot token cannot be empty. Use --bot-token or set SLACK_BOT_TOKEN",
                )?;
                let app = require_non_empty_token(
                    raw_app,
                    "Slack app token cannot be empty. Use --app-token or set SLACK_APP_TOKEN",
                )?;

                let mut slack = toml::value::Table::new();
                slack.insert("bot_token".to_string(), toml::Value::String(bot));
                slack.insert("app_token".to_string(), toml::Value::String(app));
                if let Some(profile_name) = normalized_profile.as_ref() {
                    slack.insert(
                        "profile".to_string(),
                        toml::Value::String(profile_name.clone()),
                    );
                }
                channels.insert("slack".to_string(), toml::Value::Table(slack));
            }
            _ => return Err(format!("{:?} is not supported yet", channel_type)),
        }
    }

    if let Some(target_value) = normalized_target.as_deref() {
        apply_default_notification_target(&mut root, &channel_type.to_string(), target_value)?;
    }

    write_toml_root_table(config_path, root)?;

    Ok(normalized_target)
}

fn remove_channel(config_path: &Path, channel_type: ChannelType) -> Result<(), String> {
    let mut config = vac_messaging_gateway::GatewayConfig::load_unvalidated(
        config_path,
        &vac_messaging_gateway::GatewayCliFlags::default(),
    )
    .map_err(|e| format!("Failed to load config: {e}"))?;

    match &channel_type {
        ChannelType::Telegram => config.channels.telegram = None,
        ChannelType::Discord => config.channels.discord = None,
        ChannelType::Slack => config.channels.slack = None,
        _ => return Err(format!("{:?} is not supported yet", channel_type)),
    }

    config
        .save(config_path)
        .map_err(|e| format!("Failed to save config: {e}"))?;

    Ok(())
}

async fn run_channel_command(
    command: AutopilotChannelCommands,
    config: &AppConfig,
) -> Result<(), String> {
    let config_path = AutopilotConfigFile::path();
    match command {
        AutopilotChannelCommands::List => {
            let config = load_gateway_config_allowing_no_channels(config_path.as_path())?;

            let channels = config.enabled_channels();
            if channels.is_empty() {
                println!("No channels configured.");
                println!("  Add one: vac autopilot channel add slack --bot-token X --app-token Y");
                return Ok(());
            }

            println!("{:<15} STATUS", "CHANNEL");
            if config.channels.telegram.is_some() {
                println!("{:<15} configured", "telegram");
            }
            if config.channels.discord.is_some() {
                println!("{:<15} configured", "discord");
            }
            if config.channels.slack.is_some() {
                println!("{:<15} configured", "slack");
            }
            Ok(())
        }
        AutopilotChannelCommands::Add {
            channel_type,
            token,
            bot_token,
            app_token,
            target,
            profile,
        } => {
            let explicit_profile = normalize_optional_string(profile);
            let default_profile = normalize_optional_string(Some(config.profile_name.clone()));
            let requested_profile = explicit_profile.clone().or(default_profile);

            if let Some(profile_name) = requested_profile.as_deref() {
                validate_profile_reference(profile_name, config)?;
                if explicit_profile.is_none() {
                    println!(
                        "ℹ Using profile '{}' (override with --profile)",
                        profile_name
                    );
                }
            }

            let saved_target = add_channel_with_optional_target(
                config_path.as_path(),
                channel_type.clone(),
                token,
                bot_token,
                app_token,
                target,
                requested_profile,
            )?;

            if let Some(target_value) = saved_target {
                println!(
                    "✓ Default notification target set for {}: {}",
                    channel_type, target_value
                );
            }

            println!("✓ Channel {} added", channel_type);
            Ok(())
        }
        AutopilotChannelCommands::Remove { channel_type } => {
            remove_channel(config_path.as_path(), channel_type.clone())?;
            println!("✓ Channel {} removed", channel_type);
            Ok(())
        }
        AutopilotChannelCommands::Test => {
            let config = vac_messaging_gateway::GatewayConfig::load(
                config_path.as_path(),
                &vac_messaging_gateway::GatewayCliFlags::default(),
            )
            .map_err(|e| format!("Failed to load config: {e}"))?;

            let channels = vac_messaging_gateway::build_channels(&config)
                .map_err(|e| format!("Failed to build channels: {e}"))?;

            if channels.is_empty() {
                return Err("No channels configured. Add one first: vac autopilot channel add slack --bot-token X --app-token Y".to_string());
            }

            for channel in channels.values() {
                match channel.test().await {
                    Ok(result) => println!(
                        "  ✓ {}: {} ({})",
                        result.channel, result.identity, result.details
                    ),
                    Err(error) => println!("  ✗ {}: {}", channel.display_name(), error),
                }
            }
            Ok(())
        }
    }
}

async fn list_schedules() -> Result<(), String> {
    let config = AutopilotConfigFile::load_or_default_async().await?;
    if config.schedules.is_empty() {
        println!("No schedules configured.");
        return Ok(());
    }

    println!(
        "{:<20} {:<16} {:<10} {:<8} {:<24}",
        "NAME", "CRON", "STATUS", "SANDBOX", "NEXT RUN"
    );

    for schedule in &config.schedules {
        let next_run =
            next_run_for_cron(&schedule.cron, schedule.enabled).unwrap_or_else(|| "-".to_string());
        println!(
            "{:<20} {:<16} {:<10} {:<8} {:<24}",
            truncate_text(&schedule.name, 20),
            truncate_text(&schedule.cron, 16),
            if schedule.enabled {
                "enabled"
            } else {
                "disabled"
            },
            if schedule.sandbox { "yes" } else { "no" },
            truncate_text(&next_run, 24)
        );
    }

    Ok(())
}

fn validate_schedule(schedule: &AutopilotScheduleConfig) -> Result<(), String> {
    if schedule.name.trim().is_empty() {
        return Err("Schedule name cannot be empty".to_string());
    }

    if schedule.name.trim() == crate::commands::watch::RELOAD_SENTINEL {
        return Err(format!(
            "Schedule name '{}' is reserved",
            crate::commands::watch::RELOAD_SENTINEL
        ));
    }

    Cron::from_str(&schedule.cron)
        .map_err(|e| format!("Invalid cron expression '{}': {}", schedule.cron, e))?;

    if schedule.prompt.trim().is_empty() {
        return Err("Schedule prompt cannot be empty".to_string());
    }

    if let Some(profile_name) = schedule.profile.as_deref()
        && profile_name.trim().is_empty()
    {
        return Err("Schedule profile cannot be empty".to_string());
    }

    if let Some(check_path) = schedule.check.as_deref() {
        let expanded = crate::commands::watch::config::expand_tilde(check_path);
        let expanded_str = expanded.to_string_lossy();

        if uses_home_tilde_prefix(check_path) && uses_home_tilde_prefix(&expanded_str) {
            return Err(format!(
                "Cannot expand '~' in check script path for schedule '{}': {}. Home directory for the running user could not be determined; use an absolute path.",
                schedule.name, check_path
            ));
        }

        if !expanded.exists() {
            return Err(format!(
                "Check script not found for schedule '{}': {}",
                schedule.name, check_path
            ));
        }

        if !expanded.is_file() {
            return Err(format!(
                "Check script path is not a file for schedule '{}': {}",
                schedule.name, check_path
            ));
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = std::fs::metadata(&expanded).map_err(|e| {
                format!(
                    "Cannot read check script metadata for schedule '{}': {}",
                    schedule.name, e
                )
            })?;
            let permissions = metadata.permissions();
            if permissions.mode() & 0o111 == 0 {
                return Err(format!(
                    "Check script is not executable for schedule '{}': {}",
                    schedule.name, check_path
                ));
            }
        }
    }

    Ok(())
}

fn uses_home_tilde_prefix(path: &str) -> bool {
    path == "~" || path.starts_with("~/") || path.starts_with("~\\")
}

fn add_schedule_in_config(
    config: &mut AutopilotConfigFile,
    schedule: AutopilotScheduleConfig,
) -> Result<(), String> {
    validate_schedule(&schedule)?;

    if config.find_schedule(&schedule.name).is_some() {
        return Err(format!("Schedule '{}' already exists", schedule.name));
    }

    config.schedules.push(schedule);
    Ok(())
}

fn remove_schedule_in_config(config: &mut AutopilotConfigFile, name: &str) -> Result<(), String> {
    let initial_len = config.schedules.len();
    config.schedules.retain(|schedule| schedule.name != name);

    if config.schedules.len() == initial_len {
        return Err(format!("Schedule '{}' not found", name));
    }

    Ok(())
}

fn set_schedule_enabled_in_config(
    config: &mut AutopilotConfigFile,
    name: &str,
    enabled: bool,
) -> Result<(), String> {
    let schedule = config
        .find_schedule_mut(name)
        .ok_or_else(|| format!("Schedule '{}' not found", name))?;

    schedule.enabled = enabled;
    Ok(())
}

fn print_schedule_mutation_feedback(name: &str, action: &str, signaled: bool) {
    if is_autopilot_running().is_some() {
        if signaled {
            println!("✓ Schedule '{}' {} (takes effect within ~1s)", name, action);
        } else {
            println!("✓ Schedule '{}' {} (takes effect within ~5s)", name, action);
        }
    } else {
        println!(
            "✓ Schedule '{}' {} (takes effect when autopilot starts)",
            name, action
        );
    }
}

async fn signal_scheduler_reload() -> bool {
    let db_path = match autopilot_db_path() {
        Ok(path) => path,
        Err(_) => return false,
    };

    let db = match crate::commands::watch::ScheduleDb::new(&db_path).await {
        Ok(db) => db,
        Err(_) => return false,
    };

    db.request_config_reload().await.is_ok()
}

fn autopilot_db_path() -> Result<String, String> {
    let config = crate::commands::watch::ScheduleConfig::load_default()
        .map_err(|error| format!("Failed to load watch config: {}", error))?;
    let db_path = config.db_path();

    db_path
        .to_str()
        .map(|value| value.to_string())
        .ok_or_else(|| "Invalid db path".to_string())
}

#[derive(Debug, Clone)]
struct NotificationDefaults {
    channel: String,
    chat_id: Option<String>,
}

fn load_notification_defaults(path: &Path) -> Result<NotificationDefaults, String> {
    let root = load_toml_root_table(path)?;
    let notifications = root
        .get("notifications")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| "Notifications are not configured".to_string())?;

    let channel = notifications
        .get("channel")
        .and_then(toml::Value::as_str)
        .ok_or_else(|| "Notifications channel is not configured".to_string())?
        .to_string();

    let chat_id = notifications
        .get("chat_id")
        .and_then(toml::Value::as_str)
        .map(str::to_string);

    Ok(NotificationDefaults { channel, chat_id })
}

fn resolve_default_gateway_url(root: &toml::value::Table) -> String {
    root.get("server")
        .and_then(toml::Value::as_table)
        .and_then(|server| server.get("listen"))
        .and_then(toml::Value::as_str)
        .map(loopback_base_url_from_bind)
        .unwrap_or_else(|| "http://127.0.0.1:4096".to_string())
}

fn apply_default_notification_target(
    root: &mut toml::value::Table,
    channel: &str,
    target: &str,
) -> Result<(), String> {
    if channel.trim().is_empty() {
        return Err("Channel cannot be empty".to_string());
    }

    if target.trim().is_empty() {
        return Err("Target cannot be empty".to_string());
    }

    let default_gateway_url = resolve_default_gateway_url(root);

    let notifications = ensure_toml_table(root, "notifications");
    if !notifications.contains_key("gateway_url") {
        notifications.insert(
            "gateway_url".to_string(),
            toml::Value::String(default_gateway_url),
        );
    }
    notifications.insert(
        "channel".to_string(),
        toml::Value::String(channel.trim().to_string()),
    );
    notifications.insert(
        "chat_id".to_string(),
        toml::Value::String(target.trim().to_string()),
    );

    Ok(())
}

#[cfg(test)]
fn set_default_notification_target(path: &Path, channel: &str, target: &str) -> Result<(), String> {
    let mut root = load_toml_root_table(path)?;
    apply_default_notification_target(&mut root, channel, target)?;
    write_toml_root_table(path, root)
}

fn load_gateway_config_allowing_no_channels(
    config_path: &Path,
) -> Result<vac_messaging_gateway::GatewayConfig, String> {
    let cli_flags = vac_messaging_gateway::GatewayCliFlags::default();
    let config = vac_messaging_gateway::GatewayConfig::load_unvalidated(config_path, &cli_flags)
        .map_err(|e| format!("Failed to load channel config: {e}"))?;

    match config.validate_with_error() {
        Ok(())
        | Err(vac_messaging_gateway::config::GatewayConfigValidationError::MissingChannels) => {
            Ok(config)
        }
        Err(error) => Err(format!("Channel config invalid: {error}")),
    }
}

fn gateway_channel_count(config_path: &Path) -> Result<usize, String> {
    let config = load_gateway_config_allowing_no_channels(config_path)?;
    Ok(config.enabled_channels().len())
}

async fn status_autopilot(
    config: &AppConfig,
    json: bool,
    recent_runs: Option<u32>,
) -> Result<(), String> {
    let autopilot_config = AutopilotConfigFile::load_or_default_async().await?;
    let server_config = autopilot_config.server.clone();
    let resolved_tool_policy = resolve_server_tool_policy(
        config.allowed_tools.as_ref(),
        config.auto_approve.as_ref(),
        server_config.auto_approve_all,
    );
    let server_allowed_tool_count = approved_tools_from_policy(&resolved_tool_policy).len();
    let config_path = AutopilotConfigFile::path();
    let base_url = loopback_base_url_from_bind(&server_config.listen);
    let probe_client = build_probe_http_client();

    let schedules = build_schedule_statuses(&autopilot_config.schedules);
    let gateway_config = load_gateway_config_allowing_no_channels(config_path.as_path())?;
    let notification_defaults = load_notification_defaults(config_path.as_path()).ok();
    let channels = build_channel_statuses(&gateway_config, notification_defaults.as_ref());

    let service_path = autopilot_service_path();
    let service = ServiceStatusJson {
        installed: autopilot_service_installed(),
        active: autopilot_service_active(),
        path: service_path.display().to_string(),
    };

    let server_url = format!("{}/v1/health", base_url);
    let (server_reachable, sandbox_health) = if let Some(client) = probe_client.as_ref() {
        match client.get(&server_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                // Try to parse sandbox health from the response body
                let sandbox = resp
                    .json::<serde_json::Value>()
                    .await
                    .ok()
                    .and_then(|v| v.get("sandbox").cloned())
                    .and_then(|s| {
                        Some(SandboxStatusJson {
                            mode: s.get("mode")?.as_str()?.to_string(),
                            healthy: s.get("healthy")?.as_bool(),
                            consecutive_ok: s.get("consecutive_ok")?.as_u64(),
                            consecutive_failures: s.get("consecutive_failures")?.as_u64(),
                            last_ok: s.get("last_ok").and_then(|v| v.as_str()).map(String::from),
                            last_error: s
                                .get("last_error")
                                .and_then(|v| v.as_str())
                                .map(String::from),
                        })
                    });
                (true, sandbox)
            }
            _ => (false, None),
        }
    } else {
        (false, None)
    };

    // Build sandbox status: use live health data if available, otherwise fall back to config
    let sandbox = sandbox_health.unwrap_or(SandboxStatusJson {
        mode: server_config.sandbox_mode.to_string(),
        healthy: None,
        consecutive_ok: None,
        consecutive_failures: None,
        last_ok: None,
        last_error: if server_reachable {
            None
        } else {
            Some("Server unreachable — cannot determine sandbox health".to_string())
        },
    });

    let server = EndpointStatusJson {
        expected_enabled: true,
        reachable: server_reachable,
        url: server_url,
    };

    let gateway_url = format!("{}/v1/gateway/status", base_url);
    let gateway_reachable = if let Some(client) = probe_client.as_ref() {
        endpoint_ok(client, &gateway_url).await
    } else {
        false
    };
    let gateway = EndpointStatusJson {
        expected_enabled: true,
        reachable: gateway_reachable,
        url: gateway_url,
    };

    let scheduler = collect_scheduler_status(recent_runs).await;

    if json {
        print_json(&AutopilotStatusJson {
            command: "autopilot.status",
            ok: true,
            profile: config.profile_name.clone(),
            config_path: config_path.display().to_string(),
            server_config: server_config.clone(),
            server_allowed_tool_count,
            service,
            server,
            gateway,
            sandbox,
            scheduler,
            schedules,
            channels,
        })?;
        return Ok(());
    }

    println!("Autopilot status");
    println!();
    println!("  Profile         {}", config.profile_name);
    println!("  Config          {}", config_path.display());
    println!(
        "  Service         {}",
        if service.installed {
            if service.active {
                "active"
            } else {
                "installed (inactive)"
            }
        } else {
            "not installed"
        }
    );
    println!(
        "  Server          {}",
        if server.reachable {
            format!("✓ reachable ({})", server.url)
        } else {
            format!("✗ unreachable ({})", server.url)
        }
    );
    println!(
        "  Channels        {}",
        if gateway.reachable {
            format!("✓ reachable ({})", gateway.url)
        } else {
            format!("✗ unreachable ({})", gateway.url)
        }
    );
    println!(
        "  Tools           {}",
        describe_tool_policy(&resolved_tool_policy)
    );
    // Sandbox status
    let sandbox_display = match (sandbox.healthy, sandbox.mode.as_str()) {
        (Some(true), mode) => format!("✓ healthy ({mode})"),
        (Some(false), mode) => {
            let err = sandbox.last_error.as_deref().unwrap_or("unknown error");
            format!("✗ unhealthy ({mode}) — {err}")
        }
        (None, mode) if server_reachable => format!("- {mode} (no health data)"),
        (None, mode) => format!("- {mode} (server unreachable)"),
    };
    println!("  Sandbox         {sandbox_display}");

    // Scheduler status
    let config_exists = AutopilotConfigFile::path().exists();
    if !config_exists {
        println!("  Scheduler       not configured (run: vac up)");
    } else if scheduler.config_valid {
        let sched_state = if scheduler.running {
            format!("✓ running (pid {})", scheduler.pid.unwrap_or_default())
        } else if scheduler.stale_pid {
            format!("⚠ stale (pid {})", scheduler.pid.unwrap_or_default())
        } else {
            "stopped".to_string()
        };
        println!(
            "  Scheduler       {} — {} schedules",
            sched_state, scheduler.trigger_count
        );
    } else {
        println!(
            "  Scheduler       ✗ config error: {}",
            scheduler.error.as_deref().unwrap_or("unknown")
        );
    }

    // Schedules table
    if !schedules.is_empty() {
        println!();
        println!("  Schedules:");
        println!(
            "    {:<20} {:<16} {:<10} {:<8} {:<20}",
            "NAME", "CRON", "STATUS", "SANDBOX", "NEXT RUN"
        );
        for schedule in &schedules {
            println!(
                "    {:<20} {:<16} {:<10} {:<8} {:<20}",
                truncate_text(&schedule.name, 20),
                truncate_text(&schedule.cron, 16),
                if schedule.enabled {
                    "enabled"
                } else {
                    "disabled"
                },
                if schedule.sandbox { "yes" } else { "no" },
                schedule.next_run.as_deref().unwrap_or("-")
            );
        }
    }

    // Channels table
    if !channels.is_empty() {
        println!();
        println!("  Channels:");
        println!(
            "    {:<20} {:<10} {:<24} {:<10}",
            "NAME", "TYPE", "TARGET", "STATUS"
        );
        for channel in &channels {
            println!(
                "    {:<20} {:<10} {:<24} {:<10}",
                truncate_text(&channel.name, 20),
                truncate_text(&channel.channel_type, 10),
                truncate_text(&channel.target, 24),
                if channel.enabled {
                    "enabled"
                } else {
                    "disabled"
                }
            );
        }
    }

    // Recent runs
    if !scheduler.recent_runs.is_empty() {
        println!();
        println!("  Recent runs:");
        for run in &scheduler.recent_runs {
            println!(
                "    #{} {:<16} {:<10} {}",
                run.id, run.schedule_name, run.status, run.started_at
            );
        }
    }

    Ok(())
}

fn tail_log_files(files: &[PathBuf], follow: bool, lines: Option<u32>) -> Result<(), String> {
    let mut cmd = std::process::Command::new("tail");
    if follow {
        cmd.arg("-f");
    }
    if let Some(n) = lines {
        cmd.arg("-n").arg(n.to_string());
    }
    for file in files {
        cmd.arg(file);
    }
    let status = cmd
        .status()
        .map_err(|e| format!("Failed to read autopilot logs: {}", e))?;
    if !status.success() {
        return Err("Failed to read autopilot logs".to_string());
    }
    Ok(())
}

async fn logs_autopilot(
    follow: bool,
    lines: Option<u32>,
    component: Option<String>,
) -> Result<(), String> {
    let log_dir = autopilot_log_dir();

    // Resolve which log files to show
    let log_files: Vec<PathBuf> = if let Some(ref name) = component {
        let file = log_dir.join(format!("{}.log", name));
        if !file.exists() {
            return Err(format!(
                "Component log file not found: {}\nAutopilot may not have run yet.",
                file.display()
            ));
        }
        vec![file]
    } else {
        // Show all component logs plus legacy stdout/stderr
        [
            "scheduler.log",
            "server.log",
            "gateway.log",
            "stdout.log",
            "stderr.log",
        ]
        .iter()
        .map(|f| log_dir.join(f))
        .filter(|p| p.exists())
        .collect()
    };

    if log_files.is_empty() {
        return Err(format!(
            "No log files found in {}.\nAutopilot may not have run yet.",
            log_dir.display()
        ));
    }

    match detect_platform() {
        Platform::Linux => {
            // If a component filter is set, use tail on the specific file instead of journalctl
            if component.is_some() {
                tail_log_files(&log_files, follow, lines)?;
            } else {
                let mut cmd = std::process::Command::new("journalctl");
                cmd.args(["--user", "-u", AUTOPILOT_SYSTEMD_SERVICE]);
                if follow {
                    cmd.arg("-f");
                }
                if let Some(lines) = lines {
                    cmd.arg("-n").arg(lines.to_string());
                }

                let status = cmd
                    .status()
                    .map_err(|e| format!("Failed to run journalctl: {}", e))?;
                if !status.success() {
                    return Err("Failed to read autopilot logs from journalctl".to_string());
                }
            }
        }
        Platform::MacOS => {
            tail_log_files(&log_files, follow, lines)?;
        }
        Platform::Windows | Platform::Unknown => {
            return Err(
                "Autopilot logs are currently supported on Linux (journalctl) and macOS (tail)."
                    .to_string(),
            );
        }
    }

    Ok(())
}

async fn doctor_autopilot(config: &AppConfig) -> Result<(), String> {
    println!("Autopilot doctor");

    let mut failures = 0usize;

    let autopilot_config = match AutopilotConfigFile::load_or_default() {
        Ok(cfg) => {
            println!("✓ Autopilot config loaded (listen={})", cfg.server.listen);
            cfg
        }
        Err(e) => {
            failures += 1;
            println!("✗ Autopilot config invalid: {}", e);
            AutopilotConfigFile::default()
        }
    };
    let _ = &autopilot_config;

    let base_url = loopback_base_url_from_bind(&autopilot_config.server.listen);
    let server_health_url = format!("{}/v1/health", base_url);
    let probe_client = build_probe_http_client();
    let server_reachable = if let Some(client) = probe_client.as_ref() {
        endpoint_ok(client, &server_health_url).await
    } else {
        false
    };

    let env = RealProbeEnvironment;
    let probe_ctx = AutopilotProbeContext {
        app_config: config,
        bind_addr: Some(&autopilot_config.server.listen),
        server_reachable,
    };
    let probe_results = run_autopilot_probes(ProbeMode::Doctor, &probe_ctx, &env);
    presenter::print_probe_report("Deployment readiness", &probe_results);
    failures += summarize_probe_results(&probe_results).blocking_failures;

    let gateway_path = AutopilotConfigFile::path();
    match load_gateway_config_allowing_no_channels(gateway_path.as_path()) {
        Ok(cfg) => {
            let channels = cfg.enabled_channels();
            if channels.is_empty() {
                println!("✓ No channels configured (add with: vac autopilot channel add)");
            } else {
                println!("✓ Channel config valid (channels: {})", channels.join(", "));
            }

            for warning in cfg.check_deprecations() {
                println!("⚠ {}", warning);
            }
        }
        Err(e) => {
            failures += 1;
            println!("✗ Channel config invalid: {}", e);
        }
    }

    let scheduler_status = collect_scheduler_status(None).await;
    if scheduler_status.config_valid {
        if scheduler_status.trigger_count == 0 {
            println!("✓ No schedules configured (edit ~/.vac/autopilot.toml to add)");
        } else {
            println!(
                "✓ Schedule config valid ({} schedules)",
                scheduler_status.trigger_count
            );
        }
    } else {
        failures += 1;
        println!(
            "✗ Schedule config invalid: {}",
            scheduler_status
                .error
                .unwrap_or_else(|| "unknown configuration error".to_string())
        );
    }

    if autopilot_service_installed() {
        println!("✓ Autopilot service installed");
    } else {
        failures += 1;
        println!("✗ Autopilot service not installed");
    }

    if server_reachable {
        println!("✓ Server health endpoint reachable");
    } else {
        println!("⚠ Server health endpoint not reachable (not running is OK before start)");
    }

    let resolved_tool_policy = resolve_server_tool_policy(
        config.allowed_tools.as_ref(),
        config.auto_approve.as_ref(),
        autopilot_config.server.auto_approve_all,
    );

    println!();
    println!("Security:");
    match &resolved_tool_policy {
        vac_broker::ToolApprovalPolicy::All => {
            println!("  ⚠ all tools allowed (auto_approve_all = true)");
        }
        _ => {
            println!("  ✓ {}", describe_tool_policy(&resolved_tool_policy));
        }
    }

    if failures > 0 {
        return Err(format!("Doctor found {} blocking issue(s)", failures));
    }

    println!("✓ Doctor checks passed");
    Ok(())
}

fn print_json<T: Serialize>(value: &T) -> Result<(), String> {
    let json = serde_json::to_string(value)
        .map_err(|e| format!("Failed to serialize JSON output: {}", e))?;
    println!("{}", json);
    Ok(())
}

async fn write_default_autopilot_config(path: &Path, force: bool) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Failed to create autopilot config directory: {}", e))?;
    }

    if force || !path.exists() {
        let default_config = AutopilotConfigFile::default();
        let content = toml::to_string_pretty(&default_config)
            .map_err(|e| format!("Failed to serialize default autopilot config: {}", e))?;

        let header = "# VAC Autopilot Configuration\n\
                      # Add schedules:  vac autopilot schedule add <name> --cron '...' --prompt '...'\n\
                      # Add channels:   vac autopilot channel add slack --bot-token X --app-token Y\n\
                      # Start:          vac up\n\n";

        tokio::fs::write(path, format!("{}{}", header, content))
            .await
            .map_err(|e| format!("Failed to write autopilot config: {}", e))?;
    }

    Ok(())
}

fn validate_no_auth_bind(bind: &str, no_auth: bool) -> Result<(), String> {
    if !no_auth {
        return Ok(());
    }

    let addr = bind.parse::<SocketAddr>().map_err(|_| {
        format!(
            "no_auth_requires_loopback_bind: disabled route checks require an explicit loopback SocketAddr bind; got {bind}"
        )
    })?;

    if addr.ip().is_loopback() {
        Ok(())
    } else {
        Err(format!(
            "no_auth_requires_loopback_bind: disabled route checks may only be used with loopback bind addresses; got {bind}"
        ))
    }
}

fn loopback_base_url_from_bind(bind: &str) -> String {
    match bind.parse::<SocketAddr>() {
        Ok(addr) => {
            let port = addr.port();
            match addr.ip() {
                IpAddr::V4(ip) => {
                    if ip.is_unspecified() {
                        format!("http://{}:{}", Ipv4Addr::LOCALHOST, port)
                    } else {
                        format!("http://{}:{}", ip, port)
                    }
                }
                IpAddr::V6(ip) => {
                    if ip.is_unspecified() {
                        format!("http://[{}]:{}", Ipv6Addr::LOCALHOST, port)
                    } else {
                        format!("http://[{}]:{}", ip, port)
                    }
                }
            }
        }
        Err(_) => "http://127.0.0.1:4096".to_string(),
    }
}

async fn collect_scheduler_status(recent_runs: Option<u32>) -> SchedulerStatusJson {
    let config_path = AutopilotConfigFile::path();

    let schedule_count = AutopilotConfigFile::load_or_default()
        .map(|c| c.schedules.len())
        .unwrap_or(0);

    let config_valid = config_path.exists();

    // Watch runtime uses ~/.vac/autopilot/autopilot.db regardless of config format
    let db_path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".vac/autopilot/autopilot.db");
    let db_path_str = db_path.to_string_lossy().to_string();

    let db = match db_path.to_str() {
        Some(path) => match crate::commands::watch::ScheduleDb::new(path).await {
            Ok(db) => db,
            Err(error) => {
                return SchedulerStatusJson {
                    expected_enabled: true,
                    config_path: config_path.display().to_string(),
                    config_valid,
                    trigger_count: schedule_count,
                    running: false,
                    pid: None,
                    stale_pid: false,
                    db_path: Some(db_path_str),
                    error: Some(error.to_string()),
                    recent_runs: Vec::new(),
                };
            }
        },
        None => {
            return SchedulerStatusJson {
                expected_enabled: true,
                config_path: config_path.display().to_string(),
                config_valid,
                trigger_count: schedule_count,
                running: false,
                pid: None,
                stale_pid: false,
                db_path: Some(db_path_str),
                error: Some("Invalid scheduler database path".to_string()),
                recent_runs: Vec::new(),
            };
        }
    };

    let scheduler_state = db.get_autopilot_state().await.ok().flatten();

    let (running, stale_pid, pid) = if let Some(state) = scheduler_state {
        let pid = state.pid;
        let running = u32::try_from(pid)
            .ok()
            .map(crate::commands::watch::is_process_running)
            .unwrap_or(false);
        (running, !running, Some(pid))
    } else {
        (false, false, None)
    };

    let recent_runs = if let Some(limit) = recent_runs.filter(|limit| *limit > 0) {
        match db
            .list_runs(&crate::commands::watch::ListRunsFilter {
                schedule_name: None,
                status: None,
                limit: Some(limit),
                offset: None,
            })
            .await
        {
            Ok(runs) => runs
                .into_iter()
                .map(|run| ScheduleRunSummaryJson {
                    id: run.id,
                    schedule_name: run.schedule_name,
                    status: run.status.to_string(),
                    started_at: run.started_at.to_rfc3339(),
                    finished_at: run.finished_at.map(|value| value.to_rfc3339()),
                    error_message: run.error_message,
                })
                .collect(),
            Err(_) => Vec::new(),
        }
    } else {
        Vec::new()
    };

    SchedulerStatusJson {
        expected_enabled: true,
        config_path: config_path.display().to_string(),
        config_valid,
        trigger_count: schedule_count,
        running,
        pid,
        stale_pid,
        db_path: Some(db_path_str),
        error: None,
        recent_runs,
    }
}

fn build_schedule_statuses(
    schedules: &[AutopilotScheduleConfig],
) -> Vec<AutopilotScheduleStatusJson> {
    schedules
        .iter()
        .map(|schedule| AutopilotScheduleStatusJson {
            name: schedule.name.clone(),
            cron: schedule.cron.clone(),
            enabled: schedule.enabled,
            sandbox: schedule.sandbox,
            next_run: next_run_for_cron(&schedule.cron, schedule.enabled),
        })
        .collect()
}

fn build_channel_statuses(
    gateway_config: &vac_messaging_gateway::GatewayConfig,
    notification_defaults: Option<&NotificationDefaults>,
) -> Vec<AutopilotChannelStatusJson> {
    let mut channels = Vec::new();

    if gateway_config.channels.telegram.is_some() {
        channels.push(build_single_channel_status(
            "telegram",
            notification_defaults,
        ));
    }

    if gateway_config.channels.discord.is_some() {
        channels.push(build_single_channel_status(
            "discord",
            notification_defaults,
        ));
    }

    if gateway_config.channels.slack.is_some() {
        channels.push(build_single_channel_status("slack", notification_defaults));
    }

    channels
}

fn build_single_channel_status(
    channel_name: &str,
    notification_defaults: Option<&NotificationDefaults>,
) -> AutopilotChannelStatusJson {
    let target = notification_defaults
        .filter(|defaults| defaults.channel == channel_name)
        .and_then(|defaults| defaults.chat_id.clone())
        .unwrap_or_else(|| "-".to_string());

    AutopilotChannelStatusJson {
        name: channel_name.to_string(),
        channel_type: channel_name.to_string(),
        target,
        enabled: true,
        alerts_only: false,
    }
}

fn next_run_for_cron(cron: &str, enabled: bool) -> Option<String> {
    if !enabled {
        return None;
    }

    let expression = Cron::from_str(cron).ok()?;
    let next = expression.find_next_occurrence(&Utc::now(), false).ok()?;
    Some(next.format("%Y-%m-%d %H:%M").to_string())
}

fn truncate_text(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }

    let mut truncated = value
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    truncated.push_str("...");
    truncated
}

#[cfg(test)]
fn bounded_history_limit(limit: u32) -> u32 {
    limit.clamp(1, 1000)
}

/// Ensure the sandbox container image is available locally before starting the
/// autopilot service. If the image isn't cached, runs `docker pull` with
/// inherited stdout/stderr so the user sees Docker's native progress bars
/// (layer downloads, extraction, etc.) instead of a silent wait.
fn ensure_sandbox_image_available() -> Result<(), String> {
    let image = crate::commands::warden::vac_agent_image();

    if vac_foundation::container::warden_image_exists_locally(&image) {
        return Ok(());
    }

    println!("  Pulling sandbox image: {image}");
    println!();

    vac_foundation::container::pull_warden_image(&image).map_err(|e| {
        format!(
            "{e}\n\n\
             Troubleshoot:\n  \
             docker pull --platform linux/amd64 {image}    Pull manually\n  \
             VAC_AGENT_IMAGE=<img>                     Override image"
        )
    })?;

    println!();
    println!("  ✓ Sandbox image ready");
    Ok(())
}

fn build_probe_http_client() -> Option<reqwest::Client> {
    reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(2))
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .ok()
}

async fn endpoint_ok(client: &reqwest::Client, url: &str) -> bool {
    match client.get(url).send().await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

async fn wait_for_shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(_) => {
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }
}

#[cfg(test)]
mod tests;
