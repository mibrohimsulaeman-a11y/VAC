# Uncommitted Work Batch Plan — 2026-05-24

Generated from the current dirty working tree. This file is a review aid only; no files were staged or committed.

## Guardrails
- Avoid destructive cleanup commands while this tree contains active untracked implementation work.
- Keep donor source quarantined; donor deletions should be reviewed as donor-only cleanup, not product runtime removal.
- Validate each batch before commit; prefer targeted checks and reuse the existing debug binary when possible.
- Run `df -h . vac-rs/target` before build/test work and avoid workspace-wide Cargo unless necessary.

## Summary

| Bucket | Files | Modified | Deleted | Untracked |
|---|---:|---:|---:|---:|
| `stage1-warning-cleanup` | 3 | 3 | 0 | 0 |
| `control-plane-manifests` | 33 | 27 | 0 | 6 |
| `vac-core-control-plane` | 38 | 34 | 0 | 4 |
| `local-runtime-owner` | 4 | 3 | 0 | 1 |
| `vac-tui` | 34 | 11 | 0 | 23 |
| `rust-workspace-support` | 12 | 12 | 0 | 0 |
| `donor-migration-quarantine` | 49 | 16 | 33 | 0 |
| `workflow-control-plane-docs` | 19 | 17 | 0 | 2 |
| `product-architecture-docs` | 17 | 16 | 0 | 1 |
| `scheduled-audits` | 1 | 0 | 0 | 1 |
| `scheduled-plans` | 1 | 0 | 0 | 1 |
| `scripts` | 3 | 0 | 0 | 3 |
| `misc` | 1 | 1 | 0 | 0 |

## Recommended commit sequence

### Batch 1: `stage1-warning-cleanup`
- Goal: Small warning cleanup applied in this session.
- Candidate commit: `test(plan-maint): remove stale warning-only imports`.
- Validation: `cargo test --manifest-path vac-rs/Cargo.toml -p vac-core registry_diagnostics -- --nocapture` already passed after cleanup.
- Files:
  - `M` `vac-rs/core/src/control_plane/policy_registry_tests.rs`
  - `M` `vac-rs/core/tests/suite/approvals.rs`
  - `M` `vac-rs/core/tests/suite/hooks.rs`

### Batch 2: `control-plane-manifests`
- Goal: Manifest/schema/surface/workflow alignment.
- Candidate commit: `feat(plan19): align control-plane manifests and routes`.
- Validation: `./vac-rs/target/debug/vac doctor registry .`, `surfaces`, and `policy`.
- Files:
  - `M` `.vac/README.md`
  - `M` `.vac/capabilities/README.md`
  - `M` `.vac/capabilities/approvals.yaml`
  - `M` `.vac/capabilities/architecture.yaml`
  - `M` `.vac/capabilities/build.yaml`
  - `M` `.vac/capabilities/chat.yaml`
  - `M` `.vac/capabilities/donor_migration.yaml`
  - `M` `.vac/capabilities/identity-check.yaml`
  - `M` `.vac/capabilities/identity.yaml`
  - `??` `.vac/capabilities/local_runtime_owner.yaml`
  - `M` `.vac/capabilities/ownership.yaml`
  - `M` `.vac/capabilities/release.yaml`
  - `??` `.vac/capabilities/runtime_approval_bridge.yaml`
  - `M` `.vac/capabilities/sandbox.yaml`
  - `M` `.vac/capabilities/sessions.yaml`
  - `M` `.vac/capabilities/tools.yaml`
  - `??` `.vac/capabilities/tui-pty-gate.yaml`
  - `M` `.vac/capabilities/tui.yaml`
  - `??` `.vac/capabilities/tui_session_runtime.yaml`
  - `M` `.vac/capabilities/workflow.yaml`
  - `M` `.vac/policies/README.md`
  - `??` `.vac/registry/README.md`
  - `M` `.vac/registry/domains.yaml`
  - `M` `.vac/registry/status.yaml`
  - `M` `.vac/surfaces/README.md`
  - `M` `.vac/surfaces/cli.yaml`
  - `M` `.vac/surfaces/palette.yaml`
  - `M` `.vac/surfaces/slash.yaml`
  - `M` `.vac/surfaces/tui.yaml`
  - `M` `.vac/workflows/README.md`
  - `M` `.vac/workflows/maintenance.no-duplicate-tui.yaml`
  - `M` `.vac/workflows/maintenance.release-gate.yaml`
  - `??` `.vac/workflows/maintenance.tui-pty-gate.yaml`

### Batch 3: `vac-core-control-plane`
- Goal: Control-plane Rust runtime and registry/doctor tests.
- Candidate commit: `feat(plan19): harden control-plane diagnostics`.
- Validation: Targeted `vac-core` tests for changed modules plus `vac doctor registry .`.
- Files:
  - `M` `vac-rs/core/config.schema.json`
  - `M` `vac-rs/core/src/config/config_loader_tests.rs`
  - `M` `vac-rs/core/src/config/config_tests.rs`
  - `M` `vac-rs/core/src/control_plane/approval_lifecycle.rs`
  - `??` `vac-rs/core/src/control_plane/approval_store.rs`
  - `M` `vac-rs/core/src/control_plane/architecture_invariants.rs`
  - `??` `vac-rs/core/src/control_plane/build_check.rs`
  - `M` `vac-rs/core/src/control_plane/capability_manifest.rs`
  - `M` `vac-rs/core/src/control_plane/capability_manifest_tests.rs`
  - `M` `vac-rs/core/src/control_plane/capability_registry_tests.rs`
  - `M` `vac-rs/core/src/control_plane/docs_maintenance.rs`
  - `??` `vac-rs/core/src/control_plane/donor_status.rs`
  - `M` `vac-rs/core/src/control_plane/identity_check.rs`
  - `M` `vac-rs/core/src/control_plane/mod.rs`
  - `M` `vac-rs/core/src/control_plane/no_duplicate_tui.rs`
  - `M` `vac-rs/core/src/control_plane/ownership_scan.rs`
  - `M` `vac-rs/core/src/control_plane/ownership_scan_tests.rs`
  - `M` `vac-rs/core/src/control_plane/policy_manifest.rs`
  - `M` `vac-rs/core/src/control_plane/policy_manifest_tests.rs`
  - `M` `vac-rs/core/src/control_plane/registry.rs`
  - `M` `vac-rs/core/src/control_plane/registry_diagnostics.rs`
  - `M` `vac-rs/core/src/control_plane/registry_diagnostics_tests.rs`
  - `??` `vac-rs/core/src/control_plane/registry_tests.rs`
  - `M` `vac-rs/core/src/control_plane/root_feature_catalog.rs`
  - `M` `vac-rs/core/src/control_plane/root_feature_conversion.rs`
  - `M` `vac-rs/core/src/control_plane/surface_doctor.rs`
  - `M` `vac-rs/core/src/control_plane/surface_doctor_tests.rs`
  - `M` `vac-rs/core/src/control_plane/surface_manifest.rs`
  - `M` `vac-rs/core/src/control_plane/surface_manifest_tests.rs`
  - `M` `vac-rs/core/src/control_plane/surface_registry.rs`
  - `M` `vac-rs/core/src/control_plane/surface_registry_tests.rs`
  - `M` `vac-rs/core/src/control_plane/workflow_manifest.rs`
  - `M` `vac-rs/core/src/control_plane/workflow_manifest_tests.rs`
  - `M` `vac-rs/core/src/control_plane/workflow_registry.rs`
  - `M` `vac-rs/core/src/control_plane/workflow_registry_tests.rs`
  - `M` `vac-rs/core/src/control_plane/workflow_runner.rs`
  - `M` `vac-rs/core/tests/common/lib.rs`
  - `M` `vac-rs/core/tests/common/test_vac.rs`

### Batch 4: `local-runtime-owner`
- Goal: Local runtime owner crate and TUI bootstrap seam.
- Candidate commit: `feat(plan27): add local runtime owner skeleton`.
- Validation: `cargo test --manifest-path vac-rs/Cargo.toml -p vac-local-runtime-owner` and a narrow TUI compile when disk allows.
- Files:
  - `M` `vac-rs/core/src/local_runtime/projection.rs`
  - `??` `vac-rs/local-runtime-owner/`
  - `M` `vac-rs/tui/src/app_server_session.rs`
  - `M` `vac-rs/tui/src/local_runtime_session.rs`

### Batch 5: `vac-tui`
- Goal: TUI runtime, dashboard, browser, and snapshot updates.
- Candidate commit: `feat(plan00f): route TUI through local runtime seams`.
- Validation: Focused TUI tests/snapshots, then `cargo build --manifest-path vac-rs/Cargo.toml -p vac-surface-cli` when disk allows.
- Files:
  - `M` `vac-rs/tui/Cargo.toml`
  - `M` `vac-rs/tui/src/bottom_pane/chat_composer.rs`
  - `M` `vac-rs/tui/src/bottom_pane/mod.rs`
  - `M` `vac-rs/tui/src/bottom_pane/slash_commands.rs`
  - `??` `vac-rs/tui/src/bottom_pane/snapshots/vac_tui__bottom_pane__chat_composer__tests__slash_popup_mo.snap.new`
  - `??` `vac-rs/tui/src/bottom_pane/snapshots/vac_tui__bottom_pane__chat_composer__tests__slash_popup_res.snap.new`
  - `M` `vac-rs/tui/src/capability_dashboard.rs`
  - `M` `vac-rs/tui/src/chatwidget.rs`
  - `M` `vac-rs/tui/src/chatwidget/slash_dispatch.rs`
  - `??` `vac-rs/tui/src/chatwidget/snapshots/vac_tui__chatwidget__tests__app_server_guardian_review_denied_renders_denied_request.snap.new`
  - `??` `vac-rs/tui/src/chatwidget/snapshots/vac_tui__chatwidget__tests__app_server_guardian_review_timed_out_renders_timed_out_request.snap.new`
  - `??` `vac-rs/tui/src/chatwidget/snapshots/vac_tui__chatwidget__tests__app_server_mcp_startup_failure_renders_warning_history.snap.new`
  - `??` `vac-rs/tui/src/chatwidget/snapshots/vac_tui__chatwidget__tests__chatwidget_tall.snap.new`
  - `??` `vac-rs/tui/src/chatwidget/snapshots/vac_tui__chatwidget__tests__compact_queues_user_messages_snapshot.snap.new`
  - `??` `vac-rs/tui/src/chatwidget/snapshots/vac_tui__chatwidget__tests__guardian_approved_exec_renders_approved_request.snap.new`
  - `??` `vac-rs/tui/src/chatwidget/snapshots/vac_tui__chatwidget__tests__guardian_approved_request_permissions_renders_request_summary.snap.new`
  - `??` `vac-rs/tui/src/chatwidget/snapshots/vac_tui__chatwidget__tests__guardian_denied_exec_renders_warning_and_denied_request.snap.new`
  - `??` `vac-rs/tui/src/chatwidget/snapshots/vac_tui__chatwidget__tests__guardian_parallel_reviews_render_aggregate_status.snap.new`
  - `??` `vac-rs/tui/src/chatwidget/snapshots/vac_tui__chatwidget__tests__guardian_timed_out_exec_renders_warning_and_timed_out_request.snap.new`
  - `??` `vac-rs/tui/src/chatwidget/snapshots/vac_tui__chatwidget__tests__mcp_startup_header_booting.snap.new`
  - `??` `vac-rs/tui/src/chatwidget/snapshots/vac_tui__chatwidget__tests__preamble_keeps_working_status.snap.new`
  - `??` `vac-rs/tui/src/chatwidget/snapshots/vac_tui__chatwidget__tests__renamed_thread_footer_title.snap.new`
  - `??` `vac-rs/tui/src/chatwidget/snapshots/vac_tui__chatwidget__tests__review_queues_user_messages_snapshot.snap.new`
  - `??` `vac-rs/tui/src/chatwidget/snapshots/vac_tui__chatwidget__tests__side_context_label_shows_parent_status.snap.new`
  - `??` `vac-rs/tui/src/chatwidget/snapshots/vac_tui__chatwidget__tests__status_line_model_with_reasoning_context_remaining_footer.snap.new`
  - `??` `vac-rs/tui/src/chatwidget/snapshots/vac_tui__chatwidget__tests__status_line_model_with_reasoning_fast_footer.snap.new`
  - `??` `vac-rs/tui/src/chatwidget/snapshots/vac_tui__chatwidget__tests__status_line_model_with_reasoning_plan_mode_footer.snap.new`
  - `??` `vac-rs/tui/src/chatwidget/snapshots/vac_tui__chatwidget__tests__status_widget_active.snap.new`
  - `??` `vac-rs/tui/src/chatwidget/snapshots/vac_tui__chatwidget__tests__unified_exec_begin_restores_working_status.snap.new`
  - `??` `vac-rs/tui/src/chatwidget/snapshots/vac_tui__chatwidget__tests__unified_exec_wait_status_renders_command_in_single_details_row.snap.new`
  - `M` `vac-rs/tui/src/slash_command.rs`
  - `M` `vac-rs/tui/src/surface_route_catalog.rs`
  - `M` `vac-rs/tui/src/workflow_browser.rs`
  - `M` `vac-rs/tui/src/workflow_progress.rs`

### Batch 6: `rust-workspace-support`
- Goal: Cargo/workspace/protocol/exec/config support changes.
- Candidate commit: `chore(plan-maint): update workspace support wiring`.
- Validation: Narrow Cargo/check command matching the affected crate.
- Files:
  - `M` `vac-rs/.cargo/config.toml`
  - `M` `vac-rs/Cargo.lock`
  - `M` `vac-rs/Cargo.toml`
  - `M` `vac-rs/README.md`
  - `M` `vac-rs/cli/src/doctor_cli.rs`
  - `M` `vac-rs/config/src/config_requirements.rs`
  - `M` `vac-rs/config/src/loader/mod.rs`
  - `M` `vac-rs/exec/src/lib.rs`
  - `M` `vac-rs/exec/src/runtime_adapter.rs`
  - `M` `vac-rs/exec/tests/all.rs`
  - `M` `vac-rs/protocol/src/lib.rs`
  - `M` `vac-rs/protocol/src/protocol.rs`

### Batch 7: `donor-migration-quarantine`
- Goal: Donor quarantine deletions and donor tracking docs/scripts.
- Candidate commit: `docs(plan20): refresh donor quarantine tracking`.
- Validation: `bash scripts/check-donor-status.sh all`.
- Files:
  - `M` `docs/donor-migration/DONOR_COMMIT_POLICY.md`
  - `M` `docs/donor-migration/DONOR_INVENTORY_MATRIX.md`
  - `M` `docs/donor-migration/DONOR_STATUS_BOARD.md`
  - `M` `docs/donor-migration/INDEX.md`
  - `M` `docs/donor-migration/domain-plans/01-session-engine.md`
  - `M` `docs/donor-migration/domain-plans/02-tool-contract.md`
  - `M` `docs/donor-migration/domain-plans/03-managed-connectors.md`
  - `M` `docs/donor-migration/domain-plans/04-changeset-evidence.md`
  - `M` `docs/donor-migration/domain-plans/05-trust-redaction.md`
  - `M` `docs/donor-migration/domain-plans/06-context-rag-memory.md`
  - `M` `docs/donor-migration/domain-plans/07-vil-native.md`
  - `M` `docs/donor-migration/domain-plans/08-trace-signal-trajectory.md`
  - `M` `docs/donor-migration/domain-plans/09-agent-orchestration.md`
  - `M` `docs/donor-migration/domain-plans/10-tui-concept-extraction.md`
  - `M` `docs/donor-migration/domain-plans/INDEX.md`
  - `D` `donor/vac/.github/workflows/bench.yml`
  - `D` `donor/vac/.github/workflows/ci.yml`
  - `D` `donor/vac/.github/workflows/clippy-strict.yml`
  - `D` `donor/vac/.github/workflows/codeql.yml`
  - `D` `donor/vac/.github/workflows/coverage.yml`
  - `D` `donor/vac/.github/workflows/dependency-policy.yml`
  - `D` `donor/vac/.github/workflows/doc-links.yml`
  - `D` `donor/vac/.github/workflows/fuzz-weekly.yml`
  - `D` `donor/vac/.github/workflows/mutation-weekly.yml`
  - `D` `donor/vac/.github/workflows/release-dry-run.yml`
  - `D` `donor/vac/.github/workflows/release-smoke.yml`
  - `D` `donor/vac/.github/workflows/release.yml`
  - `D` `donor/vac/.github/workflows/security-audit.yml`
  - `D` `donor/vac/.github/workflows/workflow-validate.yml`
  - `D` `donor/vac/.kilocode/task_history.json`
  - `D` `donor/vac/.trae/specs/c-track-codex-parity/checklist.md`
  - `D` `donor/vac/.trae/specs/c-track-codex-parity/spec.md`
  - `D` `donor/vac/.trae/specs/c-track-codex-parity/tasks.md`
  - `D` `donor/vac/.trae/specs/eksekusi-vac-ultraplan-superbatch/checklist.md`
  - `D` `donor/vac/.trae/specs/eksekusi-vac-ultraplan-superbatch/spec.md`
  - `D` `donor/vac/.trae/specs/eksekusi-vac-ultraplan-superbatch/tasks.md`
  - `D` `donor/vac/.trae/specs/execute-vac-hourly-code-review-1700/checklist.md`
  - `D` `donor/vac/.trae/specs/execute-vac-hourly-code-review-1700/spec.md`
  - `D` `donor/vac/.trae/specs/execute-vac-hourly-code-review-1700/tasks.md`
  - `D` `donor/vac/.trae/specs/fix-semantic-chunker-defects/checklist.md`
  - `D` `donor/vac/.trae/specs/fix-semantic-chunker-defects/spec.md`
  - `D` `donor/vac/.trae/specs/fix-semantic-chunker-defects/tasks.md`
  - `D` `donor/vac/.trae/specs/implement-vac-tui-hardening-masterplan/checklist.md`
  - `D` `donor/vac/.trae/specs/implement-vac-tui-hardening-masterplan/spec.md`
  - `D` `donor/vac/.trae/specs/implement-vac-tui-hardening-masterplan/tasks.md`
  - `D` `donor/vac/.trae/specs/integrasikan-ollama-provider-lokal/checklist.md`
  - `D` `donor/vac/.trae/specs/integrasikan-ollama-provider-lokal/spec.md`
  - `D` `donor/vac/.trae/specs/integrasikan-ollama-provider-lokal/tasks.md`
  - `M` `scripts/check-donor-status.sh`

### Batch 8: `workflow-control-plane-docs`
- Goal: Plan docs and workflow-control-plane docs sync.
- Candidate commit: `docs(plan-maint): sync workflow control-plane plans`.
- Validation: `./vac-rs/target/debug/vac doctor docs .`.
- Files:
  - `M` `docs/workflow-control-plane/IMPLEMENTATION_PLAN.md`
  - `M` `docs/workflow-control-plane/INDEX.md`
  - `M` `docs/workflow-control-plane/INITIAL_MANIFEST_SET.md`
  - `M` `docs/workflow-control-plane/plans/12-safe-workflow-runner.md`
  - `M` `docs/workflow-control-plane/plans/15-maintenance-identity-check.md`
  - `M` `docs/workflow-control-plane/plans/18-release-gate.md`
  - `M` `docs/workflow-control-plane/plans/19-root-feature-conversion.md`
  - `M` `docs/workflow-control-plane/plans/20-donor-gate.md`
  - `M` `docs/workflow-control-plane/plans/23-pty-operator-gate.md`
  - `M` `docs/workflow-control-plane/plans/24-local-runtime-owner-replacement.md`
  - `M` `docs/workflow-control-plane/plans/25-local-runtime-semantic-contract-hardening.md`
  - `M` `docs/workflow-control-plane/plans/26-local-runtime-owner-skeleton.md`
  - `M` `docs/workflow-control-plane/plans/27-retained-resources-startup-replacement.md`
  - `M` `docs/workflow-control-plane/plans/28-server-request-registry-replacement.md`
  - `M` `docs/workflow-control-plane/plans/29-event-stream-replacement.md`
  - `??` `docs/workflow-control-plane/plans/33-evidence/`
  - `??` `docs/workflow-control-plane/plans/34-zero-config-project-workspace.md`
  - `M` `docs/workflow-control-plane/plans/INDEX.md`
  - `M` `docs/workflow-control-plane/schema/workflow-manifest.schema.md`

### Batch 9: `product-architecture-docs`
- Goal: Product, architecture, validation, executor prompt, and root docs.
- Candidate commit: `docs(plan-maint): sync product architecture docs`.
- Validation: `./vac-rs/target/debug/vac doctor docs .` and relevant architecture doctor.
- Files:
  - `M` `AGENTS.md`
  - `M` `README.md`
  - `M` `docs/DOCS_AUDIT.md`
  - `M` `docs/architecture/control-plane.md`
  - `M` `docs/architecture/decisions/ADR-0007-local-runtime-contract.md`
  - `M` `docs/architecture/local-runtime-contract.md`
  - `M` `docs/executor-prompts/00D-rewire-vac-tui.md`
  - `M` `docs/executor-prompts/INDEX.md`
  - `??` `docs/legal/`
  - `M` `docs/product/CAPABILITY_MAP.md`
  - `M` `docs/product/CAPABILITY_PRD_COVERAGE.md`
  - `M` `docs/product/domain-prds/workflow-control-plane.md`
  - `M` `docs/product/requirements-matrix.md`
  - `M` `docs/product/roadmap.md`
  - `M` `docs/validation/LOCAL_RUNTIME_GATE.md`
  - `M` `docs/validation/RELEASE_GATE.md`
  - `M` `docs/validation/TUI_PTY_DOGFOOD_GATE.md`

### Batch 10: `scheduled-audits`
- Goal: Scheduled audit snapshots/index.
- Candidate commit: `docs(plan-maint): add scheduled audit snapshots`.
- Validation: `scripts/regenerate-audit-index.sh` plus docs doctor.
- Files:
  - `??` `docs/scheduled-audits/`

### Batch 11: `scheduled-plans`
- Goal: Scheduled plan artifacts.
- Candidate commit: `docs(plan-maint): add scheduled plan artifacts`.
- Validation: Docs review plus docs doctor.
- Files:
  - `??` `docs/scheduled-plans/`

### Batch 12: `scripts`
- Goal: Standalone script changes not covered above.
- Candidate commit: `chore(plan-maint): add audit helper scripts`.
- Validation: Run each touched script in its narrow check mode.
- Files:
  - `??` `scripts/audit-quick-checks.sh`
  - `??` `scripts/capture-app-server-reachability-evidence.sh`
  - `??` `scripts/regenerate-audit-index.sh`

### Batch 13: `misc`
- Goal: Miscellaneous leftovers that need manual review.
- Candidate commit: decide after manual review.
- Validation: Manual review, then narrowest applicable validation.
- Files:
  - `M` `vac-rs/core/src/session/session.rs`

