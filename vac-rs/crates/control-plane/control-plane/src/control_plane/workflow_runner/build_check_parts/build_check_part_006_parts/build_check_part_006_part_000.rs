#[cfg(test)]
mod tests {
    use super::INITIAL_ALLOWED_STEP_USES;
    use super::TuiPtyGateResult;
    use super::TuiPtyGateResultState;
    use super::WORKFLOW_STEP_VOCABULARY;
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
            Some("vac.donor_migration")
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
