                .expect("approval resolution")
        );
        while machine.advance() {}
        let report = machine.run();

        assert_eq!(report.completed_step_count(), 0);
        assert_eq!(report.blocked_step_count(), 1);
        assert!(fs::read_to_string(fake_cargo.with_extension("invoked")).is_ok());
        assert!(report.render_text().contains("error: fake fail"));
        assert!(report.render_text().contains("execution: failed"));
    }

    #[test]
    #[cfg(unix)]
    fn build_check_executor_prefers_injected_report_over_real_cargo() {
        use tempfile::tempdir;

        let tempdir = tempdir().expect("tempdir");
        let vac_root = tempdir.path().join(".vac");
        let fake_cargo = write_fake_cargo_script(tempdir.path(), 99, "", "should not run");
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
            .with_build_check_report(build_check::report_from_output(
                "injected cargo check".to_string(),
                std::time::Duration::from_millis(1),
                Some(0),
                true,
                b"injected success",
                b"",
            ))
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
        assert!(!fake_cargo.with_extension("invoked").exists());
    }

    #[test]
    fn approval_resolution_rejected_fails_waiting_workflow() {
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
            .with_approval_checks(approval_policy.checks().to_vec());

        while machine.advance() {}
        let approval_request_id = machine.approval_requests()[0].id.clone();
        let decided_at = DateTime::<Utc>::from_timestamp(10, 0).expect("valid timestamp");
        assert!(
            machine
                .resolve_approval(
                    &approval_request_id,
                    ApprovalDecision::rejected("operator", "unsafe command", decided_at),
                )
                .expect("approval rejection")
        );
        let report = machine.run();

        assert_eq!(report.completed_step_count(), 0);
        assert_eq!(report.blocked_step_count(), 1);
        assert_eq!(
            report.approval_requests()[0].status,
            ApprovalStatus::Rejected
        );
        assert!(report.render_text().contains("-> rejected: unsafe command"));
        assert!(
            report
                .render_text()
                .contains("approval rejected: unsafe command")
        );
        assert!(report.render_text().contains("execution: failed"));
    }

    #[test]
    fn alias_resolves_but_target_capability_missing_is_blocked() {
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
            capability_manifest("vac.workflow", Vec::new()),
            capability_manifest("vac.workflow", Vec::new()),
        );
        let report = evaluate_workflow_approval_policy(&workflow, &registry);
        assert_eq!(report.allowed_step_count(), 0);
        assert_eq!(report.approval_required_step_count(), 0);
        assert_eq!(report.blocked_step_count(), 1);
        let check = &report.checks()[0];
        assert_eq!(check.decision, WorkflowApprovalDecision::Blocked);
        assert_eq!(check.reason.as_deref(), Some("missing capability manifest"));
    }

    #[test]
    fn exact_match_custom_uses_resolves_without_alias() {
        use tempfile::tempdir;

        let tempdir = tempdir().expect("tempdir");
        let vac_root = tempdir.path().join(".vac");
        let workflow = minimal_manifest(vec![WorkflowStep {
            id: "custom".to_string(),
            uses: "custom.team.cap".to_string(),
            when: None,
            policy: None,
            ui: None,
        }]);
        let registry = approval_policy_registry(
            vac_root,
            capability_manifest("custom.team.cap", Vec::new()),
            capability_manifest("vac.workflow", Vec::new()),
        );
        let report = evaluate_workflow_approval_policy(&workflow, &registry);
        assert_eq!(report.allowed_step_count(), 1);
        assert_eq!(report.approval_required_step_count(), 0);
        assert_eq!(report.blocked_step_count(), 0);
    }

    #[test]
    fn workflow_step_vocabulary_is_internally_consistent() {
        use std::collections::BTreeSet;

        assert_eq!(
            WORKFLOW_STEP_VOCABULARY.len(),
            INITIAL_ALLOWED_STEP_USES.len()
        );

        let vocabulary_uses = WORKFLOW_STEP_VOCABULARY
            .iter()
            .map(|entry| entry.uses)
            .collect::<BTreeSet<_>>();
        let policy_intent_uses = WORKFLOW_STEP_POLICY_INTENTS
            .iter()
            .map(|entry| entry.uses)
            .collect::<BTreeSet<_>>();
        assert_eq!(
            vocabulary_uses, policy_intent_uses,
            "workflow step vocabulary and policy intent uses must stay set-equal"
        );

        for entry in WORKFLOW_STEP_VOCABULARY {
            assert!(
                is_supported_step_use(entry.uses),
                "uses {} missing",
                entry.uses
            );
            assert_eq!(
                resolve_workflow_step_handler(entry.uses),
                Some(entry.handler),
                "handler mismatch for {}",
                entry.uses
            );
            assert_eq!(
                resolve_workflow_approval_capability_id(entry.uses),
                Some(entry.canonical_capability_id),
                "alias mismatch for {}",
                entry.uses
            );
        }
    }

    #[test]
    fn workflow_step_policy_requires_approval_is_evaluated() {
        use crate::control_plane::workflow_manifest::WorkflowPolicyValue;
        use crate::control_plane::workflow_manifest::WorkflowStepPolicy;
        use tempfile::tempdir;

        let workflow = minimal_manifest(vec![WorkflowStep {
            id: "check".to_string(),
            uses: "capability.build.cargo_check".to_string(),
            when: None,
            policy: Some(WorkflowStepPolicy {
                default_risk: None,
                mutates_files: None,
                network: Some(WorkflowPolicyValue::Bool(true)),
                redaction: None,
                approval_required_for: Vec::new(),
            }),
            ui: None,
        }]);
        let tempdir = tempdir().expect("tempdir");
        let registry = approval_policy_registry(
            tempdir.path().join(".vac"),
            capability_manifest("vac.build", vec![]),
            capability_manifest("vac.workflow", vec![]),
        );

        let report = evaluate_workflow_approval_policy(&workflow, &registry);

        assert_eq!(report.approval_required_step_count(), 1);
        assert!(
            report
                .render_lines()
                .join("\n")
                .contains("workflow step policy marks network access")
        );
    }

    #[test]
    fn identity_check_without_report_fails_closed() {
        let workflow = minimal_manifest(vec![WorkflowStep {
            id: "identity".to_string(),
            uses: "capability.identity.check".to_string(),
            when: None,
            policy: None,
            ui: None,
        }]);
        let execution = execute_workflow_manifest(&workflow);

        assert!(execution.is_failure());
        assert!(
            execution
                .render_text()
                .contains("identity check report unavailable")
        );
    }

    #[test]
    fn approved_waiting_workflow_is_not_permanent_waiting_failure() {
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
        let report = machine.run();

        assert_eq!(report.waiting_approval_step_count(), 1);
        assert_eq!(report.completed_step_count(), 1);
        assert_eq!(report.blocked_step_count(), 0);
        assert!(!report.is_waiting_approval());
        assert!(!report.is_failure());
    }

    #[test]
    fn policy_block_precedes_approval_request_and_revalidates_on_resume() {
        use crate::control_plane::policy_manifest::PolicyDecisionReport;
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
        let block_decision = PolicyDecisionReport::Block {
            reasons: vec!["filesystem scope denied".to_string()],
        };

        let blocked_before_approval = WorkflowExecutionMachine::new(&workflow)
            .with_approval_checks(approval_policy.checks().to_vec())
            .with_policy_decisions(vec![Some(block_decision.clone())])
            .run();
        assert_eq!(blocked_before_approval.waiting_approval_step_count(), 0);
        assert_eq!(blocked_before_approval.blocked_step_count(), 1);
        assert!(
            blocked_before_approval
                .render_text()
                .contains("policy: filesystem scope denied")
        );

        let mut machine = WorkflowExecutionMachine::new(&workflow)
            .with_approval_checks(approval_policy.checks().to_vec());
        while machine.advance() {}
        machine = machine.with_policy_decisions(vec![Some(block_decision)]);
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
        let blocked_on_resume = machine.run();
        assert_eq!(blocked_on_resume.waiting_approval_step_count(), 1);
        assert_eq!(blocked_on_resume.blocked_step_count(), 1);
        assert!(!blocked_on_resume.is_waiting_approval());
        assert!(
            blocked_on_resume
                .render_text()
                .contains("policy: filesystem scope denied")
        );
    }

    #[test]
    fn custom_step_policy_decision_defaults_to_unknown_policy() {
        use crate::control_plane::policy_manifest::PolicyDecisionReport;

        let workflow = minimal_manifest(vec![WorkflowStep {
            id: "custom".to_string(),
            uses: "custom.team.capability".to_string(),
            when: None,
            policy: None,
            ui: None,
        }]);

        let decisions = evaluate_workflow_step_policy_decisions(&workflow, &[]);
        assert_eq!(decisions.len(), 1);
        match decisions.first() {
            Some(Some(PolicyDecisionReport::UnknownPolicy { reasons })) => {
                assert!(reasons[0].contains("outside the built-in policy vocabulary"));
            }
            other => panic!("expected unknown policy for custom step, got {other:?}"),
        }
    }

    #[test]
    fn tui_pty_gate_blocked_operator_is_not_release_pass() {
        let result = TuiPtyGateResult::blocked_operator(
            "no interactive TTY available in this environment",
            vec!["diagnostics=private/redacted".to_string()],
        );
        assert_eq!(result.state, TuiPtyGateResultState::BlockedOperator);
        assert!(!result.satisfies_release_gate());
        assert_eq!(
            result.report_reason(),
            "BLOCKED-OPERATOR: no interactive TTY available in this environment"
        );

        let manifest = minimal_manifest(vec![WorkflowStep {
            id: "pty".to_string(),
            uses: "capability.tui.pty_gate".to_string(),
            when: None,
            policy: None,
            ui: None,
        }]);
        let report = WorkflowExecutionMachine::new(&manifest)
            .with_tui_pty_gate_result(result)
            .run();

        assert_eq!(report.completed_step_count(), 0);
        assert_eq!(report.blocked_step_count(), 1);
        assert!(report.render_text().contains("BLOCKED-OPERATOR"));
    }

    #[test]
    fn tui_pty_gate_passed_is_release_pass() {
        let result = TuiPtyGateResult::passed(
            Some(0),
            vec![
                "root_vac_screen_output=observed".to_string(),
                "slash_input=screen_update_observed".to_string(),
                "ctrl_c_exit=exit_code_0".to_string(),
            ],
        );
        assert_eq!(result.state, TuiPtyGateResultState::Passed);
        assert!(result.satisfies_release_gate());

        let manifest = minimal_manifest(vec![WorkflowStep {
            id: "pty".to_string(),
            uses: "capability.tui.pty_gate".to_string(),
            when: None,
            policy: None,
            ui: None,
        }]);
        let report = WorkflowExecutionMachine::new(&manifest)
            .with_tui_pty_gate_result(result)
            .run();

        assert_eq!(report.completed_step_count(), 1);
        assert_eq!(report.blocked_step_count(), 0);
        assert!(report.render_text().contains("step 1. pty -> succeeded"));
    }

    #[test]
    fn gate_deprecated_not_recognized() {
        assert!(!is_supported_step_use("capability.donor_migration.gate"));
        assert_eq!(
            resolve_workflow_step_handler("capability.donor_migration.gate"),
            None
        );
        assert_eq!(
            resolve_workflow_approval_capability_id("capability.donor_migration.gate"),
            None
        );
    }

    fn donor_status_report_with_failures(failures: Vec<DonorStatusFailure>) -> DonorStatusReport {
        let repo_root = PathBuf::from("/tmp/vac-donor-status-test");
        DonorStatusReport {
            repo_root: repo_root.clone(),
            capabilities_dir: repo_root.join(".vac/capabilities"),
            entries: Vec::new(),
            failures,
        }
    }

    fn donor_manifest_check_workflow() -> WorkflowManifest {
        minimal_manifest(vec![WorkflowStep {
            id: "donor".to_string(),
            uses: "capability.donor_migration.manifest_check".to_string(),
            when: None,
            policy: None,
            ui: None,
        }])
    }

    #[test]
    fn donor_manifest_check_clean_succeeds() {
        let workflow = donor_manifest_check_workflow();
        let report = WorkflowExecutionMachine::new(&workflow)
            .with_donor_status_report(donor_status_report_with_failures(Vec::new()))
            .run();

        assert_eq!(report.completed_step_count(), 1);
        assert_eq!(report.blocked_step_count(), 0);
        assert!(report.render_text().contains("step 1. donor -> succeeded"));
    }

    #[test]
    fn donor_manifest_check_failures_fail() {
        let workflow = donor_manifest_check_workflow();
        let report = WorkflowExecutionMachine::new(&workflow)
            .with_donor_status_report(donor_status_report_with_failures(vec![
                DonorStatusFailure {
                    manifest_path: PathBuf::from(".vac/capabilities/donor_migration.yaml"),
                    message: "invalid donor status".to_string(),
                },
            ]))
            .run();

        assert_eq!(report.completed_step_count(), 0);
        assert_eq!(report.blocked_step_count(), 1);
        assert!(
            report
                .render_text()
                .contains("donor migration check failed (1 failures)")
        );
    }

    #[test]
    fn donor_manifest_check_unavailable_fail() {
        let workflow = donor_manifest_check_workflow();
        let report = WorkflowExecutionMachine::new(&workflow).run();

        assert_eq!(report.completed_step_count(), 0);
        assert_eq!(report.blocked_step_count(), 1);
        assert!(
            report
                .render_text()
                .contains("donor status report unavailable")
        );
    }
}
