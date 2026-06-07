#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalDecisionActor {
    User,
    Guardian,
}

impl ApprovalDecisionActor {
    fn subject(self) -> &'static str {
        match self {
            Self::User => "You ",
            Self::Guardian => "Auto-reviewer ",
        }
    }
}

pub fn new_guardian_denied_patch_request(files: Vec<String>) -> Box<dyn HistoryCell> {
    let mut summary = vec![
        "Request ".into(),
        "denied".bold(),
        " for vac to apply ".into(),
    ];
    if files.len() == 1 {
        summary.push("a patch touching ".into());
        summary.push(Span::from(files[0].clone()).dim());
    } else {
        summary.push("a patch touching ".into());
        summary.push(Span::from(files.len().to_string()).dim());
        summary.push(" files".into());
    }

    Box::new(PrefixedWrappedHistoryCell::new(
        Line::from(summary),
        "✗ ".red(),
        "  ",
    ))
}

pub fn new_guardian_denied_action_request(summary: String) -> Box<dyn HistoryCell> {
    let line = Line::from(vec![
        "Request ".into(),
        "denied".bold(),
        " for ".into(),
        Span::from(summary).dim(),
    ]);
    Box::new(PrefixedWrappedHistoryCell::new(line, "✗ ".red(), "  "))
}

pub fn new_guardian_approved_action_request(summary: String) -> Box<dyn HistoryCell> {
    let line = Line::from(vec![
        "Request ".into(),
        "approved".bold(),
        " for ".into(),
        Span::from(summary).dim(),
    ]);
    Box::new(PrefixedWrappedHistoryCell::new(line, "✔ ".green(), "  "))
}

pub fn new_guardian_timed_out_patch_request(files: Vec<String>) -> Box<dyn HistoryCell> {
    let mut summary = vec![
        "Review ".into(),
        "timed out".bold(),
        " before vac could apply ".into(),
    ];
    if files.len() == 1 {
        summary.push("a patch touching ".into());
        summary.push(Span::from(files[0].clone()).dim());
    } else {
        summary.push("a patch touching ".into());
        summary.push(Span::from(files.len().to_string()).dim());
        summary.push(" files".into());
    }

    Box::new(PrefixedWrappedHistoryCell::new(
        Line::from(summary),
        "✗ ".red(),
        "  ",
    ))
}

pub fn new_guardian_timed_out_action_request(summary: String) -> Box<dyn HistoryCell> {
    let line = Line::from(vec![
        "Review ".into(),
        "timed out".bold(),
        " before ".into(),
        Span::from(summary).dim(),
    ]);
    Box::new(PrefixedWrappedHistoryCell::new(line, "✗ ".red(), "  "))
}

/// Cyan history cell line showing the current review status.
pub(crate) fn new_review_status_line(message: String) -> PlainHistoryCell {
    PlainHistoryCell {
        lines: vec![Line::from(message.cyan())],
    }
}

#[derive(Debug)]
pub(crate) struct PatchHistoryCell {
    changes: HashMap<PathBuf, FileChange>,
    cwd: PathBuf,
}

impl HistoryCell for PatchHistoryCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        create_diff_summary(&self.changes, &self.cwd, width as usize)
    }
}

#[derive(Debug)]
struct CompletedMcpToolCallWithImageOutput {
    _image: DynamicImage,
}
impl HistoryCell for CompletedMcpToolCallWithImageOutput {
    fn display_lines(&self, _width: u16) -> Vec<Line<'static>> {
        vec!["tool result (image output)".into()]
    }
}

pub(crate) const SESSION_HEADER_MAX_INNER_WIDTH: usize = 56; // Just an eyeballed value

pub(crate) fn card_inner_width(width: u16, max_inner_width: usize) -> Option<usize> {
    if width < 4 {
        return None;
    }
    let inner_width = std::cmp::min(width.saturating_sub(4) as usize, max_inner_width);
    Some(inner_width)
}

/// Render `lines` inside a border sized to the widest span in the content.
pub(crate) fn with_border(lines: Vec<Line<'static>>) -> Vec<Line<'static>> {
    with_border_internal(lines, /*forced_inner_width*/ None)
}

/// Render `lines` inside a border whose inner width is at least `inner_width`.
///
/// This is useful when callers have already clamped their content to a
/// specific width and want the border math centralized here instead of
/// duplicating padding logic in the TUI widgets themselves.
pub(crate) fn with_border_with_inner_width(
    lines: Vec<Line<'static>>,
    inner_width: usize,
) -> Vec<Line<'static>> {
    with_border_internal(lines, Some(inner_width))
}

fn with_border_internal(
    lines: Vec<Line<'static>>,
    forced_inner_width: Option<usize>,
) -> Vec<Line<'static>> {
    let max_line_width = lines
        .iter()
        .map(|line| {
            line.iter()
                .map(|span| UnicodeWidthStr::width(span.content.as_ref()))
                .sum::<usize>()
        })
        .max()
        .unwrap_or(0);
    let content_width = forced_inner_width
        .unwrap_or(max_line_width)
        .max(max_line_width);

    let mut out = Vec::with_capacity(lines.len() + 2);
    let border_inner_width = content_width + 2;
    out.push(vec![format!("╭{}╮", "─".repeat(border_inner_width)).dim()].into());

    for line in lines.into_iter() {
        let used_width: usize = line
            .iter()
            .map(|span| UnicodeWidthStr::width(span.content.as_ref()))
            .sum();
        let span_count = line.spans.len();
        let mut spans: Vec<Span<'static>> = Vec::with_capacity(span_count + 4);
        spans.push(Span::from("│ ").dim());
        spans.extend(line);
        if used_width < content_width {
            spans.push(Span::from(" ".repeat(content_width - used_width)).dim());
        }
        spans.push(Span::from(" │").dim());
        out.push(Line::from(spans));
    }

    out.push(vec![format!("╰{}╯", "─".repeat(border_inner_width)).dim()].into());

    out
}

/// Return the emoji followed by a hair space (U+200A).
/// Using only the hair space avoids excessive padding after the emoji while
/// still providing a small visual gap across terminals.
pub(crate) fn padded_emoji(emoji: &str) -> String {
    format!("{emoji}\u{200A}")
}

#[derive(Debug)]
struct TooltipHistoryCell {
    tip: String,
    cwd: PathBuf,
}

impl TooltipHistoryCell {
    fn new(tip: String, cwd: &Path) -> Self {
        Self {
            tip,
            cwd: cwd.to_path_buf(),
        }
    }
}

impl HistoryCell for TooltipHistoryCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        let indent = "  ";
        let indent_width = UnicodeWidthStr::width(indent);
        let wrap_width = usize::from(width.max(1))
            .saturating_sub(indent_width)
            .max(1);
        let mut lines: Vec<Line<'static>> = Vec::new();
        append_markdown(
            &format!("**Tip:** {}", self.tip),
            Some(wrap_width),
            Some(self.cwd.as_path()),
            &mut lines,
        );

        prefix_lines(lines, indent.into(), indent.into())
    }
}

#[derive(Debug)]
pub struct SessionInfoCell(CompositeHistoryCell);

impl HistoryCell for SessionInfoCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        self.0.display_lines(width)
    }

    fn desired_height(&self, width: u16) -> u16 {
        self.0.desired_height(width)
    }

    fn transcript_lines(&self, width: u16) -> Vec<Line<'static>> {
        self.0.transcript_lines(width)
    }
}

pub(crate) fn new_session_info(
    config: &Config,
    requested_model: &str,
    session: &ThreadSessionState,
    is_first_event: bool,
    tooltip_override: Option<String>,
    auth_plan: Option<PlanType>,
    show_fast_status: bool,
) -> SessionInfoCell {
    let status_bar = operator_status_bar_for_config(config, session.model.clone());
    // Header box rendered as history (so it appears at the very top)
    let header = SessionHeaderHistoryCell::new(
        session.model.clone(),
        session.reasoning_effort,
        show_fast_status,
        config.cwd.to_path_buf(),
        VAC_CLI_VERSION,
    )
    .with_provider(session_provider_label(config, session))
    .with_session(short_session_label(session.thread_id))
    .with_operator_context(status_bar.profile.clone(), status_bar.rulebook.clone())
    .with_yolo_mode(has_yolo_permissions(
        session.approval_policy,
        &session.permission_profile,
    ));
    let mut parts: Vec<Box<dyn HistoryCell>> = vec![Box::new(header)];

    if is_first_event {
        let idle_state =
            crate::operator_ui::IdleViewState::live(session.model.clone()).with_status_bar(status_bar);
        parts.push(Box::new(OperatorIdleHistoryCell { state: idle_state }));
    } else {
        if config.show_tooltips
            && let Some(tooltips) = tooltip_override
                .or_else(|| tooltips::get_tooltip(auth_plan, show_fast_status))
                .map(|tip| TooltipHistoryCell::new(tip, &config.cwd))
        {
            parts.push(Box::new(tooltips));
        }
        if requested_model != session.model.as_str() {
            let lines = vec![
                "model changed:".magenta().bold().into(),
                format!("requested: {requested_model}").into(),
                format!("used: {}", session.model).into(),
            ];
            parts.push(Box::new(PlainHistoryCell { lines }));
        }
    }

    SessionInfoCell(CompositeHistoryCell { parts })
}

#[derive(Debug)]
struct OperatorIdleHistoryCell {
    state: crate::operator_ui::IdleViewState,
}

impl HistoryCell for OperatorIdleHistoryCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        let inner_width = usize::from(width.saturating_sub(4)).max(1);
        crate::operator_ui::render_idle_visual_lines(
            &self.state,
            inner_width.min(u16::MAX as usize) as u16,
        )
        .into_iter()
        .map(|line| crate::line_truncation::truncate_line_to_width(line, inner_width))
        .collect()
    }
}

pub(crate) fn operator_profile_label(config: &Config) -> String {
    if let Some(profile) = config
        .active_profile
        .as_deref()
        .map(str::trim)
        .filter(|profile| !profile.is_empty())
    {
        return format!("profile {profile}");
    }
    if let Some(active_permission_profile) = config.permissions.active_permission_profile() {
        return format!("profile {}", active_permission_profile.id);
    }
    "profile runtime".to_string()
}

pub(crate) fn operator_rulebook_label(config: &Config) -> String {
    format!("rulebook {}", config.permissions.approval_policy.value())
}

pub(crate) fn operator_status_bar_for_config(
    config: &Config,
    model: impl Into<String>,
) -> crate::operator_ui::OperatorStatusBarState {
    let mut status_bar = crate::operator_ui::OperatorStatusBarState::input(model);
    let validation = status_bar.validation.clone();
    status_bar.validation =
        crate::enforcement_banner::operator_status_validation_label(config, validation);
    status_bar
        .with_profile_rulebook(operator_profile_label(config), operator_rulebook_label(config))
}

fn session_provider_label(config: &Config, session: &ThreadSessionState) -> String {
    let provider_name = config.model_provider.name.trim();
    if !provider_name.is_empty() {
        return provider_name.to_string();
    }
    if !session.model_provider_id.trim().is_empty() {
        return session.model_provider_id.clone();
    }
    config.model_provider_id.clone()
}

fn short_session_label(thread_id: ThreadId) -> String {
    let label = format!("{thread_id}");
    label.chars().take(8).collect()
}

pub(crate) fn is_yolo_mode(config: &Config) -> bool {
    has_yolo_permissions(
        config.permissions.approval_policy.value(),
        &config.permissions.permission_profile(),
    )
}

fn has_yolo_permissions(
    approval_policy: AskForApproval,
    permission_profile: &PermissionProfile,
) -> bool {
    let permission_profile = AppServerPermissionProfile::from(permission_profile.clone());
    approval_policy == AskForApproval::Never
        && matches!(
            permission_profile,
            AppServerPermissionProfile::Disabled
                | AppServerPermissionProfile::Managed {
                    file_system: PermissionProfileFileSystemPermissions::Unrestricted,
                    network: PermissionProfileNetworkPermissions { enabled: true },
                }
        )
}

fn mcp_auth_status_label(status: McpAuthStatus) -> &'static str {
    match status {
        McpAuthStatus::Unsupported => "Unsupported",
        McpAuthStatus::NotLoggedIn => "Not logged in",
        McpAuthStatus::BearerToken => "Bearer token",
        McpAuthStatus::OAuth => "OAuth",
    }
}

pub(crate) fn new_user_prompt(
    message: String,
    text_elements: Vec<TextElement>,
    local_image_paths: Vec<PathBuf>,
    remote_image_urls: Vec<String>,
) -> UserHistoryCell {
    UserHistoryCell {
        message,
        text_elements,
        local_image_paths,
        remote_image_urls,
    }
}

#[derive(Debug)]
pub(crate) struct SessionHeaderHistoryCell {
    version: &'static str,
    model: String,
    model_style: Style,
    reasoning_effort: Option<ReasoningEffortConfig>,
    show_fast_status: bool,
    directory: PathBuf,
    yolo_mode: bool,
    provider: Option<String>,
    session_label: Option<String>,
    profile_label: Option<String>,
    rulebook_label: Option<String>,
}

impl SessionHeaderHistoryCell {
    pub(crate) fn new(
        model: String,
        reasoning_effort: Option<ReasoningEffortConfig>,
        show_fast_status: bool,
        directory: PathBuf,
        version: &'static str,
    ) -> Self {
        Self::new_with_style(
            model,
            Style::default(),
            reasoning_effort,
            show_fast_status,
            directory,
            version,
        )
    }

    pub(crate) fn new_with_style(
        model: String,
        model_style: Style,
        reasoning_effort: Option<ReasoningEffortConfig>,
        show_fast_status: bool,
        directory: PathBuf,
        version: &'static str,
    ) -> Self {
        Self {
            version,
            model,
            model_style,
            reasoning_effort,
            show_fast_status,
            directory,
            yolo_mode: false,
            provider: None,
            session_label: None,
            profile_label: None,
            rulebook_label: None,
        }
    }

    pub(crate) fn with_yolo_mode(mut self, yolo_mode: bool) -> Self {
        self.yolo_mode = yolo_mode;
        self
    }

    pub(crate) fn with_provider(mut self, provider: String) -> Self {
        self.provider = Some(provider);
        self
    }

    pub(crate) fn with_session(mut self, session_label: String) -> Self {
        self.session_label = Some(session_label);
        self
    }

    pub(crate) fn with_operator_context(
        mut self,
        profile_label: String,
        rulebook_label: String,
    ) -> Self {
        self.profile_label = Some(profile_label);
        self.rulebook_label = Some(rulebook_label);
        self
    }

    fn format_directory(&self, max_width: Option<usize>) -> String {
        Self::format_directory_inner(&self.directory, max_width)
    }

    fn format_directory_inner(directory: &Path, max_width: Option<usize>) -> String {
        let formatted = if let Some(rel) = relativize_to_home(directory) {
            if rel.as_os_str().is_empty() {
                "~".to_string()
            } else {
                format!("~{}{}", std::path::MAIN_SEPARATOR, rel.display())
            }
        } else {
            directory.display().to_string()
        };

        if let Some(max_width) = max_width {
            if max_width == 0 {
                return String::new();
            }
            if UnicodeWidthStr::width(formatted.as_str()) > max_width {
                return crate::text_formatting::center_truncate_path(&formatted, max_width);
            }
        }

        formatted
    }

    fn reasoning_label(&self) -> Option<&'static str> {
        self.reasoning_effort.map(|effort| match effort {
            ReasoningEffortConfig::Minimal => "minimal",
            ReasoningEffortConfig::Low => "low",
            ReasoningEffortConfig::Medium => "medium",
            ReasoningEffortConfig::High => "high",
            ReasoningEffortConfig::XHigh => "xhigh",
            ReasoningEffortConfig::None => "none",
        })
    }
}

impl HistoryCell for SessionHeaderHistoryCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        let inner_width = usize::from(width.saturating_sub(4)).max(1);
        let provider = self
            .provider
            .clone()
            .unwrap_or_else(|| "default".to_string());
        let cwd = self.format_directory(Some(inner_width.saturating_sub(16)));
        let session = self
            .session_label
            .clone()
            .unwrap_or_else(|| "local".to_string());
        let mut snapshot = crate::operator_ui::StartupSnapshot::from_session(
            self.version,
            provider,
            self.model.clone(),
            cwd,
            session,
        );
        if let Some(profile_label) = self.profile_label.clone() {
            apply_startup_snapshot_row_value(
                &mut snapshot,
                "profile",
                strip_status_label_prefix(&profile_label, "profile"),
                "resolved from active config",
            );
            snapshot.status_bar.profile = profile_label;
        }
        if let Some(rulebook_label) = self.rulebook_label.clone() {
            apply_startup_snapshot_row_value(
                &mut snapshot,
                "rulebook",
                strip_status_label_prefix(&rulebook_label, "rulebook"),
                "resolved from policy config",
            );
            snapshot.status_bar.rulebook = rulebook_label;
        }
        if self.yolo_mode {
            snapshot
                .rows_left
                .push(crate::operator_ui::StartupSnapshotRow::new(
                    "permissions",
                    "YOLO mode",
                    "operator-gated profile",
                ));
        }
        if let Some(reasoning) = self.reasoning_label() {
            snapshot
                .rows_left
                .push(crate::operator_ui::StartupSnapshotRow::new(
                    "reasoning",
                    reasoning,
                    "active effort",
                ));
        }
        if self.show_fast_status {
            snapshot
                .rows_left
                .push(crate::operator_ui::StartupSnapshotRow::new(
                    "tier",
                    "fast",
                    "low-latency inference",
                ));
        }

        crate::operator_ui::render_first_launch_visual_lines(&snapshot, inner_width.min(u16::MAX as usize) as u16)
            .into_iter()
            .map(|line| crate::line_truncation::truncate_line_to_width(line, inner_width))
            .collect()
    }
}

fn apply_startup_snapshot_row_value(
    snapshot: &mut crate::operator_ui::StartupSnapshot,
    key: &str,
    value: String,
    detail: &str,
) {
    if let Some(row) = snapshot.rows_left.iter_mut().find(|row| row.key == key) {
        row.value = value;
        row.detail = detail.to_string();
    }
}

fn strip_status_label_prefix(value: &str, prefix: &str) -> String {
    value
        .strip_prefix(prefix)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(value)
        .to_string()
}
