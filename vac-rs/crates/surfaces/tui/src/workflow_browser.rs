use crate::bottom_pane::BottomPaneView;
use crate::bottom_pane::CancellationEvent;
use crate::legacy_core::config::Config;
use crate::render::renderable::Renderable;
use crate::workflow_progress::WorkflowProgressModel;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;
use ratatui::widgets::Wrap;
use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use vac_core::control_plane::load_workflow_run_report;
use vac_core::control_plane::workflow_manifest::WorkflowInputDefault;
use vac_core::control_plane::workflow_manifest::WorkflowInputKind;
use vac_core::control_plane::workflow_manifest::WorkflowInputSpec;
use vac_core::control_plane::workflow_manifest::WorkflowManifest;
use vac_core::control_plane::workflow_manifest::WorkflowPolicy;
use vac_core::control_plane::workflow_manifest::WorkflowPolicyValue;
use vac_core::control_plane::workflow_manifest::WorkflowStatus;
use vac_core::control_plane::workflow_manifest::WorkflowStep;
use vac_core::control_plane::workflow_manifest::WorkflowStepPolicy;
use vac_core::control_plane::workflow_manifest::WorkflowStepUi;
use vac_core::control_plane::workflow_manifest::WorkflowUi;
use vac_core::control_plane::workflow_manifest::WorkflowValidation;
use vac_core::control_plane::workflow_runner::WorkflowRunEntry;
use vac_core::control_plane::workflow_runner::WorkflowRunPreview;
use vac_core::control_plane::workflow_runner::execute_workflow_manifest;
use vac_core::control_plane::workflow_runner::format_workflow_step_resolution;
use vac_core::control_plane::workflow_runner::resolve_workflow_step;

#[derive(Debug, Clone, Default)]
pub(crate) struct WorkflowBrowserState {
    pub(crate) capability_filter: Option<String>,
    pub(crate) selected_workflow: Option<String>,
    pub(crate) run_requested: bool,
}

impl WorkflowBrowserState {
    pub(crate) fn with_capability_filter(mut self, capability: impl Into<String>) -> Self {
        let capability = normalize_capability_filter(capability.into());
        self.capability_filter = (!capability.is_empty()).then_some(capability);
        self
    }

    pub(crate) fn with_selected_workflow(mut self, workflow_id: impl Into<String>) -> Self {
        let workflow_id = workflow_id.into().trim().to_string();
        self.selected_workflow = (!workflow_id.is_empty()).then_some(workflow_id);
        self
    }

    pub(crate) fn with_run_requested(mut self) -> Self {
        self.run_requested = true;
        self
    }
}

fn normalize_capability_filter(capability: String) -> String {
    capability
        .trim()
        .strip_prefix("capability.")
        .unwrap_or(capability.trim())
        .to_string()
}

fn format_filter_hint(state: &WorkflowBrowserState) -> String {
    match state.capability_filter.as_deref() {
        Some(filter) => {
            format!("filter: capability={filter}  ·  clear with /workflow filter clear")
        }
        None => "filter: <none>  ·  apply with /workflow filter <capability>".to_string(),
    }
}

pub(crate) fn new_workflow_browser_view(
    config: &Config,
    state: WorkflowBrowserState,
) -> WorkflowBrowserView {
    WorkflowBrowserView::new(config.cwd.to_path_buf(), state)
}

pub(crate) struct WorkflowBrowserView {
    lines: Vec<Line<'static>>,
    complete: bool,
}

impl WorkflowBrowserView {
    fn new(cwd: PathBuf, state: WorkflowBrowserState) -> Self {
        Self {
            lines: render_workflow_browser_with_state(cwd.as_path(), &state),
            complete: false,
        }
    }
}

impl Renderable for WorkflowBrowserView {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        Paragraph::new(self.lines.clone())
            .wrap(Wrap { trim: false })
            .render(area, buf);
    }

    fn desired_height(&self, width: u16) -> u16 {
        Paragraph::new(self.lines.clone())
            .wrap(Wrap { trim: false })
            .line_count(width)
            .try_into()
            .unwrap_or(u16::MAX)
    }
}

impl BottomPaneView for WorkflowBrowserView {
    fn handle_key_event(&mut self, key_event: KeyEvent) {
        if matches!(key_event.code, KeyCode::Esc | KeyCode::Char('q')) {
            self.complete = true;
        }
    }

    fn is_complete(&self) -> bool {
        self.complete
    }

    fn view_id(&self) -> Option<&'static str> {
        Some("workflow-browser")
    }

    fn on_ctrl_c(&mut self) -> CancellationEvent {
        self.complete = true;
        CancellationEvent::Handled
    }
}

#[allow(dead_code)]
fn render_workflow_browser_lines(cwd: &Path) -> Vec<Line<'static>> {
    render_workflow_browser_with_state(cwd, &WorkflowBrowserState::default())
}

fn render_workflow_browser_with_state(
    cwd: &Path,
    state: &WorkflowBrowserState,
) -> Vec<Line<'static>> {
    let report = load_workflow_run_report(cwd);
    let mut lines: Vec<Line<'static>> = Vec::new();
    lines.push("/workflow".magenta().into());
    lines.push("──── Workflow Browser ────".dim().into());
    lines.push("".into());
    lines.push(format_filter_hint(state).into());
    lines.push("actions: /workflow run <id> · /workflow inspect <id> · /workflow filter <capability> · Esc/q closes".green().into());
    lines.push("".into());
    lines.push("Control plane registry:".bold().into());
    lines.extend(
        report
            .registry()
            .render_tui_lines()
            .into_iter()
            .map(|line| format!("  {line}").into()),
    );

    if let Some(registry) = report.registry().registry() {
        lines.push("".into());
        let summary =
            workflow_summary_line(registry.workflows.manifests.iter().map(|lm| &lm.manifest));
        lines.push(format!("Workflows: {summary}").bold().into());
        if registry.workflows.manifests.is_empty() {
            if state.capability_filter.is_some() || state.selected_workflow.is_some() {
                lines.push(
                    "  <none matched current /workflow filter or selection>"
                        .dim()
                        .into(),
                );
            } else {
                lines.push("  <none>".dim().into());
            }
        } else {
            let mut rendered_workflows = 0usize;
            for (index, located_manifest) in registry.workflows.manifests.iter().enumerate() {
                if index > 0 {
                    lines.push("    ────────────────".dim().into());
                }
                if let Some(selected) = state.selected_workflow.as_deref()
                    && located_manifest.manifest.id != selected
                {
                    lines.push(
                        format!(
                            "  {}. {} — skipped by workflow selection '{selected}'",
                            index + 1,
                            located_manifest.manifest.id
                        )
                        .dim()
                        .into(),
                    );
                    continue;
                }
                if let Some(filter) = state.capability_filter.as_deref() {
                    let matches_filter = located_manifest.manifest.steps.iter().any(|step| {
                        step.uses
                            .strip_prefix("capability.")
                            .is_some_and(|tail| tail == filter)
                    });
                    if !matches_filter {
                        lines.push(
                            format!(
                                "  {}. {} — skipped by capability filter '{filter}'",
                                index + 1,
                                located_manifest.manifest.id
                            )
                            .dim()
                            .into(),
                        );
                        continue;
                    }
                }
                rendered_workflows = rendered_workflows.saturating_add(1);
                let run_entry = report
                    .workflows()
                    .iter()
                    .find(|entry| entry.id == located_manifest.manifest.id);
                lines.extend(render_workflow_manifest_lines(
                    index + 1,
                    &located_manifest.manifest,
                    run_entry,
                    state,
                ));
            }
            if rendered_workflows == 0 {
                lines.push(
                    "  <none matched current /workflow filter or selection>"
                        .dim()
                        .into(),
                );
            }
        }
    }

    lines.push("".into());
    lines.push("──── End of Workflow Browser ────".dim().into());
    lines.push(
        "persistent pane: active; Esc/q closes · refresh with /workflow"
            .dim()
            .into(),
    );

    lines
}

fn render_workflow_manifest_lines(
    index: usize,
    manifest: &WorkflowManifest,
    run_entry: Option<&WorkflowRunEntry>,
    state: &WorkflowBrowserState,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    lines.push(
        format!("  {index}. {} — {}", manifest.id, manifest.title)
            .bold()
            .into(),
    );
    if state.selected_workflow.as_deref() == Some(manifest.id.as_str()) {
        lines.push("     selected: true".into());
        if state.run_requested {
            lines.push("     run: executing via TUI workflow runner (not display-only)".into());
        }
    }
    lines.push(
        format!(
            "     status: {} {}",
            format_workflow_status(manifest.status),
            format_workflow_status_icon(manifest.status)
        )
        .into(),
    );
    lines.push(format!("     inputs: {}", format_workflow_inputs(&manifest.inputs)).into());

    if manifest.steps.is_empty() {
        lines.push("     steps: <none>".into());
    } else {
        lines.push("     steps:".bold().into());
        lines.push("     step resolution:".into());
        for (step_index, step) in manifest.steps.iter().enumerate() {
            lines.extend(render_workflow_step_lines(step_index + 1, step));
        }
    }

    lines.push(format!("     ui: {}", format_workflow_ui(&manifest.ui)).into());
    lines.push(format!("     policy: {}", format_workflow_policy(&manifest.policy)).into());
    if let Some(entry) = run_entry {
        for line in entry.approval_policy.render_lines() {
            lines.push(format!("     {line}").into());
        }
        for decision in entry.policy_decisions.iter().flatten() {
            lines.push(format!("     policy decision: {}", decision.summary()).into());
        }
        if manifest.ui.evidence_surface {
            if let Some(identity_check) = entry.identity_check.as_ref() {
                lines.push("     identity check:".into());
                for line in identity_check.render_lines() {
                    lines.push(format!("       {line}").into());
                }
            }
            if let Some(no_duplicate_tui) = entry.no_duplicate_tui.as_ref() {
                lines.push("     tui uniqueness:".into());
                for line in no_duplicate_tui.render_lines() {
                    lines.push(format!("       {line}").into());
                }
            }
            if let Some(architecture_invariants) = entry.architecture_invariants.as_ref() {
                lines.push("     architecture invariants:".into());
                for line in architecture_invariants.render_lines() {
                    lines.push(format!("       {line}").into());
                }
            }
            if let Some(ownership_scan) = entry.ownership_scan.as_ref() {
                lines.push("     ownership scan:".into());
                for line in ownership_scan.render_lines() {
                    lines.push(format!("       {line}").into());
                }
            }
        } else {
            lines.push("     evidence surface: disabled by manifest.ui.evidence_surface".into());
        }
        lines.push(
            format!(
                "     validation: {}",
                format_workflow_validation(&manifest.validation)
            )
            .into(),
        );
        lines.push(
            format!(
                "     runner: {}",
                format_workflow_runner_preview(&entry.preview)
            )
            .into(),
        );
        lines.push(
            format!(
                "     dry-run: supported={} blocked={}",
                entry.dry_run.supported_step_count(),
                entry.dry_run.blocked_step_count()
            )
            .into(),
        );
        for line in entry.dry_run.render_lines() {
            lines.push(format!("       {line}").into());
        }
        let progress =
            WorkflowProgressModel::from_manifest_and_execution(manifest, &entry.execution);
        if manifest.ui.progress_panel {
            for line in progress.render_lines_with(manifest.ui.activity_log) {
                lines.push(format!("       {line}").into());
            }
        } else {
            lines.push("     progress panel: disabled by manifest.ui.progress_panel".into());
        }
        if !manifest.ui.activity_log {
            lines.push("     activity log: disabled by manifest.ui.activity_log".into());
        }
        lines.push(
            format!(
                "     controls: /workflow run {id}  ·  /workflow inspect {id}",
                id = manifest.id
            )
            .into(),
        );
        if manifest.ui.approval_surface {
            for (step_id, request_id) in progress.pending_approval_actions() {
                lines.push(
                    format!(
                        "     approve: step={step_id} request={request_id}  ·  /approvals respond {request_id}"
                    )
                    .into(),
                );
            }
        } else {
            lines.push("     approval surface: disabled by manifest.ui.approval_surface".into());
        }
        if state.run_requested && state.selected_workflow.as_deref() == Some(manifest.id.as_str()) {
            let live_execution = execute_workflow_manifest(manifest);
            let live_progress =
                WorkflowProgressModel::from_manifest_and_execution(manifest, &live_execution);
            lines.push(
                "     live execution: completed by TUI workflow runner"
                    .green()
                    .bold()
                    .into(),
            );
            for line in live_progress.render_lines_with(true) {
                lines.push(format!("       {line}").into());
            }
        }
        let capability_ids = progress.capability_ids();
        if !capability_ids.is_empty() {
            lines.push(format!("     capabilities: {}", capability_ids.join(", ")).into());
        }
    } else {
        lines.push(
            format!(
                "     validation: {}",
                format_workflow_validation(&manifest.validation)
            )
            .into(),
        );
        if state.run_requested && state.selected_workflow.as_deref() == Some(manifest.id.as_str()) {
            let live_execution = execute_workflow_manifest(manifest);
            let live_progress =
                WorkflowProgressModel::from_manifest_and_execution(manifest, &live_execution);
            lines.push(
                "     runner: executed directly from manifest (registry run report missing)"
                    .green()
                    .bold()
                    .into(),
            );
            for line in live_progress.render_lines_with(true) {
                lines.push(format!("       {line}").into());
            }
        } else {
            lines.push("     runner: unavailable: workflow run report missing".into());
        }
        lines.push(
            format!(
                "     controls: /workflow run {id}  ·  /workflow inspect {id}  ·  (direct run available)",
                id = manifest.id
            )
            .into(),
        );
    }
    lines
}

fn render_workflow_step_lines(index: usize, step: &WorkflowStep) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    lines.push(format!("       {index}. id: {} uses: {}", step.id, step.uses).into());
    let resolution = resolve_workflow_step(step);
    lines.push(
        format!(
            "          runner: {}",
            format_workflow_step_resolution(&resolution)
        )
        .into(),
    );
    if let Some(when) = step.when.as_deref() {
        lines.push(format!("          when: {when}").into());
    }
    if let Some(policy) = step.policy.as_ref() {
        lines.push(format!("          policy: {}", format_workflow_step_policy(policy)).into());
    }
    if let Some(ui) = step.ui.as_ref() {
        lines.push(format!("          ui: {}", format_workflow_step_ui(ui)).into());
    }
    lines
}

fn format_workflow_status_icon(status: WorkflowStatus) -> &'static str {
    match status {
        WorkflowStatus::Planned => "○",
        WorkflowStatus::Partial => "◐",
        WorkflowStatus::Ready => "●",
        WorkflowStatus::Disabled => "✕",
    }
}

fn workflow_summary_line<'a>(manifests: impl Iterator<Item = &'a WorkflowManifest>) -> String {
    let mut total = 0usize;
    let mut ready = 0usize;
    let mut partial = 0usize;
    let mut planned = 0usize;
    let mut disabled = 0usize;
    for manifest in manifests {
        total = total.saturating_add(1);
        match manifest.status {
            WorkflowStatus::Ready => ready = ready.saturating_add(1),
            WorkflowStatus::Partial => partial = partial.saturating_add(1),
            WorkflowStatus::Planned => planned = planned.saturating_add(1),
            WorkflowStatus::Disabled => disabled = disabled.saturating_add(1),
        }
    }
    if total == 0 {
        return "0 total".to_string();
    }
    let mut parts: Vec<String> = vec![format!("{total} total")];
    if ready > 0 {
        parts.push(format!("● {ready} ready"));
    }
    if partial > 0 {
        parts.push(format!("◐ {partial} partial"));
    }
    if planned > 0 {
        parts.push(format!("○ {planned} planned"));
    }
    if disabled > 0 {
        parts.push(format!("✕ {disabled} disabled"));
    }
    parts.join(" · ")
}

fn format_workflow_status(status: WorkflowStatus) -> &'static str {
    match status {
        WorkflowStatus::Planned => "planned",
        WorkflowStatus::Partial => "partial",
        WorkflowStatus::Ready => "ready",
        WorkflowStatus::Disabled => "disabled",
    }
}

fn format_workflow_inputs(inputs: &BTreeMap<String, WorkflowInputSpec>) -> String {
    if inputs.is_empty() {
        return "<none>".to_string();
    }
    let mut parts = Vec::new();
    for (name, spec) in inputs {
        let mut entry = format!("{name}={}", format_workflow_input_spec(spec));
        if let Some(description) = spec.description.as_deref() {
            entry.push_str(&format!(" desc={description}"));
        }
        parts.push(entry);
    }
    parts.join(", ")
}

fn format_workflow_input_spec(spec: &WorkflowInputSpec) -> String {
    let mut parts: Vec<String> = vec![format_workflow_input_kind(spec.kind).to_string()];
    if spec.required {
        parts.push("required".to_string());
    } else {
        parts.push("optional".to_string());
    }
    if let Some(default) = spec.default.as_ref() {
        parts.push(format!(
            "default={}",
            format_workflow_input_default(default)
        ));
    }
    if !spec.values.is_empty() {
        parts.push(format!("values=[{}]", spec.values.join(", ")));
    }
    parts.join(" ")
}

fn format_workflow_input_kind(kind: WorkflowInputKind) -> &'static str {
    match kind {
        WorkflowInputKind::String => "string",
        WorkflowInputKind::Enum => "enum",
        WorkflowInputKind::Boolean => "boolean",
        WorkflowInputKind::Integer => "integer",
        WorkflowInputKind::Number => "number",
    }
}

fn format_workflow_input_default(default: &WorkflowInputDefault) -> String {
    match default {
        WorkflowInputDefault::String(value) => value.clone(),
        WorkflowInputDefault::Boolean(value) => value.to_string(),
        WorkflowInputDefault::Integer(value) => value.to_string(),
        WorkflowInputDefault::Number(value) => value.to_string(),
    }
}

fn format_workflow_ui(ui: &WorkflowUi) -> String {
    let mut parts = vec![format!("surface={}", ui.surface)];
    if let Some(inspect_surface) = ui.inspect_surface.as_deref() {
        parts.push(format!("inspect_surface={inspect_surface}"));
    }
    parts.push(format!("progress_panel={}", ui.progress_panel));
    parts.push(format!("activity_log={}", ui.activity_log));
    parts.push(format!("approval_surface={}", ui.approval_surface));
    parts.push(format!("evidence_surface={}", ui.evidence_surface));
    parts.join(", ")
}

fn format_workflow_policy(policy: &WorkflowPolicy) -> String {
    let mut parts = Vec::new();
    if let Some(default_risk) = policy.default_risk.as_deref() {
        parts.push(format!("default_risk={default_risk}"));
    }
    if let Some(mutates_files) = policy.mutates_files.as_ref() {
        parts.push(format!(
            "mutates_files={}",
            format_workflow_policy_value(mutates_files)
        ));
    }
    if let Some(network) = policy.network.as_ref() {
        parts.push(format!("network={}", format_workflow_policy_value(network)));
    }
    if let Some(redaction) = policy.redaction.as_ref() {
        parts.push(format!(
            "redaction={}",
            format_workflow_policy_value(redaction)
        ));
    }
    if !policy.approval_required_for.is_empty() {
        parts.push(format!(
            "approval_required_for=[{}]",
            policy.approval_required_for.join(", ")
        ));
    }
    if parts.is_empty() {
        "<none>".to_string()
    } else {
        parts.join(", ")
    }
}

fn format_workflow_policy_value(value: &WorkflowPolicyValue) -> String {
    match value {
        WorkflowPolicyValue::Bool(value) => value.to_string(),
        WorkflowPolicyValue::Text(value) => value.clone(),
    }
}

fn format_workflow_step_policy(policy: &WorkflowStepPolicy) -> String {
    let mut parts = Vec::new();
    if let Some(default_risk) = policy.default_risk.as_deref() {
        parts.push(format!("default_risk={default_risk}"));
    }
    if let Some(mutates_files) = policy.mutates_files.as_ref() {
        parts.push(format!(
            "mutates_files={}",
            format_workflow_policy_value(mutates_files)
        ));
    }
    if let Some(network) = policy.network.as_ref() {
        parts.push(format!("network={}", format_workflow_policy_value(network)));
    }
    if let Some(redaction) = policy.redaction.as_ref() {
        parts.push(format!(
            "redaction={}",
            format_workflow_policy_value(redaction)
        ));
    }
    if !policy.approval_required_for.is_empty() {
        parts.push(format!(
            "approval_required_for=[{}]",
            policy.approval_required_for.join(", ")
        ));
    }
    if parts.is_empty() {
        "<none>".to_string()
    } else {
        parts.join(", ")
    }
}

fn format_workflow_step_ui(ui: &WorkflowStepUi) -> String {
    let mut parts = Vec::new();
    if let Some(surface) = ui.surface.as_deref() {
        parts.push(format!("surface={surface}"));
    }
    if let Some(inspect_surface) = ui.inspect_surface.as_deref() {
        parts.push(format!("inspect_surface={inspect_surface}"));
    }
    if let Some(progress_panel) = ui.progress_panel {
        parts.push(format!("progress_panel={progress_panel}"));
    }
    if let Some(activity_log) = ui.activity_log {
        parts.push(format!("activity_log={activity_log}"));
    }
    if let Some(approval_surface) = ui.approval_surface {
        parts.push(format!("approval_surface={approval_surface}"));
    }
    if let Some(evidence_surface) = ui.evidence_surface {
        parts.push(format!("evidence_surface={evidence_surface}"));
    }
    if parts.is_empty() {
        "<none>".to_string()
    } else {
        parts.join(", ")
    }
}

fn format_workflow_validation(validation: &WorkflowValidation) -> String {
    let mut parts = Vec::new();
    if !validation.commands.is_empty() {
        parts.push(format!("commands=[{}]", validation.commands.join(", ")));
    }
    if !validation.gates.is_empty() {
        parts.push(format!("gates=[{}]", validation.gates.join(", ")));
    }
    if parts.is_empty() {
        "<none>".to_string()
    } else {
        parts.join(", ")
    }
}

fn format_workflow_runner_preview(preview: &WorkflowRunPreview) -> String {
    if preview.supported {
        return "supported by initial safe runner".to_string();
    }

    let blocked_steps = preview
        .blocked_steps
        .iter()
        .map(|step| format!("{}={}", step.id, step.uses))
        .collect::<Vec<_>>()
        .join(", ");
    format!("blocked: unsupported steps=[{}]", blocked_steps)
}

#[cfg(test)]
mod tests {
    use super::WorkflowBrowserState;
    use super::render_workflow_browser_lines;
    use super::render_workflow_browser_with_state;
    use ratatui::text::Line;
    use std::fs;
    use std::path::Path;
    use tempfile::tempdir;
    use vac_core::control_plane::load_control_plane_registry_report;

    fn render_to_text(lines: &[Line<'static>]) -> String {
        lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn seed_allow_policy(root: &Path) {
        fs::create_dir_all(root.join(".vac/policies")).expect("policies dir");
        fs::write(
            root.join(".vac/policies/default-local.yaml"),
            r#"
schema_version: 1
kind: policy
id: vac.test-allow
title: Test allow policy
default_decision: approval_required
rules:
  - id: allow-project-reads
    match:
      action: filesystem_read
      path: project
    decision: allow
    reason: "Allow filesystem reads during tests"
"#,
        )
        .expect("policy manifest");
    }

    fn seed_core_capabilities(root: &Path) {
        fs::create_dir_all(root.join(".vac/capabilities")).expect("capabilities dir");
        fs::write(
            root.join(".vac/capabilities/build.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.build
title: Build
status: planned
reason: "Planned capability for product build"
owner:
  crate: vac-surface-cli
  module: main
description: Build and maintenance validation gates for the root product.
depends_on:
  - vac.workflow
surfaces:
  palette: true
states:
- empty
- loading
- success
- failure
policy:
  risk: safe_read
  mutates_files: false
  network: false
  redaction: false
  approval_required_for:
    - execute_process
validation:
  commands:
    - CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.95.0 check -p vac-surface-cli
"#,
        )
        .expect("build capability manifest");
        fs::write(
            root.join(".vac/capabilities/workflow.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.workflow
title: Workflow
status: partial
reason: "Partial capability for workflow browser"
owner:
  crate: vac-core
  module: control_plane
description: Typed workflow manifests, registry loading, and diagnostics.
depends_on:
  - vac.sessions
surfaces:
  palette: true
  cli:
    enabled: true
    commands:
      - vac doctor registry
states:
- empty
- loading
- success
- failure
policy:
  risk: safe_read
  mutates_files: false
  network: false
  redaction: false
  approval_required_for:
    - write_files
    - execute_process
validation:
  commands:
    - CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.95.0 check -p vac-surface-cli
"#,
        )
        .expect("workflow capability manifest");
        fs::write(
            root.join(".vac/capabilities/identity-check.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.identity.check
title: Identity Check
status: partial
reason: "Partial capability for identity check"
owner:
  crate: vac-core
  module: control_plane
description: Product identity and legacy term scanning for the root product.
depends_on:
  - vac.workflow
surfaces:
  palette: true
states:
- empty
- loading
- success
- failure
policy:
  risk: safe_read
  mutates_files: false
  network: false
  redaction: false
validation:
  commands:
    - cargo +1.95.0 check -p vac-surface-cli
"#,
        )
        .expect("identity check capability manifest");
    }

    fn seed_tui_uniqueness_capabilities(root: &Path) {
        fs::create_dir_all(root.join(".vac/capabilities")).expect("capabilities dir");
        fs::write(
            root.join(".vac/capabilities/tui.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.tui
title: TUI
status: partial
reason: "Partial capability for product TUI"
owner:
  crate: vac-surface-tui
  module: lib
description: Root interactive frontend and operator cockpit for the product.
depends_on:
  - vac.workflow
  - vac.approvals
surfaces:
  tui:
    routes:
      - /
      - /capabilities
      - /workflow
      - /debug-config
      - /activity
      - /approvals
      - /evidence
  slash:
    commands:
      - /capabilities
      - /workflow
      - /debug-config
      - /activity
      - /approvals
      - /evidence
  palette: true
states:
- empty
- loading
- success
- failure
policy:
  risk: safe_read
  mutates_files: false
  network: false
  redaction: false
validation:
  commands:
    - CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.95.0 check -p vac-surface-tui --tests
"#,
        )
        .expect("tui capability manifest");
        fs::write(
            root.join(".vac/capabilities/workflow.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.workflow
title: Workflow
status: partial
reason: "Partial capability for workflow"
owner:
  crate: vac-core
  module: control_plane
description: Typed workflow manifests, registry loading, and diagnostics.
depends_on:
  - vac.sessions
surfaces:
  palette: true
  cli:
    enabled: true
    commands:
      - vac doctor workflow
states:
- empty
- loading
- success
- failure
policy:
  risk: safe_read
  mutates_files: false
  network: false
  redaction: false
  approval_required_for:
    - write_files
    - execute_process
validation:
  commands:
    - CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.95.0 check -p vac-surface-cli
"#,
        )
        .expect("workflow capability manifest");
    }

    #[test]
    fn workflow_browser_reports_missing_root() {
        let tempdir = tempdir().expect("tempdir");

        let rendered = render_to_text(&render_workflow_browser_lines(tempdir.path()));
        assert!(rendered.contains("registry: empty"));
        assert!(!rendered.contains("registry: failure"));
    }

    #[test]
    fn workflow_browser_lists_loaded_workflows() {
        let tempdir = tempdir().expect("tempdir");
        seed_core_capabilities(tempdir.path());
        fs::write(
            tempdir.path().join(".vac/capabilities/repo-read.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.repo.read
title: Repo read
status: planned
reason: "Planned capability for repo read"
owner:
  crate: vac-core
  module: control_plane
description: Test-only custom capability used to exercise unsupported workflow handlers.
depends_on:
  - vac.workflow
surfaces:
  palette: true
states:
- empty
- loading
- success
- failure
policy:
  risk: safe_read
  mutates_files: false
  network: false
  redaction: false
validation:
  commands:
    - cargo +1.95.0 check -p vac-surface-cli
"#,
        )
        .expect("repo read capability manifest");
        fs::write(
            tempdir.path().join(".vac/capabilities/build-run.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.build.run
title: Build run
status: planned
reason: "Planned capability for build run"
owner:
  crate: vac-surface-cli
  module: main
description: Test-only custom capability used to exercise unsupported workflow handlers.
depends_on:
  - vac.build
surfaces:
  palette: true
states:
- empty
- loading
- success
- failure
policy:
  risk: safe_read
  mutates_files: false
  network: false
  redaction: false
validation:
  commands:
    - cargo +1.95.0 check -p vac-surface-cli
"#,
        )
        .expect("build run capability manifest");
        fs::create_dir_all(tempdir.path().join(".vac/workflows")).expect("workflows dir");
        fs::write(
            tempdir
                .path()
                .join(".vac/workflows/maintenance.build-check.yaml"),
            r#"
schema_version: 1
kind: workflow
id: maintenance.build-check
title: Build check
status: ready
inputs:
  workspace:
    type: string
    required: false
    default: .
steps:
  - id: checkout
    uses: capability.repo.read
  - id: build
    uses: capability.build.run
    when: workspace
ui:
  surface: /workflow
  inspect_surface: /capabilities
  progress_panel: true
  activity_log: true
  approval_surface: false
  evidence_surface: true
policy:
  default_risk: safe_read
  mutates_files: false
  network: false
  redaction: false
  approval_required_for: [write_files]
validation:
  commands:
    - cargo +1.95.0 check -p vac-surface-cli
  gates:
    - build
"#,
        )
        .expect("workflow manifest");

        let report = load_control_plane_registry_report(tempdir.path());
        assert!(report.registry().is_some());

        let rendered = render_to_text(&render_workflow_browser_lines(tempdir.path()));
        assert!(rendered.contains("registry: loaded 6 manifests"));
        assert!(rendered.contains("Workflows:"));
        assert!(rendered.contains("maintenance.build-check"));
        assert!(rendered.contains("Build check"));
        assert!(rendered.contains("status: ready"));
        assert!(rendered.contains("inputs:"));
        assert!(rendered.contains("steps:"));
        assert!(rendered.contains("policy:"));
        assert!(rendered.contains("approval policy:"));
        assert!(rendered.contains("validation:"));
        assert!(rendered.contains("runner: blocked"));
        assert!(rendered.contains("step resolution:"));
        assert!(rendered.contains("dry-run: supported=0 blocked=2"));
        assert!(rendered.contains("dry-run: step 1. checkout -> blocked:"));
        assert!(
            rendered.contains("progress: status=failed"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("detail: missing capability manifest"),
            "rendered:\n{rendered}"
        );
        assert!(rendered.contains("/workflow"));
    }

    #[test]
    fn workflow_browser_marks_supported_runner_preview() {
        let tempdir = tempdir().expect("tempdir");
        seed_core_capabilities(tempdir.path());
        fs::create_dir_all(tempdir.path().join(".vac/workflows")).expect("workflows dir");
        fs::write(
            tempdir.path().join(".vac/workflows/safe.workflow.yaml"),
            r#"
schema_version: 1
kind: workflow
id: maintenance.safe-workflow
title: Safe workflow
status: ready
inputs: {}
steps:
  - id: check
    uses: capability.build.cargo_check
  - id: report
    uses: capability.activity.emit
ui:
  surface: /workflow
  inspect_surface: /capabilities
  progress_panel: true
  activity_log: true
  approval_surface: false
  evidence_surface: true
policy:
  default_risk: safe_read
  mutates_files: false
  network: false
  redaction: false
  approval_required_for: [write_files]
validation:
  commands:
    - cargo +1.95.0 check -p vac-surface-cli
  gates:
    - check
"#,
        )
        .expect("workflow manifest");

        let rendered = render_to_text(&render_workflow_browser_lines(tempdir.path()));
        assert!(
            rendered.contains("runner: supported by initial safe runner"),
            "rendered:\n{rendered}"
        );
        assert!(rendered.contains("approval policy: capability registry=loaded 3"));
        assert!(rendered.contains("runner: handler=build.cargo_check"));
        assert!(rendered.contains("dry-run: supported=2 blocked=0"));
        assert!(rendered.contains("dry-run: step 1. check -> handler=build.cargo_check"));
        assert!(
            rendered.contains("progress: status=waiting_approval"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("lifecycle: waiting_approval"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("waiting approval"),
            "rendered:\n{rendered}"
        );
    }

    #[test]
    fn workflow_browser_reports_no_duplicate_tui_failure() {
        let tempdir = tempdir().expect("tempdir");
        seed_tui_uniqueness_capabilities(tempdir.path());
        seed_allow_policy(tempdir.path());
        fs::create_dir_all(tempdir.path().join("vac-rs/src")).expect("product source dir");
        fs::write(
            tempdir.path().join("vac-rs/src/lib.rs"),
            format!(
                "pub const LEGACY: &str = \"{}\";",
                concat!("vac", "_tui", "_runtime")
            ),
        )
        .expect("product source fixture");
        fs::create_dir_all(tempdir.path().join(".vac/workflows")).expect("workflows dir");
        fs::write(
            tempdir
                .path()
                .join(".vac/workflows/maintenance.no-duplicate-tui.yaml"),
            r#"
schema_version: 1
kind: workflow
id: maintenance.no-duplicate-tui
title: TUI uniqueness check
status: ready
inputs: {}
steps:
  - id: orient
    uses: capability.tui.no_duplicate_runtime
  - id: report
    uses: capability.activity.emit
ui:
  surface: /workflow
  inspect_surface: /capabilities
  progress_panel: true
  activity_log: true
  approval_surface: false
  evidence_surface: true
policy:
  default_risk: safe_read
  mutates_files: false
  network: false
  redaction: false
validation:
  commands:
    - cargo +1.95.0 check -p vac-surface-cli
"#,
        )
        .expect("workflow manifest");

        let rendered = render_to_text(&render_workflow_browser_lines(tempdir.path()));
        assert!(
            rendered.contains("runner: supported by initial safe runner"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("approval policy: capability registry=loaded 2"),
            "rendered:\n{rendered}"
        );
        assert!(rendered.contains("tui uniqueness: scanned="));
        assert!(rendered.contains("findings=1"));
        assert!(rendered.contains("vac-rs/src/lib.rs:1"));
        assert!(rendered.contains("progress: status=failed"));
        assert!(
            rendered.contains("detail: TUI Uniqueness Invariant Violation"),
            "rendered:\n{rendered}"
        );
    }

    #[test]
    fn workflow_browser_tracks_approval_aware_progress() {
        let tempdir = tempdir().expect("tempdir");
        seed_core_capabilities(tempdir.path());
        fs::write(
            tempdir.path().join(".vac/capabilities/identity-check.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.identity.check
title: Identity Check
status: partial
reason: "Partial capability for identity check"
owner:
  crate: vac-core
  module: control_plane
description: Product identity and legacy term scanning for the root product.
depends_on:
  - vac.workflow
surfaces:
  palette: true
states:
- empty
- loading
- success
- failure
policy:
  risk: safe_read
  mutates_files: false
  network: false
  redaction: false
validation:
  commands:
    - cargo +1.95.0 check -p vac-surface-cli
"#,
        )
        .expect("identity check capability manifest");
        fs::create_dir_all(tempdir.path().join(".vac/workflows")).expect("workflows dir");
        fs::write(
            tempdir
                .path()
                .join(".vac/workflows/maintenance.build-check.yaml"),
            r#"
schema_version: 1
kind: workflow
id: maintenance.build-check
title: Build check
status: ready
inputs: {}
steps:
  - id: orient
    uses: capability.identity.check
  - id: validate
    uses: capability.build.cargo_check
  - id: report
    uses: capability.activity.emit
ui:
  surface: /workflow
  inspect_surface: /capabilities
  progress_panel: true
  activity_log: true
  approval_surface: true
  evidence_surface: true
policy:
  default_risk: safe_read
  approval_required_for:
    - execute_process
validation:
  commands:
    - cargo +1.95.0 check -p vac-surface-cli
"#,
        )
        .expect("workflow manifest");

        let rendered = render_to_text(&render_workflow_browser_lines(tempdir.path()));
        assert!(rendered.contains("registry: loaded 4 manifests"));
        assert!(rendered.contains("approval policy: capability registry=loaded 3"));
        assert!(rendered.contains("progress: status=waiting_approval"));
        assert!(rendered.contains("activity log:"));
        assert!(
            rendered.contains("started workflow=maintenance.build-check title=Build check steps=3")
        );
        assert!(rendered.contains("lifecycle: waiting_approval"));
        assert!(rendered.contains("waiting approval"));
    }

    #[test]
    fn workflow_browser_uses_core_runner_report_for_root_seed_coverage() {
        let tempdir = tempdir().expect("tempdir");
        fs::create_dir_all(tempdir.path().join(".vac/capabilities")).expect("capabilities dir");
        fs::create_dir_all(tempdir.path().join(".vac/workflows")).expect("workflows dir");
        seed_allow_policy(tempdir.path());
        fs::write(
            tempdir.path().join(".vac/capabilities/workflow.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.workflow
title: Workflow
status: partial
reason: "Partial capability for workflow"
owner:
  crate: vac-core
  module: control_plane
description: Typed workflow manifests, registry loading, and diagnostics.
surfaces:
  palette: true
states:
- empty
- loading
- success
- failure
policy:
  risk: safe_read
  mutates_files: false
  network: false
  redaction: false
validation:
  commands:
    - cargo +1.95.0 check -p vac-surface-cli
"#,
        )
        .expect("workflow capability manifest");
        fs::write(
            tempdir
                .path()
                .join(".vac/workflows/maintenance.root-seed.yaml"),
            r#"
schema_version: 1
kind: workflow
id: maintenance.root-seed
title: Root seed coverage
status: planned
inputs: {}
steps:
  - id: root_seed_coverage
    uses: capability.root_seed.coverage
ui:
  surface: /workflow
  progress_panel: true
  activity_log: true
  approval_surface: false
  evidence_surface: true
policy:
  default_risk: safe_read
validation:
  commands:
    - vac doctor registry .
"#,
        )
        .expect("workflow manifest");

        let rendered = render_to_text(&render_workflow_browser_lines(tempdir.path()));
        assert!(
            rendered.contains("root seed coverage: blocked"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("runner: handler=root_seed.coverage"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("progress: status=failed"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("detail: root seed coverage: blocked"),
            "rendered:\n{rendered}"
        );
    }

    #[test]
    fn workflow_browser_reports_identity_check_failure() {
        let tempdir = tempdir().expect("tempdir");
        seed_core_capabilities(tempdir.path());
        seed_allow_policy(tempdir.path());
        fs::create_dir_all(tempdir.path().join("vac-rs/src")).expect("product source dir");
        fs::write(
            tempdir.path().join("vac-rs/src/lib.rs"),
            format!(
                "pub const LEGACY: &str = \"{}\";",
                concat!("vac", "_tui", "_runtime")
            ),
        )
        .expect("product source fixture");
        fs::create_dir_all(tempdir.path().join(".vac/workflows")).expect("workflows dir");
        fs::write(
            tempdir
                .path()
                .join(".vac/workflows/maintenance.identity-check.yaml"),
            r#"
schema_version: 1
kind: workflow
id: maintenance.identity-check
title: Identity check
status: planned
inputs: {}
steps:
  - id: orient
    uses: capability.identity.check
  - id: report
    uses: capability.activity.emit
ui:
  surface: /workflow
  inspect_surface: /capabilities
  progress_panel: true
  activity_log: true
  approval_surface: false
  evidence_surface: true
policy:
  default_risk: safe_read
validation:
  commands:
    - cargo +1.95.0 check -p vac-surface-cli
"#,
        )
        .expect("workflow manifest");

        let rendered = render_to_text(&render_workflow_browser_lines(tempdir.path()));
        assert!(
            rendered.contains("registry: loaded"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("identity check: scanned="),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("identity check exemptions:"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("docs/product/CAPABILITY_MAP.md"),
            "rendered:\n{rendered}"
        );
        assert!(rendered.contains("findings=1"), "rendered:\n{rendered}");
        assert!(
            rendered.contains("vac-rs/src/lib.rs:1"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("progress: status=failed"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("reason: identity check found 1 forbidden term"),
            "rendered:\n{rendered}"
        );
        let _ = fs::remove_dir_all(&tempdir);
    }

    #[test]
    fn workflow_browser_reports_build_check_failure() {
        struct EnvVarGuard {
            original_value: Option<String>,
            original_repo_root: Option<String>,
        }
        impl Drop for EnvVarGuard {
            fn drop(&mut self) {
                unsafe {
                    if let Some(ref v) = self.original_value {
                        std::env::set_var("VAC_SKIP_BUILD_CHECK", v);
                    } else {
                        std::env::remove_var("VAC_SKIP_BUILD_CHECK");
                    }
                    if let Some(ref v) = self.original_repo_root {
                        std::env::set_var("VAC_BUILD_CHECK_REPO_ROOT", v);
                    } else {
                        std::env::remove_var("VAC_BUILD_CHECK_REPO_ROOT");
                    }
                }
            }
        }
        let _guard = EnvVarGuard {
            original_value: std::env::var("VAC_SKIP_BUILD_CHECK").ok(),
            original_repo_root: std::env::var("VAC_BUILD_CHECK_REPO_ROOT").ok(),
        };
        unsafe {
            std::env::remove_var("VAC_SKIP_BUILD_CHECK");
            std::env::remove_var("VAC_BUILD_CHECK_REPO_ROOT");
        }
        let tempdir = tempdir().expect("tempdir");
        seed_core_capabilities(tempdir.path());
        fs::write(
            tempdir.path().join(".vac/capabilities/build.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.build
title: Build
status: ready
owner:
  crate: vac-surface-cli
  module: main
description: Build and maintenance validation gates for the root product.
depends_on:
  - vac.workflow
surfaces:
  palette: true
states:
- empty
- loading
- success
- failure
policy:
  risk: safe_read
  mutates_files: false
  network: false
  redaction: false
validation:
  commands:
    - cargo +1.95.0 check -p vac-surface-cli
"#,
        )
        .expect("build capability manifest");
        fs::create_dir_all(tempdir.path().join(".vac/workflows")).expect("workflows dir");
        fs::write(
            tempdir
                .path()
                .join(".vac/workflows/maintenance.build-check.yaml"),
            r#"
schema_version: 1
kind: workflow
id: maintenance.build-check
title: Build check
status: ready
inputs: {}
steps:
  - id: validate
    uses: capability.build.cargo_check
  - id: report
    uses: capability.activity.emit
ui:
  surface: /workflow
  inspect_surface: /capabilities
  progress_panel: true
  activity_log: true
  approval_surface: false
  evidence_surface: true
policy:
  default_risk: safe_read
validation:
  commands:
    - cargo +1.95.0 check -p vac-surface-cli
"#,
        )
        .expect("workflow manifest");

        let rendered = render_to_text(&render_workflow_browser_lines(tempdir.path()));
        assert!(
            rendered.contains("runner: handler=build.cargo_check"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("progress: status=failed"),
            "rendered:\n{rendered}"
        );
        assert!(rendered.contains("activity log:"), "rendered:\n{rendered}");
        assert!(
            rendered.contains(
                "step 1. validate -> failed: build check report unavailable; executor requires explicit approval-gated evidence"
            ),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains(
                "reason: build check report unavailable; executor requires explicit approval-gated evidence"
            ),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains(
                "detail: build check report unavailable; executor requires explicit approval-gated evidence"
            ),
            "rendered:\n{rendered}"
        );
    }

    #[test]
    fn workflow_browser_renders_execution_controls_and_filter_hint() {
        let tempdir = tempdir().expect("tempdir");
        seed_core_capabilities(tempdir.path());
        fs::create_dir_all(tempdir.path().join(".vac/workflows")).expect("workflows dir");
        fs::write(
            tempdir
                .path()
                .join(".vac/workflows/maintenance.controls-render.yaml"),
            r#"
schema_version: 1
kind: workflow
id: maintenance.controls-render
title: Controls render
status: ready
inputs: {}
steps:
  - id: validate
    uses: capability.build.cargo_check
  - id: report
    uses: capability.activity.emit
ui:
  surface: /workflow
  inspect_surface: /capabilities
  progress_panel: true
  activity_log: true
  approval_surface: false
  evidence_surface: true
policy:
  default_risk: safe_read
validation:
  commands:
    - cargo +1.95.0 check -p vac-surface-cli
"#,
        )
        .expect("workflow manifest");

        let rendered = render_to_text(&render_workflow_browser_lines(tempdir.path()));
        assert!(
            rendered.contains("──── Workflow Browser ────"),
            "expected header; rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("filter: <none>"),
            "expected filter hint; rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("/workflow filter <capability>"),
            "expected filter usage; rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("controls: /workflow run maintenance.controls-render"),
            "expected controls hint; rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("capabilities: "),
            "expected capabilities listing; rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("End of Workflow Browser"),
            "expected footer; rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("persistent pane: active"),
            "expected persistent pane footer note; rendered:\n{rendered}"
        );
    }

    #[test]
    fn workflow_browser_state_normalizes_filter_and_reports_empty_matches() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let cwd = tmp.path();
        seed_core_capabilities(cwd);
        seed_allow_policy(cwd);

        let lines = render_to_text(&render_workflow_browser_with_state(
            cwd,
            &WorkflowBrowserState::default()
                .with_capability_filter(" capability.vac.missing ")
                .with_selected_workflow(" missing.workflow "),
        ));

        assert!(lines.contains("filter: capability=vac.missing"));
        assert!(lines.contains("<none matched current /workflow filter or selection>"));
    }

    #[test]
    fn workflow_browser_state_filter_skips_non_matching_workflows() {
        let tempdir = tempdir().expect("tempdir");
        seed_core_capabilities(tempdir.path());
        fs::create_dir_all(tempdir.path().join(".vac/workflows")).expect("workflows dir");
        fs::write(
            tempdir
                .path()
                .join(".vac/workflows/maintenance.build-only.yaml"),
            r#"
schema_version: 1
kind: workflow
id: maintenance.build-only
title: Build only
status: ready
inputs: {}
steps:
  - id: validate
    uses: capability.build.cargo_check
ui:
  surface: /workflow
  progress_panel: true
  activity_log: true
  approval_surface: false
  evidence_surface: true
policy:
  default_risk: safe_read
validation:
  commands:
    - cargo +1.95.0 check -p vac-surface-cli
"#,
        )
        .expect("build-only manifest");
        fs::write(
            tempdir
                .path()
                .join(".vac/workflows/maintenance.identity-only.yaml"),
            r#"
schema_version: 1
kind: workflow
id: maintenance.identity-only
title: Identity only
status: ready
inputs: {}
steps:
  - id: orient
    uses: capability.identity.check
ui:
  surface: /workflow
  progress_panel: true
  activity_log: true
  approval_surface: false
  evidence_surface: true
policy:
  default_risk: safe_read
validation:
  commands:
    - cargo +1.95.0 check -p vac-surface-cli
"#,
        )
        .expect("identity-only manifest");

        let state = WorkflowBrowserState::default()
            .with_capability_filter("build.cargo_check")
            .with_selected_workflow("maintenance.build-only")
            .with_run_requested();
        assert_eq!(
            state.selected_workflow.as_deref(),
            Some("maintenance.build-only")
        );

        let rendered = render_to_text(&render_workflow_browser_with_state(tempdir.path(), &state));
        assert!(
            rendered.contains("filter: capability=build.cargo_check"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("maintenance.build-only"),
            "rendered:\n{rendered}"
        );
        assert!(rendered.contains("selected: true"), "rendered:\n{rendered}");
        assert!(
            rendered.contains("run: executing via TUI workflow runner (not display-only)"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("maintenance.identity-only — skipped by workflow selection"),
            "rendered:\n{rendered}"
        );
    }

    #[test]
    fn workflow_browser_respects_manifest_ui_flags() {
        let tempdir = tempdir().expect("tempdir");
        seed_core_capabilities(tempdir.path());
        fs::create_dir_all(tempdir.path().join(".vac/workflows")).expect("workflows dir");
        fs::write(
            tempdir
                .path()
                .join(".vac/workflows/maintenance.ui-disabled.yaml"),
            r#"
schema_version: 1
kind: workflow
id: maintenance.ui-disabled
title: UI disabled workflow
status: ready
inputs: {}
steps:
  - id: orient
    uses: capability.identity.check
  - id: validate
    uses: capability.build.cargo_check
  - id: report
    uses: capability.activity.emit
ui:
  surface: /workflow
  progress_panel: false
  activity_log: false
  approval_surface: false
  evidence_surface: false
policy:
  default_risk: safe_read
validation:
  commands:
    - cargo +1.95.0 check -p vac-surface-cli
"#,
        )
        .expect("workflow manifest");

        let rendered = render_to_text(&render_workflow_browser_lines(tempdir.path()));
        assert!(
            rendered.contains("progress panel: disabled by manifest.ui.progress_panel"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("activity log: disabled by manifest.ui.activity_log"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("approval surface: disabled by manifest.ui.approval_surface"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("evidence surface: disabled by manifest.ui.evidence_surface"),
            "rendered:\n{rendered}"
        );
        assert!(
            !rendered.contains("identity check: scanned="),
            "evidence should be gated; rendered:\n{rendered}"
        );
        assert!(
            !rendered.contains("progress: status="),
            "progress should be gated; rendered:\n{rendered}"
        );
        assert!(
            !rendered.contains("            started workflow=maintenance.ui-disabled"),
            "activity log should be gated; rendered:\n{rendered}"
        );
    }
}
