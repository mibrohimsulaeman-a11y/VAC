            policy: None,
            ui: None,
        }]);

        let execution = WorkflowExecutionMachine::new(&manifest).run();
        assert_eq!(execution.started_step_count(), 1);
        assert_eq!(execution.completed_step_count(), 0);
        assert_eq!(execution.waiting_approval_step_count(), 0);
        assert_eq!(execution.blocked_step_count(), 1);
        assert!(matches!(
            execution.state_trace().last(),
            Some(WorkflowExecutionState::Failed { reason, .. }) if reason.contains("not yet mapped")
        ));
        assert_eq!(
            format_workflow_execution_state(&WorkflowExecutionState::Cancelled {
                workflow_id: "test.workflow".to_string(),
                title: "Test workflow".to_string(),
                reason: "operator cancel".to_string(),
                started_step_count: 1,
                completed_step_count: 0,
                waiting_approval_step_count: 0,
                blocked_step_count: 1,
            }),
            "execution: cancelled workflow=test.workflow title=Test workflow reason=operator cancel started=1 completed=0 waiting_approval=0 blocked=1"
        );
        assert!(
            format_workflow_execution_event(&WorkflowExecutionEvent::StepFailed {
                index: 1,
                resolution: super::WorkflowStepResolution {
                    id: "orient".to_string(),
                    uses: "capability.workflow".to_string(),
                    handler: None,
                    blocked_reason: Some(
                        "not yet mapped to an initial safe workflow handler".to_string(),
                    ),
                },
                reason: "not yet mapped to an initial safe workflow handler".to_string(),
            })
            .contains("failed: not yet mapped")
        );
    }

    #[test]
    fn renders_workflow_run_report() {
        use std::fs;
        use tempfile::tempdir;

        let tempdir = tempdir().expect("tempdir");
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
validation:
  commands:
    - cargo +1.95.0 check -p vac-surface-cli
"#,
        )
        .expect("workflow manifest");

        let report = load_workflow_run_report(tempdir.path());
        assert_eq!(report.supported_workflow_count(), 1);
        assert_eq!(report.blocked_workflow_count(), 0);
        let rendered = report.render_text();
        assert!(rendered.contains("workflow runner: supported=1 blocked=0"));
        assert!(rendered.contains("maintenance.build-check"));
        assert!(rendered.contains("runner: supported by initial safe runner"));
        assert!(rendered.contains("step resolution:"));
        assert!(rendered.contains("runner: handler=build.cargo_check"));
        assert!(rendered.contains("execution: started="));
        assert!(rendered.contains("execution state:"));
    }

    #[test]
    fn load_workflow_run_report_waits_for_approval_when_policy_requires_it() {
        use std::fs;
        use tempfile::tempdir;

        let tempdir = tempdir().expect("tempdir");
        let vac_root = tempdir.path().join(".vac");
        seed_core_capabilities(&vac_root);
        fs::create_dir_all(vac_root.join("workflows")).expect("workflows dir");
        fs::write(
            vac_root.join("workflows/maintenance.build-check.yaml"),
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
  mutates_files: false
  network: false
  redaction: false
  approval_required_for:
    - execute_process
validation:
  commands:
    - cargo +1.95.0 check -p vac-surface-cli
"#,
        )
        .expect("workflow manifest");

        let report = load_workflow_run_report(tempdir.path());
        assert_eq!(report.supported_workflow_count(), 1);
        assert_eq!(report.blocked_workflow_count(), 0);
        let workflow = &report.workflows()[0];
        assert_eq!(workflow.execution.started_step_count(), 1);
        assert_eq!(workflow.execution.completed_step_count(), 0);
        assert_eq!(workflow.execution.waiting_approval_step_count(), 1);
        assert_eq!(workflow.execution.blocked_step_count(), 0);
        assert!(
            workflow
                .execution
                .state_trace()
                .iter()
                .any(|state| matches!(
                    state,
                    WorkflowExecutionState::Step {
                        lifecycle: super::WorkflowStepLifecycle::WaitingApproval,
                        ..
                    }
                ))
        );
        let rendered = report.render_text();
        assert!(rendered.contains("workflow runner: supported=1 blocked=0"));
        assert!(rendered.contains("approval policy: capability registry=loaded 3"));
        assert!(rendered.contains("execution events:"));
        assert!(rendered.contains("step 1. orient -> waiting approval"));
    }

    #[test]
    fn load_workflow_run_report_fails_identity_check_when_forbidden_terms_are_present() {
        use std::fs;
        use tempfile::tempdir;

        let tempdir = tempdir().expect("tempdir");
        let vac_root = tempdir.path().join(".vac");
        seed_core_capabilities(&vac_root);
        seed_permissive_policy(&vac_root);
        fs::create_dir_all(tempdir.path().join("vac-rs/src")).expect("product source dir");
        fs::write(
            tempdir.path().join("vac-rs/src/lib.rs"),
            format!(
                "pub const LEGACY: &str = \"{}\";",
                concat!("vac", "_tui", "_runtime")
            ),
        )
        .expect("product source fixture");
        fs::create_dir_all(vac_root.join("workflows")).expect("workflows dir");
        fs::write(
            vac_root.join("workflows/maintenance.identity-check.yaml"),
            r#"
schema_version: 1
kind: workflow
id: maintenance.identity-check
title: Identity check
status: ready
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
  mutates_files: false
  network: false
  redaction: false
validation:
  commands:
    - cargo +1.95.0 check -p vac-surface-cli
"#,
        )
        .expect("workflow manifest");

        let report = load_workflow_run_report(tempdir.path());
        assert_eq!(report.supported_workflow_count(), 1);
        assert_eq!(report.blocked_workflow_count(), 0);
        let workflow = &report.workflows()[0];
        assert_eq!(workflow.execution.started_step_count(), 1);
        assert_eq!(workflow.execution.completed_step_count(), 0);
        assert_eq!(workflow.execution.waiting_approval_step_count(), 0);
        assert_eq!(workflow.execution.blocked_step_count(), 1);
        assert!(
            workflow
                .execution
                .state_trace()
                .iter()
                .any(|state| matches!(state, WorkflowExecutionState::Failed { .. }))
        );
        let rendered = report.render_text();
        assert!(rendered.contains("identity check: scanned="));
        assert!(rendered.contains("findings=1"));
        assert!(rendered.contains("vac-rs/src/lib.rs:1"));
        assert!(rendered.contains("execution: failed"));
        assert!(
            rendered.contains("reason=identity check found 1 forbidden term"),
            "rendered:\n{rendered}"
        );
    }

    #[test]
    fn load_workflow_run_report_fails_no_duplicate_tui_when_forbidden_terms_are_present() {
        use std::fs;
        use tempfile::tempdir;

        let tempdir = tempdir().expect("tempdir");
        let vac_root = tempdir.path().join(".vac");
        seed_tui_uniqueness_capabilities(&vac_root);
        seed_permissive_policy(&vac_root);
        fs::create_dir_all(tempdir.path().join("vac-rs/src")).expect("product source dir");
        fs::write(
            tempdir.path().join("vac-rs/src/lib.rs"),
            format!(
                "pub const LEGACY: &str = \"{}\";",
                concat!("vac", "_tui", "_runtime")
            ),
        )
        .expect("product source fixture");
        fs::create_dir_all(vac_root.join("workflows")).expect("workflows dir");
        fs::write(
            vac_root.join("workflows/maintenance.no-duplicate-tui.yaml"),
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

        let report = load_workflow_run_report(tempdir.path());
        assert_eq!(report.supported_workflow_count(), 1);
        assert_eq!(report.blocked_workflow_count(), 0);
        let workflow = &report.workflows()[0];
        assert_eq!(workflow.execution.started_step_count(), 1);
        assert_eq!(workflow.execution.completed_step_count(), 0);
        assert_eq!(workflow.execution.waiting_approval_step_count(), 0);
        assert_eq!(workflow.execution.blocked_step_count(), 1);
        assert!(
            workflow
                .execution
                .state_trace()
                .iter()
                .any(|state| matches!(state, WorkflowExecutionState::Failed { .. }))
        );
        let rendered = report.render_text();
        assert!(
            rendered.contains("tui uniqueness: scanned="),
            "rendered:\n{rendered}"
        );
        assert!(rendered.contains("findings=1"), "rendered:\n{rendered}");
        assert!(
            rendered.contains("vac-rs/src/lib.rs:1"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("execution: failed"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("reason=TUI Uniqueness Invariant Violation"),
            "rendered:\n{rendered}"
        );
    }

    #[test]
    fn load_workflow_run_report_renders_ownership_scan_matrix() {
        use std::fs;
        use tempfile::tempdir;

        let tempdir = tempdir().expect("tempdir");
        let vac_root = tempdir.path().join(".vac");
        seed_core_capabilities(&vac_root);
        fs::create_dir_all(vac_root.join("capabilities")).expect("capabilities dir");
        fs::write(
            vac_root.join("capabilities/ownership.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.ownership
title: Ownership
status: partial
reason: test fixture partial capability
states:
  empty: empty
  loading: loading
  success: success
  failure: failure
owner:
  crate: vac-core
  module: control_plane
ownership:
  crates:
    - vac-core
  modules:
    - control_plane
surfaces:
  palette: true
policy:
  mutates_files: false
validation:
  commands: []
"#,
        )
        .expect("ownership capability manifest");
        fs::create_dir_all(vac_root.join("workflows")).expect("workflows dir");
        fs::write(
            vac_root.join("workflows/maintenance.ownership-scan.yaml"),
            r#"
schema_version: 1
kind: workflow
id: maintenance.ownership-scan
title: Ownership scan
status: ready
inputs: {}
steps:
  - id: orient
    uses: capability.ownership.scan
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

        let report = load_workflow_run_report(tempdir.path());
        assert_eq!(report.supported_workflow_count(), 1);
        assert_eq!(report.blocked_workflow_count(), 0);
        let rendered = report.render_text();
        assert!(rendered.contains("workflow runner: supported=1 blocked=0"));
        assert!(
            rendered.contains("ownership scan: total=4 owned=1 unowned=3 ready_unowned=0"),
            "rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("ownership matrix:"),
            "rendered:\n{rendered}"
        );
        assert!(rendered.contains("vac.ownership"), "rendered:\n{rendered}");
        assert!(
            rendered.contains("execution: started="),
            "rendered:\n{rendered}"
        );
        let _ = fs::remove_dir_all(&tempdir);
    }

    #[test]
    fn renders_workflow_approval_policy_report() {
        use tempfile::tempdir;

        let tempdir = tempdir().expect("tempdir");
        let vac_root = tempdir.path().join(".vac");
        let workflow = minimal_manifest(vec![
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
        let registry = approval_policy_registry(
            vac_root.clone(),
            capability_manifest("capability.build.cargo_check", vec!["execute_process"]),
            capability_manifest("capability.activity.emit", Vec::new()),
        );
        let approval_policy = evaluate_workflow_approval_policy(&workflow, &registry);
        let report = WorkflowRunReport {
            registry: RegistryLoadReport::success(registry),
            workflows: vec![WorkflowRunEntry {
                path: vac_root
                    .join("workflows/maintenance.build-check.yaml")
                    .display()
                    .to_string(),
                id: workflow.id.clone(),
                title: workflow.title.clone(),
                preview: preview_workflow_manifest(&workflow),
                approval_policy,
                policy_decisions: Vec::new(),
                identity_check: None,
                no_duplicate_tui: None,
                architecture_invariants: None,
                ownership_scan: None,
                step_resolutions: workflow.steps.iter().map(resolve_workflow_step).collect(),
                dry_run: dry_run_workflow_manifest(&workflow),
                execution: execute_workflow_manifest(&workflow),
            }],
        };
        let rendered = report.render_text();
        assert!(rendered.contains("approval policy: capability registry=loaded 2"));
        assert!(rendered.contains("approval_required=1"));
        assert!(rendered.contains("blocked=0"));
        assert!(rendered.contains("approval policy checks:"));
        assert!(
            rendered.contains("reason: capability policy requires approval for [execute_process]")
        );
    }

    #[test]
    fn renders_workflow_approval_policy_with_canonical_capability_ids() {
        use tempfile::tempdir;

        let tempdir = tempdir().expect("tempdir");
        let vac_root = tempdir.path().join(".vac");
        let workflow = minimal_manifest(vec![
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
        let registry = approval_policy_registry(
            vac_root.clone(),
            capability_manifest("vac.build", vec!["execute_process"]),
            capability_manifest("vac.workflow", vec!["execute_process"]),
        );
        let approval_policy = evaluate_workflow_approval_policy(&workflow, &registry);
        assert_eq!(approval_policy.allowed_step_count(), 0);
        assert_eq!(approval_policy.approval_required_step_count(), 2);
        assert_eq!(approval_policy.blocked_step_count(), 0);
        let report = WorkflowRunReport {
            registry: RegistryLoadReport::success(registry),
            workflows: vec![WorkflowRunEntry {
                path: vac_root
                    .join("workflows/maintenance.build-check.yaml")
                    .display()
                    .to_string(),
                id: workflow.id.clone(),
                title: workflow.title.clone(),
                preview: preview_workflow_manifest(&workflow),
                approval_policy,
                policy_decisions: Vec::new(),
                identity_check: None,
                no_duplicate_tui: None,
                architecture_invariants: None,
                ownership_scan: None,
                step_resolutions: workflow.steps.iter().map(resolve_workflow_step).collect(),
                dry_run: dry_run_workflow_manifest(&workflow),
                execution: execute_workflow_manifest(&workflow),
            }],
        };
        let rendered = report.render_text();
        assert!(rendered.contains("approval policy: capability registry=loaded 2"));
        assert!(rendered.contains("approval_required=2"));
        assert!(rendered.contains("blocked=0"));
        assert!(
            rendered.contains("reason: capability policy requires approval for [execute_process]")
        );
    }

    #[test]
    fn execution_waits_for_approval_policy_required_steps() {
        use tempfile::tempdir;

        let tempdir = tempdir().expect("tempdir");
        let vac_root = tempdir.path().join(".vac");
        let workflow = minimal_manifest(vec![
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
        let registry = approval_policy_registry(
            vac_root,
            capability_manifest("vac.build", vec!["execute_process"]),
            capability_manifest("vac.workflow", vec!["execute_process"]),
        );
        let approval_policy = evaluate_workflow_approval_policy(&workflow, &registry);
        let execution = execute_workflow_manifest_with_approval_policy(&workflow, &approval_policy);

        assert_eq!(execution.started_step_count(), 1);
        assert_eq!(execution.completed_step_count(), 0);
        assert_eq!(execution.waiting_approval_step_count(), 1);
        assert_eq!(execution.blocked_step_count(), 0);
        assert!(execution.state_trace().iter().any(|state| matches!(
            state,
            WorkflowExecutionState::Step {
                lifecycle: super::WorkflowStepLifecycle::WaitingApproval,
                ..
            }
        )));
        assert_eq!(execution.approval_requests().len(), 1);
        let approval_request = &execution.approval_requests()[0];
        assert!(
            approval_request
                .id
                .as_str()
                .starts_with("approval:test.workflow:")
        );
        assert!(approval_request.id.as_str().ends_with(":check"));
        assert_eq!(approval_request.workflow_id, "test.workflow");
        assert_eq!(approval_request.step_id, "check");
        assert_eq!(approval_request.capability_id, "vac.build");
        assert_eq!(
            approval_request.action,
            super::ApprovalAction::ProcessExecute
        );
        assert_eq!(approval_request.risk, super::ApprovalRisk::Execute);
        assert_eq!(approval_request.status, ApprovalStatus::Pending);
        assert!(execution.render_text().contains("waiting_approval"));
        assert!(execution.render_text().contains("waiting approval"));
        assert!(
            execution
                .render_text()
                .contains("approval_id=approval:test.workflow:")
        );
        assert!(execution.render_text().contains("approval requests:"));
        assert!(execution.render_text().contains("action=process_execute"));
        assert!(execution.render_text().contains("risk=execute"));
        assert!(execution.render_text().contains("status=pending"));
    }

    #[test]
    fn approval_request_id_includes_run_and_attempt_for_parallel_stability() {
        let manifest = minimal_manifest(vec![WorkflowStep {
            id: "gate".to_string(),
            uses: "capability.approval.request".to_string(),
            when: None,
            policy: None,
            ui: None,
        }]);
        let mut machine =
            WorkflowExecutionMachine::new(&manifest).with_run_attempt_id("run-a", "attempt-2");

        assert!(machine.advance());
        assert!(machine.advance());

        assert_eq!(
            machine.approval_requests()[0].id.as_str(),
            "approval:test.workflow:run:run-a:attempt:attempt-2:step:gate"
        );
    }

    #[test]
    fn approval_request_has_ttl_and_expiry_fails_waiting_workflow() {
        let manifest = minimal_manifest(vec![WorkflowStep {
            id: "check".to_string(),
            uses: "capability.approval.request".to_string(),
            when: None,
            policy: None,
            ui: None,
        }]);
        let mut machine = WorkflowExecutionMachine::new(&manifest);
        assert!(machine.advance());
        assert!(machine.advance());
        let request = machine.approval_requests()[0].clone();
        let expires_at = request.expires_at.expect("approval request has ttl");

        let before = machine
            .expire_pending_approvals(expires_at - Duration::seconds(1))
            .expect("early expiry sweep succeeds");
        assert_eq!(before, 0);
        assert_eq!(
            machine.approval_requests()[0].status,
            ApprovalStatus::Pending
        );

        let expired = machine
            .expire_pending_approvals(expires_at)
            .expect("expiry sweep succeeds");
        assert_eq!(expired, 1);
        let report = machine.run();
        assert_eq!(
            report.approval_requests()[0].status,
            ApprovalStatus::Expired
        );
        assert!(report.render_text().contains("approval expired"));
        assert!(report.is_failure());
    }

    #[test]
    fn approval_resolution_approved_fails_build_with_exit_status_report() {
        use tempfile::tempdir;

        let tempdir = tempdir().expect("tempdir");
        let vac_root = tempdir.path().join(".vac");
        let workflow = minimal_manifest(vec![
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
        let registry = approval_policy_registry(
            vac_root,
            capability_manifest("vac.build", vec!["execute_process"]),
            capability_manifest("vac.workflow", Vec::new()),
        );
        let approval_policy = evaluate_workflow_approval_policy(&workflow, &registry);
        let mut machine = WorkflowExecutionMachine::new(&workflow)
            .with_approval_checks(approval_policy.checks().to_vec())
            .with_build_check_report(build_check::report_from_output(
                "cargo +1.95.0 check --manifest-path vac-rs/Cargo.toml -p vac-surface-cli".to_string(),
                std::time::Duration::from_millis(3),
                Some(42),
                false,
                b"",
                b"",
            ));

        while machine.advance() {}
        let approval_request_id = machine.approval_requests()[0].id.clone();
        let decided_at = DateTime::<Utc>::from_timestamp(10, 0).expect("valid timestamp");
        assert!(
            machine
                .resolve_approval(
                    &approval_request_id,
                    ApprovalDecision::approved("operator", "reviewed", decided_at),
                )
                .expect("approval resolution")
        );
        while machine.advance() {}
        let report = machine.run();

        assert_eq!(report.completed_step_count(), 0);
        assert_eq!(report.blocked_step_count(), 1);
        assert_eq!(
            report.approval_requests()[0].status,
            ApprovalStatus::Approved
        );
        assert!(report.render_text().contains("resumed_after_approval"));
        assert!(report.render_text().contains("-> approved: reviewed"));
        assert!(
            report
                .render_text()
                .contains("build check failed exit_status=42")
        );
        assert!(report.render_text().contains(
            "execution: failed workflow=test.workflow title=Test workflow reason=build check failed exit_status=42 started=1 completed=0 waiting_approval=0 blocked=1"
        ));
    }

    #[test]
    fn approval_resolution_approved_completes_build_with_injected_report() {
        use tempfile::tempdir;

        let tempdir = tempdir().expect("tempdir");
        let vac_root = tempdir.path().join(".vac");
        let workflow = minimal_manifest(vec![WorkflowStep {
            id: "check".to_string(),
            uses: "capability.build.cargo_check".to_string(),
            when: None,
            policy: None,
            ui: None,
        }]);
        let registry = approval_policy_registry(
            vac_root,
            capability_manifest("vac.build", vec!["execute_process"]),
            capability_manifest("vac.workflow", Vec::new()),
        );
        let approval_policy = evaluate_workflow_approval_policy(&workflow, &registry);
        let mut machine = WorkflowExecutionMachine::new(&workflow)
            .with_approval_checks(approval_policy.checks().to_vec())
            .with_build_check_report(build_check::report_from_output(
                "cargo +1.95.0 check --manifest-path vac-rs/Cargo.toml -p vac-surface-cli".to_string(),
                std::time::Duration::from_millis(3),
                Some(0),
                true,
                b"",
                b"",
            ));

        while machine.advance() {}
        let approval_request_id = machine.approval_requests()[0].id.clone();
        let decided_at = DateTime::<Utc>::from_timestamp(10, 0).expect("valid timestamp");
        assert!(
            machine
                .resolve_approval(
                    &approval_request_id,
                    ApprovalDecision::approved("operator", "reviewed", decided_at),
                )
                .expect("approval resolution")
        );
        while machine.advance() {}
        let report = machine.run();

        assert_eq!(report.completed_step_count(), 1);
        assert_eq!(report.blocked_step_count(), 0);
        assert_eq!(
            report.approval_requests()[0].status,
            ApprovalStatus::Approved
        );
        assert!(report.render_text().contains("resumed_after_approval"));
        assert!(report.render_text().contains("-> approved: reviewed"));
        assert!(report.render_text().contains(
            "execution: finished workflow=test.workflow title=Test workflow started=1 completed=1 waiting_approval=0 blocked=0"
        ));
    }

    #[test]
    #[cfg(unix)]
    fn build_check_executor_invokes_real_cargo_on_success() {
        use std::fs;
        use tempfile::tempdir;

        let tempdir = tempdir().expect("tempdir");
        let vac_root = tempdir.path().join(".vac");
        let fake_cargo = write_fake_cargo_script(tempdir.path(), 0, "fake cargo success", "");
        let fake_repo_root = prepare_fake_repo(&tempdir);
        let workflow = minimal_manifest(vec![WorkflowStep {
            id: "check".to_string(),
            uses: "capability.build.cargo_check".to_string(),
            when: None,
            policy: None,
            ui: None,
        }]);
        let registry = approval_policy_registry(
            vac_root,
            capability_manifest("vac.build", vec!["execute_process"]),
            capability_manifest("vac.workflow", Vec::new()),
        );
        let approval_policy = evaluate_workflow_approval_policy(&workflow, &registry);
        let mut machine = WorkflowExecutionMachine::new(&workflow)
            .with_approval_checks(approval_policy.checks().to_vec())
            .with_build_check_cargo_program(fake_cargo.clone())
            .with_build_check_repo_root(fake_repo_root);

        while machine.advance() {}
        let approval_request_id = machine.approval_requests()[0].id.clone();
        let decided_at = DateTime::<Utc>::from_timestamp(10, 0).expect("valid timestamp");
        assert!(
            machine
                .resolve_approval(
                    &approval_request_id,
                    ApprovalDecision::approved("operator", "reviewed", decided_at),
                )
                .expect("approval resolution")
        );
        while machine.advance() {}
        let report = machine.run();

        assert_eq!(report.completed_step_count(), 1);
        assert_eq!(report.blocked_step_count(), 0);
        let invoked = fs::read_to_string(fake_cargo.with_extension("invoked")).expect("invoked");
        assert!(
            invoked.contains("+1.95.0 check --manifest-path vac-rs/Cargo.toml -p vac-surface-cli"),
            "{invoked}"
        );
        assert!(report.render_text().contains(
            "execution: finished workflow=test.workflow title=Test workflow started=1 completed=1 waiting_approval=0 blocked=0"
        ));
    }

    #[test]
    #[cfg(unix)]
    fn build_check_executor_invokes_real_cargo_on_failure() {
        use std::fs;
        use tempfile::tempdir;

        let tempdir = tempdir().expect("tempdir");
        let vac_root = tempdir.path().join(".vac");
        let fake_cargo = write_fake_cargo_script(tempdir.path(), 42, "", "error: fake fail");
        let fake_repo_root = prepare_fake_repo(&tempdir);
        let workflow = minimal_manifest(vec![WorkflowStep {
            id: "check".to_string(),
            uses: "capability.build.cargo_check".to_string(),
            when: None,
            policy: None,
            ui: None,
        }]);
        let registry = approval_policy_registry(
            vac_root,
            capability_manifest("vac.build", vec!["execute_process"]),
            capability_manifest("vac.workflow", Vec::new()),
        );
        let approval_policy = evaluate_workflow_approval_policy(&workflow, &registry);
        let mut machine = WorkflowExecutionMachine::new(&workflow)
            .with_approval_checks(approval_policy.checks().to_vec())
            .with_build_check_cargo_program(fake_cargo.clone())
            .with_build_check_repo_root(fake_repo_root);

        while machine.advance() {}
        let approval_request_id = machine.approval_requests()[0].id.clone();
        let decided_at = DateTime::<Utc>::from_timestamp(10, 0).expect("valid timestamp");
        assert!(
            machine
                .resolve_approval(
                    &approval_request_id,
                    ApprovalDecision::approved("operator", "reviewed", decided_at),
                )
