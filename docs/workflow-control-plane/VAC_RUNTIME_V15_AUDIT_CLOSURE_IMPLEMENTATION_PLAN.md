# VAC Runtime v1.5 Audit Closure Implementation Plan

## Scope

This plan records the source-level closure path for the original VAC Runtime v1.5 audit. It is intentionally tied to the executable SV gate `scripts/vac-runtime-audit-closure-sv.py` and the tracked plan `.vac/plans/runtime-v15-audit-closure.yaml`.

The closure is L1 cooperative. It does not claim L2 broker isolation, OS sandbox enforcement, broker-held key custody, or external attestation.

## Finding-to-slice map

| Finding | Slice | Closure evidence |
|---|---|---|
| P0-RUNTIME-001 | Real agent-loop mediation | `BoundRuntimeToolBoundary::from_context_metadata`, `vac_boundary.gate_tool_call`, and completion-lock closeout in `vac-agent-loop/src/agent.rs` |
| P0-RUNTIME-002 | Pre-plan and pre-patch fail-closed gates | `pre_patch_gate`, approved plan checks, and bounded patch attempt stamping in `bound_tool.rs` |
| P0-RUNTIME-003 | Actual patch range resolution | `find_all_matches`, zero/multiple match rejection, `byte_range_to_line_range`, and `resolve_actual_semantic_anchor` |
| P0-RUNTIME-004 | Structured command authority | `resolve_vac_structured_command_authority`, typed `structured_command`, and shell-free execution in MCP/task paths |
| P0-RUNTIME-005 | Checkpoint integrity | `scripts/check-checkpoint-integrity.py` verifies compiled source hashes, checkpoint index counts, and readiness authority |
| P1-RUNTIME-010 | Policy snapshot scaffolding | `vac-policy::PolicySnapshot`, scoped grants, and hardcoded safety denials |
| P1-RUNTIME-011 | Approval binding scaffolding | `ApprovalBindingV2`, nonce, expiry, and policy snapshot hash validation |
| P1-RUNTIME-012 | Evidence scaffolding | `EvidenceStore`, Merkle root, integrity hints, and CAS conflict detection |
| P1-RUNTIME-013 | Deterministic index scaffolding | AST path, normalized fingerprint, anchor resolution, and scanner confidence |
| P1-RUNTIME-014 | Assessment and SpecSync scaffolding | span-grounded assessment and changed-file/spec drift mapping |
| P1-RUNTIME-015 | Doctor/readiness/memory control-plane scaffolding | release doctor aggregation, readiness reduction, and memory schemas with redaction guards |

## Execution order

1. Wire runtime boundary into the real agent lifecycle before tool execution.
2. Gate patch and command attempts from the approved VAC semantic plan, not from tool-supplied authority.
3. Resolve patch preimages and ranges from workspace content.
4. Reject shell-free violations and structured-command mismatches before process launch.
5. Bind checkpoint integrity and registry truth to source hashes.
6. Keep L2, CI custody, and external audit claims explicitly out of scope until separate proof exists.

## Acceptance

The implementation is accepted only when `scripts/vac-runtime-audit-closure-sv.py .` reports PASS and the final VAC gate continues to label L2 as `NotImplemented`.
