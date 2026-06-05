use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use vac_core::control_plane::workflow_manifest::WorkflowManifest;
use vac_core::control_plane::workflow_runner::WorkflowExecutionEvent;
use vac_core::control_plane::workflow_runner::WorkflowExecutionReport;
use vac_core::control_plane::workflow_runner::WorkflowExecutionState;
use vac_core::control_plane::workflow_runner::WorkflowStepLifecycle;
use vac_core::control_plane::workflow_runner::format_workflow_execution_event;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkflowProgressLifecycle {
    Pending,
    Running,
    WaitingApproval,
    ResumedAfterApproval,
    Success,
    Failed,
    Cancelled,
}

impl WorkflowProgressLifecycle {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::WaitingApproval => "waiting_approval",
            Self::ResumedAfterApproval => "resumed_after_approval",
            Self::Success => "success",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub(crate) fn severity_style(self) -> Style {
        match self {
            Self::Pending => Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::DIM),
            Self::Running => Style::default().fg(Color::Cyan),
            Self::WaitingApproval => Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            Self::ResumedAfterApproval => Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
            Self::Success => Style::default().fg(Color::Green),
            Self::Failed => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            Self::Cancelled => Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::ITALIC),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkflowProgressStep {
    pub(crate) index: usize,
    pub(crate) id: String,
    pub(crate) uses: String,
    pub(crate) capability_id: Option<String>,
    pub(crate) lifecycle: WorkflowProgressLifecycle,
    pub(crate) detail: Option<String>,
    pub(crate) hint: Option<String>,
    pub(crate) approval_request_id: Option<String>,
    pub(crate) resumed_after_approval: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkflowProgressModel {
    pub(crate) workflow_id: String,
    pub(crate) title: String,
    pub(crate) terminal: WorkflowProgressLifecycle,
    pub(crate) started_step_count: usize,
    pub(crate) completed_step_count: usize,
    pub(crate) waiting_approval_step_count: usize,
    pub(crate) blocked_step_count: usize,
    pub(crate) reason: Option<String>,
    pub(crate) activity_events: Vec<String>,
    pub(crate) steps: Vec<WorkflowProgressStep>,
}

impl WorkflowProgressModel {
    pub(crate) fn from_manifest_and_execution(
        manifest: &WorkflowManifest,
        execution: &WorkflowExecutionReport,
    ) -> Self {
        let mut steps = manifest
            .steps
            .iter()
            .enumerate()
            .map(|(index, step)| WorkflowProgressStep {
                index: index + 1,
                id: step.id.clone(),
                uses: step.uses.clone(),
                capability_id: capability_id_from_uses(&step.uses),
                lifecycle: WorkflowProgressLifecycle::Pending,
                detail: None,
                hint: None,
                approval_request_id: None,
                resumed_after_approval: false,
            })
            .collect::<Vec<_>>();
        let mut terminal = WorkflowProgressLifecycle::Pending;
        let mut reason = None;
        let activity_events = execution
            .events()
            .iter()
            .map(format_workflow_execution_event)
            .collect::<Vec<_>>();

        for event in execution.events() {
            match event {
                WorkflowExecutionEvent::Cancelled {
                    reason: event_reason,
                } => {
                    terminal = WorkflowProgressLifecycle::Cancelled;
                    reason = Some(event_reason.clone());
                }
                WorkflowExecutionEvent::StepFailed {
                    index,
                    reason: event_reason,
                    ..
                } => {
                    if let Some(step) = steps.get_mut(index.saturating_sub(1)) {
                        step.detail = Some(event_reason.clone());
                        step.hint = Some(
                            "review the step use and retry with a safe capability".to_string(),
                        );
                    }
                }
                WorkflowExecutionEvent::StepWaitingApproval {
                    index,
                    approval_request_id,
                    ..
                } => {
                    if let Some(step) = steps.get_mut(index.saturating_sub(1)) {
                        step.approval_request_id = Some(approval_request_id.to_string());
                        step.hint = Some("open the approval UI and respond".to_string());
                    }
                }
                _ => {}
            }
        }

        for state in execution.state_trace() {
            match state {
                WorkflowExecutionState::Step {
                    index,
                    resolution,
                    lifecycle,
                    ..
                } => {
                    if let Some(step) = steps.get_mut(index.saturating_sub(1)) {
                        if matches!(lifecycle, WorkflowStepLifecycle::ResumedAfterApproval) {
                            step.resumed_after_approval = true;
                        }
                        step.lifecycle = match lifecycle {
                            WorkflowStepLifecycle::Running => WorkflowProgressLifecycle::Running,
                            WorkflowStepLifecycle::WaitingApproval => {
                                WorkflowProgressLifecycle::WaitingApproval
                            }
                            WorkflowStepLifecycle::ResumedAfterApproval => {
                                WorkflowProgressLifecycle::ResumedAfterApproval
                            }
                            WorkflowStepLifecycle::Succeeded => WorkflowProgressLifecycle::Success,
                            WorkflowStepLifecycle::Failed => WorkflowProgressLifecycle::Failed,
                        };
                        if matches!(lifecycle, WorkflowStepLifecycle::Succeeded) {
                            step.detail = Some("completed".to_string());
                        } else if matches!(lifecycle, WorkflowStepLifecycle::Failed) {
                            step.detail.get_or_insert_with(|| {
                                resolution
                                    .blocked_reason
                                    .clone()
                                    .unwrap_or_else(|| "workflow step failed".to_string())
                            });
                            step.hint.get_or_insert_with(|| {
                                "review the step use and retry with a safe capability".to_string()
                            });
                        }
                    }
                }
                WorkflowExecutionState::Finished { .. } => {
                    terminal = WorkflowProgressLifecycle::Success;
                }
                WorkflowExecutionState::Cancelled {
                    reason: event_reason,
                    ..
                } => {
                    terminal = WorkflowProgressLifecycle::Cancelled;
                    reason = Some(event_reason.clone());
                }
                WorkflowExecutionState::Failed {
                    reason: event_reason,
                    ..
                } => {
                    terminal = WorkflowProgressLifecycle::Failed;
                    reason = Some(event_reason.clone());
                }
                WorkflowExecutionState::Ready { .. } => {}
            }
        }

        if matches!(terminal, WorkflowProgressLifecycle::Pending) {
            terminal = if execution.waiting_approval_step_count() > 0 {
                WorkflowProgressLifecycle::WaitingApproval
            } else if execution.blocked_step_count() > 0 {
                WorkflowProgressLifecycle::Failed
            } else if execution.completed_step_count() == steps.len() && !steps.is_empty() {
                WorkflowProgressLifecycle::Success
            } else if execution.started_step_count() > 0 {
                WorkflowProgressLifecycle::Running
            } else {
                WorkflowProgressLifecycle::Pending
            };
        }

        Self {
            workflow_id: manifest.id.clone(),
            title: manifest.title.clone(),
            terminal,
            started_step_count: execution.started_step_count(),
            completed_step_count: execution.completed_step_count(),
            waiting_approval_step_count: execution.waiting_approval_step_count(),
            blocked_step_count: execution.blocked_step_count(),
            reason,
            activity_events,
            steps,
        }
    }

    pub(crate) fn summary_line(&self) -> String {
        format!(
            "status={} started={} completed={} waiting_approval={} blocked={}",
            self.terminal.as_str(),
            self.started_step_count,
            self.completed_step_count,
            self.waiting_approval_step_count,
            self.blocked_step_count
        )
    }

    #[allow(dead_code)]
    pub(crate) fn render_lines(&self) -> Vec<Line<'static>> {
        self.render_lines_with(true)
    }

    pub(crate) fn render_lines_with(&self, include_activity_log: bool) -> Vec<Line<'static>> {
        let mut lines: Vec<Line<'static>> = Vec::new();
        lines.push(Line::from(vec![
            Span::raw("progress: "),
            Span::styled(self.summary_line(), self.terminal.severity_style()),
        ]));
        lines.push(format!("   workflow: {} — {}", self.workflow_id, self.title).into());
        if let Some(reason) = self.reason.as_deref() {
            lines.push(format!("   reason: {reason}").into());
        }
        if include_activity_log && !self.activity_events.is_empty() {
            lines.push("   activity log:".into());
            for event in &self.activity_events {
                lines.push(format!("     {event}").into());
            }
        }
        if self.steps.is_empty() {
            lines.push("   steps: <none>".into());
            return lines;
        }

        lines.push("   steps:".into());
        for step in &self.steps {
            lines.push(format!("     {}. {} uses {}", step.index, step.id, step.uses).into());
            lines.push(Line::from(vec![
                Span::raw("        lifecycle: "),
                Span::styled(
                    step.lifecycle.as_str().to_string(),
                    step.lifecycle.severity_style(),
                ),
            ]));
            if step.resumed_after_approval {
                lines.push("        resumed_after_approval: true".into());
            }
            if let Some(detail) = step.detail.as_deref() {
                lines.push(format!("        detail: {detail}").into());
            }
            if let Some(approval_request_id) = step.approval_request_id.as_deref() {
                lines.push(format!("        approval_id: {approval_request_id}").into());
            }
            if let Some(hint) = step.hint.as_deref() {
                lines.push(format!("        hint: {hint}").into());
            }
        }

        lines
    }

    #[allow(dead_code)]
    pub(crate) fn filter_by_capability(&self, capability: &str) -> Vec<&WorkflowProgressStep> {
        self.steps
            .iter()
            .filter(|step| step.capability_id.as_deref() == Some(capability))
            .collect()
    }

    pub(crate) fn capability_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self
            .steps
            .iter()
            .filter_map(|step| step.capability_id.clone())
            .collect();
        ids.sort();
        ids.dedup();
        ids
    }

    pub(crate) fn pending_approval_actions(&self) -> Vec<(String, String)> {
        self.steps
            .iter()
            .filter_map(|step| {
                step.approval_request_id
                    .as_ref()
                    .map(|id| (step.id.clone(), id.clone()))
            })
            .collect()
    }
}

pub(crate) fn capability_id_from_uses(uses: &str) -> Option<String> {
    uses.strip_prefix("capability.")
        .map(|tail| tail.to_string())
        .filter(|tail| !tail.is_empty())
}

#[cfg(test)]
mod tests {
    use super::WorkflowProgressLifecycle;
    use super::WorkflowProgressModel;
    use std::collections::BTreeMap;
    use vac_core::control_plane::workflow_manifest::WorkflowInputKind;
    use vac_core::control_plane::workflow_manifest::WorkflowInputSpec;
    use vac_core::control_plane::workflow_manifest::WorkflowManifest;
    use vac_core::control_plane::workflow_manifest::WorkflowManifestKind;
    use vac_core::control_plane::workflow_manifest::WorkflowPolicy;
    use vac_core::control_plane::workflow_manifest::WorkflowStatus;
    use vac_core::control_plane::workflow_manifest::WorkflowStep;
    use vac_core::control_plane::workflow_manifest::WorkflowUi;
    use vac_core::control_plane::workflow_manifest::WorkflowValidation;
    use vac_core::control_plane::workflow_runner::WorkflowExecutionMachine;
    use vac_core::control_plane::workflow_runner::execute_workflow_manifest;

    fn minimal_manifest(steps: Vec<WorkflowStep>) -> WorkflowManifest {
        WorkflowManifest {
            schema_version: 1,
            kind: WorkflowManifestKind::Workflow,
            id: "test.workflow".to_string(),
            title: "Test workflow".to_string(),
            status: WorkflowStatus::Ready,
            inputs: BTreeMap::from([(
                "workspace".to_string(),
                WorkflowInputSpec {
                    kind: WorkflowInputKind::String,
                    required: false,
                    values: Vec::new(),
                    default: None,
                    description: None,
                },
            )]),
            steps,
            ui: WorkflowUi {
                surface: "/workflow".to_string(),
                inspect_surface: None,
                progress_panel: true,
                activity_log: true,
                approval_surface: false,
                evidence_surface: false,
            },
            policy: WorkflowPolicy {
                default_risk: Some("safe_read".to_string()),
                mutates_files: None,
                network: None,
                redaction: None,
                approval_required_for: Vec::new(),
            },
            validation: WorkflowValidation {
                commands: Vec::new(),
                gates: Vec::new(),
            },
        }
    }

    #[test]
    fn workflow_progress_tracks_approval_wait_and_pending_steps() {
        let manifest = minimal_manifest(vec![
            WorkflowStep {
                id: "check".to_string(),
                uses: "capability.build.cargo_check".to_string(),
                when: None,
                policy: None,
                ui: None,
            },
            WorkflowStep {
                id: "approve".to_string(),
                uses: "capability.approval.request".to_string(),
                when: None,
                policy: None,
                ui: None,
            },
            WorkflowStep {
                id: "report".to_string(),
                uses: "capability.activity.emit".to_string(),
                when: None,
                policy: None,
                ui: None,
            },
        ]);

        let execution = execute_workflow_manifest(&manifest);
        let progress = WorkflowProgressModel::from_manifest_and_execution(&manifest, &execution);
        assert_eq!(
            progress.terminal,
            WorkflowProgressLifecycle::WaitingApproval
        );
        assert_eq!(progress.steps.len(), 3);
        assert_eq!(
            progress.steps[0].lifecycle,
            WorkflowProgressLifecycle::Success
        );
        assert_eq!(
            progress.steps[1].lifecycle,
            WorkflowProgressLifecycle::WaitingApproval
        );
        assert_eq!(
            progress.steps[2].lifecycle,
            WorkflowProgressLifecycle::Pending
        );
        assert!(progress.render_lines().iter().any(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
                .contains("progress: status=waiting_approval")
        }));
        assert!(progress.render_lines().iter().any(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
                .contains("activity log:")
        }));
        assert!(progress.render_lines().iter().any(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
                .contains("started workflow=test.workflow title=Test workflow steps=3")
        }));
        assert!(progress.render_lines().iter().any(|line| {
            let rendered = line
                .spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>();
            rendered.contains("approval_id: approval:test.workflow:run:")
                && rendered.contains(":attempt:attempt-1:step:approve")
        }));
    }

    #[test]
    fn workflow_progress_tracks_cancelled_terminal_state() {
        let manifest = minimal_manifest(vec![WorkflowStep {
            id: "check".to_string(),
            uses: "capability.build.cargo_check".to_string(),
            when: None,
            policy: None,
            ui: None,
        }]);
        let mut machine = WorkflowExecutionMachine::new(&manifest);
        assert!(machine.cancel("operator cancel"));
        let execution = machine.run();
        let progress = WorkflowProgressModel::from_manifest_and_execution(&manifest, &execution);

        assert_eq!(progress.terminal, WorkflowProgressLifecycle::Cancelled);
        assert_eq!(progress.reason.as_deref(), Some("operator cancel"));
        assert_eq!(progress.steps.len(), 1);
        assert_eq!(
            progress.steps[0].lifecycle,
            WorkflowProgressLifecycle::Pending
        );
        let rendered = progress
            .render_lines()
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            rendered.contains("progress: status=cancelled"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("reason: operator cancel"),
            "rendered:\n{rendered}"
        );
        assert!(rendered.contains("activity log:"), "rendered:\n{rendered}");
        assert!(
            rendered.contains("cancelled reason=operator cancel"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("lifecycle: pending"),
            "rendered:\n{rendered}"
        );
    }

    #[test]
    fn workflow_progress_tracks_success_terminal_state() {
        let manifest = minimal_manifest(vec![
            WorkflowStep {
                id: "check".to_string(),
                uses: "capability.build.cargo_check".to_string(),
                when: None,
                policy: None,
                ui: None,
            },
            WorkflowStep {
                id: "report".to_string(),
                uses: "capability.activity.emit".to_string(),
                when: None,
                policy: None,
                ui: None,
            },
        ]);

        let execution = execute_workflow_manifest(&manifest);
        let progress = WorkflowProgressModel::from_manifest_and_execution(&manifest, &execution);

        assert_eq!(progress.terminal, WorkflowProgressLifecycle::Success);
        assert_eq!(progress.steps.len(), 2);
        assert_eq!(
            progress.steps[0].lifecycle,
            WorkflowProgressLifecycle::Success
        );
        assert_eq!(
            progress.steps[1].lifecycle,
            WorkflowProgressLifecycle::Success
        );
        assert_eq!(progress.started_step_count, 2);
        assert_eq!(progress.completed_step_count, 2);
        assert_eq!(progress.waiting_approval_step_count, 0);
        assert_eq!(progress.blocked_step_count, 0);
        assert!(progress.reason.is_none());

        let rendered = progress
            .render_lines()
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            rendered.contains("progress: status=success"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("started workflow=test.workflow title=Test workflow steps=2"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("step 1. check -> succeeded"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("step 2. report -> succeeded"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("finished started=2 completed=2 waiting_approval=0 blocked=0"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("lifecycle: success"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("detail: completed"),
            "rendered:\n{rendered}"
        );
    }

    #[test]
    fn workflow_progress_tracks_failure_state() {
        let manifest = minimal_manifest(vec![WorkflowStep {
            id: "orient".to_string(),
            uses: "capability.workflow".to_string(),
            when: None,
            policy: None,
            ui: None,
        }]);

        let execution = execute_workflow_manifest(&manifest);
        let progress = WorkflowProgressModel::from_manifest_and_execution(&manifest, &execution);
        assert_eq!(progress.terminal, WorkflowProgressLifecycle::Failed);
        assert_eq!(
            progress.steps[0].lifecycle,
            WorkflowProgressLifecycle::Failed
        );
        let rendered = progress
            .render_lines()
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            rendered.contains("progress: status=failed"),
            "rendered:\n{rendered}"
        );
        assert!(rendered.contains("activity log:"), "rendered:\n{rendered}");
        assert!(
            rendered.contains("detail: not yet mapped"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("hint: review the step use and retry with a safe capability"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("step 1. orient -> running"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("step 1. orient -> failed: not yet mapped"),
            "rendered:\n{rendered}"
        );
    }

    #[test]
    fn workflow_progress_tracks_resumed_after_approval_state() {
        use chrono::Utc;
        use vac_core::control_plane::approval_lifecycle::ApprovalDecision;
        use vac_core::control_plane::approval_lifecycle::ApprovalStatus;
        use vac_core::control_plane::workflow_runner::WorkflowExecutionState;
        use vac_core::control_plane::workflow_runner::WorkflowStepLifecycle;

        let manifest = minimal_manifest(vec![WorkflowStep {
            id: "approve".to_string(),
            uses: "capability.approval.request".to_string(),
            when: None,
            policy: None,
            ui: None,
        }]);

        let mut machine = WorkflowExecutionMachine::new(&manifest);
        while machine.advance() {}
        let approval_id = machine
            .approval_requests()
            .iter()
            .find(|req| matches!(req.status, ApprovalStatus::Pending))
            .expect("pending approval emitted")
            .id
            .clone();
        machine
            .resolve_approval(
                &approval_id,
                ApprovalDecision::approved("operator", "ok", Utc::now()),
            )
            .expect("approval resolution succeeds");
        let execution = machine.run();
        let progress = WorkflowProgressModel::from_manifest_and_execution(&manifest, &execution);

        let resumed_seen = execution.state_trace().iter().any(|state| {
            matches!(
                state,
                WorkflowExecutionState::Step {
                    lifecycle: WorkflowStepLifecycle::ResumedAfterApproval,
                    ..
                }
            )
        });
        assert!(resumed_seen, "expected ResumedAfterApproval state in trace");
        assert!(
            progress.steps[0].resumed_after_approval,
            "expected sticky resumed_after_approval flag"
        );
        let rendered = progress
            .render_lines()
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            rendered.contains("resumed_after_approval: true"),
            "expected resumed marker; rendered:\n{rendered}"
        );
    }

    #[test]
    fn workflow_progress_filter_and_capability_helpers_round_trip() {
        let manifest = minimal_manifest(vec![
            WorkflowStep {
                id: "check".to_string(),
                uses: "capability.build.cargo_check".to_string(),
                when: None,
                policy: None,
                ui: None,
            },
            WorkflowStep {
                id: "approve".to_string(),
                uses: "capability.approval.request".to_string(),
                when: None,
                policy: None,
                ui: None,
            },
            WorkflowStep {
                id: "report".to_string(),
                uses: "capability.activity.emit".to_string(),
                when: None,
                policy: None,
                ui: None,
            },
        ]);

        let execution = execute_workflow_manifest(&manifest);
        let progress = WorkflowProgressModel::from_manifest_and_execution(&manifest, &execution);

        let capabilities = progress.capability_ids();
        assert_eq!(
            capabilities,
            vec![
                "activity.emit".to_string(),
                "approval.request".to_string(),
                "build.cargo_check".to_string(),
            ]
        );

        let build_steps = progress.filter_by_capability("build.cargo_check");
        assert_eq!(build_steps.len(), 1);
        assert_eq!(build_steps[0].id, "check");
        assert_eq!(
            build_steps[0].capability_id.as_deref(),
            Some("build.cargo_check")
        );

        let pending = progress.pending_approval_actions();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].0, "approve");
        assert!(pending[0].1.starts_with("approval:test.workflow:run:"));
        assert!(pending[0].1.ends_with(":attempt:attempt-1:step:approve"));
    }
}
