#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowExecutionReport {
    state_trace: Vec<WorkflowExecutionState>,
    events: Vec<WorkflowExecutionEvent>,
    approval_requests: Vec<ApprovalRequest>,
}

impl WorkflowExecutionReport {
    pub fn state_trace(&self) -> &[WorkflowExecutionState] {
        &self.state_trace
    }

    pub fn events(&self) -> &[WorkflowExecutionEvent] {
        &self.events
    }

    pub fn approval_requests(&self) -> &[ApprovalRequest] {
        &self.approval_requests
    }

    pub fn started_step_count(&self) -> usize {
        self.events
            .iter()
            .filter(|event| matches!(event, WorkflowExecutionEvent::StepStarted { .. }))
            .count()
    }

    pub fn completed_step_count(&self) -> usize {
        self.events
            .iter()
            .filter(|event| matches!(event, WorkflowExecutionEvent::StepSucceeded { .. }))
            .count()
    }

    pub fn waiting_approval_step_count(&self) -> usize {
        self.events
            .iter()
            .filter(|event| matches!(event, WorkflowExecutionEvent::StepWaitingApproval { .. }))
            .count()
    }

    pub fn blocked_step_count(&self) -> usize {
        self.events
            .iter()
            .filter(|event| matches!(event, WorkflowExecutionEvent::StepFailed { .. }))
            .count()
    }

    pub fn is_waiting_approval(&self) -> bool {
        self.approval_requests
            .iter()
            .any(|request| matches!(request.status, ApprovalStatus::Pending))
            || matches!(
                self.state_trace.last(),
                Some(WorkflowExecutionState::Step {
                    lifecycle: WorkflowStepLifecycle::WaitingApproval,
                    ..
                })
            )
    }

    pub fn is_failure(&self) -> bool {
        // Waiting for approval is a safe paused state, not a failed workflow.
        // Failure remains reserved for explicit blocks, failed steps, or cancellation.
        self.blocked_step_count() > 0
            || self.state_trace.iter().any(|state| {
                matches!(
                    state,
                    WorkflowExecutionState::Failed { .. }
                        | WorkflowExecutionState::Cancelled { .. }
                )
            })
    }

    pub fn render_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        if !self.state_trace.is_empty() {
            lines.push("execution state:".to_string());
            for state in &self.state_trace {
                lines.push(format!("  {}", format_workflow_execution_state(state)));
            }
        }
        if !self.events.is_empty() {
            lines.push("execution events:".to_string());
            for event in &self.events {
                lines.push(format!("  {}", format_workflow_execution_event(event)));
            }
        }
        if !self.approval_requests.is_empty() {
            lines.push("approval requests:".to_string());
            for request in &self.approval_requests {
                lines.push(format!(
                    "  approval: id={} workflow={} step={} capability={} action={} risk={} status={}",
                    request.id,
                    request.workflow_id,
                    request.step_id,
                    request.capability_id,
                    request.action,
                    request.risk,
                    request.status,
                ));
                if !request.reasons.is_empty() {
                    lines.push(format!("     reasons: {}", request.reasons.join("; ")));
                }
            }
        }
        lines
    }

    pub fn render_text(&self) -> String {
        self.render_lines().join("\n")
    }
}

fn build_check_skip_enabled() -> bool {
    std::env::var(SKIP_BUILD_CHECK_ENV)
        .map(|value| {
            let normalized = value.trim().to_ascii_lowercase();
            matches!(normalized.as_str(), "1" | "true" | "yes" | "on")
        })
        .unwrap_or(false)
}

pub fn execute_workflow_manifest(manifest: &WorkflowManifest) -> WorkflowExecutionReport {
    WorkflowExecutionMachine::new(manifest).run()
}

pub fn execute_workflow_manifest_with_root_seed_registry_report(
    manifest: &WorkflowManifest,
    root_seed_registry_report: &RegistryLoadReport,
) -> WorkflowExecutionReport {
    WorkflowExecutionMachine::new(manifest)
        .with_root_seed_registry_report(root_seed_registry_report.clone())
        .run()
}

pub fn execute_workflow_manifest_with_approval_policy(
    manifest: &WorkflowManifest,
    approval_policy: &WorkflowApprovalReport,
) -> WorkflowExecutionReport {
    execute_workflow_manifest_with_approval_policy_and_identity_check(
        manifest,
        approval_policy,
        None,
    )
}

pub fn is_supported_step_use(uses: &str) -> bool {
    INITIAL_ALLOWED_STEP_USES.contains(&uses)
}

pub fn resolve_workflow_step_handler(uses: &str) -> Option<WorkflowStepHandler> {
    WORKFLOW_STEP_VOCABULARY
        .iter()
        .find(|entry| entry.uses == uses)
        .map(|entry| entry.handler)
}

pub fn resolve_workflow_step(
    step: &super::workflow_manifest::WorkflowStep,
) -> WorkflowStepResolution {
    let handler = resolve_workflow_step_handler(&step.uses);
    let blocked_reason = handler
        .is_none()
        .then(|| unsupported_step_reason(&step.uses).to_string());
    WorkflowStepResolution {
        id: step.id.clone(),
        uses: step.uses.clone(),
        handler,
        blocked_reason,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowRunEntry {
    pub path: String,
    pub id: String,
    pub title: String,
    pub preview: WorkflowRunPreview,
    pub approval_policy: WorkflowApprovalReport,
    pub policy_decisions: Vec<Option<super::policy_manifest::PolicyDecisionReport>>,
    pub identity_check: Option<IdentityCheckReport>,
    pub no_duplicate_tui: Option<NoDuplicateTuiReport>,
    pub architecture_invariants: Option<ArchitectureInvariantReport>,
    pub ownership_scan: Option<OwnershipScanReport>,
    pub step_resolutions: Vec<WorkflowStepResolution>,
    pub dry_run: WorkflowDryRunReport,
    pub execution: WorkflowExecutionReport,
}

impl WorkflowRunEntry {
    pub fn is_failure(&self) -> bool {
        !self.preview.supported
            || self.approval_policy.blocked_step_count() > 0
            || self.execution.is_failure()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowRunReport {
    registry: RegistryLoadReport,
    workflows: Vec<WorkflowRunEntry>,
}

impl WorkflowRunReport {
    pub fn registry(&self) -> &RegistryLoadReport {
        &self.registry
    }

    pub fn workflows(&self) -> &[WorkflowRunEntry] {
        &self.workflows
    }

    pub fn supported_workflow_count(&self) -> usize {
        self.workflows
            .iter()
            .filter(|entry| entry.preview.supported)
            .count()
    }

    pub fn blocked_workflow_count(&self) -> usize {
        self.workflows
            .iter()
            .filter(|entry| !entry.preview.supported)
            .count()
    }

    pub fn is_failure(&self) -> bool {
        self.registry.is_failure() || self.workflows.iter().any(WorkflowRunEntry::is_failure)
    }

    pub fn render_lines(&self) -> Vec<String> {
        let mut lines = self.registry.render_lines();
        lines.push(format!(
            "workflow runner: supported={} blocked={}",
            self.supported_workflow_count(),
            self.blocked_workflow_count()
        ));
        if self.workflows.is_empty() {
            lines.push("workflows: <none>".to_string());
            return lines;
        }

        lines.push("workflows:".to_string());
        for (index, entry) in self.workflows.iter().enumerate() {
            lines.push(format!("  {}. {} — {}", index + 1, entry.id, entry.title));
            lines.push(format!("     path: {}", entry.path));
            lines.push(format!(
                "     runner: {}",
                format_workflow_run_preview(&entry.preview)
            ));
            for line in entry.approval_policy.render_lines() {
                lines.push(format!("     {line}"));
            }
            if let Some(identity_check) = entry.identity_check.as_ref() {
                lines.push("     identity check:".to_string());
                for line in identity_check.render_lines() {
                    lines.push(format!("       {line}"));
                }
            }
            if let Some(no_duplicate_tui) = entry.no_duplicate_tui.as_ref() {
                lines.push("     tui uniqueness:".to_string());
                for line in no_duplicate_tui.render_lines() {
                    lines.push(format!("       {line}"));
                }
            }
            if let Some(architecture_invariants) = entry.architecture_invariants.as_ref() {
                lines.push("     architecture invariants:".to_string());
                for line in architecture_invariants.render_lines() {
                    lines.push(format!("       {line}"));
                }
            }
            if let Some(ownership_scan) = entry.ownership_scan.as_ref() {
                lines.push("     ownership scan:".to_string());
                for line in ownership_scan.render_lines() {
                    lines.push(format!("       {line}"));
                }
            }
            if !entry.step_resolutions.is_empty() {
                lines.push("     step resolution:".to_string());
                for (step_index, resolution) in entry.step_resolutions.iter().enumerate() {
                    lines.push(format!(
                        "       {}. {} uses {}",
                        step_index + 1,
                        resolution.id,
                        resolution.uses
                    ));
                    lines.push(format!(
                        "          runner: {}",
                        format_workflow_step_resolution(resolution)
                    ));
                }
            }
            lines.push(format!(
                "     dry-run: supported={} blocked={}",
                entry.dry_run.supported_step_count(),
                entry.dry_run.blocked_step_count()
            ));
            for line in entry.dry_run.render_lines() {
                lines.push(format!("       {line}"));
            }
            lines.push(format!(
                "     execution: started={} completed={} waiting_approval={} blocked={}",
                entry.execution.started_step_count(),
                entry.execution.completed_step_count(),
                entry.execution.waiting_approval_step_count(),
                entry.execution.blocked_step_count()
            ));
            for line in entry.execution.render_lines() {
                lines.push(format!("       {line}"));
            }
        }
        lines
    }

    pub fn render_text(&self) -> String {
        self.render_lines().join("\n")
    }
}

pub fn load_workflow_run_report(start: impl AsRef<Path>) -> WorkflowRunReport {
    let registry = super::registry::load_control_plane_registry_report(start);
    let workflows = registry
        .registry()
        .map(|registry| load_workflow_run_entries(registry))
        .unwrap_or_default();
    WorkflowRunReport {
        registry,
        workflows,
    }
}

fn unsupported_step_reason(uses: &str) -> &'static str {
    if uses.starts_with("capability.") {
        "not yet mapped to an initial safe workflow handler"
    } else {
        "unsupported step use"
    }
}

fn workflow_uses_handler(
    manifest: &super::workflow_manifest::WorkflowManifest,
    handler: WorkflowStepHandler,
) -> bool {
    manifest
        .steps
        .iter()
        .any(|step| resolve_workflow_step(step).handler == Some(handler))
}

fn load_workflow_run_entries(registry: &ControlPlaneRegistry) -> Vec<WorkflowRunEntry> {
    let repo_root = registry
        .vac_root
        .parent()
        .unwrap_or(registry.vac_root.as_path());
    let has_root_seed_coverage_workflow = registry.workflows.manifests.iter().any(|entry| {
        workflow_uses_handler(&entry.manifest, WorkflowStepHandler::RootSeedCoverage)
            || workflow_uses_handler(&entry.manifest, WorkflowStepHandler::RegistrySchemaCheck)
            || workflow_uses_handler(&entry.manifest, WorkflowStepHandler::WorkflowBrowserCheck)
    });
    let has_identity_check_workflow =
        registry.workflows.manifests.iter().any(|entry| {
            workflow_uses_handler(&entry.manifest, WorkflowStepHandler::IdentityCheck)
        });
    let has_no_duplicate_tui_workflow =
        registry.workflows.manifests.iter().any(|entry| {
            workflow_uses_handler(&entry.manifest, WorkflowStepHandler::NoDuplicateTui)
        });
    let has_architecture_invariants_workflow = registry.workflows.manifests.iter().any(|entry| {
        workflow_uses_handler(&entry.manifest, WorkflowStepHandler::ArchitectureInvariants)
    });
    let has_ownership_scan_workflow = registry.workflows.manifests.iter().any(|entry| {
        workflow_uses_handler(&entry.manifest, WorkflowStepHandler::OwnershipScan)
            || workflow_uses_handler(
                &entry.manifest,
                WorkflowStepHandler::CapabilityDashboardCheck,
            )
    });
    let identity_check_report =
        has_identity_check_workflow.then(|| load_identity_check_report(repo_root));
    let no_duplicate_tui_report =
        has_no_duplicate_tui_workflow.then(|| load_no_duplicate_tui_report(repo_root));
    let architecture_invariants_report = has_architecture_invariants_workflow
        .then(|| super::architecture_invariants::load_architecture_invariant_report(repo_root));
    let ownership_scan_report =
        has_ownership_scan_workflow.then(|| load_ownership_scan_report(repo_root));
    let root_seed_registry_report = has_root_seed_coverage_workflow
        .then(|| super::registry::load_control_plane_registry_report(repo_root));
    // Do not execute cargo while loading workflow reports/doctor output.
    // Build-check evidence must be injected by an explicit approval-gated runner.
    let build_check_report: Option<build_check::BuildCheckReport> = None;
    let policy_manifests: Vec<super::policy_manifest::PolicyManifest> = registry
        .policies
        .manifests
        .iter()
        .map(|located| located.manifest.clone())
        .collect();

    registry
        .workflows
        .manifests
        .iter()
        .map(|entry| {
            let approval_policy = evaluate_workflow_approval_policy(&entry.manifest, registry);
            let workflow_has_root_seed_coverage =
                workflow_uses_handler(&entry.manifest, WorkflowStepHandler::RootSeedCoverage)
                    || workflow_uses_handler(
                        &entry.manifest,
                        WorkflowStepHandler::RegistrySchemaCheck,
                    )
                    || workflow_uses_handler(
                        &entry.manifest,
                        WorkflowStepHandler::WorkflowBrowserCheck,
                    );
            let workflow_has_identity_check =
                workflow_uses_handler(&entry.manifest, WorkflowStepHandler::IdentityCheck);
            let workflow_has_no_duplicate_tui =
                workflow_uses_handler(&entry.manifest, WorkflowStepHandler::NoDuplicateTui);
            let workflow_has_architecture_invariants =
                workflow_uses_handler(&entry.manifest, WorkflowStepHandler::ArchitectureInvariants);
            let workflow_has_ownership_scan =
                workflow_uses_handler(&entry.manifest, WorkflowStepHandler::OwnershipScan)
                    || workflow_uses_handler(
                        &entry.manifest,
                        WorkflowStepHandler::CapabilityDashboardCheck,
                    );
            let workflow_has_build_check =
                workflow_uses_handler(&entry.manifest, WorkflowStepHandler::BuildCargoCheck);
            let entry_root_seed_registry_report = if workflow_has_root_seed_coverage {
                root_seed_registry_report.clone()
            } else {
                None
            };
            let entry_identity_check = if workflow_has_identity_check {
                identity_check_report.clone()
            } else {
                None
            };
            let entry_no_duplicate_tui = if workflow_has_no_duplicate_tui {
                no_duplicate_tui_report.clone()
            } else {
                None
            };
            let entry_architecture_invariants = if workflow_has_architecture_invariants {
                architecture_invariants_report.clone()
            } else {
                None
            };
            let entry_ownership_scan = if workflow_has_ownership_scan {
                ownership_scan_report.clone()
            } else {
                None
            };
            let entry_build_check = if workflow_has_build_check {
                build_check_report.clone()
            } else {
                None
            };
            let entry_policy_decisions = if policy_manifests.is_empty() {
                vec![None; entry.manifest.steps.len()]
            } else {
                evaluate_workflow_step_policy_decisions(&entry.manifest, &policy_manifests)
            };
            let mut machine = WorkflowExecutionMachine::new(&entry.manifest)
                .with_approval_checks(approval_policy.checks().to_vec())
                .with_policy_decisions(entry_policy_decisions.clone());
            if let Some(report) = entry_root_seed_registry_report.as_ref() {
                machine = machine.with_root_seed_registry_report(report.clone());
            }
            if let Some(report) = entry_identity_check.as_ref() {
                machine = machine.with_identity_check_report(report.clone());
            }
            if let Some(report) = entry_no_duplicate_tui.as_ref() {
                machine = machine.with_no_duplicate_tui_report(report.clone());
            }
            if let Some(report) = entry_architecture_invariants.as_ref() {
                machine = machine.with_architecture_invariants_report(report.clone());
            }
            if let Some(report) = entry_ownership_scan.as_ref() {
                machine = machine.with_ownership_scan_report(report.clone());
            }
            if let Some(report) = entry_build_check.as_ref() {
                machine = machine.with_build_check_report(report.clone());
            }
            let execution = machine.run();
            WorkflowRunEntry {
                approval_policy,
                policy_decisions: entry_policy_decisions,
                identity_check: entry_identity_check,
                no_duplicate_tui: entry_no_duplicate_tui,
                architecture_invariants: entry_architecture_invariants,
                ownership_scan: entry_ownership_scan,
                path: entry.path.display().to_string(),
                id: entry.manifest.id.clone(),
                title: entry.manifest.title.clone(),
                preview: preview_workflow_manifest(&entry.manifest),
                step_resolutions: entry
                    .manifest
                    .steps
                    .iter()
                    .map(resolve_workflow_step)
                    .collect(),
                dry_run: dry_run_workflow_manifest(&entry.manifest),
                execution,
            }
        })
        .collect()
}

pub fn execute_workflow_manifest_with_approval_policy_and_identity_check(
    manifest: &super::workflow_manifest::WorkflowManifest,
    approval_policy: &WorkflowApprovalReport,
    identity_check_report: Option<&IdentityCheckReport>,
) -> WorkflowExecutionReport {
    execute_workflow_manifest_with_approval_policy_and_identity_check_and_no_duplicate_tui(
        manifest,
        approval_policy,
        identity_check_report,
        None,
    )
}

pub fn execute_workflow_manifest_with_approval_policy_and_identity_check_and_no_duplicate_tui(
    manifest: &super::workflow_manifest::WorkflowManifest,
    approval_policy: &WorkflowApprovalReport,
    identity_check_report: Option<&IdentityCheckReport>,
    no_duplicate_tui_report: Option<&NoDuplicateTuiReport>,
) -> WorkflowExecutionReport {
    execute_workflow_manifest_with_approval_policy_and_identity_check_and_no_duplicate_tui_and_architecture_invariants(
        manifest,
        approval_policy,
        identity_check_report,
        no_duplicate_tui_report,
        None,
    )
}

pub fn execute_workflow_manifest_with_approval_policy_and_identity_check_and_no_duplicate_tui_and_architecture_invariants(
    manifest: &super::workflow_manifest::WorkflowManifest,
    approval_policy: &WorkflowApprovalReport,
    identity_check_report: Option<&IdentityCheckReport>,
    no_duplicate_tui_report: Option<&NoDuplicateTuiReport>,
    architecture_invariants_report: Option<&ArchitectureInvariantReport>,
) -> WorkflowExecutionReport {
    let mut machine = WorkflowExecutionMachine::new(manifest)
        .with_approval_checks(approval_policy.checks().to_vec());
    if let Some(report) = identity_check_report {
        machine = machine.with_identity_check_report(report.clone());
    }
    if let Some(report) = no_duplicate_tui_report {
        machine = machine.with_no_duplicate_tui_report(report.clone());
    }
    if let Some(report) = architecture_invariants_report {
        machine = machine.with_architecture_invariants_report(report.clone());
    }
    machine.run()
}

fn approval_action_and_risk_for_step(uses: &str) -> (ApprovalAction, ApprovalRisk) {
    let Some(intent) = resolve_workflow_step_policy_intent(uses) else {
        return (ApprovalAction::Other, ApprovalRisk::Unknown);
    };
    match intent.primary_action {
        PolicyAction::FilesystemWrite => (ApprovalAction::FilesystemWrite, ApprovalRisk::SafeWrite),
        PolicyAction::FilesystemDelete => {
            (ApprovalAction::FilesystemDelete, ApprovalRisk::Destructive)
        }
        PolicyAction::ProcessExecute => (ApprovalAction::ProcessExecute, ApprovalRisk::Execute),
        PolicyAction::NetworkAccess => (ApprovalAction::NetworkAccess, ApprovalRisk::Network),
        PolicyAction::ToolCall => (ApprovalAction::ToolCall, ApprovalRisk::Unknown),
        PolicyAction::SessionWrite | PolicyAction::CheckpointWrite => {
            (ApprovalAction::SessionWrite, ApprovalRisk::SafeWrite)
        }
        PolicyAction::CredentialRead => (ApprovalAction::Other, ApprovalRisk::Unknown),
        PolicyAction::FilesystemRead => (ApprovalAction::Other, ApprovalRisk::SafeRead),
    }
}

pub fn format_workflow_step_resolution(resolution: &WorkflowStepResolution) -> String {
    match resolution.handler {
        Some(handler) => format!("handler={handler}"),
        None => format!(
            "blocked: {}",
            resolution
                .blocked_reason
                .as_deref()
                .unwrap_or("unsupported step use")
        ),
    }
}

pub fn format_workflow_dry_run_event(event: &WorkflowDryRunEvent) -> String {
    match event {
        WorkflowDryRunEvent::Started {
            workflow_id,
            title,
            step_count,
        } => format!("started workflow={workflow_id} title={title} steps={step_count}"),
        WorkflowDryRunEvent::StepResolved { index, resolution } => format!(
            "step {index}. {} -> {}",
            resolution.id,
            format_workflow_step_resolution(resolution)
        ),
        WorkflowDryRunEvent::StepBlocked { index, resolution } => format!(
            "step {index}. {} -> {}",
            resolution.id,
            format_workflow_step_resolution(resolution)
        ),
        WorkflowDryRunEvent::Finished {
            supported_step_count,
            blocked_step_count,
        } => format!("finished supported={supported_step_count} blocked={blocked_step_count}"),
    }
}

