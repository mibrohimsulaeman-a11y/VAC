# VAC-Init Execution Plan

Status: Batch 0-1 implementation baseline  
Baseline source artifact: `vac-source-tui-operator-ui-hardening4-10.zip`  
Spec baseline: `VAC_Init_Control_Plane_Spec_v1_alpha_refined.docx`  
Owner: `vac-core/control_plane`

## Tujuan

VAC-Init mengubah repository biasa menjadi workspace agentic yang dikontrol oleh `.vac/` sebagai Workflow Control Plane. Implementasi harus bertahap, fail-closed, manifest-driven, dan setiap batch menghasilkan source artifact baru.

## Artifact Protocol

Artifact checkpoints are mandatory for every sandbox batch and are treated as commit-like source checkpoints.

## Execution Cadence

Setiap batch wajib menghasilkan:

- source zip;
- `.MANIFEST.txt`;
- `.validation.log`;
- `.zip.sha256`.

Setiap batch wajib mengecualikan:

- `target/`;
- `.git/`;
- compiled objects: `*.rlib`, `*.rmeta`, `*.o`, `*.d`;
- dependency cache/toolchain/vendor kecuali eksplisit diminta.

## Batch 0 — Baseline Audit & Spec Mapping

Target:

- petakan spec VAC-Init ke source tree aktual;
- tandai yang `implemented`, `partial`, `missing`, dan `compatibility`;
- tidak menyentuh command execution atau scanner berat;
- buat gate baseline ringan.

Deliverables:

- `docs/vac-init/VAC_INIT_EXECUTION_PLAN.md`;
- `docs/vac-init/VAC_INIT_IMPLEMENTATION_MAP.md`;
- `docs/validation/VAC_INIT_BASELINE_AUDIT.md`;
- `scripts/check-vac-init-baseline-contract.sh`.

Validation:

- docs exist;
- spec-to-source map contains phases 1-4;
- baseline gate delegates schema envelope contract;
- `.vac` YAML parse passes.

## Batch 1 — Schema Envelope + Kind Registry

Target:

- semua manifest `.vac/` memiliki `schema_version`, `kind`, dan `id` envelope;
- kind harus terdaftar di VAC-Init Kind Registry;
- ID memakai dotted identifier, dengan compatibility exception untuk current root product descriptor `id: vac`;
- implementasi dependency-free agar bisa divalidasi dengan `rustc --test` tanpa full workspace build.

Deliverables:

- `vac-rs/core/src/control_plane/schema_envelope.rs`;
- `vac-rs/core/src/control_plane/kind_registry.rs`;
- `scripts/check-vac-init-schema-envelope-contract.sh`;
- `.vac/capabilities/vac-init-schema-envelope.yaml`;
- `.vac/capabilities/vac-init-control-plane.yaml`;
- `.vac/workflows/maintenance.vac-init-baseline-audit.yaml`;
- `.vac/workflows/maintenance.vac-init-schema-envelope.yaml`.

Validation:

- `rustfmt --edition 2024 --check`;
- `scripts/check-vac-init-schema-envelope-contract.sh` direct `rustc --test` for `kind_registry.rs` and direct `rustc --cfg vac_standalone_schema_envelope --test` for `schema_envelope.rs`;
- Python `.vac` YAML parse;
- top-level envelope scan for `.vac/**/*.yaml`;
- no unknown kind outside registry;
- source hygiene gate.

## Batch 2 — Manifest Structs

Target:

- typed structs for capability, policy, workflow, and surface refinements;
- reconcile existing root manifest vocabulary with refined spec;
- add validation errors with stable codes.

Expected files:

- `vac-rs/core/src/control_plane/manifest_contract.rs`;
- `vac-rs/core/src/control_plane/validation_error.rs`;
- updates to existing `capability_manifest.rs`, `policy_manifest.rs`, `workflow_manifest.rs`, `surface_manifest.rs`.

Gate:

- serde roundtrip where crate-level test is feasible;
- targeted static contract checks otherwise.

## Batch 3 — Registry Validator + `vac doctor registry`

Target:

- registry validator consumes schema envelope + typed manifest readers;
- duplicate IDs rejected;
- fatal parse diagnostics render actionable error messages;
- CLI doctor path prepared.

Gate:

- invalid kind rejected;
- duplicate ID rejected;
- missing required field rejected;
- diagnostic panel non-blank.

## Batch 4 — `vac init` Lifecycle State Machine

Target:

- implement init states: `uninitialized`, `discovered`, `partition_selected`, `policy_inferred`, `manifests_synthesized`, `doctor_verified`, `ready`;
- implement failure states: `scan_failed`, `operator_cancelled`, `policy_conflict`, `ownership_missing`, `doctor_failed`;
- persist `.vac/.init/state.yaml`.

Gate:

- invalid transition rejected;
- resume from non-ready state;
- dry-run does not mutate.

## Batch 5 — Source Inventory + Ownership Scanner

Target:

- generate source inventory;
- map files to capability ownership targets;
- detect `complete`, `partial`, `hidden`, `overclaimed`, and `unowned`;
- produce quarantine actions.

Gate:

- unowned source detected;
- overclaimed source detected;
- generated/vendor/build outputs ignored.

## Batch 6 — Risk Scanner + Policy Inference

Target:

- scan risk-bearing patterns;
- output `risk_finding` records;
- infer default local policy without auto-allowing ambiguous risk.

Gate:

- process/network/filesystem/secrets patterns detected;
- confidence thresholds applied;
- ambiguous findings require operator review.

## Batch 7 — Fail-Closed Policy Evaluator

Target:

- decision precedence: `deny > approval_required > allow`;
- no policy loaded means blocked;
- multi-policy merge is most-restrictive-wins.

Gate:

- explicit deny cannot be overridden;
- approval cannot be downgraded;
- unmatched action follows `default_decision`.

## Batch 8 — Structured Command + Pre-Command Gate

Target:

- validation commands must be structured;
- free-form shell, pipes, redirection, wildcard expansion, and arbitrary executable paths are blocked unless represented and policy-gated.

Gate:

- `runner: cargo` + explicit args accepted;
- `bash -c`, pipes, redirects, and destructive shell rejected.

## Batch 9 — Semantic Plan Validator

Target:

- implement `kind: plan` validator;
- enforce capability, allowed files, ownership, patch bounds, forbidden actions, and structured validation commands.

Gate:

- unowned target file rejected;
- planned/deprecated capability rejected for execution;
- high-risk plan requires approval.

## Batch 10 — Approval Request + Replay Protection

Target:

- implement approval request contract;
- validate `plan_hash`, `diff_hash`, `policy_snapshot_hash`, `nonce`, and `expires_at`.

Gate:

- hash drift invalidates approval;
- expired approval rejected;
- nonce replay rejected.

## Batch 11 — Patch Guard / Bounded Patch Contract

Target:

- patch must target only `plan.allowed_files`;
- range/anchor must resolve;
- patch budget enforced;
- undeclared new file rejected.

Gate:

- file outside plan rejected;
- operation mismatch rejected;
- line delta exceeded rejected.

## Batch 12 — Evidence Canonical Hash Chain

Target:

- canonical YAML hashing;
- append-only evidence chain;
- broken chain detection.

Gate:

- `self_hash` excludes itself;
- previous hash checked;
- broken chain blocks completion.

## Batch 13 — `vac why` Safe Rationale Index

Target:

- safe rationale lookup by file/line/symbol;
- no raw/private chain-of-thought;
- evidence, policy, memory, approval references surfaced.

Gate:

- empty lookup returns diagnostic;
- depth limit works;
- raw CoT excluded.

## Batch 14 — Memory Governance Contract

Target:

- memory record schema;
- tier policy: working, episodic, semantic, team;
- credential-like content rejected.

Gate:

- TTL validated;
- size limit validated;
- team write approval-gated.

## Batch 15 — Doctor Aggregate Gates

Target:

- `vac doctor registry/surfaces/policy/ownership/workflow/evidence/build/memory/init/release` taxonomy;
- exit codes: `0`, `1`, `2`.

Gate:

- release gate blocks hard quarantine, failed registry, failed evidence.

## Batch 16 — TUI Integration Final Pass

Target:

- connect registry diagnostics, ownership report, policy diagnostics, approval requests, evidence state, and init lifecycle to existing operator TUI.

Gate:

- capability dashboard consumes real diagnostics;
- approval popup consumes approval request model;
- existing snapshot gates still pass.

## Batch 17 — Fixtures & Regression Matrix

Target:

- fixture matrix for schema, state machine, policy, workspace, evidence, plans, patches, doctor, memory, and trajectory.

Gate:

- positive and negative fixture per category;
- static fixture presence script.

## Batch 18 — Final Sandbox Release Gate

Target:

- aggregate VAC-Init gates;
- produce `vac-source-vac-init-control-plane-complete.zip`.

Gate:

- all targeted Rust tests pass;
- YAML parse pass;
- all VAC-Init static contract checks pass;
- TUI hardening regression lock pass;
- source hygiene pass.


## Sandbox build posture

Full workspace build/link is intentionally not used as a required gate for this batch; validation stays on targeted `rustc --test`, YAML parse, static contracts, and source hygiene.

## Artifact Discipline

Artifact checkpoints are treated as commit-like source snapshots. Each completed slice must create a source zip, manifest, validation log, and SHA256 checksum, while excluding `target/`, `.git/`, compiled objects, dependency cache, and transient build output.

## Batch 2-5 Checkpoint Status

The Batch 2-5 checkpoint implements the first executable core contracts after the schema envelope foundation:

1. typed manifest contract models;
2. registry validator diagnostics;
3. init lifecycle state machine;
4. source inventory and ownership scanner.

The next recommended implementation checkpoint is Batch 6-8:

- Risk scanner + policy inference;
- fail-closed policy evaluator + multi-policy merge;
- structured command model + pre-command gate.
