# VAC workflow control plane split implementation plans

This directory splits the workflow control plane migration into execution-sized plans.

ADR-0007 adds a mandatory pre-control-plane gate: the local product path must use Local Runtime Contract before `.vac` registry implementation begins.

Plan quality standard: [Production-grade plan quality rubric](PLAN_QUALITY_RUBRIC.md).

## Execution order

### Pre-control-plane gate

- [Plan 00 — Operating contract and architecture invariants](00-operating-contract.md)
- [Plan 00A — Build unblock before control plane](00A-build-unblock.md)
- [Plan 00B — Local Runtime Contract implementation](00B-local-runtime-contract.md)
- [Plan 00C — Rewire `vac exec` to Local Runtime Contract](00C-rewire-vac-exec.md)
- [Plan 00D — Rewire root `vac` TUI path to Local Runtime Contract](00D-rewire-vac-tui.md)
- [Plan 00E — Old runtime reachability and delete gate](00E-runtime-reachability-delete-gate.md)
- [Plan 00F — TUI legacy transport retirement](00F-tui-legacy-transport-retirement.md)
- [Phase 00 closeout status](../../migration/PHASE00_CLOSEOUT_STATUS.md)

### Control-plane implementation

- [Plan 01 — Create `.vac` control plane skeleton](01-repo-layout-skeleton.md)
- [Plan 02 — Capability manifest schema](02-capability-schema.md)
- [Plan 03 — Workflow manifest schema](03-workflow-schema.md)
- [Plan 04 — Policy manifest schema](04-policy-schema.md)
- [Plan 05 — Surface manifest schema](05-surface-schema.md)
- [Plan 06 — Rust registry loader](06-registry-loader.md)
- [Plan 07 — Registry diagnostics and error UX](07-registry-diagnostics.md)
- [Plan 08 — Initial root capability and workflow manifests](08-initial-root-manifests.md)
- [Plan 09 — TUI capability dashboard](09-capability-dashboard.md)
- [Plan 10 — TUI workflow browser](10-workflow-browser.md)
- [Plan 11 — Slash, palette, CLI surface convergence](11-slash-palette-convergence.md) — complete for route readiness and shared surface registry — complete with ready route-status sync
- [Plan 12 — Minimal safe workflow runner](12-safe-workflow-runner.md)
- [Plan 13 — Workflow progress and lifecycle in TUI](13-workflow-progress-tui.md)
- [Plan 14 — Approval and policy integration](14-approval-policy-integration.md) — complete for approval readiness
- [Plan 15 — Identity check maintenance workflow](15-maintenance-identity-check.md) — complete for identity-check readiness
- [Plan 16 — Build check maintenance workflow](16-maintenance-build-check.md) — complete for targeted build readiness
- [Plan 17 — No duplicate TUI maintenance workflow](17-maintenance-no-duplicate-tui.md) — complete for duplicate TUI prevention gate
- [Plan 18 — Release gate workflow](18-release-gate.md) — complete for release-readiness evidence aggregation
- [Plan 19 — Convert root features into control-plane manifests](19-root-feature-conversion.md) — complete for root capability/status synchronization
- [Plan 20 — Donor-backed capability gate](20-donor-gate.md) — complete for donor safety gate readiness
- [Plan 21 — Dead code and ownership enforcement](21-dead-code-and-ownership.md) — complete for ownership scan gate readiness
- [Plan 22 — Enterprise hygiene and documentation alignment](22-enterprise-hygiene.md) — complete for domain/status sync
- [Plan 23 — PTY operator gate as workflow capability](23-pty-operator-gate.md) — complete for BLOCKED-OPERATOR non-pass release semantics
- [Plan 24 — Local runtime owner replacement](24-local-runtime-owner-replacement.md) — complete umbrella for Plans 25–34 default path
- [Plan 25 — Local runtime semantic contract hardening](25-local-runtime-semantic-contract-hardening.md) — complete for semantic contract boundary hardening
- [Plan 26 — Local runtime owner skeleton](26-local-runtime-owner-skeleton.md) — complete for owner skeleton crate placement
- [Plan 27 — Retained resources and startup replacement](27-retained-resources-startup-replacement.md) — complete for default owner-native startup/retained-resource path
- [Plan 28 — Server-request registry replacement](28-server-request-registry-replacement.md) — complete for owner-native server-request registry path
- [Plan 29 — Event stream replacement](29-event-stream-replacement.md) — complete for default owner-native event stream path
- [Plan 30 — Prompt submit and active controls cutover](30-prompt-and-active-controls-cutover.md) — complete for default owner-native TUI session operation parity
- [Plan 31 — Protocol compatibility retirement](31-protocol-compatibility-retirement.md) — complete for default DTO owner-native path
- [Plan 32 — `.vac` runtime-owner gates](32-vac-runtime-owner-gates.md) — complete for default hard-gate threshold
- [Plan 33 — App-server Cargo retirement and delete/defer proof](33-app-server-cargo-retirement-delete-defer-proof.md) — complete for default product Cargo path, workspace crate deletion deferred
- [Plan 34 — Zero-config project workspace bootstrap](34-zero-config-project-workspace.md) — complete for core classifier, CLI/TUI prompt baseline, rich confirmation dialog, and strict promotion UX.

## Global acceptance rule

A plan is done only when it preserves one product command, one product TUI, visible TUI/operator state, typed workflow/capability metadata, policy gates, and validation. Backend-only work does not count.

## Hard gate

Plan 01 is gated by the Phase 00 closeout status document.
See `docs/migration/PHASE00_CLOSEOUT_STATUS.md` for the current gate result
and deferred 00E classification.

## Optional post-closeout proposals

Plan 34 is indexed as complete for the default zero-config workspace UX baseline. Future work may polish UI rendering, but classifier, approved-bootstrap helper, CLI/TUI prompt baseline, rich confirmation dialog, and strict promotion semantics are implemented.


## 2026-05-28 sandbox registry/surface closeout

Registry status is `ready`; all root capability/workflow/domain manifests are ready; visible surface routes are ready for the default product registry. Historical Plan 31 mapping blockers are retained as evidence only, and Plan 32 threshold wording now reflects active hard-error gates.
