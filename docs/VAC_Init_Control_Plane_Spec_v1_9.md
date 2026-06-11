# VAC Init Control Plane Specification v1.9

**Status:** Current lock baseline for P0; progressive baseline for P1/P2  
**Project:** VAC - Vastar Agentic CLI  
**Scope:** VAC Init Engine, `.vac/` workflow control plane, authority manifests, compiled snapshots, local runtime journal, manifest-bound runtime state, trust-vector claims, bounded agent governance, assessment-grounded init, SpecSync, deterministic indexing, memory governance, and progressive CI/broker/external audit.

This is a current-state specification. It contains no prior-version history, decision changelog, or migration narrative.

---

## 1. Purpose

VAC turns a software repository into an agentic workspace whose agent work is bounded, inspectable, and honest about the strength of its own guarantees.

P0 is a local-first control plane for a cooperative-but-fallible agent and an honest-but-rushed operator. P0 does not defend against malicious local actors, direct shell/Git/SQLite/filesystem bypass, forged local records, hostile insiders, or operator self-deception. Stronger claims require P1/P2 trust upgrades.

Core P0 outcomes:

- `.vac/` holds small tracked authority manifests.
- Runtime state is stored in a local SQLite journal, not in source-controlled per-session task/spec/todo files.
- Every runtime record is bound to the authority manifest set and workspace Git state used when it was created.
- Runtime claims carry stored trust claims and proof references.
- Derived trust is computed at read time and never trusted from stored verdict fields.
- Completion depends on DB state, validation state, decision locks, manifest synchronization, and governance gates.
- Local evidence is an integrity hint unless attested by CI, broker, or external anchoring.

---

## 2. Normative Terms

| Term | Meaning |
|---|---|
| MUST | Required. Violation is invalid or blocked for VAC-managed actions. |
| MUST NOT | Forbidden. Approval cannot bypass unless a scoped override policy explicitly allows it. |
| SHOULD | Strong recommendation. Exceptions require a recorded reason. |
| MAY | Optional. |
| VAC-managed action | Action performed through VAC runtime APIs, gates, command runners, patch executors, or broker. |
| Out-of-band action | Direct shell, filesystem, Git, network, database, or process action outside VAC mediation. |
| Authority manifest | Tracked `.vac/` declaration that changes runtime or governance authority. |
| Manifest set | The canonical set of authority manifests compiled into one runtime snapshot. |
| Runtime journal | Local SQLite operational store for sessions, events, state, decisions, and validation summaries. |
| Trust vector | Derived claim over execution and custody dimensions. |
| Manifest-bound record | Runtime record stamped with the `manifest_set_hash` and workspace Git state it depends on. |

All paths are relative to workspace root unless explicitly marked absolute.

---

## 3. Threat Model and Conformance Levels

### 3.1 P0 threat model

P0 assumes:

- the agent is cooperative but fallible;
- the operator is honest but may rush, rubber-stamp, or overuse escape hatches;
- the local runtime may be bypassed accidentally or deliberately;
- the local database is useful for coordination and evidence hints, not tamper-proof audit.

P0 does not claim defense against:

- malicious agent or malicious operator;
- local database rewrite;
- direct `git`, `sqlite3`, shell, network, or filesystem bypass;
- forged local trust claims;
- out-of-band source edits;
- clock manipulation as an authority source.

### 3.2 Conformance levels

| Level | Execution model | Valid claims |
|---|---|---|
| L1 cooperative | Agent/runtime can act locally; VAC records and gates VAC-managed actions only. | discipline, local coordination, drift detection, integrity hints, advisory governance |
| L2 brokered | Agent emits structured intent; broker mediates filesystem/process/network. | enforced bounded execution for broker-mediated actions |

P0 ships as L1 cooperative. L2 requires broker mediation, OS-level isolation, and key custody outside the agent process.

### 3.3 Surface honesty rule

No CLI, TUI, doctor, release gate, or audit output may claim a guarantee stronger than the weakest derived trust vector of the records, proof material, manifest set, and configuration it depends on.

---

## 4. Architecture

VAC has five layers.

| Layer | Role |
|---|---|
| Repository | Source code, tests, docs, config, fixtures, generated/build output. |
| Authority plane | Tracked `.vac/` manifests: capabilities, policies, workflows, surfaces, confirmed specs, schemas, migrations. |
| Execution plane | Compiled registry snapshot consumed by planner, gates, TUI, doctor, and broker. |
| Runtime journal | Local SQLite store for sessions, events, state, decisions, validation, and evidence references. |
| Operator surfaces | CLI, TUI, slash/MCP tools, approvals, status, `vac why`, doctor gates. |

Default structure:

```text
.vac/
  capabilities/           # tracked authority manifests
  policies/               # tracked policy and governance manifests
  workflows/              # tracked workflow contracts
  surfaces/               # tracked CLI/TUI/tool bindings
  specs/confirmed/        # tracked confirmed intent specs
  schemas/                # tracked JSON schemas
  migrations/             # tracked manifest/runtime DB migrations
  db/runtime.db           # ignored local journal
  cache/                  # ignored deterministic cache
  exports/                # optional release/debug exports
```

Tracked source authority is deliberately small. Runtime drafts, plan revisions, todo progress, raw command summaries, assessment runs, SpecSync proposals, validation traces, and local evidence hints are runtime journal records unless explicitly promoted.

---

## 5. Storage Classes and Manifest Binding

### 5.1 Storage classes

| Class | Examples | Primary storage | Tracked by Git? |
|---|---|---|---|
| Authority manifest | capability, policy, workflow, surface, confirmed intent spec, schema | `.vac/*.yaml` / `.schema.json` | Yes |
| Compiled snapshot | normalized strict JSON from authority manifests | `.vac/cache/compiled/` or DB | No by default |
| Runtime state | plans, todo state, validation state, session phase | `runtime.db` | No |
| Decision record | approval, scoped grant, baseline debt, readiness override | `runtime.db`; P1 may promote | No by default |
| Evidence hint | local validation summary, diff hash, command result hash | `runtime.db` | No |
| Audit anchor | CI/broker/external attestation | CI artifact, protected ref, evidence service, transparency log | Optional |
| Export bundle | release handoff or debug report | `.vac/exports/` or external artifact | Optional |

Generated state MUST NOT be treated as source authority unless explicitly promoted through the authority path.

### 5.2 Manifest set hash

`manifest_set_hash` is the JCS canonical content hash of the compiled authority snapshot used to authorize a runtime record.

The compiled snapshot hash:

- includes canonical content of all active authority manifests;
- includes schema versions, normalized IDs, resolved references, policy thresholds, and sensitive-path authority;
- excludes volatile metadata such as compile timestamp, host, elapsed time, verifier, and absolute local root;
- is deterministic for identical manifest content.

### 5.3 Runtime record binding

Every runtime record that can influence future action MUST carry:

```yaml
manifest_binding:
  manifest_set_hash: sha256:...
  compiled_snapshot_id: snapshot.<hash-or-seq>
  git_head: <sha-or-null>
  git_dirty_tree_hash: sha256:... | null
```

This applies at minimum to sessions, events, decisions, plan state, validation state, readiness calculations, evidence hints, and SpecSync proposals.

A decision or validation state created under a stale `manifest_set_hash` MUST NOT authorize new work against the current manifest set until refreshed.

### 5.4 Manifest synchronization doctor

`vac doctor manifest-sync .` compares runtime journal records with the current compiled authority snapshot.

| State | Condition | Action |
|---|---|---|
| current | record `manifest_set_hash` equals current snapshot hash | usable |
| branch_drift | Git HEAD changed but `manifest_set_hash` unchanged | warn; keep usable |
| stale_manifest | record hash differs from current hash | mark stale; block authorization |
| ghost_state | stale record would authorize a current action | quarantine; require plan/decision refresh |
| orphan_state | record references unknown snapshot hash | quarantine; require operator review |

A stale session may resume only by recording a manifest-sync refresh event under the current `manifest_set_hash` and recomputing any plan, policy, readiness, or validation state that depends on authority manifests.

---

## 6. Trust Vector Model

### 6.1 Stored trust claim vs derived trust

Records may store only:

```yaml
trust_claim:
  execution: observed_l1 | mediated_l2
  custody: local_only | self_promoted | ci_attested | broker_attested | external_attested
  proof_ref: null | <proof-id-or-uri>
```

Records MUST NOT store authoritative values for:

```text
trust_derivation
verified_at
verifier
derived_trust_level
```

Those are verification outputs, not record inputs. They are computed when the record is read by a doctor, TUI, CLI, release gate, or auditor.

### 6.2 Verification result

A reader derives:

```yaml
derived_trust:
  execution: observed_l1 | mediated_l2
  custody: local_only | self_promoted | ci_attested | broker_attested | external_attested
  derivation: unverified | verified | verified_downgrade
  downgrade_reason: null | missing_signature | invalid_signature | missing_inclusion_proof | stale_key | unsupported_proof | unavailable_anchor
```

Custody above `self_promoted` MUST be verified from proof material at read time. If proof material is missing or invalid, the custody claim MUST downgrade to the strongest verified lower custody value.

### 6.3 Claim language

| Execution | Custody | Allowed wording |
|---|---|---|
| observed_l1 | local_only | local self-reported trace; integrity hint only |
| observed_l1 | self_promoted | shared cooperative record; not tamper-evident |
| observed_l1 | ci_attested | CI-attested self-report; execution not mediated |
| observed_l1 | external_attested | externally timestamped self-report; existence not truth |
| mediated_l2 | local_only | broker-mediated action with local-only record |
| mediated_l2 | ci_attested | CI-attested broker-mediated record if proof validates both |
| mediated_l2 | broker_attested | broker-attested mediated execution |
| mediated_l2 | external_attested | externally anchored broker-mediated evidence |

External attestation proves that a record existed at time T and has not changed since. It does not prove the record was truthful when written. No custody tier proves truth by itself.

---

## 7. Runtime Journal

### 7.1 Role

`runtime.db` is a local operational journal. It provides:

- session state;
- event ordering;
- decision locks;
- plan revisions;
- validation summaries;
- local evidence hints;
- compact queryable history.

It does not provide tamper-evidence, team-wide authority, distributed reconciliation, or protection against out-of-band writes.

### 7.2 Core tables

Minimum P0 tables:

```sql
CREATE TABLE runtime_sessions (
  session_id TEXT PRIMARY KEY,
  started_at TEXT NOT NULL,
  closed_at TEXT,
  status TEXT NOT NULL,
  user_prompt_summary TEXT NOT NULL,
  current_phase TEXT NOT NULL,
  manifest_set_hash TEXT NOT NULL,
  compiled_snapshot_id TEXT NOT NULL,
  git_head TEXT,
  git_dirty_tree_hash TEXT,
  default_execution_claim TEXT NOT NULL,
  default_custody_claim TEXT NOT NULL
);

CREATE TABLE runtime_events (
  event_id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  seq INTEGER NOT NULL,
  occurred_at TEXT NOT NULL,
  phase TEXT NOT NULL,
  event_type TEXT NOT NULL,
  severity TEXT NOT NULL,
  summary TEXT NOT NULL,
  manifest_set_hash TEXT NOT NULL,
  git_head TEXT,
  payload_cbor BLOB,
  content_hash TEXT NOT NULL,
  previous_hash TEXT,
  trust_claim_override_cbor BLOB,
  proof_ref TEXT,
  UNIQUE(session_id, seq)
);

CREATE TABLE runtime_decisions (
  decision_id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  decision_class TEXT NOT NULL,
  decision_type TEXT NOT NULL,
  subject_type TEXT NOT NULL,
  subject_id TEXT NOT NULL,
  decided_by TEXT NOT NULL,
  decided_at TEXT NOT NULL,
  decision TEXT NOT NULL,
  reason_summary TEXT NOT NULL,
  scope_hash TEXT NOT NULL,
  policy_snapshot_hash TEXT,
  manifest_set_hash TEXT NOT NULL,
  git_head TEXT,
  content_hash TEXT NOT NULL,
  locked INTEGER NOT NULL DEFAULT 1,
  superseded_by TEXT,
  proof_ref TEXT
);
```

Payloads may use CBOR/MessagePack blobs for compactness. Canonical hashes MUST be computed from deterministic projections, not from volatile DB row layout.

### 7.3 Concurrency

P0 uses one cooperative writer lease per workspace and allows concurrent readers.

Required SQLite settings:

```sql
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 5000;
```

Writer lease and sequence tables:

```sql
CREATE TABLE runtime_writer_leases (
  workspace_id TEXT PRIMARY KEY,
  holder_id TEXT NOT NULL,
  holder_process TEXT,
  lease_reason TEXT NOT NULL,
  acquired_at TEXT NOT NULL,
  heartbeat_at TEXT NOT NULL,
  heartbeat_counter INTEGER NOT NULL DEFAULT 0,
  expires_at TEXT,
  session_id TEXT NOT NULL
);

CREATE TABLE runtime_lease_observations (
  workspace_id TEXT NOT NULL,
  observer_id TEXT NOT NULL,
  observed_holder_id TEXT NOT NULL,
  observed_counter INTEGER NOT NULL,
  stable_read_count INTEGER NOT NULL DEFAULT 1,
  first_observed_at TEXT NOT NULL,
  last_observed_at TEXT NOT NULL,
  clock_regression_detected INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (workspace_id, observer_id)
);

CREATE TABLE runtime_session_sequences (
  session_id TEXT PRIMARY KEY,
  next_seq INTEGER NOT NULL DEFAULT 1,
  updated_at TEXT NOT NULL
);
```

Rules:

- A VAC writer MUST acquire the workspace lease before writing journal state or applying VAC-managed patches.
- Lease acquisition and stale-lease recovery MUST run inside `BEGIN IMMEDIATE` or stronger transaction mode; SQLite `DEFERRED` transactions are insufficient for this gate.
- Fresh lease acquisition MAY rely on the `workspace_id` primary key.
- The lease holder MUST increment `heartbeat_counter` on heartbeat and before mutable VAC-managed work.
- `expires_at` is secondary telemetry only. It MUST NOT be the sole authority for stale-lease recovery.
- Stale-lease recovery is permitted only when the observed `heartbeat_counter` has not advanced for the policy-defined number of consecutive observation cycles and no clock regression has been detected.
- If wall-clock regression is detected for the holder or observer, recovery MUST pause for operator review rather than rely on time deltas.
- Recovery MUST atomically verify the unchanged counter, replace the lease, and write a `lease_recovery` event before any mutable action.
- `holder_process` SHOULD identify the local process when available; `lease_reason` MUST explain why the lease exists.
- Event `seq` allocation MUST read/update `runtime_session_sequences` in the same transaction as event insertion.
- `runtime_events` MUST enforce `UNIQUE(session_id, seq)`.
- Concurrent non-writer sessions MAY read journal state but MUST NOT mutate runtime state.
- P0 lease discipline is cooperative. It does not prevent direct SQLite, shell, Git, or filesystem bypass.

P1/P2 may add multi-machine reconciliation, server-serialized append, or broker-enforced leases.

### 7.4 Runtime DB migration

Runtime DB migrations MUST be versioned under `.vac/migrations/runtime-db/` and applied transactionally.

Each migration records:

```yaml
id: runtime_db.0003
from_version: 2
to_version: 3
applies_to: runtime.db
forward_sql_hash: sha256:...
rollback_supported: true | false
verification_command: vac doctor runtime-db .
```

`vac doctor runtime-db .` MUST report schema version, unapplied migrations, failed migrations, unsupported downgrade risk, and manifest binding support.

---

## 8. Authority Manifests and Compiled Snapshot

### 8.1 Authority manifests

Tracked authority manifests are the only source files that may change runtime authority.

| Manifest | Purpose |
|---|---|
| capability | Feature/domain ownership, readiness inputs, surfaces, validation. |
| policy | Action allow/deny/approval rules, security-sensitive paths, governance thresholds. |
| workflow | Ordered typed workflow steps. |
| surface | CLI/TUI/tool route bindings. |
| intent_spec | Confirmed business/product intent. |
| schema | Validation schema for authoring and runtime projections. |
| migration | Schema or DB migration contract. |

### 8.2 Authority custody

P0 authority manifests are `self_promoted` unless a higher-custody proof exists. P1 can make policy/governance manifests `ci_attested` by having CI sign the canonical manifest hash and exposing proof material to readers.

A governance gate cannot claim stronger custody than the policy config that defines its thresholds.

### 8.3 Compiled snapshot

Runtime components consume compiled strict JSON snapshots, not raw YAML.

Snapshot hash rules:

- computed over JCS canonical JSON projection;
- excludes volatile fields such as compile timestamp, host, environment, verifier, elapsed time, and local path absolute root;
- includes source manifest content hashes, schema version, normalized IDs, and resolved references;
- fails closed on dangling refs, duplicate IDs, invalid state transition, or non-deterministic output.

A snapshot may carry metadata such as `compiled_at`, but metadata MUST NOT affect `content_hash` or `manifest_set_hash`.

---

## 9. Decision Lock Model

### 9.1 Decision classes

| Class | Meaning | Default persistence |
|---|---|---|
| ephemeral_runtime | Retry strategy, local diagnosis, local todo ordering. | DB only |
| slice_local | Plan approval and validation acceptance for a landed local slice. | DB; P1 may export |
| team_relevant | Baseline debt, readiness override, scoped grant, policy-sensitive decision. | DB; P1 should promote |
| release | Release seal, RC approval, release-risk acceptance. | DB; P1/P2 promotion required for hard claims |
| authority_mutation | Manifest/schema/policy/workflow/spec change. | tracked authority file plus decision record |

### 9.2 Decision classifier

`decision_class` MUST be assigned by a deterministic classifier using registry policy, touched paths, action type, capability risk, and authority manifest rules.

Rules:

- `agent_self_only` classification is invalid for `team_relevant`, `release`, and `authority_mutation`.
- Ambiguous classification MUST escalate to the higher-risk class.
- Security-sensitive paths impose a mechanical floor of `team_relevant`.
- Policy/governance manifest changes impose `authority_mutation`.
- Dependency addition, network allowlist change, process execution expansion, and sandbox weakening impose at least `team_relevant`.

### 9.3 Mechanical floors

Default sensitive path floors:

```yaml
mechanical_floors:
  hard_deny_patch:
    - ".vac/db/**"
    - ".vac/cache/**"
  authority_mutation:
    - ".vac/policies/**"
    - ".vac/workflows/**"
    - ".vac/capabilities/**"
    - ".vac/schemas/**"
  team_relevant:
    - "**/auth/**"
    - "**/payment/**"
    - "**/crypto/**"
    - "**/migration/**"
    - "**/*secret*"
```

`hard_deny_patch` paths MUST NOT be modified through the bounded patch executor. They are runtime-owned or generated stores, not source targets. This does not prevent deliberate out-of-band local writes at L1; it prevents accidental VAC-managed self-corruption.

The authoritative sensitive-path list MUST come from policy manifests. Index heuristics may suggest candidates but MUST NOT become blocking authority by themselves.

### 9.4 Structured-command seam

`hard_deny_patch` protects the bounded patch executor. It does not automatically protect subprocesses launched by structured commands.

For routine validation commands, the command runner SHOULD avoid exposing `.vac/db/**` as a writable output target. If the command must run in the workspace with normal user rights, VAC MUST snapshot `.vac/db/**` hashes before and after the command. Unexpected runtime DB mutation by a child process MUST block completion and emit `runtime_db_touched_by_subprocess`.

P0 labels this as cooperative detection. L2 may enforce it with broker filesystem policy.

---

## 10. Bounded Runtime Workflow

A normal coding session follows this state machine:

```text
intake
  -> plan_draft
  -> plan_checked
  -> approved_or_policy_allowed
  -> executing
  -> validating
  -> closing
  -> done | paused_for_operator | abandoned
```

P0 managed workflow:

1. Load authority snapshot, memory hints, and runtime journal context.
2. Build a Semantic Plan with capability, allowed files, forbidden actions, validation commands, and budgets.
3. Run Pre-Plan Gate.
4. Obtain policy allowance or decision lock.
5. Apply bounded patch only through VAC-managed executor.
6. Run structured validation commands.
7. Run closeout checks: completion, ownership, manifest-sync, SpecSync, evidence hint, governance health.
8. Close only if all blocking state is terminal or explicitly paused.

L1 cannot stop out-of-band actions. Surfaces MUST describe P0 as cooperative governance for VAC-managed actions only.

---

## 11. Policy, Command, and Patch Gates

### 11.1 Policy precedence

Policy resolution is most-restrictive-wins:

```text
hardcoded deny > workspace policy > capability policy > workflow policy > plan policy > scoped grant
```

`forbidden.files` and explicit deny always win over allowed lists.

### 11.2 Structured commands

Validation commands MUST be structured:

```yaml
commands:
  - id: cargo.test.core
    runner: cargo
    args: ["test", "--manifest-path", "vac-rs/Cargo.toml", "-p", "vac-core"]
    risk: execute_process
    approval: policy
```

MUST NOT be accepted as structured command:

- shell strings;
- pipes and redirection;
- wildcard expansion by shell;
- unknown executable path;
- implicit network/process side effect not covered by policy.

### 11.3 Patch gate

A patch is valid only if:

- the target file is listed in the plan;
- the operation matches plan scope;
- semantic anchor resolves uniquely or line range remains valid;
- file is owned by the active capability;
- no forbidden path/action is touched;
- patch budget is not exceeded;
- new file creation is declared;
- plan `manifest_set_hash` matches the current compiled snapshot, or the plan has been refreshed.

If anchors or manifest bindings drift, VAC MUST refresh the plan or pause.

---

## 12. Ownership Governance

Every source file relevant to product behavior SHOULD map to a capability ownership target.

Ownership states:

| State | Meaning | Managed action |
|---|---|---|
| owned | One ready/partial capability claims file. | allow according to readiness/policy |
| shared | Multiple capabilities claim explicit roles. | allow only for permitted operations |
| infrastructure | Shared infra file with declared owner and consumers. | allow by role |
| hidden | Product-relevant file not surfaced in capability. | warn/block depending risk |
| overclaimed | Multiple claims without roles. | block write |
| unowned | No capability claims file. | quarantine write |

Shared ownership MUST declare primary owner, consumers, and allowed operation types.

---

## 13. Readiness Governance

### 13.1 Readiness fields

```yaml
readiness:
  declared:
    state: planned | partial | ready | deprecated | blocked
    trust_ref: <decision-or-manifest-ref>
  mechanical:
    state: planned | partial | ready | deprecated | blocked
    blockers: []
    evidence_refs: []
  assessment:
    blocking_findings: []
    advisory_findings: []
  effective:
    state: planned | partial | ready | deprecated | blocked
    manifest_set_hash: sha256:...
    trust_summary: <derived-at-read>
```

Assessment can add blockers or advisory findings. Assessment MUST NOT raise readiness above mechanical evidence.

### 13.2 State reduction

Use explicit reduction, not generic `min()`.

| Inputs | Effective |
|---|---|
| any input `blocked` | blocked |
| declared `deprecated` | deprecated unless active migration override exists |
| mechanical `deprecated` | deprecated |
| declared `planned` | planned |
| mechanical `planned` | planned |
| declared `ready` + mechanical `ready` + no blocking assessment | ready |
| declared `ready` + mechanical `partial` | partial |
| declared `partial` + mechanical `ready` | partial |
| any unresolved critical assessment blocker | blocked or partial per policy |

### 13.3 Aggregate vs scoped claims

Aggregate release readiness inherits the weakest required derived trust among its required inputs.

Scoped claims may show stronger evidence for a named sub-scope, for example a CI-attested validation command, but MUST NOT generalize that stronger sub-claim to the aggregate release.

The required dependency set for an aggregate claim MUST be derived from a tracked authority manifest or compiled snapshot with its own trust vector. An ad hoc self-promoted dependency set cannot raise aggregate trust.

---

## 14. Deterministic Index and Read-Plan Tickets

### 14.1 Records

VAC builds a deterministic index before semantic assessment.

| Record | Required fields |
|---|---|
| FileRecord | path, role, language, raw file sha256, generated/vendor/test classification |
| SymbolRecord | symbol id, name, kind, path, byte range, AST path |
| SpanRecord | span id, path, byte range, AST path, normalized fingerprint, raw-byte `span_sha256` |
| RelationRecord | imports, calls, reads/writes, entrypoints, dependency edges |
| RiskRecord | process/network/file/credential/migration/unsafe/secret-like findings with confidence |
| ReadPlanTicket | bounded read authorization for semantic inspection |

### 14.2 Determinism rules

- Index records MUST be sorted by stable keys before hashing or writing canonical output.
- `span_sha256` MUST be computed over raw source bytes, not lossy decoded text.
- Canonical index artifacts MUST NOT use floating-point values.
- Low-confidence parser output MUST be represented as low confidence, not as clean absence.
- Scanner coverage below policy threshold lowers mechanical readiness or requires approval.

### 14.3 Span normalization

`span_sha256` and normalized fingerprints have different jobs.

| Field | Purpose | Stability expectation |
|---|---|---|
| `span_sha256` | byte-exact provenance over the checked-out source bytes | changes on CRLF, formatter, comments, and whitespace changes |
| `normalized_fingerprint` | semantic anchor relocation and drift classification | SHOULD survive formatting-only changes when a language normalizer exists |

Default normalization rules:

- Normalize line endings to LF for fingerprinting.
- Strip trailing whitespace for fingerprinting.
- Normalize final newline presence for fingerprinting.
- Preserve internal whitespace for text formats unless a language-specific normalizer says otherwise.
- For languages with AST/token support, fingerprint SHOULD use AST path, node kind, symbol path, and normalized tokens; comments and pure formatting SHOULD NOT change it.
- For formats without a parser, fingerprint is text-normalized but not semantic.

`.gitattributes text=auto` may change checked-out bytes across environments. `span_sha256` is therefore provenance for the current checkout, while `normalized_fingerprint` is the cross-checkout anchor signal. Patch gates SHOULD refresh anchors when raw hash changes but normalized fingerprint remains stable. Patch gates MUST pause or replan when normalized fingerprint changes or becomes ambiguous.

### 14.4 Missing-code proof

A finding may claim:

| Result | Meaning | Allowed language |
|---|---|---|
| absent | Search space fully covered for required record types; target not present. | bounded absence proof |
| not_found_in_index | Target not found, but coverage is incomplete or low confidence. | not found in available index |
| coverage_insufficient | Scanner/index cannot support absence claim. | no absence claim |

`absent` is valid only if either:

1. required record types have coverage `1.0` across all searched roots with no low-confidence residues; or
2. every uncovered residue is enumerated and deterministically ruled irrelevant by authority policy.

Otherwise the result MUST be `not_found_in_index` or `coverage_insufficient`. This downgrade is the correct behavior: VAC refuses to claim absence it did not prove.

Absence proof must include:

```yaml
missing_code_proof:
  result: absent | not_found_in_index | coverage_insufficient
  coverage_set_id: <id>
  query_spec_hash: sha256:...
  searched_roots: []
  excluded_roots: []
  required_record_types: [symbol, route, call]
  coverage:
    required_coverage: 1.0
    actual_coverage: 1.0
    low_confidence_residues: []
    residues_ruled_irrelevant: []
```

---

## 15. Assessment, Baseline Debt, and SpecSync

Assessment joins three sources:

- confirmed intent;
- deterministic as-is index;
- engineering baseline.

Gap directions:

| Direction | Question | Finding type |
|---|---|---|
| intent -> code | Does intended behavior have implementation? | missing, partial, wrong behavior, insufficient tests |
| code -> intent | Does significant code have justified intent? | orphan, legacy, over-engineering, scope creep |
| baseline -> code | Does implementation meet engineering policy? | security, maintainability, ownership, test, side-effect risk |

Every semantic finding MUST cite deterministic evidence: span id/hash, file hash, relation record, or bounded missing-code proof. Model-assisted text is explanation, not authority.

Baseline debt is a decision record that accepts an unresolved gap for a bounded scope. It MUST carry `manifest_set_hash`, scope, expiry/review policy, severity, and decision class. Stale baseline debt cannot authorize current work after manifest drift.

SpecSync stores drift proposals in the runtime journal/cache by default. It changes tracked specs/manifests only after validation and a decision lock appropriate to the decision class.

`vac doctor manifest-sync .` is part of closeout and release. Critical stale state or ghost state blocks VAC-managed completion.

---

## 16. Completion Lock

A session MAY close as `done` only if:

- plan state is terminal and manifest-current;
- blocking todo items are checked or explicitly paused;
- required validation state is terminal and manifest-current;
- decision records for risk-bearing actions are locked and manifest-current;
- ownership gate has no unresolved hard violation;
- manifest-sync has no ghost state;
- SpecSync has no unresolved critical drift;
- governance health policy does not require pause;
- evidence summary is present with its true derived trust level.

Terminal states:

| State | Meaning |
|---|---|
| done | Work completed and required checks terminal. |
| paused_for_operator | Cannot complete without operator decision; not done. |
| abandoned | Operator cancelled or unrecoverable failure; recorded. |

`needs_discussion` is not a free completion state. It moves the session to `paused_for_operator` unless policy explicitly allows done-with-open-questions for low-risk non-release work.

---

## 17. Micro-Slice Fast Path

Micro-slice exists to avoid artifact friction for tiny low-risk changes. Eligibility is classifier-driven, not agent-self-declared.

All required:

- capability risk is safe_read or low;
- touched paths are not security sensitive;
- no public API/schema/policy/dependency/permission change;
- no auth/payment/crypto/migration/network/process boundary;
- operation is copy text, typo, local fixture, comment, minor doc, or safe styling;
- classifier source is not agent self-only.

Line delta is a secondary bound only.

---

## 18. Memory System

Memory tiers:

| Tier | Store | Purpose | Authority |
|---|---|---|---|
| working | RAM/session state | active facts for current iteration | none |
| runtime journal | `runtime.db` | session events, decisions, validation state | local operational state |
| episodic | `episodic.db` | anti-loop, failures, recovery notes | hint only |
| semantic | `semantic.db` | architecture facts, reuse hints | hint only |
| team rule | tracked policy/spec/manifest | governed rules | authority if manifest valid |

Memory MUST NEVER relax a policy, ownership, readiness, approval, or command gate. Memory may suggest, never authorize.

Secrets and raw private chain-of-thought MUST NOT be persisted. Redaction is best-effort and must be labeled as such. Raw stdout/stderr should be minimized; hashes and summaries are preferred.

---

## 19. Evidence and Audit Progression

P0 evidence is local and cooperative:

```yaml
evidence_hint:
  session_id: <id>
  manifest_set_hash: sha256:...
  git_head: <sha-or-null>
  plan_hash: sha256:...
  diff_hash: sha256:...
  validation_summary_hash: sha256:...
  trust_claim:
    execution: observed_l1
    custody: local_only
```

P1 may add CI-attested records. P2 may add broker-mediated execution and external anchoring.

Evidence authority levels:

| Level | Requirement | Claim |
|---|---|---|
| local_only | runtime DB record only | integrity hint |
| self_promoted | exported/shared local record | shared cooperative record |
| ci_attested | CI signs canonical record hash | CI-attested record |
| broker_attested | broker signs mediated execution record | broker-attested execution |
| external_attested | TSA/transparency inclusion proof | externally timestamped existence |

`vac why` returns a safe explanation that links evidence, policy refs, decision records, and diffs. The explanation is non-authoritative; authority lives in linked records and proof material.

---

## 20. Governance Health

P0 records governance events and slice risk points. P0 MAY display a local preview score, but P0 score output is advisory/local only. Normative windowed scoring becomes enforceable only at P1/P2 when journal history, policy config, and supporting evidence have sufficient custody.

Default bootstrap weights are conservative defaults, not empirically calibrated truth:

```yaml
governance_weights:
  calibration_status: conservative_default | workspace_calibrated
  needs_discussion: 12
  needs_discussion_release_relevant: 80
  scoped_grant: 80
  readiness_override: 120
  policy_downgrade: 300
```

Slice risk point table:

```yaml
slice_risk_points:
  micro: 5
  low: 10
  medium: 30
  high: 80
  critical: 200
  release: 300
```

`medium: 30` is the conservative default to keep the scale monotonic and visibly separated from `low: 10` and `high: 80`. `release: 300` is used for release-candidate or release-authority slices; ordinary release-relevant governance events are scored through event weights.

Score scope and denominator:

```yaml
governance_score_scope:
  default_window: last_30_days
  numerator: weighted governance events in the window
  denominator: max(1, risk_weighted_slice_points in the same window)
  micro_slices: accrue denominator points unless policy excludes them with higher-custody rationale
```

Score formula:

```text
governance_risk_score = weighted_event_points / max(1, risk_weighted_slice_points)
```

Correct 30-day window example:

```yaml
governance_risk_score:
  window: last_30_days
  weighted_events:
    needs_discussion: { count: 3, weight: 12, points: 36 }
    needs_discussion_release_relevant: { count: 2, weight: 80, points: 160 }
    scoped_grant: { count: 1, weight: 80, points: 80 }
    policy_downgrade: { count: 1, weight: 300, points: 300 }
  weighted_event_points: 576
  denominator_breakdown:
    micro: { count: 6, points_each: 5, points: 30 }
    low: { count: 5, points_each: 10, points: 50 }
    medium: { count: 4, points_each: 30, points: 120 }
    high: { count: 3, points_each: 80, points: 240 }
    critical: { count: 1, points_each: 200, points: 200 }
    release: { count: 1, points_each: 300, points: 300 }
  risk_weighted_slice_points: 940
  score: 0.613
```

Default thresholds:

```yaml
governance_thresholds:
  warn_at: 0.25
  advisory_block_at: 0.45
  release_block_at: 0.70
```

P0 blocks are advisory/local unless the governance policy manifest and supporting evidence have sufficient custody. P1/P2 may enforce release blocks only when policy config custody and evidence custody meet release policy.

`needs_discussion` density:

```yaml
needs_discussion_policy:
  max_session_density: 0.30
  max_release_window_density: 0.15
  repeated_template_warning: true
```

If density is exceeded, the session MUST close as `paused_for_operator` rather than `done`, unless a higher-custody policy explicitly allows otherwise.

---

## 21. Git and CI/CD Relationship

Git and CI/CD remain necessary. VAC does not replace them.

Git captures source history, content addressing, review diffs, and branch topology. CI captures reproducible build/test execution under a configured workload identity. VAC captures what Git and ordinary CI do not capture by default:

- agent plan and scope provenance;
- decision locks and scoped grants;
- manifest-set binding for runtime state;
- trust-vector wording for claims;
- governance escape-hatch usage;
- bounded missing-code proof;
- rationale links for `vac why` without raw private reasoning.

P0 VAC records are local cooperative evidence. P1 turns selected records into CI-attested records by signing canonical hashes of manifests, validation results, and evidence summaries. P2 adds broker-mediated execution and external audit anchoring.

VAC should feed CI rather than duplicate it: local journal records become CI inputs, and CI attestations upgrade custody for specific scoped claims.

---

## 22. Adoption and Friction

This section is non-normative design guidance.

The instrumented path must be faster than bypass for ordinary safe work. Otherwise an honest-but-rushed operator will leave the VAC path and the journal will become incomplete.

Design implications:

- Micro-slice must be cheap and visible.
- Low-risk `done-with-open-questions` may exist only under explicit policy.
- Pauses should ask one clear question, not produce compliance noise.
- Doctor output should identify the next unblock action, not only the violation.
- Out-of-band drift should be detected by `manifest-sync`, Git state checks, and journal/source drift doctors rather than falsely claimed as prevented in P0.

---

## 23. CLI, TUI, and Tool Surfaces

Required P0 commands:

| Command | Purpose |
|---|---|
| `vac init --assess` | Bootstrap authority manifests and assessment baseline. |
| `vac compile registry .` | Compile authority manifests into deterministic runtime snapshot. |
| `vac doctor registry .` | Validate authority manifests, IDs, refs, and schemas. |
| `vac doctor manifest-sync .` | Detect stale/ghost runtime state after manifest or branch changes. |
| `vac doctor runtime-db .` | Validate journal schema, migrations, lease health, and manifest binding. |
| `vac doctor policy .` | Validate policy consistency and security-sensitive path authority. |
| `vac doctor ownership .` | Detect hidden, unowned, overclaimed, and shared ownership issues. |
| `vac doctor index .` | Validate deterministic index, hashes, coverage, and low-confidence residues. |
| `vac doctor readiness .` | Compute readiness and derived trust summary. |
| `vac doctor governance .` | Record/preview severity-weighted governance health. |
| `vac why <target>` | Explain linked rationale without raw private reasoning. |

TUI routes SHOULD expose:

- capability readiness and blockers;
- runtime sessions and paused items;
- stale/ghost manifest-bound state;
- decision locks and scoped grants;
- governance health;
- trust-vector details per claim;
- scoped claims vs aggregate claims.

---

## 24. Doctor Gate Taxonomy

| Gate | Blocks when |
|---|---|
| registry | invalid schema, duplicate IDs, dangling references |
| compiled | non-deterministic snapshot, source hash mismatch |
| manifest-sync | ghost state, stale authorization, unknown snapshot hash |
| runtime-db | missing migration, broken schema, invalid lease state, duplicate sequence |
| policy | contradictory policy, missing sensitive-path authority |
| ownership | hard unowned/overclaimed paths in touched scope |
| command | unknown runner, shell string, denied process/network action |
| index | hash mismatch, unstable ordering, insufficient coverage for claimed proof |
| assessment | finding lacks deterministic evidence or valid missing-code proof |
| spec-sync | unresolved critical drift |
| readiness | effective state overclaims evidence or trust |
| governance | weighted events exceed policy threshold with sufficient custody, or advisory threshold in P0 |
| evidence | invalid hash/proof, invalid signature, unsupported claim language |
| release | any required gate blocks or aggregate claim overstates trust |

Doctor output MUST display trust wording derived at read time.

---

## 25. Conformance Fixtures and Traceability

P0 implementation MUST include fixtures that make normative gates executable, not merely documented.

| Fixture group | Must cover | Minimum negative case |
|---|---|---|
| runtime-db | migrations, WAL settings, `BEGIN IMMEDIATE`, heartbeat-counter lease recovery, sequence allocation | `DEFERRED` lease recovery race rejected; duplicate `(session_id, seq)` rejected |
| manifest-sync | `manifest_set_hash`, Git HEAD binding, stale state, ghost state | stale decision cannot authorize current action |
| trust-vector | stored claim vs read-time derived trust, downgrade on missing proof | record claiming `ci_attested` without proof downgrades |
| policy/patch | structured command, forbidden paths, `hard_deny_patch` | bounded patch attempts `.vac/db/**` and is rejected |
| command seam | subprocess DB mutation detection | validation command mutates `.vac/db/**` unexpectedly and completion blocks |
| governance | weighted numerator, 30-day denominator, denominator breakdown, `max(1, ...)`, thresholds | zero-denominator window does not divide by zero |
| span/index | raw-byte hash, normalized fingerprint, EOL normalization, formatter-stable anchor | CRLF-only change refreshes raw hash without semantic-overclaim |
| missing-code | `absent`, `not_found_in_index`, `coverage_insufficient` | low-confidence residue prevents `absent` unless ruled irrelevant |
| readiness | explicit state reduction and trust wording | assessment text cannot raise readiness |
| adoption/friction | low-risk micro-slice and pause UX | low-risk typo does not require full high-risk closeout |

Clause traceability:

- Every normative required statement SHOULD map to at least one doctor gate, fixture, or explicit manual-review criterion.
- P0 lock artifacts SHOULD include a traceability table with: section, requirement id, implementation module, doctor gate, fixture id, and TV/SV status.
- Non-automatable UX principles MAY map to manual QA criteria instead of unit fixtures.

Release candidates MUST NOT claim P0 acceptance until the fixture groups required for implemented surfaces pass.

---

## 26. Implementation Roadmap

### P0 - local honest baseline

- Authority manifest loader and validator.
- Compiled snapshot generator with deterministic `manifest_set_hash`.
- Runtime journal with WAL, heartbeat-counter writer lease, DB migrations, event sequencing, and manifest binding.
- Manifest-sync doctor for stale/ghost runtime state.
- Trust-claim schema and read-time derivation/downgrade.
- Decision classifier with mechanical floors.
- Ownership, policy, structured command, subprocess seam, and patch gates.
- Deterministic index with span normalization and coverage-aware missing-code proof.
- DB-backed completion lock.
- Readiness and governance event doctors.
- CLI/TUI surfaces that label L1 claims honestly.

### P1 - team/CI-attested mode

- CI signs canonical manifest, policy, validation, and evidence hashes.
- Team export/import for promoted decisions.
- CI-attested scoped validation claims.
- Windowed governance scoring with sufficient history custody.
- Governance release blocks when config and evidence custody are sufficient.

### P2 - broker and external audit mode

- L2 broker mediation for filesystem/process/network.
- Broker-held key custody.
- External TSA/transparency anchor.
- Multi-machine append service or server-serialized evidence store.
- Strong release claims only when execution and custody axes both support them.

---

## 27. Acceptance Criteria

P0 is acceptable when:

- source tree contains only authority manifests, not per-session artifact noise;
- compiled snapshot produces stable `manifest_set_hash` for identical authority content;
- runtime records are stamped with `manifest_set_hash` and Git state;
- `vac doctor manifest-sync .` quarantines ghost state after branch/manifest changes;
- runtime journal schema validates and survives repeated session open/close;
- writer lease acquisition and stale recovery use `BEGIN IMMEDIATE` or stronger mode;
- stale recovery uses heartbeat-counter liveness, not wall-clock expiry alone;
- duplicate `(session_id, seq)` cannot be inserted;
- writer lease prevents two VAC-managed writers from mutating concurrently;
- every runtime claim displays derived trust wording;
- derived trust is recomputed at read time and downgrades on missing/invalid proof;
- mechanical readiness cannot be raised by assessment text;
- assessment findings cite deterministic evidence or valid missing-code proof;
- `absent` cannot be claimed without full required coverage or enumerated/ruled-out residues;
- span raw hash and normalized fingerprint behavior is deterministic and tested;
- governance denominator and numerator are reproducible from visible event and slice breakdown;
- P0 records raw governance events and does not pretend advisory local scoring is release-grade enforcement;
- `needs_discussion` cannot be used as a silent done state;
- structured commands cannot silently mutate `.vac/db/**` without detection;
- doctor release does not overclaim L1 local records as audit-grade evidence.

P1/P2 are acceptable only when their stronger claims are backed by verifiable proof material.

---

## 28. Glossary

| Term | Meaning |
|---|---|
| Authority plane | Tracked declarations that define runtime/governance authority. |
| Manifest set hash | Deterministic hash of the compiled authority snapshot used by runtime records. |
| Ghost state | Runtime state created under a manifest set that no longer matches current authority but would still authorize action. |
| Compiled snapshot | Deterministic strict JSON projection of authority manifests. |
| Runtime journal | Local SQLite session/event/decision store. |
| Decision lock | Locked record of a risk-bearing choice with scope, policy hash, and manifest binding. |
| Trust claim | Stored execution/custody assertion and proof reference. |
| Derived trust | Read-time verification result from proof material. |
| Scoped claim | Claim about a named sub-scope such as one validation command. |
| Aggregate claim | Claim about a larger unit such as release readiness. |
| Missing-code proof | Coverage-bounded evidence that expected code is absent. |
| Integrity hint | Local hash/order check that is not tamper-evident audit. |
