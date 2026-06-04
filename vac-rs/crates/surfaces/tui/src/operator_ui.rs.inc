// Spec-driven operator-console render models for the VAC TUI.
//
// The module owns deterministic semantic state and keeps legacy plain-text
// snapshot helpers for source/static gates. Production operator surfaces should
// use the `*_visual_lines` helpers so ratatui widget foreground, background, and
// modifier styles survive the off-screen buffer boundary.

use std::fmt;
use std::fs;
use std::path::Path;

#[cfg(vac_standalone_operator_ui)]
#[path = "operator_style.rs"]
mod operator_style;

#[cfg(not(vac_standalone_operator_ui))]
use crate::operator_style;

use operator_style::OperatorLineSpec;
use operator_style::OperatorSpanSpec;
use operator_style::OperatorStyleRole;
use operator_style::operator_line_specs_to_plain_text;
use operator_style::style_operator_text_specs;

pub(crate) const TOOL_TIMELINE_LIMIT: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ToolTimelineState {
    Queued,
    Running,
    Streaming,
    Passed,
    Failed,
    Cancelled,
}

impl ToolTimelineState {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Streaming => "streaming",
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    const fn glyph(self) -> &'static str {
        match self {
            Self::Queued => "○",
            Self::Running | Self::Streaming => "●",
            Self::Passed => "●",
            Self::Failed => "●",
            Self::Cancelled => "○",
        }
    }
}

impl fmt::Display for ToolTimelineState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StartupSnapshotRow {
    pub(crate) key: String,
    pub(crate) value: String,
    pub(crate) detail: String,
}

impl StartupSnapshotRow {
    pub(crate) fn new(
        key: impl Into<String>,
        value: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            detail: detail.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OperatorStatusBarState {
    pub(crate) mode: String,
    pub(crate) model: String,
    pub(crate) token_usage: String,
    pub(crate) automation: String,
    pub(crate) validation: String,
    pub(crate) profile: String,
    pub(crate) rulebook: String,
}

impl OperatorStatusBarState {
    pub(crate) fn input(model: impl Into<String>) -> Self {
        Self {
            mode: "INPUT".to_string(),
            model: model.into(),
            token_usage: "0 tok".to_string(),
            automation: "manual".to_string(),
            validation: "validity tracked by runtime gates".to_string(),
            profile: "profile runtime".to_string(),
            rulebook: "rulebook runtime".to_string(),
        }
    }

    pub(crate) fn conversation(
        model: impl Into<String>,
        token_usage: impl Into<String>,
        validation: impl Into<String>,
    ) -> Self {
        Self {
            mode: "CONVERSATION".to_string(),
            model: model.into(),
            token_usage: token_usage.into(),
            automation: "manual".to_string(),
            validation: validation.into(),
            profile: "profile runtime".to_string(),
            rulebook: "rulebook runtime".to_string(),
        }
    }

    pub(crate) fn workbench(
        model: impl Into<String>,
        token_usage: impl Into<String>,
        validation: impl Into<String>,
    ) -> Self {
        Self {
            mode: "WORKBENCH".to_string(),
            model: model.into(),
            token_usage: token_usage.into(),
            automation: "manual".to_string(),
            validation: validation.into(),
            profile: "profile runtime".to_string(),
            rulebook: "rulebook runtime".to_string(),
        }
    }

    pub(crate) fn with_profile_rulebook(
        mut self,
        profile: impl Into<String>,
        rulebook: impl Into<String>,
    ) -> Self {
        self.profile = profile.into();
        self.rulebook = rulebook.into();
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StartupSnapshot {
    pub(crate) version: String,
    pub(crate) mode: String,
    pub(crate) cwd: String,
    pub(crate) session: String,
    pub(crate) rows_left: Vec<StartupSnapshotRow>,
    pub(crate) rows_right: Vec<StartupSnapshotRow>,
    pub(crate) recent_task: Option<String>,
    pub(crate) status_bar: OperatorStatusBarState,
}

impl StartupSnapshot {
    pub(crate) fn from_session(
        version: impl Into<String>,
        provider: impl Into<String>,
        model: impl Into<String>,
        cwd: impl Into<String>,
        session: impl Into<String>,
    ) -> Self {
        let version = version.into();
        let provider = provider.into();
        let model = model.into();
        Self {
            version: version.clone(),
            mode: "interactive".to_string(),
            cwd: cwd.into(),
            session: session.into(),
            status_bar: OperatorStatusBarState::input(model.clone()),
            rows_left: vec![
                StartupSnapshotRow::new("version", version, "control-plane hardening active"),
                StartupSnapshotRow::new("provider", provider, "runtime selected"),
                StartupSnapshotRow::new("model", model, "runtime selected"),
                StartupSnapshotRow::new("fallback", "configured", "loaded from provider stack"),
                StartupSnapshotRow::new("profile", "runtime", "tool policy loaded"),
                StartupSnapshotRow::new("rulebook", "runtime", "control-plane contract active"),
            ],
            rows_right: vec![
                StartupSnapshotRow::new("runtime", "host", "in-process owner-native"),
                StartupSnapshotRow::new("isolation", "policy", "execution gates active"),
                StartupSnapshotRow::new(
                    "autopilot",
                    "runtime",
                    "scheduler state loaded if present",
                ),
                StartupSnapshotRow::new("mcp", "runtime", "configured connectors only"),
                StartupSnapshotRow::new("sessions", "runtime", "history loaded if present"),
                StartupSnapshotRow::new(
                    "workspace",
                    "registry",
                    "inventory resolved by control plane",
                ),
            ],
            recent_task: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RecentTaskLine {
    pub(crate) title: String,
    pub(crate) meta: String,
}

impl RecentTaskLine {
    pub(crate) fn new(title: impl Into<String>, meta: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            meta: meta.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct IdleViewState {
    pub(crate) engine_status: String,
    pub(crate) rulebook_status: String,
    pub(crate) recent_tasks: Vec<RecentTaskLine>,
    pub(crate) update_notice: Option<String>,
    pub(crate) bottom_mode: String,
    pub(crate) model: String,
    pub(crate) token_usage: String,
    pub(crate) validation: String,
    pub(crate) status_bar: OperatorStatusBarState,
}

impl IdleViewState {
    pub(crate) fn ready(model: impl Into<String>) -> Self {
        Self::live(model)
    }

    pub(crate) fn live(model: impl Into<String>) -> Self {
        let model = model.into();
        let status_bar = OperatorStatusBarState::input(model.clone());
        Self {
            engine_status: "local runtime connected".to_string(),
            rulebook_status: "control-plane gates active".to_string(),
            recent_tasks: vec![RecentTaskLine::new(
                "no persisted recent task loaded",
                ".vac/registry/trajectory is the runtime source of truth",
            )],
            update_notice: None,
            bottom_mode: "INPUT".to_string(),
            model,
            token_usage: "0 tok".to_string(),
            validation: "validity tracked by runtime gates".to_string(),
            status_bar,
        }
    }

    pub(crate) fn with_status_bar(mut self, status_bar: OperatorStatusBarState) -> Self {
        self.bottom_mode = status_bar.mode.clone();
        self.model = status_bar.model.clone();
        self.token_usage = status_bar.token_usage.clone();
        self.validation = status_bar.validation.clone();
        self.rulebook_status = format!("{} enforcing", status_bar.rulebook);
        self.status_bar = status_bar;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ToolTimelineEntry {
    pub(crate) name: String,
    pub(crate) target: String,
    pub(crate) state: ToolTimelineState,
    pub(crate) detail: String,
}

impl ToolTimelineEntry {
    pub(crate) fn new(
        name: impl Into<String>,
        target: impl Into<String>,
        state: ToolTimelineState,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            target: target.into(),
            state,
            detail: detail.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AgentStreamingState {
    pub(crate) user_prompt: String,
    pub(crate) user_time: String,
    pub(crate) agent_model: String,
    pub(crate) turn: u32,
    pub(crate) agent_time: String,
    pub(crate) agent_message: String,
    pub(crate) tools: Vec<ToolTimelineEntry>,
    pub(crate) thinking: String,
    pub(crate) context_used: u64,
    pub(crate) context_limit: u64,
    pub(crate) composer_hint: String,
    pub(crate) status_bar: OperatorStatusBarState,
}

pub(crate) fn render_operator_snapshot_text(
    scenario: SnapshotScenario,
    viewport: OperatorViewport,
) -> String {
    operator_line_specs_to_plain_text(&render_operator_snapshot_specs(scenario, viewport))
}

pub(crate) fn render_operator_snapshot_ansi_text(
    scenario: SnapshotScenario,
    viewport: OperatorViewport,
) -> String {
    style_operator_text_specs(&render_operator_snapshot_specs(scenario, viewport))
}

pub(crate) fn render_operator_snapshot_filename(scenario: SnapshotScenario) -> String {
    render_operator_snapshot_filename_for_viewport(scenario, scenario.default_viewport())
}

pub(crate) fn render_operator_snapshot_filename_for_viewport(
    scenario: SnapshotScenario,
    viewport: OperatorViewport,
) -> String {
    format!(
        "{}-{}x{}.txt",
        scenario.slug(),
        viewport.width,
        viewport.height
    )
}

impl AgentStreamingState {
    pub(crate) fn visible_tools(&self) -> &[ToolTimelineEntry] {
        let start = self.tools.len().saturating_sub(TOOL_TIMELINE_LIMIT);
        &self.tools[start..]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ApprovalRisk {
    Low,
    Medium,
    High,
    Destructive,
}

impl ApprovalRisk {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Low => "LOW",
            Self::Medium => "MEDIUM",
            Self::High => "HIGH",
            Self::Destructive => "DESTRUCTIVE",
        }
    }

    pub(crate) const fn requires_policy_gate(self) -> bool {
        matches!(self, Self::Medium | Self::High | Self::Destructive)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ApprovalSafetyContext {
    pub(crate) runtime_context: String,
    pub(crate) network_context: String,
    pub(crate) write_context: String,
}

impl ApprovalSafetyContext {
    pub(crate) fn host_write_network_inherit() -> Self {
        Self {
            runtime_context: "runtime host".to_string(),
            network_context: "network inherit".to_string(),
            write_context: "writes allowed".to_string(),
        }
    }

    pub(crate) fn render_inline(&self) -> String {
        format!(
            "{} · {} · {}",
            self.runtime_context, self.network_context, self.write_context
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ApprovalPopupState {
    pub(crate) action_kind: String,
    pub(crate) operation: String,
    pub(crate) cwd: String,
    pub(crate) safety_context: ApprovalSafetyContext,
    pub(crate) risk: ApprovalRisk,
    pub(crate) risk_detail: String,
    pub(crate) policy_reason: String,
    pub(crate) batch_progress: String,
    pub(crate) destructive: bool,
    pub(crate) status_bar: OperatorStatusBarState,
}

impl ApprovalPopupState {
    pub(crate) fn destructive_bash(command: impl Into<String>, cwd: impl Into<String>) -> Self {
        let risk = ApprovalRisk::Destructive;
        Self {
            action_kind: "BASH".to_string(),
            operation: command.into(),
            cwd: cwd.into(),
            safety_context: ApprovalSafetyContext::host_write_network_inherit(),
            risk,
            risk_detail: "recursive delete · no restore snapshot".to_string(),
            policy_reason: "vil.core → shell.destructive requires explicit approval".to_string(),
            batch_progress: "batch 1/2 — next: file_edit handlers/input.rs".to_string(),
            destructive: risk == ApprovalRisk::Destructive,
            status_bar: OperatorStatusBarState::conversation(
                "runtime",
                "0 tok",
                "approval pending",
            ),
        }
    }

    pub(crate) fn policy_gate_summary(&self) -> String {
        if self.risk.requires_policy_gate() {
            "policy-gated approval queue".to_string()
        } else {
            "policy-visible confirmation".to_string()
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AutopilotJobLine {
    pub(crate) state: String,
    pub(crate) kind: String,
    pub(crate) id: String,
    pub(crate) trigger: String,
    pub(crate) age: String,
    pub(crate) next_run: String,
}

impl AutopilotJobLine {
    pub(crate) fn new(
        state: impl Into<String>,
        kind: impl Into<String>,
        id: impl Into<String>,
        trigger: impl Into<String>,
        age: impl Into<String>,
        next_run: impl Into<String>,
    ) -> Self {
        Self {
            state: state.into(),
            kind: kind.into(),
            id: id.into(),
            trigger: trigger.into(),
            age: age.into(),
            next_run: next_run.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AutopilotActionSpec {
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) policy_gate: String,
}

impl AutopilotActionSpec {
    pub(crate) fn new(
        key: impl Into<String>,
        label: impl Into<String>,
        policy_gate: impl Into<String>,
    ) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            policy_gate: policy_gate.into(),
        }
    }

    pub(crate) fn render_hotkey(&self) -> String {
        format!("{} {}", self.key, self.label)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AutopilotSchedulerState {
    pub(crate) status: String,
    pub(crate) pid: String,
    pub(crate) uptime: String,
    pub(crate) mode: String,
    pub(crate) env: String,
    pub(crate) queued: u32,
    pub(crate) running: u32,
    pub(crate) jobs: Vec<AutopilotJobLine>,
    pub(crate) selected_job: String,
    pub(crate) task_graph: String,
    pub(crate) node_progress: String,
    pub(crate) tokens: String,
    pub(crate) spend: String,
    pub(crate) retry: String,
    pub(crate) actions: Vec<AutopilotActionSpec>,
    pub(crate) status_bar: OperatorStatusBarState,
}

impl AutopilotSchedulerState {
    pub(crate) fn default_monitor_actions() -> Vec<AutopilotActionSpec> {
        vec![
            AutopilotActionSpec::new("c", "cancel", "policy.autopilot.cancel"),
            AutopilotActionSpec::new("r", "retry", "policy.autopilot.retry"),
            AutopilotActionSpec::new("i", "open in editor", "policy.workspace.read"),
            AutopilotActionSpec::new("enter", "attach", "policy.session.attach"),
        ]
    }

    pub(crate) fn monitor_only_sample() -> Self {
        Self {
            status: "running".to_string(),
            pid: "48213".to_string(),
            uptime: "2h 14m".to_string(),
            mode: "monitor-only".to_string(),
            env: "host".to_string(),
            queued: 3,
            running: 1,
            jobs: vec![
                AutopilotJobLine::new(
                    "running",
                    "oneshot",
                    "7c0f1a",
                    "refactor handlers/input.rs",
                    "00:04:18",
                    "—",
                ),
                AutopilotJobLine::new(
                    "queued",
                    "cron",
                    "b82d4e",
                    "0 */6 * * * · validate",
                    "00:00:42",
                    "in 3h 41m",
                ),
                AutopilotJobLine::new(
                    "queued",
                    "filewatch",
                    "f19ac3",
                    "crates/vil_llm/**/*.rs",
                    "00:12:06",
                    "on write",
                ),
                AutopilotJobLine::new(
                    "ok",
                    "oneshot",
                    "2e8b71",
                    "bump dependencies",
                    "01:22:41",
                    "—",
                ),
                AutopilotJobLine::new(
                    "ok",
                    "cron",
                    "a4d90e",
                    "@hourly · mutation gate",
                    "00:18:11",
                    "in 41m",
                ),
                AutopilotJobLine::new(
                    "failed",
                    "oneshot",
                    "9aa0c2",
                    "knowledge loader: reindex",
                    "02:01:55",
                    "—",
                ),
                AutopilotJobLine::new(
                    "cancel",
                    "oneshot",
                    "0b7411",
                    "refactor workflow runner",
                    "yesterday",
                    "—",
                ),
            ],
            selected_job: "7c0f1a".to_string(),
            task_graph: "[plan→read→write→edit→check→done]".to_string(),
            node_progress: "node 3/6 · 62%".to_string(),
            tokens: "68,240 / 200k".to_string(),
            spend: "$0.142".to_string(),
            retry: "0/3".to_string(),
            actions: Self::default_monitor_actions(),
            status_bar: OperatorStatusBarState::workbench(
                "claude-sonnet-4.5",
                "68,240 tok",
                "valid 92%",
            )
            .with_profile_rulebook("profile default", "rulebook runtime"),
        }
    }

    pub(crate) fn from_workspace(workspace_root: &Path) -> Self {
        let status_path = workspace_root.join(".vac/registry/autopilot/status.yaml");
        let source = fs::read_to_string(status_path).unwrap_or_default();
        let status = yaml_scalar_by_key(&source, "status").unwrap_or_else(|| "idle".to_string());
        let queued = yaml_scalar_by_key(&source, "queued")
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        let running = yaml_scalar_by_key(&source, "running")
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        let pid = yaml_scalar_by_key(&source, "pid").unwrap_or_else(|| "—".to_string());
        let uptime = yaml_scalar_by_key(&source, "uptime").unwrap_or_else(|| "—".to_string());
        let mode =
            yaml_scalar_by_key(&source, "mode").unwrap_or_else(|| "monitor-only".to_string());
        let jobs = autopilot_jobs_from_status_yaml(&source);
        let selected_job = jobs
            .first()
            .map(|job| job.id.clone())
            .unwrap_or_else(|| "none".to_string());
        let tokens = yaml_scalar_by_key(&source, "tokens").unwrap_or_else(|| "0 / 0".to_string());
        let validation =
            yaml_scalar_by_key(&source, "validation").unwrap_or_else(|| format!("runtime {status}"));
        let model = yaml_scalar_by_key(&source, "model").unwrap_or_else(|| "runtime".to_string());
        let profile = yaml_scalar_by_key(&source, "profile")
            .map(|value| format!("profile {value}"))
            .unwrap_or_else(|| "profile runtime".to_string());
        let rulebook = yaml_scalar_by_key(&source, "rulebook")
            .map(|value| format!("rulebook {value}"))
            .unwrap_or_else(|| "rulebook runtime".to_string());
        Self {
            status,
            pid,
            uptime,
            mode,
            env: yaml_scalar_by_key(&source, "env").unwrap_or_else(|| "host".to_string()),
            queued,
            running,
            jobs,
            selected_job,
            task_graph: yaml_scalar_by_key(&source, "task_graph")
                .unwrap_or_else(|| "[idle]".to_string()),
            node_progress: yaml_scalar_by_key(&source, "node_progress")
                .unwrap_or_else(|| "node 0/0 · 0%".to_string()),
            tokens: tokens.clone(),
            spend: yaml_scalar_by_key(&source, "spend").unwrap_or_else(|| "$0.000".to_string()),
            retry: yaml_scalar_by_key(&source, "retry").unwrap_or_else(|| "0/0".to_string()),
            actions: Self::default_monitor_actions(),
            status_bar: OperatorStatusBarState::workbench(model, tokens, validation)
                .with_profile_rulebook(profile, rulebook),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CapabilityManifestStatus {
    Ready,
    Partial,
    Planned,
    Deprecated,
    Disabled,
    Broken,
    Experimental,
    CliOnly,
}

impl CapabilityManifestStatus {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Partial => "partial",
            Self::Planned => "planned",
            Self::Deprecated => "deprecated",
            Self::Disabled => "disabled",
            Self::Broken => "broken",
            Self::Experimental => "experimental",
            Self::CliOnly => "cli-only",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "ready" => Some(Self::Ready),
            "partial" => Some(Self::Partial),
            "planned" => Some(Self::Planned),
            "deprecated" => Some(Self::Deprecated),
            "disabled" => Some(Self::Disabled),
            "broken" => Some(Self::Broken),
            "experimental" => Some(Self::Experimental),
            "cli-only" | "cli_only" => Some(Self::CliOnly),
            _ => None,
        }
    }

    pub(crate) const fn is_control_plane_valid(self) -> bool {
        matches!(
            self,
            Self::Ready
                | Self::Partial
                | Self::Planned
                | Self::Deprecated
                | Self::Experimental
                | Self::CliOnly
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CapabilityDashboardRecord {
    pub(crate) id: String,
    pub(crate) status: CapabilityManifestStatus,
    pub(crate) owner: String,
    pub(crate) surfaces: String,
    pub(crate) policy: String,
    pub(crate) validation: String,
}

impl CapabilityDashboardRecord {
    pub(crate) fn new(
        id: impl Into<String>,
        status: CapabilityManifestStatus,
        owner: impl Into<String>,
        surfaces: impl Into<String>,
        policy: impl Into<String>,
        validation: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            status,
            owner: owner.into(),
            surfaces: surfaces.into(),
            policy: policy.into(),
            validation: validation.into(),
        }
    }

    pub(crate) fn render_table_row(&self) -> String {
        format!(
            "{} | {} | {} | {} | {} | {}",
            self.id,
            self.status.as_str(),
            self.owner,
            self.surfaces,
            self.policy,
            self.validation
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

impl DiagnosticSeverity {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ControlPlaneDiagnostic {
    pub(crate) severity: DiagnosticSeverity,
    pub(crate) source: String,
    pub(crate) message: String,
}

impl ControlPlaneDiagnostic {
    pub(crate) fn new(
        severity: DiagnosticSeverity,
        source: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            source: source.into(),
            message: message.into(),
        }
    }

    pub(crate) fn from_text(value: impl Into<String>) -> Self {
        let message = value.into();
        let lower = message.to_ascii_lowercase();
        let severity = if lower.contains("error")
            || lower.contains("invalid")
            || lower.contains("broken")
            || lower.contains("failed")
        {
            DiagnosticSeverity::Error
        } else if lower.contains("warn") || lower.contains("degraded") || lower.contains("missing")
        {
            DiagnosticSeverity::Warning
        } else {
            DiagnosticSeverity::Info
        };
        Self::new(severity, "control-plane", message)
    }

    pub(crate) fn render_line(&self) -> String {
        format!(
            "{} {}: {}",
            self.severity.as_str(),
            self.source,
            self.message
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CapabilityDashboardState {
    pub(crate) chrome_title: String,
    pub(crate) chrome_mode: String,
    pub(crate) capability_count: usize,
    pub(crate) owned_domains: usize,
    pub(crate) unowned_domains: usize,
    pub(crate) valid_percent: u8,
    pub(crate) registry_summary: Vec<String>,
    pub(crate) capability_rows: Vec<String>,
    pub(crate) diagnostics: Vec<String>,
    pub(crate) capability_records: Vec<CapabilityDashboardRecord>,
    pub(crate) diagnostic_records: Vec<ControlPlaneDiagnostic>,
    pub(crate) status_bar: OperatorStatusBarState,
}

impl CapabilityDashboardState {
    pub(crate) fn from_manifest_records(
        capability_records: Vec<CapabilityDashboardRecord>,
        diagnostic_records: Vec<ControlPlaneDiagnostic>,
        registry_summary: Vec<String>,
        owned_domains: usize,
        unowned_domains: usize,
    ) -> Self {
        let capability_count = capability_records.len();
        let valid_count = capability_records
            .iter()
            .filter(|record| record.status.is_control_plane_valid())
            .count();
        let valid_percent = if capability_count == 0 {
            0
        } else {
            ((valid_count * 100) / capability_count).min(100) as u8
        };
        let capability_rows = capability_records
            .iter()
            .map(CapabilityDashboardRecord::render_table_row)
            .collect();
        let diagnostics = diagnostic_records
            .iter()
            .map(ControlPlaneDiagnostic::render_line)
            .collect();
        Self {
            chrome_title: "vac · interactive".to_string(),
            chrome_mode: "registry /capabilities".to_string(),
            capability_count,
            owned_domains,
            unowned_domains,
            valid_percent,
            registry_summary,
            capability_rows,
            diagnostics,
            capability_records,
            diagnostic_records,
            status_bar: OperatorStatusBarState::conversation(
                "runtime",
                "0 tok",
                format!("valid {valid_percent}%"),
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OperatorViewport {
    pub(crate) width: usize,
    pub(crate) height: usize,
}

impl OperatorViewport {
    pub(crate) const VISUAL_FIDELITY_MATRIX: [Self; 3] = [
        Self {
            width: 120,
            height: 36,
        },
        Self {
            width: 140,
            height: 40,
        },
        Self {
            width: 180,
            height: 48,
        },
    ];

    pub(crate) fn new(width: usize, height: usize) -> Self {
        Self {
            width: width.clamp(72, 220),
            height: height.clamp(24, 80),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SnapshotScenario {
    FirstLaunch,
    Idle,
    AgentWorking,
    ApprovalPopup,
    RuntimeJobs,
    CapabilityDashboard,
}

impl SnapshotScenario {
    pub(crate) const ALL: [Self; 6] = [
        Self::FirstLaunch,
        Self::Idle,
        Self::AgentWorking,
        Self::ApprovalPopup,
        Self::RuntimeJobs,
        Self::CapabilityDashboard,
    ];

    pub(crate) const fn slug(self) -> &'static str {
        match self {
            Self::FirstLaunch => "first-launch",
            Self::Idle => "idle",
            Self::AgentWorking => "agent-working",
            Self::ApprovalPopup => "approval-popup",
            Self::RuntimeJobs => "runtime-jobs",
            Self::CapabilityDashboard => "capability-dashboard",
        }
    }

    pub(crate) const fn title(self) -> &'static str {
        match self {
            Self::FirstLaunch => "first launch",
            Self::Idle => "idle",
            Self::AgentWorking => "agent working",
            Self::ApprovalPopup => "destructive bash approval",
            Self::RuntimeJobs => "runtime jobs",
            Self::CapabilityDashboard => "capability dashboard",
        }
    }

    pub(crate) const fn default_viewport(self) -> OperatorViewport {
        match self {
            Self::CapabilityDashboard => OperatorViewport {
                width: 180,
                height: 48,
            },
            _ => OperatorViewport {
                width: 120,
                height: 36,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OperatorSemanticScreen {
    FirstLaunch,
    Idle,
    AgentStreaming,
    ApprovalPopup,
    AutopilotScheduler,
    CapabilityDashboard,
}

impl From<SnapshotScenario> for OperatorSemanticScreen {
    fn from(value: SnapshotScenario) -> Self {
        match value {
            SnapshotScenario::FirstLaunch => Self::FirstLaunch,
            SnapshotScenario::Idle => Self::Idle,
            SnapshotScenario::AgentWorking => Self::AgentStreaming,
            SnapshotScenario::ApprovalPopup => Self::ApprovalPopup,
            SnapshotScenario::RuntimeJobs => Self::AutopilotScheduler,
            SnapshotScenario::CapabilityDashboard => Self::CapabilityDashboard,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OperatorChromeState {
    pub(crate) title: String,
    pub(crate) mode: String,
    pub(crate) active_tab: String,
    pub(crate) tabs: Vec<String>,
    pub(crate) bottom_mode: String,
    pub(crate) model: String,
    pub(crate) token_usage: String,
    pub(crate) automation_mode: String,
    pub(crate) validation: String,
    pub(crate) lsp: String,
    pub(crate) profile: String,
    pub(crate) rulebook: String,
    pub(crate) composer_hint: String,
}

impl OperatorChromeState {
    pub(crate) fn for_mode(mode: impl Into<String>, active_tab: impl Into<String>) -> Self {
        Self {
            title: "vac".to_string(),
            mode: mode.into(),
            active_tab: active_tab.into(),
            tabs: vec![
                "chat".to_string(),
                "runtime".to_string(),
                "review".to_string(),
                "workbench".to_string(),
                "mcp".to_string(),
            ],
            bottom_mode: "CONVERSATION".to_string(),
            model: "claude-sonnet-4.5".to_string(),
            token_usage: "68,240 tok".to_string(),
            automation_mode: "manual".to_string(),
            validation: "valid 92%".to_string(),
            lsp: "lsp E0 W3".to_string(),
            profile: "profile default".to_string(),
            rulebook: "rulebook runtime".to_string(),
            composer_hint: "/ commands   @ files   shift+tab plan   ctrl+j newline".to_string(),
        }
    }

    pub(crate) fn input_mode(mode: impl Into<String>, active_tab: impl Into<String>) -> Self {
        let mut state = Self::for_mode(mode, active_tab);
        state.bottom_mode = "INPUT".to_string();
        state.token_usage = "0 tok".to_string();
        state.validation = "valid 100%".to_string();
        state.lsp = "lsp E0 W0".to_string();
        state
    }

    pub(crate) fn workbench_mode(mode: impl Into<String>) -> Self {
        let mut state = Self::for_mode(mode, "runtime");
        state.bottom_mode = "WORKBENCH".to_string();
        state.composer_hint =
            "runtime jobs monitor — c cancel · r retry · enter attach".to_string();
        state
    }
}

pub(crate) fn render_first_launch_visual_lines(
    snapshot: &StartupSnapshot,
    width: u16,
) -> Vec<ratatui::text::Line<'static>> {
    crate::operator_widget_render::render_startup_lines(snapshot, width)
}

pub(crate) fn render_idle_visual_lines(
    state: &IdleViewState,
    width: u16,
) -> Vec<ratatui::text::Line<'static>> {
    crate::operator_widget_render::render_idle_lines(state, width)
}

#[allow(dead_code)]
pub(crate) fn render_agent_streaming_visual_lines(
    state: &AgentStreamingState,
    width: u16,
) -> Vec<ratatui::text::Line<'static>> {
    crate::operator_widget_render::render_agent_streaming_lines(state, width)
}

#[allow(dead_code)]
pub(crate) fn render_approval_popup_visual_lines(
    state: &ApprovalPopupState,
    width: u16,
) -> Vec<ratatui::text::Line<'static>> {
    crate::operator_widget_render::render_approval_lines(state, width)
}

pub(crate) fn render_autopilot_scheduler_visual_lines(
    state: &AutopilotSchedulerState,
    width: u16,
) -> Vec<ratatui::text::Line<'static>> {
    crate::operator_widget_render::render_runtime_jobs_lines(state, width)
}

pub(crate) fn render_capability_dashboard_visual_lines(
    state: &CapabilityDashboardState,
    width: u16,
) -> Vec<ratatui::text::Line<'static>> {
    crate::operator_widget_render::render_capability_dashboard_lines(state, width)
}

#[allow(dead_code)]
pub(crate) fn render_first_launch_lines(snapshot: &StartupSnapshot) -> Vec<String> {
    crate::operator_widget_render::render_startup_lines(snapshot, 112)
        .into_iter()
        .map(crate::operator_widget_render::line_to_string)
        .collect()
}

#[allow(dead_code)]
pub(crate) fn render_idle_lines(state: &IdleViewState) -> Vec<String> {
    crate::operator_widget_render::render_idle_lines(state, 112)
        .into_iter()
        .map(crate::operator_widget_render::line_to_string)
        .collect()
}

pub(crate) fn render_agent_streaming_lines(state: &AgentStreamingState) -> Vec<String> {
    crate::operator_widget_render::render_agent_streaming_lines(state, 112)
        .into_iter()
        .map(crate::operator_widget_render::line_to_string)
        .collect()
}

pub(crate) fn render_approval_popup_lines(state: &ApprovalPopupState) -> Vec<String> {
    crate::operator_widget_render::render_approval_lines(state, 112)
        .into_iter()
        .map(crate::operator_widget_render::line_to_string)
        .collect()
}

pub(crate) fn render_autopilot_scheduler_lines(state: &AutopilotSchedulerState) -> Vec<String> {
    crate::operator_widget_render::render_runtime_jobs_lines(state, 112)
        .into_iter()
        .map(crate::operator_widget_render::line_to_string)
        .collect()
}

pub(crate) fn render_autopilot_scheduler_line_specs(
    state: &AutopilotSchedulerState,
) -> Vec<OperatorLineSpec> {
    annotate_operator_lines(
        OperatorSemanticScreen::AutopilotScheduler,
        render_autopilot_scheduler_lines(state),
        false,
    )
}

pub(crate) fn render_capability_dashboard_shell(state: &CapabilityDashboardState) -> Vec<String> {
    render_capability_dashboard_body_lines(state, 240)
}

pub(crate) fn render_capability_dashboard_shell_specs(
    state: &CapabilityDashboardState,
) -> Vec<OperatorLineSpec> {
    annotate_operator_lines(
        OperatorSemanticScreen::CapabilityDashboard,
        render_capability_dashboard_shell(state),
        false,
    )
}

pub(crate) fn render_operator_snapshot_specs(
    scenario: SnapshotScenario,
    viewport: OperatorViewport,
) -> Vec<OperatorLineSpec> {
    annotate_operator_lines(
        OperatorSemanticScreen::from(scenario),
        render_operator_snapshot(scenario, viewport),
        true,
    )
}

pub(crate) fn render_operator_snapshot(
    scenario: SnapshotScenario,
    viewport: OperatorViewport,
) -> Vec<String> {
    crate::operator_widget_render::render_snapshot_lines(scenario, viewport)
}


impl AgentStreamingState {
    pub(crate) fn sample() -> Self {
        Self {
            user_prompt: "refactor vac-rs/crates/surfaces/tui/src/app.rs — extract popup routing"
                .to_string(),
            user_time: "14:22:06".to_string(),
            agent_model: "claude-sonnet-4.5".to_string(),
            turn: 3,
            agent_time: "14:22:08".to_string(),
            agent_message: "The controller file is ~83k bytes — monolithic. I'll read it first,\nthen extract popup routing into handlers/popup.rs per Wave 1.".to_string(),
            tools: vec![
                ToolTimelineEntry::new(
                    "glob",
                    "vac-rs/crates/surfaces/tui/src/**/*.rs",
                    ToolTimelineState::Passed,
                    "0.1s · 88 files",
                ),
                ToolTimelineEntry::new(
                    "file_read",
                    "handlers/input.rs",
                    ToolTimelineState::Passed,
                    "1.2s · 82,969 B",
                ),
                ToolTimelineEntry::new(
                    "grep",
                    "\"fn handle_popup\" in handlers/",
                    ToolTimelineState::Passed,
                    "0.3s · 8 matches",
                ),
                ToolTimelineEntry::new(
                    "file_write",
                    "handlers/popup.rs",
                    ToolTimelineState::Streaming,
                    "streaming… ███████░░",
                ),
                ToolTimelineEntry::new(
                    "file_edit",
                    "handlers/input.rs (-420 planned)",
                    ToolTimelineState::Queued,
                    "queued",
                ),
                ToolTimelineEntry::new(
                    "cargo check",
                    "-p vac-surface-tui",
                    ToolTimelineState::Queued,
                    "queued",
                ),
            ],
            thinking: "preserving precedence with backend-event skip semantics".to_string(),
            context_used: 68_240,
            context_limit: 200_000,
            composer_hint: "agent working — next message queues (esc to interrupt)".to_string(),
            status_bar: OperatorStatusBarState::conversation(
                "claude-sonnet-4.5",
                "68,240 tok",
                "valid 92%",
            )
            .with_profile_rulebook("profile default", "rulebook runtime"),
        }
    }
}

impl CapabilityDashboardState {
    pub(crate) fn sample_dashboard() -> Self {
        Self::from_manifest_records(
            vec![
                CapabilityDashboardRecord::new(
                    "vac.tui",
                    CapabilityManifestStatus::Ready,
                    "vac-surface-tui",
                    "/, /capabilities, /workflow",
                    "operator_read, approval_surface",
                    "snapshot + runtime contracts",
                ),
                CapabilityDashboardRecord::new(
                    "vac.workflow",
                    CapabilityManifestStatus::Ready,
                    "vac-control-plane",
                    "/workflow, cli",
                    "dry_run_safe, approval_trace",
                    "workflow doctor supported",
                ),
                CapabilityDashboardRecord::new(
                    "vac.init.control-plane",
                    CapabilityManifestStatus::Ready,
                    "vac-control-plane",
                    "cli, tui registry views",
                    "fail_closed, manifest_owned",
                    "doctor registry/surface/docs",
                ),
            ],
            Vec::new(),
            vec![
                ".vac/capabilities/ manifest scan loaded from root registry".to_string(),
                ".vac/policies/ strict approval gates wired".to_string(),
                "vac-surface-tui surface route summary ready".to_string(),
            ],
            3,
            0,
        )
    }
}

fn annotate_operator_lines(
    screen: OperatorSemanticScreen,
    lines: Vec<String>,
    include_chrome: bool,
) -> Vec<OperatorLineSpec> {
    let status_start = lines.len().saturating_sub(4);
    lines
        .into_iter()
        .enumerate()
        .map(|(index, line)| {
            if include_chrome && index <= 2 {
                return OperatorLineSpec::styled(OperatorStyleRole::Chrome, line);
            }
            if include_chrome && index >= status_start {
                return semantic_chrome_footer_line(line);
            }
            semantic_body_line(screen, line)
        })
        .collect()
}

fn semantic_chrome_footer_line(line: String) -> OperatorLineSpec {
    if line.contains("profile ") || line.contains("rulebook") {
        OperatorLineSpec::styled(OperatorStyleRole::Status, line)
    } else {
        OperatorLineSpec::styled(OperatorStyleRole::Chrome, line)
    }
}

fn semantic_body_line(screen: OperatorSemanticScreen, line: String) -> OperatorLineSpec {
    if line.trim().is_empty() {
        return OperatorLineSpec::styled(OperatorStyleRole::Muted, line);
    }
    if line.chars().all(|ch| ch == '─') || line.starts_with('┌') || line.starts_with('└') {
        return OperatorLineSpec::styled(OperatorStyleRole::Chrome, line);
    }

    match screen {
        OperatorSemanticScreen::FirstLaunch => semantic_first_launch_line(line),
        OperatorSemanticScreen::Idle => semantic_idle_line(line),
        OperatorSemanticScreen::AgentStreaming => semantic_agent_streaming_line(line),
        OperatorSemanticScreen::ApprovalPopup => semantic_approval_line(line),
        OperatorSemanticScreen::AutopilotScheduler => semantic_autopilot_line(line),
        OperatorSemanticScreen::CapabilityDashboard => semantic_capability_dashboard_line(line),
    }
}

fn semantic_first_launch_line(line: String) -> OperatorLineSpec {
    if line.contains("Vastar Agentic CLI") || line.contains("hydrating startup snapshot") {
        OperatorLineSpec::styled(OperatorStyleRole::Accent, line)
    } else if line.trim() == "ready" || line.contains("[VAC]") {
        OperatorLineSpec::styled(OperatorStyleRole::Success, line)
    } else if line.contains("version") || line.contains("provider") || line.contains("runtime") {
        OperatorLineSpec::styled(OperatorStyleRole::Status, line)
    } else {
        OperatorLineSpec::styled(OperatorStyleRole::Muted, line)
    }
}

fn semantic_idle_line(line: String) -> OperatorLineSpec {
    if line.contains("[VAC]") || line.contains("✓ validated") {
        OperatorLineSpec::styled(OperatorStyleRole::Success, line)
    } else if line.contains("recent tasks") || line.contains("update") {
        OperatorLineSpec::styled(OperatorStyleRole::Accent, line)
    } else if line.contains("▲ below gate") {
        OperatorLineSpec::styled(OperatorStyleRole::Warning, line)
    } else {
        OperatorLineSpec::styled(OperatorStyleRole::Status, line)
    }
}

fn semantic_agent_streaming_line(line: String) -> OperatorLineSpec {
    if line.starts_with("you") || line.trim_start().starts_with("refactor") {
        OperatorLineSpec::styled(OperatorStyleRole::User, line)
    } else if line.starts_with("vac") || line.contains("thinking") {
        OperatorLineSpec::styled(OperatorStyleRole::Agent, line)
    } else if line.contains("tool timeline") || line.contains("context") {
        OperatorLineSpec::styled(OperatorStyleRole::Accent, line)
    } else if line.contains("failed") {
        OperatorLineSpec::styled(OperatorStyleRole::Danger, line)
    } else if line.contains("queued") {
        OperatorLineSpec::styled(OperatorStyleRole::Muted, line)
    } else if line.contains("passed") {
        OperatorLineSpec::styled(OperatorStyleRole::Success, line)
    } else {
        OperatorLineSpec::styled(OperatorStyleRole::Status, line)
    }
}

fn semantic_approval_line(line: String) -> OperatorLineSpec {
    if line.contains("DESTRUCTIVE") {
        return highlight_keyword(
            line,
            "DESTRUCTIVE",
            OperatorStyleRole::Warning,
            OperatorStyleRole::Danger,
        );
    }
    if line.contains("approval required") || line.contains("risk") || line.contains("policy") {
        OperatorLineSpec::styled(OperatorStyleRole::Warning, line)
    } else if line.trim_start().starts_with('$') {
        OperatorLineSpec::styled(OperatorStyleRole::Danger, line)
    } else if line.contains("approve once") || line.contains("reject with reason") {
        OperatorLineSpec::styled(OperatorStyleRole::Accent, line)
    } else if line.contains("passed") {
        OperatorLineSpec::styled(OperatorStyleRole::Success, line)
    } else {
        OperatorLineSpec::styled(OperatorStyleRole::Status, line)
    }
}

fn semantic_autopilot_line(line: String) -> OperatorLineSpec {
    if line.contains("autopilot") || line.contains("runtime jobs") || line.contains("task graph") {
        OperatorLineSpec::styled(OperatorStyleRole::Accent, line)
    } else if line.contains("monitor-only") || line.contains("approval gates") {
        OperatorLineSpec::styled(OperatorStyleRole::Warning, line)
    } else if line.contains("failed") {
        OperatorLineSpec::styled(OperatorStyleRole::Danger, line)
    } else if line.contains("running") || line.contains(" ok ") || line.contains("● ok") {
        OperatorLineSpec::styled(OperatorStyleRole::Success, line)
    } else if line.contains("queued") || line.contains("cancel") {
        OperatorLineSpec::styled(OperatorStyleRole::Muted, line)
    } else {
        OperatorLineSpec::styled(OperatorStyleRole::Status, line)
    }
}

fn semantic_capability_dashboard_line(line: String) -> OperatorLineSpec {
    if line.contains("Capability Dashboard")
        || line.contains("Control plane registry")
        || line.contains("metrics")
    {
        OperatorLineSpec::styled(OperatorStyleRole::Accent, line)
    } else if line.contains("Diagnostics") || line.contains("YAML diagnostic") {
        OperatorLineSpec::styled(OperatorStyleRole::Warning, line)
    } else if line.contains("broken") || line.contains("invalid enum") {
        OperatorLineSpec::styled(OperatorStyleRole::Danger, line)
    } else if line.contains("ready") || line.contains("owned") {
        OperatorLineSpec::styled(OperatorStyleRole::Success, line)
    } else {
        OperatorLineSpec::styled(OperatorStyleRole::Status, line)
    }
}

fn highlight_keyword(
    line: String,
    keyword: &str,
    base_role: OperatorStyleRole,
    keyword_role: OperatorStyleRole,
) -> OperatorLineSpec {
    let Some(index) = line.find(keyword) else {
        return OperatorLineSpec::styled(base_role, line);
    };
    let before = &line[..index];
    let highlighted = &line[index..index + keyword.len()];
    let after = &line[index + keyword.len()..];
    let mut spans = Vec::new();
    if !before.is_empty() {
        spans.push(OperatorSpanSpec::new(base_role, before));
    }
    spans.push(OperatorSpanSpec::new(keyword_role, highlighted));
    if !after.is_empty() {
        spans.push(OperatorSpanSpec::new(base_role, after));
    }
    OperatorLineSpec::from_spans(spans)
}

fn render_screen(
    chrome: &OperatorChromeState,
    mut body: Vec<String>,
    viewport: OperatorViewport,
) -> Vec<String> {
    let width = viewport.width;
    let body_height = viewport.height.saturating_sub(7).max(1);
    let mut lines = Vec::with_capacity(viewport.height);
    lines.push(fit_line(&render_titlebar(chrome), width));
    lines.push(horizontal_rule(width));
    lines.push(fit_line(&render_tabbar(chrome, width), width));

    if body.len() > body_height {
        body.truncate(body_height);
    }
    for line in body {
        lines.push(fit_line(&line, width));
    }
    while lines.len() < viewport.height.saturating_sub(4) {
        lines.push(" ".repeat(width));
    }

    lines.extend(render_composer_lines("▸", &chrome.composer_hint, width));
    lines.push(render_statusline(chrome, width));
    lines.truncate(viewport.height);
    while lines.len() < viewport.height {
        lines.push(" ".repeat(width));
    }
    lines
}

fn render_titlebar(chrome: &OperatorChromeState) -> String {
    format!(
        "● ● ●  {} · interactive — {}",
        chrome.title.trim(),
        chrome.mode.trim()
    )
}

fn render_tabbar(chrome: &OperatorChromeState, width: usize) -> String {
    let tabs = chrome
        .tabs
        .iter()
        .map(|tab| {
            if tab == &chrome.active_tab {
                format!("[{}]", tab)
            } else {
                tab.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" · ");
    let tab_index = chrome
        .tabs
        .iter()
        .position(|tab| tab == &chrome.active_tab)
        .map(|index| format!("tab {}/{}", index + 1, chrome.tabs.len()))
        .unwrap_or_default();
    fit_line(
        &format!("{tabs}"),
        width.saturating_sub(tab_index.len() + 2),
    ) + &format!(" {tab_index}")
}

fn render_first_launch_body_lines(snapshot: &StartupSnapshot, width: usize) -> Vec<String> {
    let columns = width.saturating_sub(4).max(48);
    let column_width = columns / 2;
    let mut rows = Vec::new();
    rows.push("".to_string());
    rows.push(format!("     Vastar Agentic CLI  v{}", snapshot.version));
    rows.push(
        "     VAC operator console · local runtime · control-plane hardening active"
            .to_string(),
    );
    rows.push(format!(
        "     cwd {}   ·   session {}   ·   mode {}",
        snapshot.cwd, snapshot.session, snapshot.mode
    ));
    rows.push("".to_string());
    rows.push("hydrating startup snapshot".to_string());
    rows.push(horizontal_rule(width.saturating_sub(2)));
    for index in 0..snapshot.rows_left.len().max(snapshot.rows_right.len()) {
        let left = snapshot
            .rows_left
            .get(index)
            .map(format_snapshot_row)
            .unwrap_or_default();
        let right = snapshot
            .rows_right
            .get(index)
            .map(format_snapshot_row)
            .unwrap_or_default();
        rows.push(format!(
            "  {:<left_width$}  {}",
            left,
            fit_line(&right, column_width.saturating_sub(3)),
            left_width = column_width.saturating_sub(2)
        ));
    }
    rows.push("".to_string());
    rows.push("ready".to_string());
    rows.push(
        "[VAC]  type / for commands, /help to explore, or start typing a task.".to_string(),
    );
    if let Some(recent) = snapshot.recent_task.as_ref() {
        rows.push(format!("recent  \"{recent}\"  · 2h ago"));
    }
    rows
}

fn render_idle_body_lines(state: &IdleViewState, width: usize) -> Vec<String> {
    let mut rows = vec![
        "".to_string(),
        format!(
            "[VAC]  ready  — {} · {}",
            state.engine_status, state.rulebook_status
        ),
        "".to_string(),
        "The agent will stream thinking, tools, and approvals here.".to_string(),
        "Keyboard-only:  [tab] focus   [/] commands   [@] files   [shift+tab] plan mode"
            .to_string(),
        "".to_string(),
        "recent tasks".to_string(),
        horizontal_rule(width.saturating_sub(2).min(60)),
    ];
    for task in &state.recent_tasks {
        rows.push(format!("▸ {:<42} {}", task.title, task.meta));
    }
    rows.push("".to_string());
    if let Some(notice) = &state.update_notice {
        rows.extend(box_lines(
            "update",
            &[notice.clone()],
            width.saturating_sub(8).min(108),
        ));
    }
    rows
}

fn render_agent_turn_metadata_line(state: &AgentStreamingState) -> String {
    format!(
        "vac  {} · turn {} · {} · metadata context {} / {}",
        state.agent_model,
        state.turn,
        state.agent_time,
        compact_number(state.context_used),
        compact_number(state.context_limit)
    )
}

fn render_agent_streaming_body_lines(state: &AgentStreamingState, width: usize) -> Vec<String> {
    let mut lines = vec![
        "".to_string(),
        format!("you  {}", state.user_time),
        format!("  {}", state.user_prompt),
        "".to_string(),
        render_agent_turn_metadata_line(state),
    ];
    for line in state.agent_message.lines() {
        lines.push(format!("  {line}"));
    }
    lines.push("".to_string());
    let omitted = state.tools.len().saturating_sub(TOOL_TIMELINE_LIMIT);
    let title = if omitted == 0 {
        "— tool timeline".to_string()
    } else {
        format!("— tool timeline (last {TOOL_TIMELINE_LIMIT}; +{omitted} earlier)")
    };
    lines.push(title);
    for tool in state.visible_tools() {
        lines.push(format!(
            "{} {:<12} {:<32} {:<11} {}",
            tool.state.glyph(),
            tool.name,
            fit_line(&tool.target, 32),
            tool.state,
            tool.detail
        ));
    }
    lines.push("".to_string());
    lines.push(format!("⌁ thinking · {} ▌", state.thinking));
    let bar = context_bar(state.context_used, state.context_limit, 18);
    lines.push(format!(
        "context {bar}  {:>6} / {:<6} · esc to interrupt",
        compact_number(state.context_used),
        compact_number(state.context_limit)
    ));
    lines.push(format!("input hint · {}", state.composer_hint));
    lines
        .into_iter()
        .map(|line| fit_line(&line, width))
        .collect()
}

fn render_approval_popup_body_lines(state: &ApprovalPopupState, width: usize) -> Vec<String> {
    let risk = if state.destructive {
        format!("{}  {}", state.risk.label(), state.risk_detail)
    } else {
        format!("{}  {}", state.risk.label(), state.risk_detail)
    };
    let rows = vec![
        "approval required".to_string(),
        format!(
            "{}  the agent wants to run a shell command",
            state.action_kind
        ),
        "".to_string(),
        format!("$ {}", state.operation),
        format!("cwd      {}", state.cwd),
        format!("context  {}", state.safety_context.render_inline()),
        format!("risk     {risk}"),
        format!(
            "policy   {} · {}",
            state.policy_gate_summary(),
            state.policy_reason
        ),
        "".to_string(),
        "y approve once   a approve+remember   n reject   r reject with reason".to_string(),
        state.batch_progress.clone(),
    ];
    let mut lines = vec![
        "".to_string(),
        "cargo check  passed · 2 warnings".to_string(),
        "".to_string(),
    ];
    lines.extend(box_lines(
        "approval required",
        &rows,
        width.saturating_sub(12).clamp(64, 120),
    ));
    lines
}

fn render_autopilot_scheduler_body_lines(
    state: &AutopilotSchedulerState,
    width: usize,
) -> Vec<String> {
    let mut lines = vec![
        format!(
            "autopilot ● {}   pid {}   uptime {}   mode {}   env {}   ·   queue {}   ·   running {}",
            state.status,
            state.pid,
            state.uptime,
            state.mode,
            state.env,
            state.queued,
            state.running
        ),
        "".to_string(),
        format!(
            "  {:<10} {:<10} {:<8} {:<34} {:<12} {}",
            "state", "kind", "id", "trigger", "age", "next-run"
        ),
        format!("  {}", horizontal_rule(width.saturating_sub(12).min(108))),
    ];
    for job in &state.jobs {
        lines.push(format!(
            "{} {:<10} {:<10} {:<8} {:<34} {:<12} {}",
            job_state_glyph(job.state.as_str()),
            job.state,
            job.kind,
            job.id,
            fit_line(&job.trigger, 34),
            job.age,
            job.next_run
        ));
    }
    lines.push("".to_string());
    let inspect_rows = vec![
        format!("task graph {}   {}", state.task_graph, state.node_progress),
        format!(
            "tokens {}  {}   spend {}   retry {}",
            state.tokens,
            context_bar(68_240, 200_000, 12),
            state.spend,
            state.retry
        ),
        state
            .actions
            .iter()
            .map(AutopilotActionSpec::render_hotkey)
            .collect::<Vec<_>>()
            .join("   "),
        "policy: monitor-only scheduler; actions policy-gated; destructive tasks require approval gates"
            .to_string(),
    ];
    lines.extend(box_lines(
        &format!("inspect {}", state.selected_job),
        &inspect_rows,
        width.saturating_sub(8).min(112),
    ));
    lines
}

fn render_capability_dashboard_body_lines(
    state: &CapabilityDashboardState,
    width: usize,
) -> Vec<String> {
    let left_width = (width / 5).clamp(24, 34);
    let right_width = (width / 5).clamp(28, 48);
    let center_width = width
        .saturating_sub(left_width)
        .saturating_sub(right_width)
        .saturating_sub(8)
        .max(48);

    let left = box_lines(
        "Navigation / workspace",
        &capability_dashboard_left_column(state, left_width.saturating_sub(2)),
        left_width,
    );
    let center = box_lines(
        "Capability registry",
        &capability_dashboard_center_column(state, center_width.saturating_sub(2)),
        center_width,
    );
    let right = box_lines(
        "Diagnostics",
        &capability_dashboard_right_column(state, right_width.saturating_sub(2)),
        right_width,
    );

    let mut lines = vec![
        "TUI Capability Dashboard".to_string(),
        "Control plane registry view for manifest ownership, surfaces, policy, validation, and diagnostics.".to_string(),
    ];
    lines.extend(box_lines(
        "metrics",
        &[
            format!("{} CAPABILITIES", state.capability_count),
            format!("{} OWNED DOMAINS", state.owned_domains),
            format!("{} UNOWNED", state.unowned_domains),
            format!("{}% VALID", state.valid_percent),
        ],
        width.min(112),
    ));
    lines.push("".to_string());
    lines.extend(render_columns(
        &[left, center, right],
        &[left_width, center_width, right_width],
        " │ ",
    ));
    lines
}

fn capability_dashboard_left_column(state: &CapabilityDashboardState, width: usize) -> Vec<String> {
    let mut lines = vec![
        fit_line("› /capabilities", width),
        "".to_string(),
        "WORKSPACE FEED".to_string(),
        fit_line("registry    .vac/capabilities", width),
        fit_line("workflows   .vac/workflows", width),
        fit_line("surface     tui + cli + palette", width),
        "".to_string(),
        format!("ownership   {} owned", state.owned_domains),
        format!("unowned     {} domains", state.unowned_domains),
        "policy      strict gates".to_string(),
        "".to_string(),
        "/             ready / vac.tui".to_string(),
        "/capabilities ready / registry".to_string(),
        "/runtime      ready / scheduler".to_string(),
    ];
    lines
        .iter_mut()
        .for_each(|line| *line = fit_line(line, width));
    lines
}

fn capability_dashboard_center_column(
    state: &CapabilityDashboardState,
    width: usize,
) -> Vec<String> {
    let mut lines = vec![
        "Capability registry".to_string(),
        "filters: [all] ready partial planned broken".to_string(),
        "".to_string(),
        "/capabilities                         control plane registry".to_string(),
        "Control plane registry:".to_string(),
    ];
    lines.extend(
        state
            .registry_summary
            .iter()
            .map(|line| format!("  {line}")),
    );
    lines.push("".to_string());
    lines.push("ID | STATUS | OWNER | SURFACES | POLICY | VALIDATION / DOCS".to_string());
    if !state.capability_records.is_empty() {
        lines.extend(
            state
                .capability_records
                .iter()
                .map(CapabilityDashboardRecord::render_table_row),
        );
    } else if state.capability_rows.is_empty() {
        lines.push("<none>".to_string());
    } else {
        lines.extend(state.capability_rows.iter().cloned());
    }
    lines.push("".to_string());
    lines.extend(box_lines(
        "Ownership scan",
        &[
            "vac-tui/lib                  owned".to_string(),
            "vac-core/policy              owned".to_string(),
        ],
        width,
    ));
    lines
        .iter_mut()
        .for_each(|line| *line = fit_line(line, width));
    lines
}

fn capability_dashboard_right_column(
    state: &CapabilityDashboardState,
    width: usize,
) -> Vec<String> {
    let mut lines = vec![
        "Diagnostics".to_string(),
        "Structured errors render here instead of leaving the TUI blank.".to_string(),
        "".to_string(),
    ];
    if state.diagnostic_records.is_empty() && state.diagnostics.is_empty() {
        lines.extend(box_lines(
            "Registry parse",
            &["no YAML/control-plane errors detected".to_string()],
            width,
        ));
    } else {
        let rendered_diagnostics = if state.diagnostic_records.is_empty() {
            state.diagnostics.clone()
        } else {
            state
                .diagnostic_records
                .iter()
                .map(ControlPlaneDiagnostic::render_line)
                .collect::<Vec<_>>()
        };
        let mut diagnostic_rows = Vec::new();
        for diagnostic in &rendered_diagnostics {
            diagnostic_rows.extend(wrap_plain(diagnostic, width.saturating_sub(4)));
        }
        lines.extend(box_lines("YAML diagnostic", &diagnostic_rows, width));
    }
    lines.push("".to_string());
    lines.extend(box_lines(
        "Policy gates",
        &["ready".to_string(), "████████████████████".to_string()],
        width,
    ));
    lines.push("".to_string());
    lines.extend(box_lines(
        "Surface routes",
        &["ready".to_string(), "██████████████████░░".to_string()],
        width,
    ));
    lines
        .iter_mut()
        .for_each(|line| *line = fit_line(line, width));
    lines
}

fn autopilot_jobs_from_status_yaml(source: &str) -> Vec<AutopilotJobLine> {
    let mut jobs = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("job:") else {
            continue;
        };
        let parts = rest.split('|').map(str::trim).collect::<Vec<_>>();
        if parts.len() >= 6 {
            jobs.push(AutopilotJobLine::new(
                parts[0], parts[1], parts[2], parts[3], parts[4], parts[5],
            ));
        }
    }
    if jobs.is_empty() {
        jobs.push(AutopilotJobLine::new(
            "idle",
            "runtime",
            "none",
            "no scheduler jobs loaded",
            "—",
            "—",
        ));
    }
    jobs
}

fn yaml_scalar_by_key(source: &str, key: &str) -> Option<String> {
    source.lines().find_map(|line| {
        let (left, right) = line.split_once(':')?;
        if left.trim() == key {
            Some(
                right
                    .trim()
                    .trim_matches('\"')
                    .trim_matches('\'')
                    .to_string(),
            )
        } else {
            None
        }
    })
}

fn render_columns(columns: &[Vec<String>], widths: &[usize], separator: &str) -> Vec<String> {
    let max_height = columns.iter().map(Vec::len).max().unwrap_or(0);
    let mut rows = Vec::with_capacity(max_height);
    for index in 0..max_height {
        let mut row = String::new();
        for (column_index, column) in columns.iter().enumerate() {
            if column_index > 0 {
                row.push_str(separator);
            }
            let value = column.get(index).map_or("", String::as_str);
            row.push_str(&pad_line(value, widths[column_index]));
        }
        rows.push(row);
    }
    rows
}

fn render_composer_lines(prompt: &str, hint: &str, width: usize) -> Vec<String> {
    let inner_width = width.saturating_sub(2).max(1);
    vec![
        format!("┌{}┐", "─".repeat(inner_width)),
        fit_line(
            &format!(
                "│ {prompt:<2} {:<hint_width$}│",
                hint,
                hint_width = inner_width.saturating_sub(4)
            ),
            width,
        ),
        format!("└{}┘", "─".repeat(inner_width)),
    ]
}

fn render_statusline(chrome: &OperatorChromeState, width: usize) -> String {
    let expanded = format!(
        "{} | model {} | {} | {} | {} | {} | {} · {}",
        chrome.bottom_mode,
        chrome.model,
        chrome.token_usage,
        chrome.automation_mode,
        chrome.validation,
        chrome.lsp,
        chrome.profile,
        chrome.rulebook
    );
    if expanded.chars().count() <= width {
        return fit_line(&expanded, width);
    }

    let compact = format!(
        "{} | model {} | {} | {} | {} {} | {} · {}",
        chrome.bottom_mode,
        chrome.model,
        chrome.token_usage,
        chrome.automation_mode,
        chrome.validation,
        chrome.lsp,
        chrome.profile,
        chrome.rulebook
    );
    if compact.chars().count() <= width {
        return fit_line(&compact, width);
    }

    let right = format!(
        "{} {} | {} · {}",
        chrome.validation, chrome.lsp, chrome.profile, chrome.rulebook
    );
    let left_budget = width.saturating_sub(right.chars().count() + 3);
    let left = fit_line(
        &format!(
            "{} | model {} | {} | {}",
            chrome.bottom_mode, chrome.model, chrome.token_usage, chrome.automation_mode
        ),
        left_budget,
    );
    fit_line(&format!("{left} | {right}"), width)
}

fn box_lines(title: &str, rows: &[String], width: usize) -> Vec<String> {
    let width = width.max(12);
    let inner = width.saturating_sub(4).max(1);
    let mut output = vec![format!(
        "┌─ {} {}┐",
        title,
        "─".repeat(inner.saturating_sub(title.len().min(inner)))
    )];
    for row in rows {
        let wrapped = wrap_plain(row, inner);
        for line in wrapped {
            output.push(format!("│ {} │", pad_line(&line, inner)));
        }
    }
    output.push(format!("└{}┘", "─".repeat(width.saturating_sub(2))));
    output
        .into_iter()
        .map(|line| fit_line(&line, width))
        .collect()
}

fn wrap_plain(value: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![String::new()];
    }
    let mut out = Vec::new();
    let mut current = String::new();
    for word in value.split_whitespace() {
        let separator = usize::from(!current.is_empty());
        if current.len() + separator + word.len() > width && !current.is_empty() {
            out.push(current);
            current = String::new();
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if current.is_empty() {
        out.push(String::new());
    } else {
        out.push(current);
    }
    out
}

fn pad_line(value: &str, width: usize) -> String {
    let value = fit_line(value, width);
    format!("{value:<width$}")
}

fn fit_line(value: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let mut output = value.chars().take(width).collect::<String>();
    let current_width = output.chars().count();
    if current_width < width {
        output.push_str(&" ".repeat(width - current_width));
    }
    output
}

fn horizontal_rule(width: usize) -> String {
    "─".repeat(width.max(1))
}

fn job_state_glyph(state: &str) -> &'static str {
    match state {
        "running" => "●",
        "queued" => "○",
        "ok" => "●",
        "failed" => "●",
        "cancel" | "cancelled" => "○",
        _ => "•",
    }
}

fn format_snapshot_row(row: &StartupSnapshotRow) -> String {
    if row.detail.is_empty() {
        format!("{}  {}", row.key, row.value)
    } else {
        format!("{}  {} · {}", row.key, row.value, row.detail)
    }
}

fn compact_number(value: u64) -> String {
    if value >= 1_000_000 {
        format!("{}M", value / 1_000_000)
    } else if value >= 1_000 {
        format!("{}K", value / 1_000)
    } else {
        value.to_string()
    }
}

fn context_bar(used: u64, limit: u64, width: usize) -> String {
    let filled = if limit == 0 {
        0
    } else {
        ((used.saturating_mul(width as u64)) / limit).min(width as u64) as usize
    };
    format!(
        "{}{}",
        "█".repeat(filled),
        "░".repeat(width.saturating_sub(filled))
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_streaming_limits_tool_timeline_to_last_five() {
        let state = AgentStreamingState {
            user_prompt: "refactor input".to_string(),
            user_time: "14:22:06".to_string(),
            agent_model: "claude-sonnet-4.5".to_string(),
            turn: 3,
            agent_time: "14:22:08".to_string(),
            agent_message: "I'll inspect the handler first.".to_string(),
            tools: (0..7)
                .map(|index| {
                    ToolTimelineEntry::new(
                        format!("tool_{index}"),
                        "handlers/input.rs",
                        ToolTimelineState::Queued,
                        "queued",
                    )
                })
                .collect(),
            thinking: "preserving precedence".to_string(),
            context_used: 68_240,
            context_limit: 200_000,
            composer_hint: "agent working".to_string(),
            status_bar: OperatorStatusBarState::conversation(
                "claude-sonnet-4.5",
                "68,240 tok",
                "valid 92%",
            )
            .with_profile_rulebook("profile default", "rulebook runtime"),
        };

        let rendered = render_agent_streaming_lines(&state).join("\n");
        assert!(!rendered.contains("tool_0"));
        assert!(!rendered.contains("tool_1"));
        assert!(rendered.contains("tool_2"));
        assert!(rendered.contains("tool_6"));
        assert!(rendered.contains("last 5"));
    }

    #[test]
    fn tool_state_contract_names_are_stable() {
        let states = [
            ToolTimelineState::Queued,
            ToolTimelineState::Running,
            ToolTimelineState::Streaming,
            ToolTimelineState::Passed,
            ToolTimelineState::Failed,
            ToolTimelineState::Cancelled,
        ];
        let names = states.map(ToolTimelineState::as_str);
        assert_eq!(
            names,
            [
                "queued",
                "running",
                "streaming",
                "passed",
                "failed",
                "cancelled"
            ]
        );
    }

    #[test]
    fn approval_popup_contains_required_policy_gated_labels() {
        let rendered = render_approval_popup_lines(&ApprovalPopupState::destructive_bash(
            "rm -rf target/debug/incremental",
            "~/code/vastar/vac",
        ))
        .join("\n");
        assert!(rendered.contains("approval required"));
        assert!(rendered.contains("DESTRUCTIVE"));
        assert!(rendered.contains("approve once"));
        assert!(rendered.contains("approve+remember"));
        assert!(rendered.contains("reject with reason"));
        assert!(rendered.contains("policy"));
        assert!(rendered.contains("batch 1/2"));
    }

    #[test]
    fn autopilot_scheduler_exposes_required_monitoring_surface() {
        let rendered =
            render_autopilot_scheduler_lines(&AutopilotSchedulerState::monitor_only_sample())
                .join("\n");
        assert!(rendered.contains("autopilot"));
        assert!(rendered.contains("pid"));
        assert!(rendered.contains("queue 3"));
        assert!(rendered.contains("running 1"));
        assert!(rendered.contains("task graph"));
        assert!(rendered.contains("tokens"));
        assert!(rendered.contains("spend"));
        assert!(rendered.contains("retry"));
        assert!(rendered.contains("c cancel"));
        assert!(rendered.contains("r retry"));
        assert!(rendered.contains("open in editor"));
        assert!(rendered.contains("enter attach"));
        assert!(rendered.contains("approval gates"));
    }

    #[test]
    fn capability_dashboard_never_renders_blank_diagnostics() {
        let rendered = render_capability_dashboard_shell(&CapabilityDashboardState {
            chrome_title: "vac · interactive".to_string(),
            chrome_mode: "registry /capabilities".to_string(),
            capability_count: 1,
            owned_domains: 1,
            unowned_domains: 0,
            valid_percent: 100,
            registry_summary: vec![".vac/capabilities loaded".to_string()],
            capability_rows: vec!["vac.tui | ready | vac-tui/lib".to_string()],
            diagnostics: Vec::new(),
            capability_records: Vec::new(),
            diagnostic_records: Vec::new(),
            status_bar: OperatorStatusBarState::conversation("runtime", "0 tok", "valid 100%"),
        })
        .join("\n");
        assert!(rendered.contains("Diagnostics"));
        assert!(rendered.contains("no YAML/control-plane errors detected"));
    }

    #[test]
    fn operator_snapshot_filenames_match_committed_fixtures() {
        let filenames = SnapshotScenario::ALL.map(render_operator_snapshot_filename);
        assert_eq!(
            filenames,
            [
                "first-launch-120x36.txt",
                "idle-120x36.txt",
                "agent-working-120x36.txt",
                "approval-popup-120x36.txt",
                "runtime-jobs-120x36.txt",
                "capability-dashboard-180x48.txt",
            ]
        );
        assert_eq!(SnapshotScenario::RuntimeJobs.title(), "runtime jobs");
    }

    #[test]
    fn operator_snapshots_keep_widget_chrome_and_statusline() {
        let scenarios = [
            SnapshotScenario::FirstLaunch,
            SnapshotScenario::Idle,
            SnapshotScenario::AgentWorking,
            SnapshotScenario::ApprovalPopup,
            SnapshotScenario::RuntimeJobs,
        ];
        for scenario in scenarios {
            let lines = render_operator_snapshot(scenario, OperatorViewport::new(120, 36));
            let rendered = lines.join("\n");
            assert!(!lines.is_empty(), "{scenario:?} snapshot should not be empty");
            assert!(
                lines.iter().all(|line| line.chars().count() <= 120),
                "{scenario:?} snapshot should fit viewport width"
            );
            assert!(rendered.contains("vac · interactive"));
            assert!(rendered.contains("model"));
            assert!(rendered.contains("rulebook"));
        }
        for scenario in [
            SnapshotScenario::AgentWorking,
            SnapshotScenario::ApprovalPopup,
            SnapshotScenario::RuntimeJobs,
        ] {
            let rendered =
                render_operator_snapshot(scenario, OperatorViewport::new(120, 36)).join("\n");
            assert!(
                rendered.contains('╭'),
                "{scenario:?} should use rounded widget panels"
            );
        }
    }

    #[test]
    fn operator_snapshots_cover_all_target_modes() {
        let first_launch = render_operator_snapshot(
            SnapshotScenario::FirstLaunch,
            OperatorViewport::new(120, 36),
        )
        .join("\n");
        assert!(first_launch.contains("Vastar Agentic CLI"));
        assert!(first_launch.contains("hydrating startup snapshot"));

        let idle = render_operator_snapshot(SnapshotScenario::Idle, OperatorViewport::new(120, 36))
            .join("\n");
        assert!(idle.contains("VAC"));
        assert!(idle.contains("ready"));
        assert!(idle.contains("shift+tab"));
        assert!(idle.contains("recent tasks"));

        let agent = render_operator_snapshot(
            SnapshotScenario::AgentWorking,
            OperatorViewport::new(120, 36),
        )
        .join("\n");
        assert!(agent.contains("tool timeline"));
        assert!(agent.contains("last 5"));
        assert!(agent.contains("thinking"));

        let approval = render_operator_snapshot(
            SnapshotScenario::ApprovalPopup,
            OperatorViewport::new(120, 36),
        )
        .join("\n");
        assert!(approval.contains("approval required"));
        assert!(approval.contains("DESTRUCTIVE"));
        assert!(approval.contains("approve+remember"));

        let runtime = render_operator_snapshot(
            SnapshotScenario::RuntimeJobs,
            OperatorViewport::new(120, 36),
        )
        .join("\n");
        assert!(runtime.contains("autopilot"));
        assert!(runtime.contains("runtime jobs"));
        assert!(runtime.contains("enter attach"));

        let capabilities = render_operator_snapshot(
            SnapshotScenario::CapabilityDashboard,
            OperatorViewport::new(180, 48),
        )
        .join("\n");
        assert!(capabilities.contains("TUI Capability Dashboard"));
        assert!(capabilities.contains("Diagnostics"));
        assert!(capabilities.contains("no YAML/control-plane errors detected"));
        assert!(!capabilities.contains(".vac/capabilities/tui.yml:42:13"));
        assert!(!capabilities.contains("almost_ready"));
    }

    #[test]
    fn semantic_snapshot_specs_round_trip_to_plain_snapshot() {
        for scenario in SnapshotScenario::ALL {
            let viewport = scenario.default_viewport();
            let plain = render_operator_snapshot(scenario, viewport).join("\n");
            let specs = render_operator_snapshot_specs(scenario, viewport);
            assert_eq!(operator_line_specs_to_plain_text(&specs), plain);
            assert_eq!(render_operator_snapshot_text(scenario, viewport), plain);
        }
    }

    #[test]
    fn semantic_ansi_snapshots_strip_back_to_plain_snapshot() {
        for scenario in SnapshotScenario::ALL {
            let viewport = scenario.default_viewport();
            let plain = render_operator_snapshot_text(scenario, viewport);
            let ansi = render_operator_snapshot_ansi_text(scenario, viewport);
            assert_ne!(plain, ansi);
            assert_eq!(operator_style::strip_ansi(&ansi), plain);
        }
    }

    #[test]
    fn approval_semantic_spec_marks_destructive_span_as_danger() {
        let specs = render_operator_snapshot_specs(
            SnapshotScenario::ApprovalPopup,
            OperatorViewport::new(120, 36),
        );
        let destructive = specs
            .iter()
            .flat_map(|line| line.spans.iter())
            .find(|span| span.text == "DESTRUCTIVE")
            .expect("destructive span");
        assert_eq!(destructive.role, OperatorStyleRole::Danger);
    }

    #[test]
    fn runtime_jobs_and_dashboard_have_semantic_body_specs() {
        let runtime_specs =
            render_autopilot_scheduler_line_specs(&AutopilotSchedulerState::monitor_only_sample());
        assert!(runtime_specs.iter().any(|line| {
            line.spans.iter().any(|span| {
                span.role == OperatorStyleRole::Warning && span.text.contains("monitor-only")
            })
        }));

        let dashboard_specs =
            render_capability_dashboard_shell_specs(&CapabilityDashboardState::sample_dashboard());
        assert!(dashboard_specs.iter().any(|line| {
            line.spans.iter().any(|span| {
                span.role == OperatorStyleRole::Warning && span.text.contains("Diagnostics")
            })
        }));
    }

    #[test]
    fn capability_dashboard_state_is_manifest_record_driven() {
        let state = CapabilityDashboardState::from_manifest_records(
            vec![
                CapabilityDashboardRecord::new(
                    "vac.tui",
                    CapabilityManifestStatus::Ready,
                    "vac-tui/lib",
                    "/capabilities",
                    "safe_read",
                    "snapshot ok",
                ),
                CapabilityDashboardRecord::new(
                    "vac.experimental",
                    CapabilityManifestStatus::Experimental,
                    "vac-lab",
                    "cli-only",
                    "approval_trace",
                    "pending dogfood",
                ),
            ],
            vec![ControlPlaneDiagnostic::from_text(
                ".vac/capabilities/lab.yaml warning: owner map pending",
            )],
            vec![".vac/capabilities loaded".to_string()],
            1,
            1,
        );

        assert_eq!(state.capability_count, 2);
        assert_eq!(state.valid_percent, 100);
        assert!(state.capability_rows[0].contains("vac.tui | ready"));
        assert!(state.diagnostics[0].contains("warning"));
        let rendered = render_capability_dashboard_shell(&state).join(
            "
",
        );
        assert!(rendered.contains("Capability registry"));
        assert!(rendered.contains("Diagnostics"));
        assert!(rendered.contains("vac.experimental | experimental"));
    }

    #[test]
    fn capability_status_enum_accepts_vac_schema_aliases() {
        assert_eq!(
            CapabilityManifestStatus::parse("ready"),
            Some(CapabilityManifestStatus::Ready)
        );
        assert_eq!(
            CapabilityManifestStatus::parse("cli_only"),
            Some(CapabilityManifestStatus::CliOnly)
        );
        assert_eq!(CapabilityManifestStatus::parse("almost_ready"), None);
    }

    #[test]
    fn approval_popup_uses_explicit_policy_risk_model() {
        let state = ApprovalPopupState::destructive_bash("rm -rf target", "/repo");
        assert_eq!(state.risk, ApprovalRisk::Destructive);
        assert!(state.risk.requires_policy_gate());
        assert_eq!(state.risk.label(), "DESTRUCTIVE");
        assert!(state.policy_gate_summary().contains("policy-gated"));
    }

    #[test]
    fn autopilot_actions_are_policy_gated_monitor_controls() {
        let state = AutopilotSchedulerState::monitor_only_sample();
        assert_eq!(state.actions.len(), 4);
        assert!(
            state
                .actions
                .iter()
                .all(|action| action.policy_gate.starts_with("policy."))
        );
        let rendered = render_autopilot_scheduler_lines(&state).join(
            "
",
        );
        assert!(rendered.contains("c cancel"));
        assert!(rendered.contains("actions policy-gated"));
    }

    #[test]
    fn visual_fidelity_matrix_covers_three_terminal_sizes() {
        assert_eq!(OperatorViewport::VISUAL_FIDELITY_MATRIX.len(), 3);
        for scenario in SnapshotScenario::ALL {
            for viewport in OperatorViewport::VISUAL_FIDELITY_MATRIX {
                let rendered = render_operator_snapshot(scenario, viewport);
                assert!(!rendered.is_empty());
                assert!(rendered.len() <= viewport.height);
                assert!(
                    rendered
                        .iter()
                        .all(|line| line.chars().count() <= viewport.width)
                );
                if matches!(
                    scenario,
                    SnapshotScenario::AgentWorking
                        | SnapshotScenario::ApprovalPopup
                        | SnapshotScenario::RuntimeJobs
                        | SnapshotScenario::CapabilityDashboard
                ) {
                    assert!(rendered.join("\n").contains('╭'));
                }
                let filename = render_operator_snapshot_filename_for_viewport(scenario, viewport);
                assert!(filename.ends_with(&format!("{}x{}.txt", viewport.width, viewport.height)));
            }
        }
    }
}
