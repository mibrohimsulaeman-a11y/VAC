# VAC Runtime v1.5 Re-audit Closure Implementation Plan

## Scope

This plan records the follow-up re-audit closure for VAC Runtime v1.5. It maps the re-audit findings to executable source gates, tracked plans, and ledger evidence without widening claims beyond L1 cooperative runtime governance.

Primary executable gate: `scripts/vac-runtime-audit-closure-sv.py`.
Tracked plan: `.vac/plans/runtime-v15-reaudit-closure.yaml`.
Tracked ledger: `.vac/ledger/runtime-v15-reaudit-closure.yaml`.

## Re-audit findings

| Finding | Closure slice | Required evidence |
|---|---|---|
| F-001 | Agent-loop boundary is real, not documented only | `agent.rs` routes tool calls through `BoundRuntimeToolBoundary` and blocks completion lock failures |
| F-002 | Tool-supplied policy authority is rejected | compiled policy snapshot and runtime policy checks are read from control-plane state |
| F-003 | Patch bridge resolves actual workspace ranges | `find_all_matches`, `old_str` ambiguity rejection, line-range derivation, and semantic anchor fingerprinting |
| F-004 | Command execution is typed and shell-free | `structured_command`, command mirror mismatch rejection, and `Command::new(&structured.runner)` |
| F-005 | Checkpoint integrity and generated-state hygiene are enforced | `compiled_source_hash_mismatches`, `checkpoint_index_counts`, and readiness authority checks |

## Slice closure map

| Slice | Intent | Status evidence |
|---|---|---|
| P0.3 | Replace plan-echo patch authority with actual patch preimage/range resolution | `bound_tool.rs` rejects zero/multiple matches and stamps `patch_index` from runtime state |
| P0.4 | Replace free-form process authority with structured command authority | MCP and task manager paths reject shell strings and execute structured runners only |
| P0.5 | Keep checkpoint integrity executable | `scripts/check-checkpoint-integrity.py` compares compiled source hashes and source-workspace readiness |
| P1.1 | Add policy snapshot/control-plane scaffolding | `vac-policy`, `vac-control-plane`, and registry compiler contracts remain present |
| P1.2 | Add evidence and approval scaffolding | `vac-evidence` and approval binding records expose required fields |
| P1.3 | Add index/assessment/spec-sync/memory/readiness/doctor scaffolding | control-plane crates expose deterministic contracts and redaction/readiness guards |
| P2 | Explicitly defer broker-enforced execution | L2 broker remains `NotImplemented`; no P2 claim is made by this closure |

## Non-goals

- No claim of malicious-agent containment.
- No claim of OS-level filesystem/process/network mediation.
- No claim of broker-held signing keys.
- No claim of external audit anchoring.

## Acceptance

The re-audit closure is accepted only when:

1. `scripts/vac-runtime-audit-closure-sv.py .` reports PASS.
2. `.vac/plans/runtime-v15-reaudit-closure.yaml` remains present and references `check-checkpoint-integrity`.
3. `.vac/ledger/runtime-v15-reaudit-closure.yaml` records `compiled_source_hash_mismatches` and `actual_patch_range_resolution`.
4. Final release surfaces keep `l2_broker=NotImplemented` until an actual P2 broker slice lands.
