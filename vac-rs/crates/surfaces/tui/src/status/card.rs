use crate::history_cell::CompositeHistoryCell;
use crate::history_cell::HistoryCell;
use crate::history_cell::PlainHistoryCell;
use crate::history_cell::with_border_with_inner_width;
use crate::legacy_core::config::Config;
use crate::session_protocol::AskForApproval;
use crate::token_usage::TokenUsage;
use crate::token_usage::TokenUsageInfo;
use crate::version::VAC_CLI_VERSION;
use chrono::DateTime;
use chrono::Local;
use ratatui::prelude::*;
use ratatui::style::Stylize;
use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;
use url::Url;
use vac_model_provider_info::WireApi;
use vac_protocol::ThreadId;
use vac_protocol::account::PlanType;
use vac_protocol::config_types::ApprovalsReviewer;
use vac_protocol::models::ActivePermissionProfile;
use vac_protocol::models::ActivePermissionProfileModification;
use vac_protocol::models::PermissionProfile;
use vac_protocol::vastar_models::ReasoningEffort;
use vac_utils_sandbox_summary::summarize_permission_profile;

use super::account::StatusAccountDisplay;
use super::format::FieldFormatter;
use super::format::line_display_width;
use super::format::push_label;
use super::format::truncate_line_to_width;
use super::helpers::compose_account_display;
use super::helpers::compose_model_display;
use super::helpers::format_directory_display;
use super::helpers::format_tokens_compact;
use super::output_contract::STATUS_OPERATOR_DISPLAY_POLICY;
use super::output_contract::StatusDisplayField;
use super::rate_limits::RateLimitSnapshotDisplay;
use super::rate_limits::StatusRateLimitData;
use super::rate_limits::compose_rate_limit_data;
use super::rate_limits::compose_rate_limit_data_many;
use std::sync::Arc;
use std::sync::RwLock;

#[derive(Debug, Clone)]
struct StatusContextWindowData {
    percent_remaining: i64,
    tokens_in_context: i64,
    window: i64,
}

#[derive(Debug, Clone)]
pub(crate) struct StatusTokenUsageData {
    total: i64,
    input: i64,
    output: i64,
    context_window: Option<StatusContextWindowData>,
}

#[derive(Debug)]
struct StatusRateLimitState {
    rate_limits: StatusRateLimitData,
    refreshing_rate_limits: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct StatusHistoryHandle {
    rate_limit_state: Arc<RwLock<StatusRateLimitState>>,
}

impl StatusHistoryHandle {
    pub(crate) fn finish_rate_limit_refresh(
        &self,
        rate_limits: &[RateLimitSnapshotDisplay],
        now: DateTime<Local>,
    ) {
        let rate_limits = if rate_limits.len() <= 1 {
            compose_rate_limit_data(rate_limits.first(), now)
        } else {
            compose_rate_limit_data_many(rate_limits, now)
        };
        let Ok(mut state) = self.rate_limit_state.write() else {
            tracing::warn!("status history rate-limit state lock poisoned");
            return;
        };
        state.rate_limits = rate_limits;
        state.refreshing_rate_limits = false;
    }
}

#[derive(Debug)]
struct StatusHistoryCell {
    model_name: String,
    model_details: Vec<String>,
    directory: PathBuf,
    permissions: String,
    agents_summary: Arc<RwLock<String>>,
    collaboration_mode: Option<String>,
    model_provider: String,
    model_provider_registry: Option<String>,
    account: Option<StatusAccountDisplay>,
    thread_name: Option<String>,
    session_id: Option<String>,
    branched_from: Option<String>,
    token_usage: StatusTokenUsageData,
    rate_limit_state: Arc<RwLock<StatusRateLimitState>>,
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn new_status_output(
    config: &Config,
    account_display: Option<&StatusAccountDisplay>,
    token_info: Option<&TokenUsageInfo>,
    total_usage: &TokenUsage,
    session_id: &Option<ThreadId>,
    thread_name: Option<String>,
    branched_from: Option<ThreadId>,
    rate_limits: Option<&RateLimitSnapshotDisplay>,
    _plan_type: Option<PlanType>,
    now: DateTime<Local>,
    model_name: &str,
    collaboration_mode: Option<&str>,
    reasoning_effort_override: Option<Option<ReasoningEffort>>,
) -> CompositeHistoryCell {
    let snapshots = rate_limits.map(std::slice::from_ref).unwrap_or_default();
    new_status_output_with_rate_limits(
        config,
        account_display,
        token_info,
        total_usage,
        session_id,
        thread_name,
        branched_from,
        snapshots,
        _plan_type,
        now,
        model_name,
        collaboration_mode,
        reasoning_effort_override,
        /*refreshing_rate_limits*/ false,
    )
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn new_status_output_with_rate_limits(
    config: &Config,
    account_display: Option<&StatusAccountDisplay>,
    token_info: Option<&TokenUsageInfo>,
    total_usage: &TokenUsage,
    session_id: &Option<ThreadId>,
    thread_name: Option<String>,
    branched_from: Option<ThreadId>,
    rate_limits: &[RateLimitSnapshotDisplay],
    _plan_type: Option<PlanType>,
    now: DateTime<Local>,
    model_name: &str,
    collaboration_mode: Option<&str>,
    reasoning_effort_override: Option<Option<ReasoningEffort>>,
    refreshing_rate_limits: bool,
) -> CompositeHistoryCell {
    new_status_output_with_rate_limits_handle(
        config,
        /*runtime_model_provider_base_url*/ None,
        account_display,
        token_info,
        total_usage,
        session_id,
        thread_name,
        branched_from,
        rate_limits,
        _plan_type,
        now,
        model_name,
        collaboration_mode,
        reasoning_effort_override,
        "<none>".to_string(),
        refreshing_rate_limits,
    )
    .0
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn new_status_output_with_rate_limits_handle(
    config: &Config,
    runtime_model_provider_base_url: Option<&str>,
    account_display: Option<&StatusAccountDisplay>,
    token_info: Option<&TokenUsageInfo>,
    total_usage: &TokenUsage,
    session_id: &Option<ThreadId>,
    thread_name: Option<String>,
    branched_from: Option<ThreadId>,
    rate_limits: &[RateLimitSnapshotDisplay],
    _plan_type: Option<PlanType>,
    now: DateTime<Local>,
    model_name: &str,
    collaboration_mode: Option<&str>,
    reasoning_effort_override: Option<Option<ReasoningEffort>>,
    agents_summary: String,
    refreshing_rate_limits: bool,
) -> (CompositeHistoryCell, StatusHistoryHandle) {
    let command = PlainHistoryCell::new(vec!["/status".magenta().into()]);
    let (card, handle) = StatusHistoryCell::new(
        config,
        runtime_model_provider_base_url,
        account_display,
        token_info,
        total_usage,
        session_id,
        thread_name,
        branched_from,
        rate_limits,
        _plan_type,
        now,
        model_name,
        collaboration_mode,
        reasoning_effort_override,
        agents_summary,
        refreshing_rate_limits,
    );

    (
        CompositeHistoryCell::new(vec![Box::new(command), Box::new(card)]),
        handle,
    )
}

impl StatusHistoryCell {
    #[allow(clippy::too_many_arguments)]
    fn new(
        config: &Config,
        runtime_model_provider_base_url: Option<&str>,
        account_display: Option<&StatusAccountDisplay>,
        token_info: Option<&TokenUsageInfo>,
        total_usage: &TokenUsage,
        session_id: &Option<ThreadId>,
        thread_name: Option<String>,
        branched_from: Option<ThreadId>,
        rate_limits: &[RateLimitSnapshotDisplay],
        _plan_type: Option<PlanType>,
        now: DateTime<Local>,
        model_name: &str,
        collaboration_mode: Option<&str>,
        reasoning_effort_override: Option<Option<ReasoningEffort>>,
        agents_summary: String,
        refreshing_rate_limits: bool,
    ) -> (Self, StatusHistoryHandle) {
        let approval_policy = AskForApproval::from(config.permissions.approval_policy.value());
        let permission_profile = config.permissions.permission_profile();
        let mut config_entries = vec![
            ("workdir", config.cwd.display().to_string()),
            ("model", model_name.to_string()),
            ("provider", config.model_provider_id.clone()),
            (
                "approval",
                config.permissions.approval_policy.value().to_string(),
            ),
            (
                "sandbox",
                summarize_permission_profile(&permission_profile, config.cwd.as_path()),
            ),
        ];
        if config.model_provider.wire_api == WireApi::Responses {
            let effort_value = reasoning_effort_override
                .unwrap_or(config.model_reasoning_effort)
                .map(|effort| effort.to_string())
                .unwrap_or_else(|| "none".to_string());
            config_entries.push(("reasoning effort", effort_value));
            config_entries.push((
                "reasoning summaries",
                config
                    .model_reasoning_summary
                    .map(|summary| summary.to_string())
                    .unwrap_or_else(|| "auto".to_string()),
            ));
        }
        let (model_name, model_details) = compose_model_display(model_name, &config_entries);
        let approval = config_entries
            .iter()
            .find(|(k, _)| *k == "approval")
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| "<unknown>".to_string());
        let active_permission_profile = config.permissions.active_permission_profile();
        let sandbox = status_permission_summary(&permission_profile, config.cwd.as_path());
        let approval = status_approval_label(approval_policy, config.approvals_reviewer, &approval);
        let permissions = status_permissions_label(
            active_permission_profile.as_ref(),
            &permission_profile,
            approval_policy,
            &sandbox,
            &approval,
        );
        let model_provider = format_model_provider(config, runtime_model_provider_base_url);
        let model_provider_registry = format_model_provider_registry(config);
        let account = compose_account_display(account_display);
        let session_id = session_id.as_ref().map(std::string::ToString::to_string);
        let branched_from = branched_from.map(|id| id.to_string());
        let default_usage = TokenUsage::default();
        let (context_usage, context_window) = match token_info {
            Some(info) => (&info.last_token_usage, info.model_context_window),
            None => (&default_usage, config.model_context_window),
        };
        let context_window = context_window.map(|window| StatusContextWindowData {
            percent_remaining: context_usage.percent_of_context_window_remaining(window),
            tokens_in_context: context_usage.tokens_in_context_window(),
            window,
        });

        let token_usage = StatusTokenUsageData {
            total: total_usage.blended_total(),
            input: total_usage.non_cached_input(),
            output: total_usage.output_tokens,
            context_window,
        };
        debug_assert!(!STATUS_OPERATOR_DISPLAY_POLICY.permits_rate_quota_rows());
        debug_assert!(!STATUS_OPERATOR_DISPLAY_POLICY.permits_credit_balance_rows());
        let rate_limits = if rate_limits.len() <= 1 {
            compose_rate_limit_data(rate_limits.first(), now)
        } else {
            compose_rate_limit_data_many(rate_limits, now)
        };
        let rate_limit_state = Arc::new(RwLock::new(StatusRateLimitState {
            rate_limits,
            refreshing_rate_limits,
        }));
        let agents_summary = Arc::new(RwLock::new(agents_summary));

        (
            Self {
                model_name,
                model_details,
                directory: config.cwd.to_path_buf(),
                permissions,
                collaboration_mode: collaboration_mode.map(ToString::to_string),
                model_provider,
                model_provider_registry,
                account,
                thread_name,
                session_id,
                branched_from,
                token_usage,
                agents_summary,
                rate_limit_state: rate_limit_state.clone(),
            },
            StatusHistoryHandle { rate_limit_state },
        )
    }

    fn token_usage_spans(&self) -> Vec<Span<'static>> {
        let total_fmt = format_tokens_compact(self.token_usage.total);
        let input_fmt = format_tokens_compact(self.token_usage.input);
        let output_fmt = format_tokens_compact(self.token_usage.output);

        vec![
            Span::from(total_fmt),
            Span::from(" total "),
            Span::from(" (").dim(),
            Span::from(input_fmt).dim(),
            Span::from(" input").dim(),
            Span::from(" + ").dim(),
            Span::from(output_fmt).dim(),
            Span::from(" output").dim(),
            Span::from(")").dim(),
        ]
    }

    fn context_window_spans(&self) -> Option<Vec<Span<'static>>> {
        let context = self.token_usage.context_window.as_ref()?;
        let percent = context.percent_remaining;
        let used_fmt = format_tokens_compact(context.tokens_in_context);
        let window_fmt = format_tokens_compact(context.window);

        Some(vec![
            Span::from(format!("{percent}% left")),
            Span::from(" (").dim(),
            Span::from(used_fmt).dim(),
            Span::from(" used / ").dim(),
            Span::from(window_fmt).dim(),
            Span::from(")").dim(),
        ])
    }
}

fn status_permission_summary(permission_profile: &PermissionProfile, cwd: &Path) -> String {
    let summary = summarize_permission_profile(permission_profile, cwd);
    if let Some(details) = summary.strip_prefix("read-only") {
        if details.contains("(network access enabled)") {
            return "read-only with network access".to_string();
        }
        return "read-only".to_string();
    }
    if let Some(details) = summary.strip_prefix("workspace-write") {
        if details.contains("(network access enabled)") {
            return "workspace with network access".to_string();
        }
        return "workspace".to_string();
    }
    if summary == "custom permissions (network access enabled)" {
        return "custom permissions with network access".to_string();
    }
    summary
}

fn status_permissions_label(
    active_permission_profile: Option<&ActivePermissionProfile>,
    permission_profile: &PermissionProfile,
    approval_policy: AskForApproval,
    sandbox: &str,
    approval: &str,
) -> String {
    let active_id = active_permission_profile.map(|active| active.id.as_str());
    let writable_root_modifications = active_permission_profile
        .map(|active| {
            active
                .modifications
                .iter()
                .filter(|modification| {
                    matches!(
                        modification,
                        ActivePermissionProfileModification::AdditionalWritableRoot { .. }
                    )
                })
                .count()
        })
        .unwrap_or(0);
    let modification_suffix = match writable_root_modifications {
        0 => String::new(),
        1 => " + 1 writable root".to_string(),
        count => format!(" + {count} writable roots"),
    };
    match active_id {
        Some(":read-only") => {
            let label = if sandbox == "read-only with network access" {
                "Read Only with network access"
            } else {
                "Read Only"
            };
            return format!("{label}{modification_suffix} ({approval})");
        }
        Some(":workspace") => match sandbox {
            "workspace" => return format!("Workspace{modification_suffix} ({approval})"),
            "workspace with network access" => {
                return format!("Workspace with network access{modification_suffix} ({approval})");
            }
            _ => {}
        },
        Some(":danger-no-sandbox") if permission_profile == &PermissionProfile::Disabled => {
            return if approval_policy == AskForApproval::Never {
                "Full Access".to_string()
            } else {
                format!("No Sandbox ({approval})")
            };
        }
        Some(id) => return format!("Profile {id}{modification_suffix} ({sandbox}, {approval})"),
        None => {}
    }

    if sandbox == "read-only" {
        return format!("Read Only ({approval})");
    }
    if approval_policy == AskForApproval::OnRequest && sandbox == "workspace" {
        return format!("Workspace ({approval})");
    }
    if approval_policy == AskForApproval::Never
        && permission_profile == &PermissionProfile::Disabled
    {
        return "Full Access".to_string();
    }
    format!("Custom ({sandbox}, {approval})")
}

fn status_approval_label(
    approval_policy: AskForApproval,
    approvals_reviewer: ApprovalsReviewer,
    approval: &str,
) -> String {
    if approval_policy == AskForApproval::OnRequest
        && approvals_reviewer == ApprovalsReviewer::AutoReview
    {
        "auto-review".to_string()
    } else {
        approval.to_string()
    }
}

impl HistoryCell for StatusHistoryCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        let mut lines: Vec<Line<'static>> = Vec::new();
        lines.push(Line::from(vec![
            Span::from(format!("{}>_ ", FieldFormatter::INDENT)).dim(),
            Span::from("Vastar VAC").bold(),
            Span::from(" ").dim(),
            Span::from(format!("(v{VAC_CLI_VERSION})")).dim(),
        ]));
        lines.push(Line::from(Vec::<Span<'static>>::new()));

        let available_inner_width = usize::from(width.saturating_sub(4));
        if available_inner_width == 0 {
            return Vec::new();
        }

        let account_value = self.account.as_ref().map(|account| match account {
            StatusAccountDisplay::ProviderCredential { email, plan } => match (email, plan) {
                (Some(email), Some(plan)) => format!("{email} ({plan})"),
                (Some(email), None) => email.clone(),
                (None, Some(plan)) => plan.clone(),
                (None, None) => "Provider credential".to_string(),
            },
            StatusAccountDisplay::ApiKey => {
                "API key configured (run vac login to update provider credentials)".to_string()
            }
        });

        let mut labels: Vec<String> = vec![
            StatusDisplayField::Model.label(),
            StatusDisplayField::Directory.label(),
            StatusDisplayField::Permissions.label(),
            StatusDisplayField::AgentsMd.label(),
        ]
        .into_iter()
        .map(str::to_string)
        .collect();
        let mut seen: BTreeSet<String> = labels.iter().cloned().collect();
        let thread_name = self.thread_name.as_deref().filter(|name| !name.is_empty());
        let agents_summary = self
            .agents_summary
            .read()
            .map(|summary| summary.clone())
            .unwrap_or_else(|_| {
                tracing::warn!("status history agents summary state lock poisoned");
                String::new()
            });

        push_label(
            &mut labels,
            &mut seen,
            StatusDisplayField::ModelProvider.label(),
        );
        if self.model_provider_registry.is_some() {
            push_label(
                &mut labels,
                &mut seen,
                StatusDisplayField::ModelProviders.label(),
            );
        }
        if account_value.is_some() {
            push_label(&mut labels, &mut seen, StatusDisplayField::Account.label());
        }
        if thread_name.is_some() {
            push_label(
                &mut labels,
                &mut seen,
                StatusDisplayField::ThreadName.label(),
            );
        }
        if self.session_id.is_some() {
            push_label(&mut labels, &mut seen, StatusDisplayField::Session.label());
        }
        if self.session_id.is_some() && self.branched_from.is_some() {
            push_label(
                &mut labels,
                &mut seen,
                StatusDisplayField::BranchedFrom.label(),
            );
        }
        if self.collaboration_mode.is_some() {
            push_label(
                &mut labels,
                &mut seen,
                StatusDisplayField::CollaborationMode.label(),
            );
        }
        push_label(
            &mut labels,
            &mut seen,
            StatusDisplayField::TokenUsage.label(),
        );
        if self.token_usage.context_window.is_some() {
            push_label(
                &mut labels,
                &mut seen,
                StatusDisplayField::ContextWindow.label(),
            );
        }

        let formatter = FieldFormatter::from_labels(labels.iter().map(String::as_str));
        let value_width = formatter.value_width(available_inner_width);

        let mut model_spans = vec![Span::from(self.model_name.clone())];
        if !self.model_details.is_empty() {
            model_spans.push(Span::from(" (").dim());
            model_spans.push(Span::from(self.model_details.join(", ")).dim());
            model_spans.push(Span::from(")").dim());
        }

        let directory_value = format_directory_display(&self.directory, Some(value_width));

        lines.push(formatter.line(StatusDisplayField::Model.label(), model_spans));
        lines.push(formatter.line(
            StatusDisplayField::ModelProvider.label(),
            vec![Span::from(self.model_provider.clone())],
        ));
        if let Some(model_provider_registry) = self.model_provider_registry.as_ref() {
            lines.push(formatter.line(
                StatusDisplayField::ModelProviders.label(),
                vec![Span::from(model_provider_registry.clone())],
            ));
        }
        lines.push(formatter.line(
            StatusDisplayField::Directory.label(),
            vec![Span::from(directory_value)],
        ));
        lines.push(formatter.line(
            StatusDisplayField::Permissions.label(),
            vec![Span::from(self.permissions.clone())],
        ));
        lines.push(formatter.line(
            StatusDisplayField::AgentsMd.label(),
            vec![Span::from(agents_summary)],
        ));

        if let Some(account_value) = account_value {
            lines.push(formatter.line(
                StatusDisplayField::Account.label(),
                vec![Span::from(account_value)],
            ));
        }

        if let Some(thread_name) = thread_name {
            lines.push(formatter.line(
                StatusDisplayField::ThreadName.label(),
                vec![Span::from(thread_name.to_string())],
            ));
        }
        if let Some(collab_mode) = self.collaboration_mode.as_ref() {
            lines.push(formatter.line(
                StatusDisplayField::CollaborationMode.label(),
                vec![Span::from(collab_mode.clone())],
            ));
        }
        if let Some(session) = self.session_id.as_ref() {
            lines.push(formatter.line(
                StatusDisplayField::Session.label(),
                vec![Span::from(session.clone())],
            ));
        }
        if self.session_id.is_some()
            && let Some(branched_from) = self.branched_from.as_ref()
        {
            lines.push(formatter.line(
                StatusDisplayField::BranchedFrom.label(),
                vec![Span::from(branched_from.clone())],
            ));
        }

        lines.push(Line::from(Vec::<Span<'static>>::new()));
        lines.push(formatter.line(
            StatusDisplayField::TokenUsage.label(),
            self.token_usage_spans(),
        ));

        if let Some(spans) = self.context_window_spans() {
            lines.push(formatter.line(StatusDisplayField::ContextWindow.label(), spans));
        }

        let content_width = lines.iter().map(line_display_width).max().unwrap_or(0);
        let inner_width = content_width.min(available_inner_width);
        let truncated_lines: Vec<Line<'static>> = lines
            .into_iter()
            .map(|line| truncate_line_to_width(line, inner_width))
            .collect();

        with_border_with_inner_width(truncated_lines, inner_width)
    }
}

fn format_model_provider(config: &Config, runtime_base_url: Option<&str>) -> String {
    let provider = &config.model_provider;
    let name = provider.name.trim();
    let provider_name = if name.is_empty() {
        config.model_provider_id.as_str()
    } else {
        name
    };
    let mut display = format!(
        "{provider_name} ({}, {})",
        config.model_provider_id, provider.wire_api
    );

    if let Some(base_url) = runtime_base_url
        .and_then(sanitize_base_url)
        .or_else(|| provider.base_url.as_deref().and_then(sanitize_base_url))
    {
        display.push_str(" - ");
        display.push_str(&base_url);
    }

    display
}

fn format_model_provider_registry(config: &Config) -> Option<String> {
    let provider_count = config.model_providers.len();
    if provider_count <= 1 {
        return None;
    }

    let mut provider_ids: Vec<&str> = config.model_providers.keys().map(String::as_str).collect();
    provider_ids.sort_unstable();

    let preview_limit = 5usize;
    let mut preview = Vec::new();
    for provider_id in provider_ids.iter().take(preview_limit) {
        let Some(provider) = config.model_providers.get(*provider_id) else {
            continue;
        };
        let provider_name = if provider.name.trim().is_empty() {
            *provider_id
        } else {
            provider.name.trim()
        };
        let active_marker = if *provider_id == config.model_provider_id {
            "*"
        } else {
            ""
        };
        preview.push(format!(
            "{active_marker}{provider_id}:{provider_name}/{}",
            provider.wire_api
        ));
    }

    let omitted_count = provider_count.saturating_sub(preview.len());
    let mut summary = format!(
        "{provider_count} registered; active=*{}; {}",
        config.model_provider_id,
        preview.join(", ")
    );
    if omitted_count > 0 {
        summary.push_str(&format!(", +{omitted_count} more"));
    }
    Some(summary)
}

fn sanitize_base_url(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let Ok(mut url) = Url::parse(trimmed) else {
        return None;
    };
    let _ = url.set_username("");
    let _ = url.set_password(None);
    url.set_query(None);
    url.set_fragment(None);
    Some(url.to_string().trim_end_matches('/').to_string()).filter(|value| !value.is_empty())
}
