# Plan 34 — Zero-config project workspace bootstrap


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **COMPLETE**

Source status lines:
> ## Current status
> Status: **complete for core zero-config workspace contract / rich confirmation + strict promotion UX landed** — sandbox inventory evidence is recorded in `34-evidence/2026-05-27-sandbox-zero-config-inventory.md`; classifier/doctor, soft bootstrap, CLI/TUI first-run warning, rich confirmation dialog model, and strict promotion preview/materialization are now implemented with targeted evidence.

Code evidence:
- `vac-rs/core/src/project_workspace.rs`
- `vac-rs/cli/src/doctor_cli.rs`

Evidence docs:
- `docs/workflow-control-plane/plans/34-evidence/2026-05-27-sandbox-bootstrap-core.md`
- `docs/workflow-control-plane/plans/34-evidence/2026-05-27-sandbox-cli-tui-cutover.md`
- `docs/workflow-control-plane/plans/34-evidence/2026-05-27-sandbox-rich-confirmation-strict-promotion.md`
- `docs/workflow-control-plane/plans/34-evidence/2026-05-27-sandbox-tui-confirmation-strict-promotion-closeout.md`
- `docs/workflow-control-plane/plans/34-evidence/2026-05-27-sandbox-zero-config-inventory.md`
- `docs/workflow-control-plane/plans/34-evidence/2026-05-28-sandbox-blocker-closeout-sync.md`

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Current status

Status: **complete for core zero-config workspace contract / rich confirmation + strict promotion UX landed** — sandbox inventory evidence is recorded in `34-evidence/2026-05-27-sandbox-zero-config-inventory.md`; classifier/doctor, soft bootstrap, CLI/TUI first-run warning, rich confirmation dialog model, and strict promotion preview/materialization are now implemented with targeted evidence.

Plan 34 began as a product/architecture proposal. The current implementation now covers Rust-owned classification, a doctor-visible diagnostic surface, an approval-required soft bootstrap core helper, first-run CLI/TUI startup notices, a rich confirmation dialog model, and strict promotion preview/materialization. The remaining future work is polish/integration breadth, not the core Plan 34 contract.

Zero-config runtime detection decisions are now codified in Rust-owned classifier/bootstrap/promotion helpers. Future work may add more UI polish and additional inference signals, but the current contract already separates arbitrary user-project warnings from strict VAC product-repo gates.

This plan is intentionally separate from the strict `.vac` control plane used by the VAC product repository itself. For arbitrary repositories, missing `.vac` must not block ordinary prompt submission. VAC should still let the user ask questions, inspect code, propose edits, and request approved actions with conservative defaults.

`.vac` remains declarative. It may store reviewed project profile, memory, policies, workflows, and evidence boundaries, but runtime decisions, fallback behavior, inference, validation execution, and safety enforcement stay in Rust.

## Goal

Make `vac` usable in arbitrary existing repositories without requiring a pre-existing `.vac` manifest control plane.

A user can run:

```text
cd existing-project
vac
```

and immediately start working. On first run, VAC should either:

- continue in an **in-memory** conservative profile when no file creation is approved,
- offer a **soft** `.vac` project workspace with local-only boundaries and inferred hints,
- later allow explicit promotion to **curated** project memory/profile, and
- only after review, allow promotion to **strict** manifests.

The first-run experience should be safe, understandable, and reversible. The absence of `.vac` is a warning/setup state, not a fatal error for ordinary assistance.

## Non-goals

- No automatic `.vac` creation without explicit approval.
- No claim that curated memory setup is complete.
- No forced migration for existing projects.
- No strict capability/policy/workflow/surface YAML requirement on first run.
- No hidden memory writes or unreviewed durable project facts.
- No local-only DB, index, session, artifact, log, cache, or tmp files committed by default.
- No runtime business logic in `.vac`; `.vac` remains declarative.
- No second TUI, external agent runtime clone, or donor runtime port.
- No weakening of strict `.vac` gates for the VAC product repository.
- No silent conversion from soft project profile to strict control-plane manifests.

## Resolved prerequisites

The original pre-implementation inventory is now recorded as implementation evidence:

- Plan 31/33 default-path retirement/delete-defer chain is closed or explicitly deferred for non-default compatibility.
- VAC root, workspace root, and missing `.vac` detection are represented by the Rust project-workspace classifier.
- Zero-config inference signals are surfaced as warnings/setup preview, not hidden durable facts.
- CLI/TUI first-run flows use startup notices and approval-gated bootstrap helpers.
- Doctor/registry severity distinguishes arbitrary user projects from the strict VAC product repo.
- Local-only workspace files are created only through approved materialization.
- Validation command inference is declarative preview until the user approves execution.
- Strict promotion is explicit and cannot silently convert a soft profile into strict manifests.

Plan 32's principle remains intact: `.vac` can declare ownership/gates, but runtime logic belongs in Rust.

## Blocks / blocked-by

Blocked by: none for the current core Plan 34 contract. The original blockers are resolved by the sandbox implementation slices: inventory evidence, Rust classifier, doctor severity split, approval-gated soft bootstrap, CLI/TUI startup notices, rich confirmation dialog model, and strict promotion preview/materialization.

Blocks: only future polish/integration breadth. The core first-run zero-config UX, soft bootstrap path, non-blocking doctor semantics for arbitrary user projects, and strict promotion safety semantics are implemented.

Does not block:

- Plan 30/31/32/33 runtime-owner work,
- strict `.vac` control-plane hardening for the VAC product repository,
- docs-only architecture review of related zero-config concepts.

## Product modes

| Mode | Description | Default posture |
|---|---|---|
| `in_memory` | No `.vac` files written; conservative defaults live only for the session. | read allowed, writes/exec approval-required |
| `soft` | Lightweight `.vac` workspace with profile, local DB/index/cache boundaries, and inferred validation hints. | no strict manifest requirement |
| `curated` | Reviewed project profile, memory, validation commands, and selected policies. | user/team reviewed |
| `strict` | Explicit capabilities, policies, workflows, surfaces, and doctor gates. | opt-in promotion |

Mode transitions must be explicit:

```text
missing .vac -> in_memory or approved soft workspace -> curated -> strict
```

No transition may silently persist memory, run validation, or make `.vac` mandatory before the user can submit ordinary prompts.

## Proposed workspace layout

```text
.vac/profile.yaml        inferred project profile and adoption mode
.vac/.gitignore          local-only state boundary
.vac/db/                 local SQLite/state DB candidate
.vac/memory/             curated reviewed memory
.vac/sessions/           resumable session snapshots
.vac/index/              repo/symbol/search indexes
.vac/artifacts/          patches, validations, evidence
.vac/logs/               local event/tool-call logs
.vac/cache/              derived cache; safe to delete
.vac/tmp/                scratch files; safe to delete
.vac/manifests/          optional strict control-plane manifests
```

Commit boundary:

```text
commit-friendly: profile.yaml, reviewed memory, reviewed manifests
local-only: db/, sessions/, index/, artifacts/, logs/, cache/, tmp/
```

The exact layout began as a proposal. The approved-bootstrap core helper now materializes the lightweight subset `.vac/profile.yaml`, `.vac/.gitignore`, and the local-only directories after explicit caller approval. It still does not generate strict manifests, reviewed memory, indexes, databases, logs, artifacts, or session snapshots by default. Implementation may adjust names if Rust/runtime constraints require it, but the local-only vs commit-friendly distinction must remain visible.

## Missing `.vac` fallback contract

When `.vac` is missing:

- ordinary prompt submission must continue,
- read-only assistance should use conservative defaults,
- write/exec/mutation remains approval-gated,
- VAC may infer project hints from visible files but must label them as inferred,
- VAC may ask for approval before creating `.vac`,
- if creation is denied, VAC continues in `in_memory` mode,
- doctor/registry should report setup warnings, not fatal errors, for arbitrary user projects,
- strict manifest-only features may be unavailable until promotion, but chat/coding assistance must remain usable.

Fatal errors are reserved for unsafe states that prevent the requested operation itself, not for the mere absence of `.vac`.

## Implementation slices

### 34A — Current behavior inventory

Inventory existing CLI, TUI, config, doctor, registry, session, memory, and workspace-root behavior before changing contracts.

Questions to answer:

- Where does VAC currently look for `.vac`?
- Which code paths fail when `.vac` is absent?
- Which diagnostics are fatal vs warning today?
- Where is local state persisted today?
- Which user surfaces expose setup state?
- Which tests already cover missing config or missing control-plane manifests?

Evidence expected:

```text
inventory notes with file paths
current failure/warning examples
existing tests or gaps
no code changes yet
```

Done when the team has a concrete baseline and can identify which user-visible failures must change.


Implementation note 2026-05-27 sandbox: `34-evidence/2026-05-27-sandbox-zero-config-inventory.md` records the current strict product-repo gate behavior and the zero-config implication. This is inventory only; it does not weaken `.vac` product-repo gates and does not claim arbitrary-project fallback implementation.

### 34B — Missing `.vac` fallback contract

Define the Rust-owned fallback contract for arbitrary user projects with no `.vac`.

Contract requirements:

- missing `.vac` must not block ordinary prompt submission,
- in-memory profile is the safe default when writing `.vac` is not approved,
- inferred stack/test/build hints are advisory until reviewed,
- mutations still require approval,
- strict-only features degrade with clear explanation,
- `.vac` remains declarative and does not contain runtime branching logic.

Done when the fallback behavior can be described as a stable user-facing contract and mapped to Rust owners/tests.


### 2026-05-27 sandbox slice — 34B side-effect-free workspace classifier

Added `vac_core::project_workspace` as a Rust-owned classifier for missing-`.vac` posture, plus `vac doctor project-workspace <path>` as an explicit diagnostic surface. Arbitrary user projects without `.vac` classify as `InMemory` with setup-warning semantics and approval-required durable writes; strict product repositories without `.vac` fail closed for strict gates. The classifier is wired into CLI/TUI startup and doctor bootstrap/promotion UX; Plan 34 is complete for the default zero-config baseline.

### 2026-05-27 sandbox slice — 34C–34E approved bootstrap core

Added an approval-gated soft workspace bootstrap helper in `vac_core::project_workspace`:

- `build_soft_workspace_bootstrap_plan(root)` renders the first-run UX preview without writing files.
- `materialize_soft_workspace_bootstrap(root, approved)` returns `approval_required` when approval is absent and only creates `.vac/profile.yaml`, `.vac/.gitignore`, and local-only boundaries after explicit approval.
- Denied bootstrap leaves the project in `in_memory` mode and keeps ordinary prompt semantics non-fatal.
- Existing strict workspaces are refused by the soft bootstrap helper, preventing silent downgrade or manifest overwrite.
- `ProjectWorkspaceReport::render_text()` now includes an indented bootstrap preview when `.vac` is missing in an arbitrary user project.
- `vac doctor project-workspace <path> --strict-product-repo` exposes the strict product-repo severity path, where missing `.vac` is an error and no bootstrap preview is reported as green.

This slice implements the Rust-owned disk-change contract for the proposed first-run UX, but still does not wire the interactive CLI/TUI prompt path or create `.vac` automatically.

### 34C — Zero-config bootstrap UX

Design first-run CLI/TUI behavior for missing `.vac`.

Expected UX:

```text
VAC did not find a .vac workspace.
You can continue in-memory now, or approve creation of a lightweight .vac workspace.
```

The setup summary should show:

- detected repo root,
- inferred stack/package manager,
- inferred validation hints,
- proposed local-only directories,
- what would be commit-friendly,
- what remains disabled until curated/strict promotion.

Done when the UX copy and state transitions are clear enough to implement without inventing policy in code review.

### 34D — Doctor/registry warning semantics

Define severity rules for doctor and registry checks in arbitrary user projects.

Expected semantics:

- missing `.vac` in a user project is a **warning/setup** state,
- missing `.vac` in the VAC product repo may remain strict/fatal if product gates require it,
- missing strict manifests disables strict-only checks rather than blocking normal chat,
- doctor output should recommend `in_memory`, `soft`, `curated`, or `strict` next steps,
- warnings must be machine-readable enough for TUI/status surfaces,
- false-green is forbidden: warnings cannot be reported as passing strict gates.

Done when warning vs fatal behavior is unambiguous for both arbitrary user projects and the VAC product repo.

### 34E — Migration path to explicit `.vac`

Define promotion from in-memory/soft workspace to explicit reviewed `.vac` state.

Migration path:

```text
in_memory -> soft profile -> curated profile/memory -> strict manifests
```

Rules:

- promotion requires user approval,
- generated profile/memory must be reviewable before commit,
- local-only directories stay ignored,
- strict manifests are opt-in and doctor-visible,
- no automatic YAML migration occurs,
- rollback from soft workspace to in-memory should be safe by removing local-only files and reviewed profile files.

Done when users can understand what changes on disk, what can be committed, and what remains local-only.

### 34F — Tests/evidence

Define future test and evidence requirements before implementation is accepted.

Expected coverage:

- no `.vac` + ordinary prompt submission succeeds,
- no `.vac` + denied workspace creation falls back to in-memory,
- no `.vac` + approved bootstrap creates ignored local-only boundaries,
- soft workspace does not require strict manifests,
- doctor/registry warns for missing `.vac` in user projects and errors for strict product-repo gates,
- strict product-repo gates still fail when required manifests are absent,
- promotion requires explicit approval,
- generated docs/evidence never claim implementation before code exists.

Done when future implementation has a narrow validation matrix and evidence path.

## 2026-05-27 sandbox implementation baseline — 34A–34D

Implemented a narrow Rust-backed baseline for the zero-config workspace contract:

- Added `vac_core::control_plane::project_workspace` with explicit modes: `in_memory`, `soft`, `curated`, and `strict`.
- Missing `.vac` now classifies as `in_memory` with `ordinary_prompt_allowed: true`, `bootstrap_offer_available: true`, and a warning-level `missing_vac_workspace` diagnostic rather than a fatal state.
- Existing `.vac` workspaces classify from `.vac/profile.yaml` when present, or from strict manifest directories when present.
- Local-only boundary warnings check `.vac/.gitignore` for `db/`, `sessions/`, `index/`, `artifacts/`, `logs/`, `cache/`, and `tmp/`.
- Added `vac doctor project-workspace <path>` as the user/operator-facing diagnostic surface for the contract.
- Added focused Rust coverage for missing `.vac`, soft workspace local-only boundary warnings, and strict workspace detection.

Supersession: the follow-up 34F/34H slices below close this baseline's remaining UX and strict-promotion items. This baseline section is retained only to show the staged rollout order.

## 2026-05-27 sandbox implementation slice — 34F CLI/TUI prompt cutover

Recorded `34-evidence/2026-05-27-sandbox-cli-tui-cutover.md`. The zero-config workspace contract now reaches the user-facing prompt path:

- `vac_core::project_workspace::ProjectWorkspaceStartupNotice` renders first-run CLI/TUI warnings without writing files.
- `vac_core::project_workspace::project_workspace_startup_notice(root)` returns a notice only when `.vac` is missing in an arbitrary user project and ordinary prompts may continue.
- The top-level CLI emits a preflight notice before launching the interactive TUI for a missing-`.vac` root.
- The TUI appends the same notice into `startup_warnings`, so the app-server/runtime event surface can show setup guidance without making `.vac` mandatory.
- `vac doctor project-workspace <path> --bootstrap-soft --yes` is the explicit approval-gated write path; without `--yes` it renders the preview and exits approval-required.

Completion note:

- The rich setup dialog is represented by `ProjectWorkspaceConfirmationDialog` and surfaced through CLI/TUI startup warnings plus doctor preview.
- Strict manifest promotion has an explicit review-only preview and an approval-gated `--promote-strict --yes` materialization path.

## Validation

Docs-only validation for this plan:

```bash
git diff --check -- docs/workflow-control-plane/plans/34-zero-config-project-workspace.md docs/workflow-control-plane/plans/INDEX.md
rg -n "34A|34B|34C|34D|34E|34F|Non-goals|Stop conditions|Done criteria|missing .vac" docs/workflow-control-plane/plans/34-zero-config-project-workspace.md
```

Future implementation validation should be proportional and should start with narrow missing-workspace fixtures before broad Cargo lanes. Candidate commands:

```bash
cargo test --manifest-path vac-rs/Cargo.toml -p vac-core project_workspace -- --nocapture
cargo test --manifest-path vac-rs/Cargo.toml -p vac-surface-cli project_workspace -- --nocapture
cargo test --manifest-path vac-rs/Cargo.toml -p vac-surface-tui project_workspace -- --nocapture
./vac-rs/target/debug/vac doctor registry .
./vac-rs/target/debug/vac doctor surfaces .
```

Do not run heavy Cargo validation for this docs-only proposal unless an implementation slice changes Rust code later.

## Stop conditions

Stop if any slice:

- makes `.vac` mandatory before VAC can answer normal coding prompts,
- treats missing `.vac` as fatal for ordinary prompt submission in arbitrary user projects,
- writes local-only state without ignore boundaries,
- silently persists memory without source/evidence/review,
- conflates soft profile with strict control-plane manifests,
- puts runtime branching/business logic into `.vac`,
- weakens strict gates for the VAC product repository,
- introduces another TUI/runtime path,
- claims Plan 34 is implemented before Rust code and evidence exist.

## Done criteria

Historical docs-only done criteria:

- Current status, goal, non-goals, prerequisites, blocks/blocked-by, implementation slices, stop conditions, validation, and done criteria are documented.
- Slices 34A–34F are concrete and preserve proposed status.
- Missing `.vac` is documented as non-blocking for ordinary prompt submission.
- `.vac` is documented as declarative while runtime logic stays in Rust.
- In-memory, soft, curated, and strict modes are distinct.
- Local-only/commit-friendly boundaries are explicit.
- Future implementation has a narrow validation/evidence path.

Implementation done criteria:

- Zero-config project workspace behavior exists in Rust-backed CLI/TUI flows.
- Doctor/registry semantics distinguish user-project warnings from product-repo strict gates, including a strict product-repo doctor flag.
- First-run UX does not force YAML/control-plane migration.
- Tests prove missing `.vac` fallback, denied bootstrap fallback, approved soft bootstrap, rich confirmation actions, denied strict promotion, and approved strict promotion.
- Evidence is captured without claiming strict gates passed unless strict manifests actually validate.


## 2026-05-27 sandbox implementation slice — 34G rich confirmation and strict promotion

Recorded `34-evidence/2026-05-27-sandbox-rich-confirmation-strict-promotion.md`. Plan 34 now has a Rust-owned rich confirmation model, CLI doctor preview flags, approval-gated strict promotion materialization, and targeted tests for denied/approved transitions.


## 2026-05-27 sandbox implementation slice — 34H strict promotion UX closeout

Recorded `34-evidence/2026-05-27-sandbox-tui-confirmation-strict-promotion-closeout.md`. The zero-config UX now has a rich confirmation dialog model, CLI preview flags, TUI startup warning surfacing, and approval-gated strict promotion materialization. Strict product-repo gates remain fail-closed; arbitrary user projects can continue in-memory until setup is approved.


## 2026-05-28 sandbox sync

- Plan 34 rich dialog, strict promotion, CLI/TUI prompt baseline, and docs sync are complete in the default path.
