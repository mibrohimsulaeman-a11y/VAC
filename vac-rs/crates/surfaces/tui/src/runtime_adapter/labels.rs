// History label primitives and text/preview/redact helpers.

#![allow(unused_imports)]

use vac_core::local_runtime::ApprovalAction;
use vac_core::local_runtime::ApprovalDecision;
use vac_core::local_runtime::RuntimeEvent;
use vac_core::local_runtime::RuntimeTaskKind;
use vac_core::local_runtime::ToolCallFinished;
use vac_core::local_runtime::ToolCallId;
use vac_core::local_runtime::ToolCallStarted;
use vac_core::local_runtime::ValidationStatus;

use vac_protocol::protocol::ExecCommandStatus;
use vac_protocol::protocol::TurnAbortReason;

/// Maximum number of characters in a runtime-row preview; longer previews
/// are truncated with an ellipsis. Kept short so a single TUI history row
/// stays readable on narrow terminals.
pub(super) const RUNTIME_PREVIEW_MAX_CHARS: usize = 96;

pub(super) fn short_runtime_id<T: ToString>(id: T) -> String {
    id.to_string().chars().take(8).collect()
}

pub(super) fn task_kind_label(kind: RuntimeTaskKind) -> &'static str {
    match kind {
        RuntimeTaskKind::SemanticCoding => "semantic coding",
        RuntimeTaskKind::Workflow => "workflow",
        RuntimeTaskKind::Review => "review",
        RuntimeTaskKind::Apply => "apply",
        RuntimeTaskKind::Maintenance => "maintenance",
    }
}

pub(super) fn validation_status_label(status: ValidationStatus) -> &'static str {
    match status {
        ValidationStatus::Passed => "passed",
        ValidationStatus::Failed => "failed",
        ValidationStatus::Skipped => "skipped",
        ValidationStatus::Cancelled => "cancelled",
    }
}

pub(super) fn approval_decision_label(decision: ApprovalDecision) -> &'static str {
    match decision {
        ApprovalDecision::Approved => "approved",
        ApprovalDecision::Rejected => "rejected",
        ApprovalDecision::Cancelled => "cancelled",
        ApprovalDecision::Timeout => "timed out",
    }
}

/// Display mode for runtime history rows. `Activity` is the default
/// user-facing presentation introduced by slice 00D-7; `VerboseRuntime`
/// preserves the original `vac runtime:` audit trace and is opt-in via the
/// `VAC_RUNTIME_VERBOSE` environment variable. Internal terminology
/// (Local Runtime Contract, RuntimeEvent, RuntimeBridge) is unchanged —
/// only the rendered prefix shifts so users see "work log" framing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuntimeLabelMode {
    Activity,
    VerboseRuntime,
}

pub(super) fn runtime_label_mode() -> RuntimeLabelMode {
    if std::env::var_os("VAC_RUNTIME_VERBOSE").is_some() {
        RuntimeLabelMode::VerboseRuntime
    } else {
        RuntimeLabelMode::Activity
    }
}

/// Public entry-point: render an optional history row label for a
/// [`RuntimeEvent`]. In activity mode (default) noisy events are suppressed
/// (`None`) and visible rows use the user-facing "VAC activity:" prefix.
/// In verbose mode every event renders as a `vac runtime:` audit row.
pub(crate) fn runtime_history_label(event: &RuntimeEvent) -> Option<String> {
    match runtime_label_mode() {
        RuntimeLabelMode::Activity => activity_history_label(event),
        RuntimeLabelMode::VerboseRuntime => Some(verbose_runtime_history_label(event)),
    }
}

/// User-facing, milestone-only label for a [`RuntimeEvent`]. Returns `None`
/// for events that should not surface in normal mode (session start, raw
/// assistant streaming deltas, exec started — those are evident from the
/// existing `Explored`/tool cells or from the paired "command completed" row).
pub(crate) fn activity_history_label(event: &RuntimeEvent) -> Option<String> {
    match event {
        RuntimeEvent::SessionStarted(_) => None,
        RuntimeEvent::TaskStarted(t) => Some(format!(
            "VAC activity: task started — {}",
            task_kind_label(t.task.kind),
        )),
        RuntimeEvent::AssistantDelta(_) => None,
        RuntimeEvent::ToolCallStarted(t) if t.tool_name == "exec" => None,
        RuntimeEvent::ToolCallStarted(t) => Some(activity_label_for_tool_started(t)),
        RuntimeEvent::ToolCallFinished(t) => Some(activity_label_for_tool_finished(t)),
        RuntimeEvent::ApprovalRequested(req) => Some(format!(
            "VAC activity: approval requested — {}",
            approval_kind_label(req.action),
        )),
        RuntimeEvent::ApprovalResolved(res) => Some(format!(
            "VAC activity: approval {} — request",
            approval_decision_label(res.decision),
        )),
        RuntimeEvent::ValidationStarted(v) => Some(format!(
            "VAC activity: check started — {}",
            preview_inline(&v.command_display),
        )),
        RuntimeEvent::ValidationFinished(v) => Some(format!(
            "VAC activity: check {} — {}",
            validation_status_label(v.status),
            preview_inline(&v.command_display),
        )),
        RuntimeEvent::TaskCompleted(_) => Some("VAC activity: task completed".to_string()),
        RuntimeEvent::TaskFailed(f) => Some(format!(
            "VAC activity: task failed — {}",
            preview_inline(&f.error.message),
        )),
        RuntimeEvent::TaskCancelled(c) => match c.reason.as_deref() {
            Some("interrupted") => Some("VAC activity: task interrupted".to_string()),
            Some(reason) => Some(format!(
                "VAC activity: task cancelled — {}",
                preview_inline(reason),
            )),
            None => Some("VAC activity: task cancelled".to_string()),
        },
        RuntimeEvent::SessionEnded(_) => None,
        RuntimeEvent::EnteredReviewMode(_) => None,
        RuntimeEvent::ExitedReviewMode(_) => None,
    }
}

pub(super) fn activity_label_for_tool_started(t: &ToolCallStarted) -> String {
    if let Some(rest) = t.tool_name.strip_prefix("mcp:") {
        return format!("VAC activity: mcp started — {}", preview_inline(rest));
    }
    match t.input_preview.as_deref() {
        Some(p) => format!(
            "VAC activity: tool started — {}: {}",
            preview_inline(&t.tool_name),
            preview_inline(p),
        ),
        None => format!(
            "VAC activity: tool started — {}",
            preview_inline(&t.tool_name)
        ),
    }
}

pub(super) fn activity_label_for_tool_finished(t: &ToolCallFinished) -> String {
    let outcome = if t.success { "completed" } else { "failed" };

    if t.tool_name == "exec" {
        return match t.output_preview.as_deref() {
            Some(p) => format!("VAC activity: command {outcome} — {}", preview_inline(p)),
            None => format!("VAC activity: command {outcome}"),
        };
    }

    if let Some(rest) = t.tool_name.strip_prefix("mcp:") {
        return format!("VAC activity: mcp {outcome} — {}", preview_inline(rest));
    }

    format!(
        "VAC activity: tool {outcome} — {}",
        preview_inline(&t.tool_name)
    )
}

/// Verbose, full-trace label kept under `VAC_RUNTIME_VERBOSE`. Same content
/// as the original 00D-6 `runtime_history_label`; existing audit tests are
/// pinned against this directly so the verbose grammar is locked.
pub(crate) fn verbose_runtime_history_label(event: &RuntimeEvent) -> String {
    match event {
        RuntimeEvent::SessionStarted(s) => format!(
            "vac runtime: session started — {} · {} · {}",
            short_runtime_id(s.session.id),
            s.session.entrypoint,
            s.session.autonomy_mode,
        ),
        RuntimeEvent::TaskStarted(t) => format!(
            "vac runtime: task started — {}",
            task_kind_label(t.task.kind),
        ),
        RuntimeEvent::AssistantDelta(_) => "vac runtime: assistant streaming".to_string(),
        RuntimeEvent::ToolCallStarted(t) => label_for_tool_started(t),
        RuntimeEvent::ToolCallFinished(t) => label_for_tool_finished(t),
        RuntimeEvent::ApprovalRequested(req) => format!(
            "vac runtime: approval requested — {}",
            approval_kind_label(req.action),
        ),
        RuntimeEvent::ApprovalResolved(res) => format!(
            "vac runtime: approval {} — request",
            approval_decision_label(res.decision),
        ),
        RuntimeEvent::ValidationStarted(v) => format!(
            "vac runtime: validation started — {}",
            preview_inline(&v.command_display),
        ),
        RuntimeEvent::ValidationFinished(v) => format!(
            "vac runtime: validation {} — {}",
            validation_status_label(v.status),
            preview_inline(&v.command_display),
        ),
        RuntimeEvent::TaskCompleted(_) => "vac runtime: task completed".to_string(),
        RuntimeEvent::TaskFailed(f) => format!(
            "vac runtime: task failed — {}",
            preview_inline(&f.error.message),
        ),
        RuntimeEvent::TaskCancelled(c) => match c.reason.as_deref() {
            Some("interrupted") => "vac runtime: task interrupted".to_string(),
            Some(reason) => format!("vac runtime: task cancelled — {}", preview_inline(reason),),
            None => "vac runtime: task cancelled".to_string(),
        },
        RuntimeEvent::SessionEnded(s) => format!(
            "vac runtime: session ended — {}",
            short_runtime_id(s.session_id),
        ),
        RuntimeEvent::EnteredReviewMode(_) => "vac runtime: entered review mode".to_string(),
        RuntimeEvent::ExitedReviewMode(_) => "vac runtime: exited review mode".to_string(),
    }
}

pub(super) fn label_for_tool_started(t: &ToolCallStarted) -> String {
    if t.tool_name == "exec" {
        return match t.input_preview.as_deref() {
            Some(p) => format!("vac runtime: exec started — {}", preview_inline(p)),
            None => "vac runtime: exec started".to_string(),
        };
    }
    if let Some(rest) = t.tool_name.strip_prefix("mcp:") {
        return format!("vac runtime: mcp started — {}", preview_inline(rest));
    }
    match t.input_preview.as_deref() {
        Some(p) => format!(
            "vac runtime: tool started — {}: {}",
            preview_inline(&t.tool_name),
            preview_inline(p),
        ),
        None => format!(
            "vac runtime: tool started — {}",
            preview_inline(&t.tool_name),
        ),
    }
}

pub(super) fn label_for_tool_finished(t: &ToolCallFinished) -> String {
    let outcome = if t.success { "done" } else { "failed" };
    if t.tool_name == "exec" {
        return match t.output_preview.as_deref() {
            Some(p) => format!("vac runtime: exec {outcome} — {}", preview_inline(p)),
            None => format!("vac runtime: exec {outcome}"),
        };
    }
    if let Some(rest) = t.tool_name.strip_prefix("mcp:") {
        return format!("vac runtime: mcp {outcome} — {}", preview_inline(rest));
    }
    format!(
        "vac runtime: tool {outcome} — {}",
        preview_inline(&t.tool_name),
    )
}

pub(super) fn approval_kind_label(action: ApprovalAction) -> &'static str {
    match action {
        ApprovalAction::ExecuteProcess => "exec",
        ApprovalAction::WriteFiles => "patch",
        ApprovalAction::ConnectorCall => "mcp elicitation",
        ApprovalAction::Other => "permissions",
        ApprovalAction::NetworkAccess => "network",
        ApprovalAction::Restore => "restore",
    }
}

pub(super) fn tool_call_id_from_string(value: &str) -> ToolCallId {
    ToolCallId::from_string(value).unwrap_or_default()
}

/// Build a safe, operator-friendly display string for an exec command.
/// Strips shell wrappers (`bash -lc "…"`, `zsh -lc "…"`, etc.) using the
/// shared parser, then sanitizes the result with [`safe_preview`] so
/// runtime rows show command intent rather than raw shell mechanics or
/// obvious secrets.
pub(super) fn command_display(command: &[String]) -> String {
    if command.is_empty() {
        return "<empty command>".to_string();
    }
    safe_preview(&crate::exec_command::strip_bash_lc_and_escape(command))
}

/// Apply secret redaction and inline truncation to a raw preview string.
pub(super) fn safe_preview(raw: &str) -> String {
    preview_inline(&redact_obvious_secrets(raw))
}

/// Inline preview with empty fallback, suitable for embedding directly in
/// an `"... — {preview}"` history label.
pub(super) fn preview_inline(text: &str) -> String {
    preview_text(text).unwrap_or_else(|| "<empty>".to_string())
}

/// Collapse whitespace, drop empty input, and truncate to
/// [`RUNTIME_PREVIEW_MAX_CHARS`] characters with an ellipsis.
pub(super) fn preview_text(text: &str) -> Option<String> {
    let trimmed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.chars().count() <= RUNTIME_PREVIEW_MAX_CHARS {
        return Some(trimmed);
    }
    let mut preview = trimmed
        .chars()
        .take(RUNTIME_PREVIEW_MAX_CHARS)
        .collect::<String>();
    preview.push_str("...");
    Some(preview)
}

/// Lightweight, conservative secret scrubber for runtime preview strings.
/// Targets the three common patterns we see in real prompts:
///   * `KEY=value` style assignments where KEY contains "token", "secret",
///     "password", "passwd", "api_key"/"apikey", or "auth".
///   * Long-form CLI flags whose argument is the secret
///     (`--token <val>`, `--password <val>`, `--authorization <val>`, …).
///   * `Bearer <token>` prefixes inside header strings.
///
/// Intentionally NOT a full shell parser: false-positives are tolerated to
/// avoid leaking real secrets into TUI history. Short flags like `-p` are
/// left alone because they are commonly used for package/project (e.g.
/// `cargo check -p vac-surface-tui`).
pub(super) fn redact_obvious_secrets(text: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut redact_next = false;
    for part in text.split_whitespace() {
        if redact_next {
            out.push("[redacted]".to_string());
            redact_next = false;
            continue;
        }
        let lower = part.to_ascii_lowercase();
        if is_secret_assignment(&lower) {
            let key = part.split_once('=').map(|(k, _)| k).unwrap_or(part);
            out.push(format!("{key}=[redacted]"));
            continue;
        }
        if is_secret_flag(&lower) {
            out.push(part.to_string());
            redact_next = true;
            continue;
        }
        if lower.starts_with("bearer ") {
            out.push("Bearer [redacted]".to_string());
            continue;
        }
        out.push(part.to_string());
    }
    out.join(" ")
}

pub(super) fn is_secret_assignment(lower: &str) -> bool {
    lower.contains("token=")
        || lower.contains("secret=")
        || lower.contains("password=")
        || lower.contains("passwd=")
        || lower.contains("api_key=")
        || lower.contains("apikey=")
        || lower.contains("auth=")
}

pub(super) fn is_secret_flag(lower: &str) -> bool {
    matches!(
        lower,
        "--token"
            | "--secret"
            | "--password"
            | "--passwd"
            | "--api-key"
            | "--apikey"
            | "--authorization"
            | "--auth"
    )
}

pub(super) fn is_validation_command(command: &[String]) -> bool {
    if command.is_empty() {
        return false;
    }
    // Use the same shell-wrapper stripping as the display path so
    // `bash -lc "cargo check -p vac-surface-tui"` is correctly classified.
    let display = crate::exec_command::strip_bash_lc_and_escape(command).to_ascii_lowercase();
    [
        "cargo test",
        "cargo check",
        "cargo fmt",
        "cargo clippy",
        "npm test",
        "pnpm test",
        "bun test",
        "pytest",
        "go test",
        "just test",
        "make test",
        "deno test",
    ]
    .iter()
    .any(|needle| display.contains(needle))
}

#[allow(dead_code)]
pub(super) fn validation_status_from_exec_status(status: &ExecCommandStatus) -> ValidationStatus {
    match status {
        ExecCommandStatus::Completed => ValidationStatus::Passed,
        ExecCommandStatus::Failed => ValidationStatus::Failed,
        ExecCommandStatus::Declined => ValidationStatus::Cancelled,
    }
}

#[allow(dead_code)]
pub(super) fn turn_abort_reason_label(reason: TurnAbortReason) -> &'static str {
    match reason {
        TurnAbortReason::Interrupted => "interrupted",
        TurnAbortReason::Replaced => "replaced",
        TurnAbortReason::ReviewEnded => "review ended",
        TurnAbortReason::BudgetLimited => "budget limited",
    }
}
