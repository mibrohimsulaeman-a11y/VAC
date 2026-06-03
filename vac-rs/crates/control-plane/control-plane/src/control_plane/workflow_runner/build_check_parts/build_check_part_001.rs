impl WorkflowDryRunMachine {
    pub fn new(manifest: &WorkflowManifest) -> Self {
        let step_resolutions = manifest
            .steps
            .iter()
            .map(resolve_workflow_step)
            .collect::<Vec<_>>();
        Self {
            workflow_id: manifest.id.clone(),
            title: manifest.title.clone(),
            step_count: manifest.steps.len(),
            step_resolutions,
            cursor: 0,
            supported_step_count: 0,
            blocked_step_count: 0,
            started: false,
            finished: false,
            state_trace: vec![WorkflowDryRunState::Ready {
                workflow_id: manifest.id.clone(),
                title: manifest.title.clone(),
                step_count: manifest.steps.len(),
            }],
            events: Vec::new(),
        }
    }

    pub fn state_trace(&self) -> &[WorkflowDryRunState] {
        &self.state_trace
    }

    pub fn events(&self) -> &[WorkflowDryRunEvent] {
        &self.events
    }

    pub fn state(&self) -> WorkflowDryRunState {
        if !self.started {
            WorkflowDryRunState::Ready {
                workflow_id: self.workflow_id.clone(),
                title: self.title.clone(),
                step_count: self.step_count,
            }
        } else if self.finished {
            WorkflowDryRunState::Finished {
                workflow_id: self.workflow_id.clone(),
                title: self.title.clone(),
                supported_step_count: self.supported_step_count,
                blocked_step_count: self.blocked_step_count,
            }
        } else if self.cursor == 0 {
            WorkflowDryRunState::Started {
                workflow_id: self.workflow_id.clone(),
                title: self.title.clone(),
                step_count: self.step_count,
            }
        } else if let Some(resolution) = self.step_resolutions.get(self.cursor.saturating_sub(1)) {
            WorkflowDryRunState::Step {
                index: self.cursor,
                step_count: self.step_count,
                resolution: resolution.clone(),
                supported_step_count: self.supported_step_count,
                blocked_step_count: self.blocked_step_count,
            }
        } else {
            WorkflowDryRunState::Started {
                workflow_id: self.workflow_id.clone(),
                title: self.title.clone(),
                step_count: self.step_count,
            }
        }
    }

    pub fn advance(&mut self) -> bool {
        if !self.started {
            self.started = true;
            self.state_trace.push(WorkflowDryRunState::Started {
                workflow_id: self.workflow_id.clone(),
                title: self.title.clone(),
                step_count: self.step_count,
            });
            self.events.push(WorkflowDryRunEvent::Started {
                workflow_id: self.workflow_id.clone(),
                title: self.title.clone(),
                step_count: self.step_count,
            });
            return true;
        }

        if let Some(resolution) = self.step_resolutions.get(self.cursor).cloned() {
            self.cursor += 1;
            if resolution.supported() {
                self.supported_step_count += 1;
                self.state_trace.push(WorkflowDryRunState::Step {
                    index: self.cursor,
                    step_count: self.step_count,
                    resolution: resolution.clone(),
                    supported_step_count: self.supported_step_count,
                    blocked_step_count: self.blocked_step_count,
                });
                self.events.push(WorkflowDryRunEvent::StepResolved {
                    index: self.cursor,
                    resolution,
                });
            } else {
                self.blocked_step_count += 1;
                self.state_trace.push(WorkflowDryRunState::Step {
                    index: self.cursor,
                    step_count: self.step_count,
                    resolution: resolution.clone(),
                    supported_step_count: self.supported_step_count,
                    blocked_step_count: self.blocked_step_count,
                });
                self.events.push(WorkflowDryRunEvent::StepBlocked {
                    index: self.cursor,
                    resolution,
                });
            }
            return true;
        }

        if !self.finished {
            self.finished = true;
            self.state_trace.push(WorkflowDryRunState::Finished {
                workflow_id: self.workflow_id.clone(),
                title: self.title.clone(),
                supported_step_count: self.supported_step_count,
                blocked_step_count: self.blocked_step_count,
            });
            self.events.push(WorkflowDryRunEvent::Finished {
                supported_step_count: self.supported_step_count,
                blocked_step_count: self.blocked_step_count,
            });
            return true;
        }

        false
    }

    pub fn run(mut self) -> WorkflowDryRunReport {
        while self.advance() {}
        WorkflowDryRunReport {
            state_trace: self.state_trace,
            events: self.events,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowDryRunReport {
    state_trace: Vec<WorkflowDryRunState>,
    events: Vec<WorkflowDryRunEvent>,
}

impl WorkflowDryRunReport {
    pub fn state_trace(&self) -> &[WorkflowDryRunState] {
        &self.state_trace
    }

    pub fn events(&self) -> &[WorkflowDryRunEvent] {
        &self.events
    }

    pub fn supported(&self) -> bool {
        self.blocked_step_count() == 0
    }

    pub fn supported_step_count(&self) -> usize {
        self.events
            .iter()
            .filter(|event| matches!(event, WorkflowDryRunEvent::StepResolved { .. }))
            .count()
    }

    pub fn blocked_step_count(&self) -> usize {
        self.events
            .iter()
            .filter(|event| matches!(event, WorkflowDryRunEvent::StepBlocked { .. }))
            .count()
    }

    pub fn render_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        if !self.state_trace.is_empty() {
            lines.push("dry-run state:".to_string());
            for state in &self.state_trace {
                lines.push(format!("  {}", format_workflow_dry_run_state(state)));
            }
        }
        if !self.events.is_empty() {
            lines.push("dry-run events:".to_string());
            for event in &self.events {
                lines.push(format!(
                    "  dry-run: {}",
                    format_workflow_dry_run_event(event)
                ));
            }
        }
        lines
    }

    pub fn render_text(&self) -> String {
        self.render_lines().join("\n")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowRunPreview {
    pub supported: bool,
    pub blocked_steps: Vec<WorkflowBlockedStep>,
}

impl WorkflowRunPreview {
    pub fn blocked_step_count(&self) -> usize {
        self.blocked_steps.len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowApprovalDecision {
    Allowed,
    ApprovalRequired,
    Blocked,
}

impl WorkflowApprovalDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Allowed => "allowed",
            Self::ApprovalRequired => "approval_required",
            Self::Blocked => "blocked",
        }
    }
}

impl fmt::Display for WorkflowApprovalDecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowApprovalCheck {
    pub index: usize,
    pub id: String,
    pub uses: String,
    pub decision: WorkflowApprovalDecision,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowApprovalReport {
    capability_registry_manifest_count: usize,
    allowed_step_count: usize,
    approval_required_step_count: usize,
    blocked_step_count: usize,
    checks: Vec<WorkflowApprovalCheck>,
}

impl WorkflowApprovalReport {
    pub fn policy_registry_manifest_count(&self) -> usize {
        self.capability_registry_manifest_count
    }

    pub fn capability_registry_manifest_count(&self) -> usize {
        self.capability_registry_manifest_count
    }

    pub fn allowed_step_count(&self) -> usize {
        self.allowed_step_count
    }

    pub fn approval_required_step_count(&self) -> usize {
        self.approval_required_step_count
    }

    pub fn blocked_step_count(&self) -> usize {
        self.blocked_step_count
    }

    pub fn checks(&self) -> &[WorkflowApprovalCheck] {
        &self.checks
    }

    pub fn summary_line(&self) -> String {
        let registry_state = if self.capability_registry_manifest_count == 0 {
            "empty".to_string()
        } else {
            format!("loaded {}", self.capability_registry_manifest_count)
        };
        format!(
            "capability registry={} allowed={} approval_required={} blocked={}",
            registry_state,
            self.allowed_step_count,
            self.approval_required_step_count,
            self.blocked_step_count
        )
    }

    pub fn render_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(format!("approval policy: {}", self.summary_line()));
        if self.checks.is_empty() {
            lines.push("approval policy checks: <none>".to_string());
            return lines;
        }

        lines.push("approval policy checks:".to_string());
        for check in &self.checks {
            lines.push(format!(
                "  {}. {} uses {} -> {}",
                check.index, check.id, check.uses, check.decision
            ));
            if let Some(reason) = check.reason.as_deref() {
                lines.push(format!("     reason: {reason}"));
            }
        }

        lines
    }
}

fn workflow_policy_value_is_truthy(value: &WorkflowPolicyValue) -> bool {
    match value {
        WorkflowPolicyValue::Bool(value) => *value,
        WorkflowPolicyValue::Text(value) => !matches!(value.as_str(), "false" | "none" | "off"),
    }
}

fn capability_policy_value_is_truthy(value: &CapabilityPolicyValue) -> bool {
    match value {
        CapabilityPolicyValue::Bool(value) => *value,
        CapabilityPolicyValue::Text(value) => !matches!(value.as_str(), "false" | "none" | "off"),
    }
}

fn capability_policy_requires_approval(policy: &CapabilityPolicy) -> Vec<String> {
    let mut reasons = Vec::new();
    if !policy.approval_required_for.is_empty() {
        reasons.push(format!(
            "capability policy requires approval for [{}]",
            policy.approval_required_for.join(", ")
        ));
    }
    if policy
        .mutates_files
        .as_ref()
        .is_some_and(capability_policy_value_is_truthy)
    {
        reasons.push("capability policy marks mutating file access".to_string());
    }
    if policy
        .network
        .as_ref()
        .is_some_and(capability_policy_value_is_truthy)
    {
        reasons.push("capability policy marks network access".to_string());
    }
    if policy
        .redaction
        .as_ref()
        .is_some_and(capability_policy_value_is_truthy)
    {
        reasons.push("capability policy marks redaction".to_string());
    }
    reasons
}

fn workflow_policy_requires_approval(
    policy: &super::workflow_manifest::WorkflowPolicy,
) -> Vec<String> {
    let mut reasons = Vec::new();
    if !policy.approval_required_for.is_empty() {
        reasons.push(format!(
            "workflow policy requires approval for [{}]",
            policy.approval_required_for.join(", ")
        ));
    }
    if policy
        .mutates_files
        .as_ref()
        .is_some_and(workflow_policy_value_is_truthy)
    {
        reasons.push("workflow policy marks mutating file changes".to_string());
    }
    if policy
        .network
        .as_ref()
        .is_some_and(workflow_policy_value_is_truthy)
    {
        reasons.push("workflow policy marks network access".to_string());
    }
    if policy
        .redaction
        .as_ref()
        .is_some_and(workflow_policy_value_is_truthy)
    {
        reasons.push("workflow policy marks redaction".to_string());
    }
    reasons
}

fn workflow_step_policy_requires_approval(
    policy: &super::workflow_manifest::WorkflowStepPolicy,
) -> Vec<String> {
    let mut reasons = Vec::new();
    if !policy.approval_required_for.is_empty() {
        reasons.push(format!(
            "workflow step policy requires approval for [{}]",
            policy.approval_required_for.join(", ")
        ));
    }
    if policy
        .mutates_files
        .as_ref()
        .is_some_and(workflow_policy_value_is_truthy)
    {
        reasons.push("workflow step policy marks mutating file changes".to_string());
    }
    if policy
        .network
        .as_ref()
        .is_some_and(workflow_policy_value_is_truthy)
    {
        reasons.push("workflow step policy marks network access".to_string());
    }
    if policy
        .redaction
        .as_ref()
        .is_some_and(workflow_policy_value_is_truthy)
    {
        reasons.push("workflow step policy marks redaction".to_string());
    }
    reasons
}

pub(crate) fn resolve_workflow_approval_capability_id(uses: &str) -> Option<&'static str> {
    WORKFLOW_STEP_VOCABULARY
        .iter()
        .find(|entry| entry.uses == uses)
        .map(|entry| entry.canonical_capability_id)
}

pub fn evaluate_workflow_approval_policy(
    manifest: &super::workflow_manifest::WorkflowManifest,
    registry: &ControlPlaneRegistry,
) -> WorkflowApprovalReport {
    let mut allowed_step_count = 0;
    let mut approval_required_step_count = 0;
    let mut blocked_step_count = 0;
    let mut checks = Vec::with_capacity(manifest.steps.len());

    for (index, step) in manifest.steps.iter().enumerate() {
        let capability = registry.capabilities.manifests.iter().find(|entry| {
            entry.manifest.id == step.uses
                || resolve_workflow_approval_capability_id(&step.uses)
                    .is_some_and(|id| entry.manifest.id == id)
        });

        let (decision, reason) = match capability {
            None => (
                WorkflowApprovalDecision::Blocked,
                Some("missing capability manifest".to_string()),
            ),
            Some(entry) if matches!(entry.manifest.status, CapabilityStatus::Disabled) => (
                WorkflowApprovalDecision::Blocked,
                Some("capability is disabled".to_string()),
            ),
            Some(entry) if matches!(entry.manifest.status, CapabilityStatus::Deprecated) => {
                let mut reasons = workflow_policy_requires_approval(&manifest.policy);
                if let Some(step_policy) = &step.policy {
                    reasons.extend(workflow_step_policy_requires_approval(step_policy));
                }
                reasons.push("capability is deprecated".to_string());
                reasons.extend(capability_policy_requires_approval(&entry.manifest.policy));
                (
                    WorkflowApprovalDecision::ApprovalRequired,
                    Some(reasons.join("; ")),
                )
            }
            Some(entry) => {
                let mut reasons = workflow_policy_requires_approval(&manifest.policy);
                if let Some(step_policy) = &step.policy {
                    reasons.extend(workflow_step_policy_requires_approval(step_policy));
                }
                reasons.extend(capability_policy_requires_approval(&entry.manifest.policy));
                if reasons.is_empty() {
                    (WorkflowApprovalDecision::Allowed, None)
                } else {
                    (
                        WorkflowApprovalDecision::ApprovalRequired,
                        Some(reasons.join("; ")),
                    )
                }
            }
        };

        match decision {
            WorkflowApprovalDecision::Allowed => allowed_step_count += 1,
            WorkflowApprovalDecision::ApprovalRequired => approval_required_step_count += 1,
            WorkflowApprovalDecision::Blocked => blocked_step_count += 1,
        }

        checks.push(WorkflowApprovalCheck {
            index: index + 1,
            id: step.id.clone(),
            uses: step.uses.clone(),
            decision,
            reason,
        });
    }

    WorkflowApprovalReport {
        capability_registry_manifest_count: registry.capabilities.manifests.len(),
        allowed_step_count,
        approval_required_step_count,
        blocked_step_count,
        checks,
    }
}

pub fn preview_workflow_manifest(manifest: &WorkflowManifest) -> WorkflowRunPreview {
    let step_resolutions = manifest
        .steps
        .iter()
        .map(resolve_workflow_step)
        .collect::<Vec<_>>();
    let blocked_steps = step_resolutions
        .iter()
        .filter_map(|resolution| {
            resolution
                .blocked_reason
                .as_ref()
                .map(|reason| WorkflowBlockedStep {
                    id: resolution.id.clone(),
                    uses: resolution.uses.clone(),
                    reason: reason.clone(),
                })
        })
        .collect::<Vec<_>>();
    WorkflowRunPreview {
        supported: blocked_steps.is_empty(),
        blocked_steps,
    }
}

pub fn dry_run_workflow_manifest(manifest: &WorkflowManifest) -> WorkflowDryRunReport {
    WorkflowDryRunMachine::new(manifest).run()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowExecutionEvent {
    Started {
        workflow_id: String,
        title: String,
        step_count: usize,
    },
    StepStarted {
        index: usize,
        resolution: WorkflowStepResolution,
    },
    StepWaitingApproval {
        index: usize,
        resolution: WorkflowStepResolution,
        approval_request_id: ApprovalRequestId,
    },
    ApprovalResolved {
        approval_request_id: ApprovalRequestId,
        status: ApprovalStatus,
        reason: String,
    },
    StepSucceeded {
        index: usize,
        resolution: WorkflowStepResolution,
    },
    StepFailed {
        index: usize,
        resolution: WorkflowStepResolution,
        reason: String,
    },
    Cancelled {
        reason: String,
    },
    Finished {
        started_step_count: usize,
        completed_step_count: usize,
        waiting_approval_step_count: usize,
        blocked_step_count: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowExecutionState {
    Ready {
        workflow_id: String,
        title: String,
        step_count: usize,
    },
    Step {
        index: usize,
        step_count: usize,
        resolution: WorkflowStepResolution,
        lifecycle: WorkflowStepLifecycle,
        started_step_count: usize,
        completed_step_count: usize,
        waiting_approval_step_count: usize,
        blocked_step_count: usize,
    },
    Finished {
        workflow_id: String,
        title: String,
        started_step_count: usize,
        completed_step_count: usize,
        waiting_approval_step_count: usize,
        blocked_step_count: usize,
    },
    Cancelled {
        workflow_id: String,
        title: String,
        reason: String,
        started_step_count: usize,
        completed_step_count: usize,
        waiting_approval_step_count: usize,
        blocked_step_count: usize,
    },
    Failed {
        workflow_id: String,
        title: String,
        reason: String,
        started_step_count: usize,
        completed_step_count: usize,
        waiting_approval_step_count: usize,
        blocked_step_count: usize,
    },
}

