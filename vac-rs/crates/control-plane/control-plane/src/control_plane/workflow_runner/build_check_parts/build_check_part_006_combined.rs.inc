#[cfg(test)]
mod tests {
    use super::INITIAL_ALLOWED_STEP_USES;
    use super::TuiPtyGateResult;
    use super::TuiPtyGateResultState;
    use super::WORKFLOW_STEP_VOCABULARY;
    use super::WORKFLOW_STEP_POLICY_INTENTS;
    use super::WorkflowApprovalDecision;
    use super::WorkflowDryRunEvent;
    use super::WorkflowDryRunMachine;
    use super::WorkflowDryRunState;
    use super::WorkflowExecutionEvent;
    use super::WorkflowExecutionMachine;
    use super::WorkflowExecutionState;
    use super::WorkflowRunEntry;
    use super::WorkflowRunReport;
    use super::WorkflowStepHandler;
    use super::build_check;
    use super::dry_run_workflow_manifest;
    use super::evaluate_workflow_approval_policy;
    use super::execute_workflow_manifest;
    use super::execute_workflow_manifest_with_approval_policy;
    use super::execute_workflow_manifest_with_root_seed_registry_report;
    use super::evaluate_workflow_step_policy_decisions;
    use super::format_workflow_dry_run_event;
    use super::format_workflow_dry_run_state;
    use super::format_workflow_execution_event;
    use super::format_workflow_execution_state;
    use super::is_supported_step_use;
    use super::load_workflow_run_report;
    use super::preview_workflow_manifest;
    use super::resolve_workflow_approval_capability_id;
    use super::resolve_workflow_step;
    use super::resolve_workflow_step_handler;
    use crate::control_plane::RegistryLoadReport;
    use crate::control_plane::approval_lifecycle::ApprovalDecision;
    use crate::control_plane::approval_lifecycle::ApprovalStatus;
    use crate::control_plane::capability_manifest::CapabilityCliSurface;
    use crate::control_plane::capability_manifest::CapabilityCommandCollection;
    use crate::control_plane::capability_manifest::CapabilityManifest;
    use crate::control_plane::capability_manifest::CapabilityManifestKind;
    use crate::control_plane::capability_manifest::CapabilityOwner;
    use crate::control_plane::capability_manifest::CapabilityPolicy;
    use crate::control_plane::capability_manifest::CapabilityPolicyValue;
    use crate::control_plane::capability_manifest::CapabilityRouteCollection;
    use crate::control_plane::capability_manifest::CapabilityStatus;
    use crate::control_plane::capability_manifest::CapabilitySurfaces;
    use crate::control_plane::capability_manifest::CapabilityValidation as CapabilityManifestValidation;
    use crate::control_plane::donor_status::DonorStatusFailure;
    use crate::control_plane::donor_status::DonorStatusReport;
    use crate::control_plane::registry::ControlPlaneRegistry;
    use crate::control_plane::registry::LocatedManifest;
    use crate::control_plane::registry::ManifestRegistry;
    use crate::control_plane::workflow_manifest::WorkflowInputKind;
    use crate::control_plane::workflow_manifest::WorkflowInputSpec;
    use crate::control_plane::workflow_manifest::WorkflowManifest;
    use crate::control_plane::workflow_manifest::WorkflowManifestKind;
    use crate::control_plane::workflow_manifest::WorkflowPolicy;
    use crate::control_plane::workflow_manifest::WorkflowStatus;
    use crate::control_plane::workflow_manifest::WorkflowStep;
    use crate::control_plane::workflow_manifest::WorkflowUi;
    use crate::control_plane::workflow_manifest::WorkflowValidation;
    use chrono::{DateTime, Duration, Utc};
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    #[cfg(unix)]
    fn write_fake_cargo_script(dir: &Path, exit_code: i32, stdout: &str, stderr: &str) -> PathBuf {
        use std::fs;
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;

        let cargo = dir.join("cargo");
        let mut script = "#!/bin/sh\n".to_string();
        script.push_str("printf '%s\\n' \"$*\" > \"$0.invoked\"\n");
        if !stdout.is_empty() {
            script.push_str("cat <<'EOF'\n");
            script.push_str(stdout);
            if !stdout.ends_with('\n') {
                script.push('\n');
            }
            script.push_str("EOF\n");
        }
        if !stderr.is_empty() {
            script.push_str("cat >&2 <<'EOF'\n");
            script.push_str(stderr);
            if !stderr.ends_with('\n') {
                script.push('\n');
            }
            script.push_str("EOF\n");
        }
        script.push_str(&format!("exit {exit_code}\n"));

        let mut file = fs::File::create(&cargo).expect("fake cargo");
        file.write_all(script.as_bytes())
            .expect("fake cargo script");
        let mut permissions = file.metadata().expect("metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&cargo, permissions).expect("permissions");
        cargo
    }

    #[cfg(unix)]
    fn prepare_fake_repo(tempdir: &tempfile::TempDir) -> PathBuf {
        use std::fs;

        let repo = tempdir.path().join("repo");
        fs::create_dir_all(repo.join("vac-rs")).expect("repo vac-rs");
        fs::write(repo.join("vac-rs/Cargo.toml"), "[workspace]\n").expect("manifest");
        repo
    }

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

    fn capability_manifest(id: &str, approval_required_for: Vec<&str>) -> CapabilityManifest {
        CapabilityManifest {
            schema_version: 1,
            kind: CapabilityManifestKind::Capability,
            id: id.to_string(),
            title: id.to_string(),
            status: CapabilityStatus::Ready,
            owner: CapabilityOwner::Path("vac-rs/core/control_plane".to_string()),
            surfaces: CapabilitySurfaces {
                tui: Some(CapabilityRouteCollection {
                    routes: vec!["/workflow".to_string()],
                }),
                slash: Some(CapabilityCommandCollection {
                    commands: vec!["/workflow".to_string()],
                }),
                palette: true,
                cli: Some(CapabilityCliSurface {
                    enabled: true,
                    commands: vec!["vac doctor workflow".to_string()],
                }),
            },
            policy: CapabilityPolicy {
                risk: Some("safe_read".to_string()),
                mutates_files: Some(CapabilityPolicyValue::Bool(false)),
                network: Some(CapabilityPolicyValue::Bool(false)),
                redaction: Some(CapabilityPolicyValue::Bool(false)),
                approval_required_for: approval_required_for
                    .into_iter()
                    .map(std::string::ToString::to_string)
                    .collect(),
            },
            compatibility_transport: Vec::new(),
            validation: CapabilityManifestValidation {
                commands: vec!["cargo +1.93.0 check -p vac-surface-cli".to_string()],
                gates: Vec::new(),
            },
            description: Some(id.to_string()),
            reason: None,
            depends_on: Vec::new(),
            optional_depends_on: Vec::new(),
            runtime_events: Vec::new(),
            states: None,
            ownership: None,
            donor_source: None,
            docs: Vec::new(),
        }
    }

    fn empty_manifest_registry<T>(vac_root: PathBuf, manifest_dir: PathBuf) -> ManifestRegistry<T> {
        ManifestRegistry {
            vac_root,
            manifest_dir,
            manifests: Vec::new(),
        }
    }

    fn approval_policy_registry(
        vac_root: PathBuf,
        build_manifest: CapabilityManifest,
        activity_manifest: CapabilityManifest,
    ) -> ControlPlaneRegistry {
        let capabilities_dir = vac_root.join("capabilities");
        let workflows_dir = vac_root.join("workflows");
        let policies_dir = vac_root.join("policies");
        let surfaces_dir = vac_root.join("surfaces");
        let vac_root_for_children = vac_root.clone();

        ControlPlaneRegistry {
            vac_root,
            capabilities: ManifestRegistry {
                vac_root: vac_root_for_children,
                manifest_dir: capabilities_dir,
                manifests: vec![
                    LocatedManifest::new("/tmp/capabilities/build.yaml", build_manifest),
                    LocatedManifest::new("/tmp/capabilities/activity.yaml", activity_manifest),
                ],
            },
            workflows: empty_manifest_registry("/tmp".into(), workflows_dir),
            policies: empty_manifest_registry("/tmp".into(), policies_dir),
            surfaces: empty_manifest_registry("/tmp".into(), surfaces_dir),
        }
    }

    fn seed_core_capabilities(root: &std::path::Path) {
        std::fs::create_dir_all(root.join("capabilities")).expect("capabilities dir");
        std::fs::write(
            root.join("capabilities/build.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.build
title: Build
status: planned
reason: test fixture planned capability
states:
  empty: empty
  loading: loading
  success: success
  failure: failure
owner:
  crate: vac-surface-cli
  module: main
description: Build and maintenance validation gates for the root product.
depends_on:
  - vac.workflow
surfaces:
  palette: true
policy:
  risk: safe_read
  mutates_files: false
  network: false
  redaction: false
  approval_required_for:
    - execute_process
validation:
  commands:
    - CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-cli
"#,
        )
        .expect("build capability manifest");
        std::fs::write(
            root.join("capabilities/workflow.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.workflow
title: Workflow
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
description: Typed workflow manifests, registry loading, and diagnostics.
depends_on:
  - vac.sessions
surfaces:
  palette: true
  cli:
    enabled: true
    commands:
      - vac doctor registry
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
    - CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-cli
"#,
        )
        .expect("workflow capability manifest");
        std::fs::write(
            root.join("capabilities/identity-check.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.identity.check
title: Identity Check
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
description: Product identity and legacy term scanning for the root product.
depends_on:
  - vac.workflow
surfaces:
  palette: true
policy:
  risk: safe_read
  mutates_files: false
  network: false
  redaction: false
validation:
  commands:
    - cargo +1.93.0 check -p vac-surface-cli
"#,
        )
        .expect("identity check capability manifest");
    }

    fn seed_tui_uniqueness_capabilities(root: &std::path::Path) {
        std::fs::create_dir_all(root.join("capabilities")).expect("capabilities dir");
        std::fs::write(
            root.join("capabilities/tui.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.tui
title: TUI
status: partial
reason: test fixture partial capability
states:
  empty: empty
  loading: loading
  success: success
  failure: failure
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
policy:
  risk: safe_read
  mutates_files: false
  network: false
  redaction: false
validation:
  commands:
    - CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-tui --tests
"#,
        )
        .expect("tui capability manifest");
        std::fs::write(
            root.join("capabilities/workflow.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.workflow
title: Workflow
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
description: Typed workflow manifests, registry loading, and diagnostics.
depends_on:
  - vac.sessions
surfaces:
  palette: true
  cli:
    enabled: true
    commands:
      - vac doctor workflow
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
    - CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-cli
"#,
        )
        .expect("workflow capability manifest");
    }

    fn seed_permissive_policy(vac_root: &std::path::Path) {
        std::fs::create_dir_all(vac_root.join("policies")).expect("policies dir");
        std::fs::write(
            vac_root.join("policies/permissive.yaml"),
            r#"
schema_version: 1
kind: policy
id: maintenance.permissive
title: Permissive Test Policy
default_decision: deny
rules:
  - id: allow_filesystem_read_project
    match:
      action: filesystem_read
      path: project
    decision: allow
    reason: test fixture allows project filesystem reads
"#,
        )
        .expect("permissive policy manifest");
    }

    #[test]
    fn recognizes_initial_safe_step_uses() {
        assert!(is_supported_step_use("capability.root_seed.coverage"));
        assert!(is_supported_step_use("capability.build.cargo_check"));
        assert!(is_supported_step_use("capability.identity.check"));
        assert!(is_supported_step_use("capability.ownership.scan"));
        assert!(!is_supported_step_use("capability.workflow"));
        assert_eq!(
            resolve_workflow_step_handler("capability.tui.pty_gate"),
            Some(WorkflowStepHandler::TuiPtyGate)
        );
        assert_eq!(
            resolve_workflow_step_handler("capability.root_seed.coverage"),
            Some(WorkflowStepHandler::RootSeedCoverage)
        );
        assert_eq!(
            resolve_workflow_step_handler("capability.ownership.scan"),
            Some(WorkflowStepHandler::OwnershipScan)
        );
        assert!(is_supported_step_use(
            "capability.donor_migration.inventory_check"
        ));
        assert!(!is_supported_step_use("capability.donor_migration.gate"));
        assert_eq!(
            resolve_workflow_step_handler("capability.donor_migration.inventory_check"),
            Some(WorkflowStepHandler::DonorMigrationInventoryCheck)
        );
        assert_eq!(
            resolve_workflow_step_handler("capability.donor_migration.gate"),
            None
        );
        assert_eq!(
            resolve_workflow_approval_capability_id("capability.donor_migration.drift_check"),
            Some("vac.donor_migration".to_string())
        );
    }

    #[test]
    fn previews_supported_workflow_steps() {
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

        let preview = preview_workflow_manifest(&manifest);
        assert!(preview.supported);
        assert!(preview.blocked_steps.is_empty());
        assert_eq!(preview.blocked_step_count(), 0);
    }

    #[test]
    fn previews_blocked_workflow_steps() {
        let manifest = minimal_manifest(vec![
            WorkflowStep {
                id: "orient".to_string(),
                uses: "capability.workflow".to_string(),
                when: None,
                policy: None,
                ui: None,
            },
            WorkflowStep {
                id: "validate".to_string(),
                uses: "capability.build".to_string(),
                when: None,
                policy: None,
                ui: None,
            },
        ]);

        let preview = preview_workflow_manifest(&manifest);
        assert!(!preview.supported);
        assert_eq!(preview.blocked_steps.len(), 2);
        assert!(preview.blocked_steps[0].reason.contains("not yet mapped"));
    }

    #[test]
    fn dry_run_reports_step_events() {
        let manifest = minimal_manifest(vec![
            WorkflowStep {
                id: "check".to_string(),
                uses: "capability.build.cargo_check".to_string(),
                when: None,
                policy: None,
                ui: None,
            },
            WorkflowStep {
                id: "orient".to_string(),
                uses: "capability.workflow".to_string(),
                when: None,
                policy: None,
                ui: None,
            },
        ]);

        let dry_run = dry_run_workflow_manifest(&manifest);
        assert_eq!(dry_run.state_trace().len(), 5);
        assert!(matches!(
            dry_run.state_trace().first(),
            Some(WorkflowDryRunState::Ready { step_count, .. }) if *step_count == 2
        ));
        assert!(matches!(
            dry_run.state_trace().get(1),
            Some(WorkflowDryRunState::Started { .. })
        ));
        assert!(matches!(
            dry_run.state_trace().get(2),
            Some(WorkflowDryRunState::Step {
                index,
                supported_step_count,
                blocked_step_count,
                ..
            }) if *index == 1 && *supported_step_count == 1 && *blocked_step_count == 0
        ));
        assert!(matches!(
            dry_run.state_trace().last(),
            Some(WorkflowDryRunState::Finished {
                supported_step_count,
                blocked_step_count,
                ..
            }) if *supported_step_count == 1 && *blocked_step_count == 1
        ));
        assert_eq!(dry_run.supported_step_count(), 1);
        assert_eq!(dry_run.blocked_step_count(), 1);
        assert!(matches!(
            dry_run.events().first(),
            Some(WorkflowDryRunEvent::Started { step_count, .. }) if *step_count == 2
        ));
        assert!(matches!(
            dry_run.events().last(),
            Some(WorkflowDryRunEvent::Finished {
                supported_step_count,
                blocked_step_count,
            }) if *supported_step_count == 1 && *blocked_step_count == 1
        ));
        let rendered = dry_run.render_text();
        assert!(rendered.contains("dry-run: started workflow=test.workflow"));
        assert!(rendered.contains("dry-run: step 1. check -> handler=build.cargo_check"));
        assert!(rendered.contains("dry-run: step 2. orient -> blocked:"));
        assert!(rendered.contains("dry-run: finished supported=1 blocked=1"));
        assert_eq!(
            format_workflow_dry_run_event(&WorkflowDryRunEvent::Finished {
                supported_step_count: 1,
                blocked_step_count: 1,
            }),
            "finished supported=1 blocked=1"
        );
        assert_eq!(
            format_workflow_dry_run_state(&WorkflowDryRunState::Finished {
                workflow_id: "test.workflow".to_string(),
                title: "Test workflow".to_string(),
                supported_step_count: 1,
                blocked_step_count: 1,
            }),
            "dry-run: finished workflow=test.workflow title=Test workflow supported=1 blocked=1"
        );
    }

    #[test]
    fn dry_run_machine_advances_through_states() {
        let manifest = minimal_manifest(vec![WorkflowStep {
            id: "check".to_string(),
            uses: "capability.build.cargo_check".to_string(),
            when: None,
            policy: None,
            ui: None,
        }]);

        let mut machine = WorkflowDryRunMachine::new(&manifest);
        assert!(matches!(
            machine.state(),
            WorkflowDryRunState::Ready { step_count, .. } if step_count == 1
        ));

        assert!(machine.advance());
        assert!(matches!(
            machine.state(),
            WorkflowDryRunState::Started { step_count, .. } if step_count == 1
        ));

        assert!(machine.advance());
        assert!(matches!(
            machine.state(),
            WorkflowDryRunState::Step {
                index,
                supported_step_count,
                blocked_step_count,
                ..
            } if index == 1 && supported_step_count == 1 && blocked_step_count == 0
        ));

        assert!(machine.advance());
        assert!(matches!(
            machine.state(),
            WorkflowDryRunState::Finished {
                supported_step_count,
                blocked_step_count,
                ..
            } if supported_step_count == 1 && blocked_step_count == 0
        ));
        assert!(!machine.advance());
    }

    #[test]
    fn execution_reports_progress_and_approval_wait() {
        let manifest = minimal_manifest(vec![WorkflowStep {
            id: "approve".to_string(),
            uses: "capability.approval.request".to_string(),
            when: None,
            policy: None,
            ui: None,
        }]);

        let execution = execute_workflow_manifest(&manifest);
        assert_eq!(execution.started_step_count(), 1);
        assert_eq!(execution.completed_step_count(), 0);
        assert_eq!(execution.waiting_approval_step_count(), 1);
        assert_eq!(execution.blocked_step_count(), 0);
        assert!(matches!(
            execution.state_trace().first(),
            Some(WorkflowExecutionState::Ready { step_count, .. }) if *step_count == 1
        ));
        assert!(execution.state_trace().iter().any(|state| matches!(
            state,
            WorkflowExecutionState::Step {
                lifecycle: super::WorkflowStepLifecycle::WaitingApproval,
                ..
            }
        )));
        let rendered = execution.render_text();
        assert!(rendered.contains("execution state:"));
        assert!(rendered.contains("lifecycle=waiting_approval"));
        assert!(rendered.contains("execution events:"));
        assert!(rendered.contains("waiting approval"));
        assert_eq!(
            format_workflow_execution_event(&WorkflowExecutionEvent::Cancelled {
                reason: "operator cancel".to_string(),
            }),
            "cancelled reason=operator cancel"
        );
        assert_eq!(
            format_workflow_execution_state(&WorkflowExecutionState::Finished {
                workflow_id: "test.workflow".to_string(),
                title: "Test workflow".to_string(),
                started_step_count: 2,
                completed_step_count: 1,
                waiting_approval_step_count: 1,
                blocked_step_count: 0,
            }),
            "execution: finished workflow=test.workflow title=Test workflow started=2 completed=1 waiting_approval=1 blocked=0"
        );
    }

    #[test]
    fn execution_blocks_root_seed_step_when_registry_report_fails() {
        let manifest = minimal_manifest(vec![WorkflowStep {
            id: "root_seed".to_string(),
            uses: "capability.root_seed.coverage".to_string(),
            when: None,
            policy: None,
            ui: None,
        }]);
        let tempdir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(tempdir.path().join(".vac")).expect("vac root");
        let failed_registry =
            crate::control_plane::registry::load_control_plane_registry_report(tempdir.path());
        assert!(failed_registry.is_failure());

        let execution =
            execute_workflow_manifest_with_root_seed_registry_report(&manifest, &failed_registry);

        assert_eq!(execution.started_step_count(), 1);
        assert_eq!(execution.completed_step_count(), 0);
        assert_eq!(execution.blocked_step_count(), 1);
        assert!(
            execution
                .render_text()
                .contains("root seed coverage: blocked"),
            "rendered:\n{}",
            execution.render_text()
        );
    }

    #[test]
    fn execution_blocks_ownership_scan_when_ready_capability_is_unowned() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let vac_root = tempdir.path().join(".vac");
        std::fs::create_dir_all(vac_root.join("capabilities")).expect("capabilities dir");
        std::fs::write(
            vac_root.join("capabilities/build.yaml"),
            r#"
schema_version: 1
kind: capability
id: vac.build
title: Build
status: ready
states:
  empty: empty
  loading: loading
  success: success
  failure: failure
owner:
  crate: vac-surface-cli
  module: main
surfaces:
  palette: true
policy:
  risk: safe_read
  mutates_files: false
  network: false
  redaction: false
validation:
  commands:
    - cargo +1.93.0 check -p vac-surface-cli
"#,
        )
        .expect("capability manifest");
        let ownership_report =
            crate::control_plane::ownership_scan::load_ownership_scan_report(tempdir.path());
        assert_eq!(
            ownership_report.ready_unowned_count(),
            1,
            "{ownership_report:#?}"
        );
        let manifest = minimal_manifest(vec![WorkflowStep {
            id: "ownership".to_string(),
            uses: "capability.ownership.scan".to_string(),
            when: None,
            policy: None,
            ui: None,
        }]);

        let execution = WorkflowExecutionMachine::new(&manifest)
            .with_ownership_scan_report(ownership_report)
            .run();

        assert_eq!(execution.started_step_count(), 1);
        assert_eq!(execution.completed_step_count(), 0);
        assert_eq!(execution.blocked_step_count(), 1);
        assert!(
            execution
                .render_text()
                .contains("ownership scan found 1 ready capability without ownership metadata"),
            "rendered:\n{}",
            execution.render_text()
        );
    }

    #[test]
    fn execution_waits_for_approval_before_build_cargo_check() {
        use tempfile::tempdir;

        let tempdir = tempdir().expect("tempdir");
        let vac_root = tempdir.path().join(".vac");
        let manifest = minimal_manifest(vec![WorkflowStep {
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
        let approval_policy = evaluate_workflow_approval_policy(&manifest, &registry);
        let execution = WorkflowExecutionMachine::new(&manifest)
            .with_approval_checks(approval_policy.checks().to_vec())
            .run();

        assert_eq!(execution.started_step_count(), 1);
        assert_eq!(execution.completed_step_count(), 0);
        assert_eq!(execution.waiting_approval_step_count(), 1);
        assert_eq!(execution.blocked_step_count(), 0);
        assert!(
            execution.render_text().contains("waiting_approval"),
            "rendered:
{}",
            execution.render_text()
        );
        assert!(execution.render_text().contains("approval requests:"));
    }

    #[test]
    fn execution_persists_approval_request_to_file_store() {
        use crate::control_plane::FileApprovalStore;
        use tempfile::tempdir;

        let tempdir = tempdir().expect("tempdir");
        let vac_root = tempdir.path().join(".vac");
        let approval_path = tempdir.path().join("approvals.json");
        let manifest = minimal_manifest(vec![WorkflowStep {
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
        let approval_policy = evaluate_workflow_approval_policy(&manifest, &registry);
        let file_store = FileApprovalStore::open(&approval_path).expect("file approval store");

        let execution = WorkflowExecutionMachine::new(&manifest)
            .with_approval_checks(approval_policy.checks().to_vec())
            .with_persistent_approval_store(file_store)
            .run();

        assert_eq!(execution.waiting_approval_step_count(), 1);
        let reopened = FileApprovalStore::open(&approval_path).expect("reopen approval store");
        assert_eq!(reopened.pending_count(), 1);
        let id = &execution.approval_requests()[0].id;
        let stored = reopened.get(id).expect("persisted approval request");
        assert_eq!(stored.status, ApprovalStatus::Pending);
        assert_eq!(stored.workflow_id, manifest.id);
    }

    #[test]
    fn execution_reports_failure_for_unsupported_step() {
        let manifest = minimal_manifest(vec![WorkflowStep {
            id: "orient".to_string(),
            uses: "capability.workflow".to_string(),
            when: None,
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
    - cargo +1.93.0 check -p vac-surface-cli
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
    - cargo +1.93.0 check -p vac-surface-cli
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
    - cargo +1.93.0 check -p vac-surface-cli
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
    - cargo +1.93.0 check -p vac-surface-cli
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
    - cargo +1.93.0 check -p vac-surface-cli
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
                "cargo +1.93.0 check --manifest-path vac-rs/Cargo.toml -p vac-surface-cli".to_string(),
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
                "cargo +1.93.0 check --manifest-path vac-rs/Cargo.toml -p vac-surface-cli".to_string(),
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
            invoked.contains("+1.93.0 check --manifest-path vac-rs/Cargo.toml -p vac-surface-cli"),
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
                Some(entry.canonical_capability_id.to_string()),
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
                "cargo +1.93.0 check --manifest-path vac-rs/Cargo.toml -p vac-surface-cli".to_string(),
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
