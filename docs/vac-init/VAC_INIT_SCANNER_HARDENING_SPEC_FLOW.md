# VAC-Init Scanner Hardening — Executable Spec-Flow Plan

`plan.scanner-hardening.spec-flow`
`capability: vac.init.scanner-policy-hardening`
`mode: bounded_worker`
`baseline: vastar-agentic-cli (committed repo, scanned 2026-05-30)`
`method: ast_exact (Phase 1) — syn/AST-exact = DEFERRED, see §Phase 2`

> This is the **hardened** version of the proposed plan, reconciled against the *actual* committed codebase. The original plan was conceptually sound but **not executable as written** because several of its assumptions do not match the repo. The corrections below are the difference between "reads well" and "compiles, runs, and passes a doctor gate."

---

## 0. Reality reconciliation (why the original plan was not yet executable)

Evidence gathered by scanning the committed tree (not the `vac-source-vac-init-command-run.zip` baseline the plan quoted).

| # | Plan assumption | Repo reality | Impact | Required correction |
|---|---|---|---|---|
| R1 | Scanner produces 2306 findings, `method: ast_exact` | Committed `.vac/.init/risk_findings.yaml` holds **1 hand-seeded finding** (`finding.seed.runtime-hardening`, `method: filename_pattern`); `state.yaml.risk_findings: null` | The 2306/ast_exact numbers are from a *different baseline zip*, not this repo | Treat counts as non-authoritative; regenerate live before asserting any number |
| R2 | Just "modify the scanner" (S1–S3) | `vac-rs/cli/src/init_cli.rs` (225 LOC) **never calls** `build_live_scanner_reports()`. It writes seed YAML via `render_scan_report_yaml()` + `count_workspace_files()`. The live scanner module is effectively **dead/test-only code** | Hardening S1–S3 alone would improve code that `vac init` never executes | **Wiring (old S5) must move to the front.** No classification matters until `vac init` actually invokes the scanner |
| R3 | Add new types `SourceScope`, `SourceClass`, `classify_source_path()`, `build_source_inventory_report()` | File already defines `LiveSourceClass`, `LiveRiskAction`, `LiveSourceEntry`, `LiveRiskFinding`, `classify_live_path()`, `build_live_source_inventory()`, `build_live_risk_findings()`, `render_*`, `build_live_scanner_reports()` | Adding parallel types creates **two competing type systems** | **Extend the existing `Live*` types**, do not invent a parallel set |
| R4 | Phase 2 uses `syn::parse_file` | Workspace has **no `syn`** (only `syntect = "5"`). No `vendor/` dir, offline build | Cannot add `syn` now (no network, no vendor) | Phase 2 stays **DEFERRED / NotEvaluated**; honest `method: ast_exact` only |
| R5 | "If repo has `ignore`, use `WalkBuilder`" | `ignore = "0.4.23"` **is** a workspace dep (used by `file-search`), already in `Cargo.lock`. But `vac-core` lists only `walkdir.workspace = true`; scanner currently walks via `fs::read_dir` (manual recursion, line ~335) | WalkBuilder is reachable without new network fetch, but core must opt in | Add `ignore.workspace = true` to `vac-rs/core/Cargo.toml`; switch walk to `ignore::WalkBuilder` (gives `.gitignore` semantics). Fallback: reuse already-present `walkdir` |
| R6 | S4 enforces "unowned product runtime => block" | `.vac/registry/ownership/report.yaml` is an **empty seed**: `total_files: 0`, `files: []`, `coverage_percent: 100.0` | If enforced literally, either everything is "unowned" (doctor never passes) or the false `100%` hides reality | S4 must detect **seed/unpopulated ownership report** → degrade to `NotEvaluated` + warning, **not** hard-block. Distinguish "no policy loaded" (block) vs "ownership not yet populated" (warn) |
| R7 | Validation gates run 8 scripts in order | `check-vac-dogfood-session.sh` **does not exist**; `check-vac-init-scanner-hardening-spec-flow.sh` does not exist yet (S0 creates it). Other 6 exist | Listing a non-existent gate as runnable breaks the gate sequence | Drop `check-vac-dogfood-session.sh` from the order (or create it as a separate slice). Only S0 introduces the new scanner gate |
| R8 | `risk_finding` schema fields: `file,line,pattern,inferred_risk,confidence,method,ambiguous,alternatives` | `LiveRiskFinding` has `id,file,line,pattern,action,confidence,method,ambiguous` — **missing `alternatives`**, and emits `inferred_risk` indirectly via `action` | Schema is non-compliant with spec | Add `alternatives: Vec<String>` to `LiveRiskFinding`; emit `inferred_risk` from `action.as_str()`; keep `ambiguous` |
| R9 | New `id: finding.live-init.report` | Current renderer emits `id: finding.live.report`; tests assert that exact string (`vac_init_live_scanner_policy.rs:500`) | Renaming breaks the existing unit test | Update the test in the same slice as the id change |
| R10 | Donor quarantine is conceptual | Vocabulary is **real**: `donor-inventory.yaml` has `decision: QUARANTINE`/`status: QUARANTINED` (lines ~264–285); `DONOR_STATUS_BOARD.md` marks `vac_shell_*` & `vac_tui_runtime` QUARANTINED | S4 `DonorQuarantined` mapping is grounded | Read these two files as the quarantine source of truth (feasible as-is) |

**Net effect:** the plan's *design* is correct; its *execution order and type/wiring assumptions* were wrong. The hardened order below fixes that.

---

## 1. Spec anchors (unchanged, confirmed against `VAC_Init_Control_Plane_Spec_v1_alpha_refined.docx`)

- `vac init` 4-phase flow: **Codebase Scanning → Strategy Prompt → AST Policy Extraction → Synthesis & Verification** (spec §6).
- `risk_finding` schema with confidence-driven disposition.
- Policy **fail-closed**: explicit deny blocks; approval-required pauses; **missing policy = block**; multi-policy merge = most-restrictive-wins.
- Ownership classes: `complete | partial | hidden | overclaimed | unowned` + quarantine; **unowned MUST NOT be agent-written**.
- All scanner changes flow through **Semantic Plan → gates → evidence → safe rationale**, never free scripts (spec §3 non-goals, §8).

---

## 2. Corrected implementation order

> Original order was S0→S1→S2→S3→S4→S5→S6→S7. The hardened order promotes **wiring** so every later slice operates on code `vac init` actually runs.

```
S0  Semantic Plan / capability / workflow / gate skeleton
S1  WIRING + walk: call build_live_scanner_reports() from init_cli (was S5, part 1)
S2  Source scope classification (extend LiveSourceClass)
S3  Full risk-finding storage (index/full/by-risk/by-scope) + schema fields
S4  Scope-aware, fail-closed policy inference
S5  Ownership + quarantine annotation (with seed-report degradation)
S6  vac init lifecycle states (--scan / --rescan-ast / resume / failed)
S7  scanner doctor gate
S8  evidence + trajectory + artifact packaging
```

---

## S0 — Semantic Plan & capability wiring

**Create:**
```
.vac/registry/plans/plan.scanner-hardening.spec-flow.yaml
.vac/capabilities/scanner-hardening-spec-flow.yaml
.vac/workflows/maintenance.scanner-hardening-spec-flow.yaml
docs/vac-init/VAC_INIT_SCANNER_HARDENING_SPEC_FLOW.md   (this document)
scripts/check-vac-init-scanner-hardening-spec-flow.sh
```
**Allowed-files (bounded patch scope):**
```
vac-rs/core/src/control_plane/vac_init_live_scanner_policy.rs
vac-rs/core/Cargo.toml                     # ADDED: needed for `ignore` dep (R5)
vac-rs/cli/src/init_cli.rs
vac-rs/core/src/control_plane/mod.rs       # ADDED: re-export new symbols if needed
+ the 5 manifest/doc/script files above
```
**Acceptance:** Semantic Plan exists; capability `ready` fields complete; workflow typed with **no inline shell command**; validation command structured (typed runner+args, not a free string).

---

## S1 — Wiring + file walk (the unblocking slice)

**Modify** `vac-rs/cli/src/init_cli.rs`:
- In the scan path, call `vac_core::control_plane::vac_init_live_scanner_policy::build_live_scanner_reports(root)` and **persist** its outputs to `.vac/.init/…` instead of seeding zeros.
- Replace the bespoke `count_workspace_files()` count with the scanner inventory totals → **single source of truth** (kills the count-drift in plan problem #4).

**Modify** `vac_init_live_scanner_policy.rs` walk:
- Add `ignore.workspace = true` to `vac-rs/core/Cargo.toml`.
- Replace `fs::read_dir` recursion (~line 335) with `ignore::WalkBuilder::new(root).hidden(false).git_ignore(true).build()`. Rationale: respects `.gitignore`, prunes `target/`, deterministic.

**Acceptance:**
```
cargo build -p vac-core -p vac-surface-cli            # must compile (R5 dep added)
# after `vac init --scan`: state.yaml.risk_findings != null
# .vac/.init/risk_findings.yaml regenerated by code, not the hand seed (R1)
```

---

## S2 — Source scope classification (extend, don't replace — R3)

**Extend** `LiveSourceClass` (do NOT add a parallel `SourceScope`). Split `Source` and add reference/quarantine/vendor scopes:
```rust
pub enum LiveSourceClass {
    ProductRuntime,    // was part of Source
    ProductTest,       // was Test
    VacManifest,       // was Manifest
    DonorReference,    // NEW
    DonorQuarantined,  // NEW
    Generated,
    VendorDependency,  // was Vendor
    BuildOutput,
    Documentation,
    Unknown,
}
```
Update `as_str()`, `is_scannable()` (only `ProductRuntime | ProductTest | VacManifest` scannable; donor/vendor/generated/build NOT), and `classify_live_path()` ordering (most-specific first):
```
BuildOutput   : target/**, .git/**, node_modules/**
VendorDep     : vac-rs/vendor/**, vendor/**, **/.cargo/**
Generated     : **/generated/**, *.generated.rs, schema/json/**, schema/typescript/**
DonorQuarantined : donor/vac/crates/vac_shell*/**, donor/vac/crates/vac_tui_runtime/**,
                   + any path whose donor row is QUARANTINED in donor-inventory.yaml
DonorReference: donor/vac/** (unless row MIGRATED and a root replacement exists)
VacManifest   : .vac/**/*.yaml
ProductTest   : **/tests/**, **/*_test.rs, **/test_*.rs, tests/fixtures/**
ProductRuntime: vac-rs/*/src/**/*.rs, vac-rs/*/Cargo.toml, declared validation-gate scripts/*.sh
Documentation : **/*.md
Unknown       : fallback
```
Add to `LiveSourceEntry`: `scan_eligible: bool`, `reason: String` (spec output requires them).

Write per-class files: `.vac/.init/source_inventory/by-class/{product,test,donor_reference,donor_quarantined,generated,vendor,build_output}.yaml` + the summary `source_inventory.yaml`.

**Acceptance:** `donor/vac/**` → donor_reference|donor_quarantined; `target/**` → build_output; `vac-rs/vendor/**` → vendor_dependency; `tests/**` → product_test; `.vac/**/*.yaml` → vac_manifest. (Note: `vendor/` is currently **absent** in repo — rule must still exist for when it is regenerated.)

---

## S3 — Full risk-finding storage + schema compliance (R8/R9)

**Struct change** — add the missing field:
```rust
pub struct LiveRiskFinding {
    pub id: String,
    pub file: String,
    pub line: usize,
    pub pattern: String,
    pub action: LiveRiskAction,      // -> emitted as `inferred_risk`
    pub confidence: f32,
    pub method: &'static str,        // "ast_exact"
    pub ambiguous: bool,
    pub alternatives: Vec<String>,   // NEW (spec-required)
}
```
**Heuristic detectors (Phase 1, no new dep)** — keep `method: ast_exact` honest:
```
execute_process : std::process::Command | tokio::process::Command | Command::new
network_access  : tokio::net | reqwest:: | hyper:: | tonic:: | websocket
filesystem_write: std::fs::write | tokio::fs::write | File::create | OpenOptions::new()...write/create/append
filesystem_delete: remove_file | remove_dir_all
credential_read : std::env::var | dotenv | keyring | token/secret near env/config read
```
**Storage layers** (replace sample-only output — plan problem #4):
```
.vac/.init/risk_findings/index.yaml      (counts + pointers)
.vac/.init/risk_findings/full.yaml       (every finding)
.vac/.init/risk_findings/by-risk/*.yaml
.vac/.init/risk_findings/by-scope/*.yaml
.vac/.init/risk_findings.yaml            (summary pointer; id: finding.live-init.report)
```
Update the unit test asserting `finding.live.report` → `finding.live-init.report` (R9), same slice.

**Acceptance:** `summary.total_findings == count(full.findings)`; `sum(by-risk) == full`; `sum(by-scope) == full`; sample-only output forbidden (gate checks no truncation marker).

---

## S4 — Scope-aware, fail-closed policy inference

Extend the existing `render_policy_inference_report_yaml` into scope-aware inference. Confidence labels (spec):
```
0.90-1.00 certain   => auto-assign
0.70-0.89 high      => auto-assign + info log
0.50-0.69 moderate  => assign + warning + operator review
0.30-0.49 low       => present, no auto-assign
0.00-0.29 uncertain => log only
```
Decision matrix:
```
credential_read   ProductRuntime & conf>=0.70 => deny
                  Test/Donor/Reference        => review_only (unless reachable from runtime)
execute_process   ProductRuntime & conf>=0.70 => approval_required
                  validation-gate script       => approval_required + structured command check
                  Test                         => review_only
network_access    ProductRuntime => approval_required ; Test => review_only ; DonorReference => reference_only
fs_write/delete   ProductRuntime => approval_required ; Generated/Build/Vendor => ignored|blocked-by-scope
                  DonorQuarantined reachable-from-product => blocked
```
Fail-closed invariants: **no policy loaded => block**; multi-policy merge = most-restrictive-wins; never auto-allow `ambiguous` or low/uncertain.

**Acceptance:** no broad-allow for any high-risk action; `credential_read` in ProductRuntime → `deny`; donor/test findings never escalate product-runtime policy unless reachable.

---

## S5 — Ownership + quarantine annotation (with seed-report degradation — R6)

**Read:** `.vac/registry/ownership/report.yaml`, `.vac/registry/donor-inventory.yaml`.
Annotate each finding:
```
ownership: { status: complete|partial|hidden|overclaimed|unowned, capability:, quarantine: none|soft|hard|auto }
```
Rules:
```
unowned     product-runtime => policy_conflict / ownership_missing
overclaimed product-runtime => block until resolved
hidden                       => review
donor_quarantined reachable  => block
```
**Critical hardening (R6):** if the ownership report is the **empty seed** (`total_files: 0` / `files: []`), the scanner MUST emit `ownership: NotEvaluated` + a warning and MUST NOT mass-block on "unowned", nor trust the cosmetic `coverage_percent: 100.0`. Hard-block only once the ownership report is genuinely populated.

**Acceptance:** scanner doctor fails on unowned/overclaimed product-runtime risk **when ownership is populated**; warns on partial; reports donor quarantine separately; emits `NotEvaluated` (not Pass, not mass-fail) when ownership is still seed.

---

## S6 — `vac init` lifecycle (greenfield states — reality: only Scan/ready exist today)

Current `init_cli.rs` supports only `Scan => discovered` and default `=> ready`. Add:
```
vac init --scan        : write source_inventory + risk_findings ; state -> discovered
vac init --rescan-ast  : refresh findings ; state -> policy_inferred   (NEW mode in VacInitCliMode)
vac init               : if state exists & not ready => resume (not reset) ; if ready => idempotent refresh
scan failure           : state -> scan_failed with error populated
```
**Acceptance:** transitions match spec lifecycle; `ready` stays idempotent; non-ready resumes (not resets); failure enters `scan_failed` with `error`.

---

## S7 — Scanner doctor gate

`scripts/check-vac-init-scanner-hardening-spec-flow.sh` (typed checks, exit 0 only if all consistent):
```
[ ] source_inventory exists & class counts sum to total
[ ] risk_findings index/full/by-risk/by-scope mutually consistent
[ ] policy_inference exists & fail-closed (no broad allow)
[ ] donor/test/product classification present
[ ] ownership annotations present (or NotEvaluated when seed)
[ ] confidence-threshold behavior valid
[ ] NO sample-only report
Exit 1 on: sample-only report ; product-runtime credential_read without deny ; unowned product-runtime (when ownership populated)
```
Expose as `vac doctor scanner .` only if CLI scope allows; otherwise keep it as the script gate (minimal CLI surface).

---

## S8 — Evidence + trajectory

Write on completion:
```
.vac/registry/evidence/evidence.2026-05-30-scanner-hardening-spec-flow.yaml   (with self_hash)
.vac/registry/trajectory/scanner-hardening-spec-flow.yaml
```
Trajectory spans: scanner module, `init_cli.rs`, `risk_findings.yaml`, `policy_inference_report.yaml`.
**Acceptance:** evidence has `self_hash`; safe rationale excludes raw chain-of-thought; `vac why` can point to the rationale.

---

## 3. Validation gates (corrected — R7)

Run in order (only scripts that **exist**; the new one is created by S0):
```bash
bash scripts/check-docs-state-refresh.sh
bash scripts/check-plan-codebase-reconciliation.sh
bash scripts/check-vac-init-registry-strictness-contract.sh
bash scripts/check-vac-workflow-spec-compliance.sh
bash scripts/check-no-hardcoded-readiness-scoreboard.sh
bash scripts/check-tui-source-artifact-hygiene.sh
bash scripts/check-vac-init-scanner-hardening-spec-flow.sh   # created in S0
```
> Removed `check-vac-dogfood-session.sh` — **does not exist** in the repo. Re-add only if/when that gate is authored as its own slice.

Cargo gates (only if toolchain/vendor ready — currently `vendor/` is absent, so expect these to be **NotEvaluated**, never `Pass`):
```bash
cargo metadata --manifest-path vac-rs/Cargo.toml --offline --no-deps
cargo test --manifest-path vac-rs/Cargo.toml -p vac-core scanner --offline
```

---

## 4. Definition of Done (hardened)

```
1. vac init ACTUALLY invokes the live scanner (wiring proven; state.risk_findings != null)
2. donor/test/generated/vendor/build are separate scopes (extended LiveSourceClass)
3. LiveRiskFinding carries `alternatives`; emits `inferred_risk`; method honestly `ast_exact`
4. risk_findings summary points to full/index/by-risk/by-scope; no sample-only
5. policy inference scope-aware + fail-closed (credential_read runtime => deny)
6. ownership/quarantine affects decisions; seed ownership => NotEvaluated (not mass-block, not fake-100%)
7. lifecycle states (--scan/--rescan-ast/resume/scan_failed) respected
8. scanner doctor/gate validates internal consistency
9. evidence + trajectory written with self_hash
10. no hardcoded readiness; Phase-2 syn/AST + cargo gates reported NotEvaluated unless actually executed
```

## 5. Out of scope / deferred (kept honest)

- **Phase 2 AST-exact (`syn`)**: blocked — no `syn` dep, no `vendor/`, offline. Author as a separate vendored dependency slice; until then `method: ast_exact`.
- **Full workspace build / E2E**: not asserted; `vendor/` absent. Report `NotEvaluated`.
- **`vac_shell_*` / `vac_tui_runtime` deletion**: separate quarantine-removal slice; here they are only *classified* as `DonorQuarantined`.


---

## Implementation status — 2026-05-30T00:00:00Z

This artifact implements O2.S0, O2.S1, O2.S3, and O2.S4:

- S0: Semantic Plan, capability, workflow, and scanner hardening gate are registered.
- S1: `vac init` uses `build_vac_init_live_scanner_report_files()` so scanner output is the source of truth.
- S3: risk findings are split into summary, index, full, by-risk, and by-scope reports.
- S4: policy inference is scope-aware and fail-closed for product runtime high-risk findings.

Phase 2 AST-exact is source-static evaluated via dependency-free lexical call-site matching; current method is `ast_exact`.
