// Display contract for `/status`.
//
// Keep this module dependency-light so it can be validated with a direct
// `rustc --test` gate in sandbox slices where a full workspace build is too
// expensive. The runtime renderer in `card.rs` imports these labels instead of
// duplicating string literals, while the static gate checks the forbidden
// fragments below.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StatusDisplayField {
    Model,
    ModelProvider,
    ModelProviders,
    Directory,
    Permissions,
    AgentsMd,
    Account,
    ThreadName,
    Session,
    BranchedFrom,
    CollaborationMode,
    TokenUsage,
    ContextWindow,
}

impl StatusDisplayField {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Model => "Model",
            Self::ModelProvider => "Model provider",
            Self::ModelProviders => "Model providers",
            Self::Directory => "Directory",
            Self::Permissions => "Permissions",
            Self::AgentsMd => "Agents.md",
            Self::Account => "Account",
            Self::ThreadName => "Thread name",
            Self::Session => "Session",
            Self::BranchedFrom => "Branched from",
            Self::CollaborationMode => "Collaboration mode",
            Self::TokenUsage => "Token usage",
            Self::ContextWindow => "Context window",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StatusProviderModelUsage {
    pub(crate) provider: String,
    pub(crate) model: String,
    pub(crate) active: bool,
    pub(crate) token_usage: Option<String>,
    pub(crate) context_window: Option<String>,
}

impl StatusProviderModelUsage {
    pub(crate) fn active(
        provider: impl Into<String>,
        model: impl Into<String>,
        token_usage: Option<String>,
        context_window: Option<String>,
    ) -> Self {
        Self {
            provider: provider.into(),
            model: model.into(),
            active: true,
            token_usage,
            context_window,
        }
    }

    pub(crate) fn inactive(
        provider: impl Into<String>,
        model: impl Into<String>,
        token_usage: Option<String>,
        context_window: Option<String>,
    ) -> Self {
        Self {
            provider: provider.into(),
            model: model.into(),
            active: false,
            token_usage,
            context_window,
        }
    }

    pub(crate) fn render_compact(&self) -> String {
        let active_marker = if self.active { "active" } else { "available" };
        let mut rendered = format!("{} / {} ({active_marker})", self.provider, self.model);
        if let Some(token_usage) = self
            .token_usage
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            rendered.push_str(" · tokens ");
            rendered.push_str(token_usage);
        }
        if let Some(context_window) = self
            .context_window
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            rendered.push_str(" · context ");
            rendered.push_str(context_window);
        }
        rendered
    }
}

pub(crate) const REQUIRED_STATUS_FIELDS: &[StatusDisplayField] = &[
    StatusDisplayField::Model,
    StatusDisplayField::ModelProvider,
    StatusDisplayField::Directory,
    StatusDisplayField::Permissions,
    StatusDisplayField::AgentsMd,
    StatusDisplayField::TokenUsage,
];

pub(crate) const OPTIONAL_STATUS_FIELDS: &[StatusDisplayField] = &[
    StatusDisplayField::ModelProviders,
    StatusDisplayField::Account,
    StatusDisplayField::ThreadName,
    StatusDisplayField::Session,
    StatusDisplayField::BranchedFrom,
    StatusDisplayField::CollaborationMode,
    StatusDisplayField::ContextWindow,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StatusDisplayPolicy {
    pub(crate) show_model_rows: bool,
    pub(crate) show_provider_model_rows: bool,
    pub(crate) show_token_context_rows: bool,
    pub(crate) show_account_session_rows: bool,
    pub(crate) show_rate_quota_rows: bool,
    pub(crate) show_credit_balance_rows: bool,
}

impl StatusDisplayPolicy {
    pub(crate) const fn operator_safe() -> Self {
        Self {
            show_model_rows: true,
            show_provider_model_rows: true,
            show_token_context_rows: true,
            show_account_session_rows: true,
            show_rate_quota_rows: false,
            show_credit_balance_rows: false,
        }
    }

    pub(crate) const fn permits_rate_quota_rows(self) -> bool {
        self.show_rate_quota_rows
    }

    pub(crate) const fn permits_credit_balance_rows(self) -> bool {
        self.show_credit_balance_rows
    }
}

pub(crate) const STATUS_OPERATOR_DISPLAY_POLICY: StatusDisplayPolicy =
    StatusDisplayPolicy::operator_safe();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StatusOutputRefreshPolicy {
    /// /status renders local provider/model/token/context metadata only.
    /// It must not trigger account limit or credit refreshes.
    NoRateLimitRefresh,
}

pub(crate) const STATUS_COMMAND_REFRESH_POLICY: StatusOutputRefreshPolicy =
    StatusOutputRefreshPolicy::NoRateLimitRefresh;

pub(crate) const fn status_command_requests_rate_limit_refresh() -> bool {
    match STATUS_COMMAND_REFRESH_POLICY {
        StatusOutputRefreshPolicy::NoRateLimitRefresh => false,
    }
}

pub(crate) const FORBIDDEN_STATUS_OUTPUT_FRAGMENTS: &[&str] = &[
    "Visit ",
    "rate limits and credits",
    "rate limits",
    "Credits:",
    "Credits",
    "Credit balance",
    "Limits:",
    "Limits",
];

pub(crate) fn contains_forbidden_status_output_fragment(line: &str) -> bool {
    FORBIDDEN_STATUS_OUTPUT_FRAGMENTS
        .iter()
        .any(|fragment| line.contains(fragment))
}

pub(crate) fn required_status_labels() -> impl Iterator<Item = &'static str> {
    REQUIRED_STATUS_FIELDS.iter().map(|field| field.label())
}

pub(crate) fn optional_status_labels() -> impl Iterator<Item = &'static str> {
    OPTIONAL_STATUS_FIELDS.iter().map(|field| field.label())
}

pub(crate) fn validate_status_output_lines(lines: &[String]) -> Result<(), String> {
    if STATUS_OPERATOR_DISPLAY_POLICY.permits_rate_quota_rows() {
        return Err("/status display policy unexpectedly permits rate quota rows".to_string());
    }
    if STATUS_OPERATOR_DISPLAY_POLICY.permits_credit_balance_rows() {
        return Err("/status display policy unexpectedly permits credit balance rows".to_string());
    }

    let joined = lines.join("\n");
    for fragment in FORBIDDEN_STATUS_OUTPUT_FRAGMENTS {
        if joined.contains(fragment) {
            return Err(format!(
                "forbidden /status output fragment rendered: {fragment}"
            ));
        }
    }

    for field in REQUIRED_STATUS_FIELDS {
        let label = format!("{}:", field.label());
        if !joined.contains(&label) {
            return Err(format!(
                "required /status output label missing: {}",
                field.label()
            ));
        }
    }

    Ok(())
}

pub(crate) fn validate_provider_model_usage_rows(
    rows: &[StatusProviderModelUsage],
) -> Result<(), String> {
    if rows.is_empty() {
        return Err("provider/model usage rows must not be empty".to_string());
    }
    if !rows.iter().any(|row| row.active) {
        return Err("provider/model usage rows must include one active model".to_string());
    }
    for row in rows {
        if row.provider.trim().is_empty() {
            return Err("provider/model usage row has empty provider".to_string());
        }
        if row.model.trim().is_empty() {
            return Err("provider/model usage row has empty model".to_string());
        }
        let rendered = row.render_compact();
        if contains_forbidden_status_output_fragment(&rendered) {
            return Err(format!(
                "provider/model usage row contains forbidden display fragment: {rendered}"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_status_fields_keep_operator_safe_rows() {
        let labels: Vec<&str> = required_status_labels().collect();
        assert!(labels.contains(&"Model"));
        assert!(labels.contains(&"Model provider"));
        assert!(labels.contains(&"Directory"));
        assert!(labels.contains(&"Permissions"));
        assert!(labels.contains(&"Token usage"));
        assert!(!labels.contains(&"Credits"));
        assert!(!labels.contains(&"Limits"));
    }

    #[test]
    fn optional_status_fields_keep_context_and_runtime_metadata() {
        let labels: Vec<&str> = optional_status_labels().collect();
        assert!(labels.contains(&"Model providers"));
        assert!(labels.contains(&"Context window"));
        assert!(labels.contains(&"Account"));
        assert!(labels.contains(&"Session"));
    }

    #[test]
    fn forbidden_status_output_fragments_detect_removed_rows() {
        assert!(contains_forbidden_status_output_fragment(
            "Visit https://example.test to check rate limits and credits"
        ));
        assert!(contains_forbidden_status_output_fragment("Credits: 12.5"));
        assert!(contains_forbidden_status_output_fragment(
            "Limits: unavailable"
        ));
        assert!(!contains_forbidden_status_output_fragment(
            "Token usage: 1.2K total (800 input + 400 output)"
        ));
        assert!(!contains_forbidden_status_output_fragment(
            "Context window: 99% left (1.2K used / 272K)"
        ));
    }

    #[test]
    fn status_command_refresh_policy_is_local_only() {
        assert_eq!(
            STATUS_COMMAND_REFRESH_POLICY,
            StatusOutputRefreshPolicy::NoRateLimitRefresh
        );
        assert!(!status_command_requests_rate_limit_refresh());
    }

    #[test]
    #[allow(clippy::assertions_on_constants)] // assert documented const display-policy invariants
    fn status_operator_display_policy_rejects_quota_and_balance_rows() {
        assert!(STATUS_OPERATOR_DISPLAY_POLICY.show_model_rows);
        assert!(STATUS_OPERATOR_DISPLAY_POLICY.show_provider_model_rows);
        assert!(STATUS_OPERATOR_DISPLAY_POLICY.show_token_context_rows);
        assert!(STATUS_OPERATOR_DISPLAY_POLICY.show_account_session_rows);
        assert!(!STATUS_OPERATOR_DISPLAY_POLICY.permits_rate_quota_rows());
        assert!(!STATUS_OPERATOR_DISPLAY_POLICY.permits_credit_balance_rows());
    }

    #[test]
    fn provider_model_usage_renders_active_and_available_rows() {
        let active = StatusProviderModelUsage::active(
            "vastar",
            "gpt-5.1-vac",
            Some("1.2K total".to_string()),
            Some("272K".to_string()),
        );
        assert_eq!(
            active.render_compact(),
            "vastar / gpt-5.1-vac (active) · tokens 1.2K total · context 272K"
        );

        let available = StatusProviderModelUsage::inactive("openai", "gpt-5.1", None, None);
        assert_eq!(available.render_compact(), "openai / gpt-5.1 (available)");
    }

    #[test]
    fn rendered_status_lines_validate_required_rows_and_reject_removed_rows() {
        let ok = vec![
            "Model: gpt-5.1-vac".to_string(),
            "Model provider: Vastar (vastar, responses)".to_string(),
            "Directory: /workspace".to_string(),
            "Permissions: Workspace".to_string(),
            "Agents.md: none".to_string(),
            "Token usage: 1.2K total (800 input + 400 output)".to_string(),
            "Context window: 99% left (1.2K used / 272K)".to_string(),
        ];
        assert_eq!(validate_status_output_lines(&ok), Ok(()));

        let mut removed = ok.clone();
        removed.push("Limits: not available".to_string());
        assert!(validate_status_output_lines(&removed).is_err());

        let mut credits = ok;
        credits.push("Credits: 10".to_string());
        assert!(validate_status_output_lines(&credits).is_err());
    }

    #[test]
    fn provider_model_usage_rows_validate_multiple_providers() {
        let rows = vec![
            StatusProviderModelUsage::active(
                "vastar",
                "gpt-5.1-vac",
                Some("1.2K total".to_string()),
                Some("272K".to_string()),
            ),
            StatusProviderModelUsage::inactive("openai", "gpt-5.1", None, Some("256K".to_string())),
            StatusProviderModelUsage::inactive("local", "qwen3-coder", None, None),
        ];
        assert_eq!(validate_provider_model_usage_rows(&rows), Ok(()));

        let rendered = rows
            .iter()
            .map(StatusProviderModelUsage::render_compact)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(rendered.contains("vastar / gpt-5.1-vac (active)"));
        assert!(rendered.contains("openai / gpt-5.1 (available)"));
        assert!(rendered.contains("local / qwen3-coder (available)"));
    }

    #[test]
    fn provider_model_usage_rows_require_active_model() {
        let rows = vec![StatusProviderModelUsage::inactive(
            "vastar",
            "gpt-5.1-vac",
            None,
            None,
        )];
        assert!(validate_provider_model_usage_rows(&rows).is_err());
    }
}
