#!/usr/bin/env python3
"""State5 operational-closure SV gate.

Encodes the State5 re-audit blockers so they cannot regress without Cargo.
"""
from __future__ import annotations
import json, re, sys
from pathlib import Path
ROOT=Path(sys.argv[1] if len(sys.argv)>1 else '.').resolve()

def read(rel: str) -> str:
    return (ROOT/rel).read_text(encoding='utf-8', errors='replace')

def exists(rel: str) -> bool:
    return (ROOT/rel).exists()

run_final=read('scripts/run-final-sv-validation.py')
bootstrap=read('vac-rs/crates/runtime/vac-agent-loop/src/session_bootstrap.rs')
session_actor=read('vac-rs/crates/runtime/vac-broker/src/session_actor.rs')
bound_runtime=read('vac-rs/crates/runtime/vac-agent-loop/src/bound_runtime.rs')
policy=read('vac-rs/crates/control-plane/vac-policy/src/lib.rs')
compiler=read('scripts/compile-vac-registry-sv.py')
local_tools=read('vac-rs/crates/integrations/vac-mcp-server/src/local_tools.rs')
assessment_script=read('scripts/generate-assessment-report-sv.py')
integrity=read('scripts/check-checkpoint-integrity.py')
manifest_gen=read('scripts/generate-checkpoint-manifest.py')

cases=[
 ('final_gate_regenerates_assessment_after_index', "('assessment-report'" in run_final and run_final.index("'deterministic-index'") < run_final.index("'assessment-report'") < run_final.index("'sv-deep'")),
 ('final_gate_uses_external_log_before_index', "tempfile.gettempdir()" in run_final and "log_hygiene=outside_indexed_root" in run_final),
 ('two_pass_idempotence_gate_exists', exists('scripts/vac-final-idempotence-sv.py') and 'first != second' in read('scripts/vac-final-idempotence-sv.py')),
 ('broker_session_wires_bootstrap', 'VacRuntimeMetadataBootstrap::new' in session_actor and 'set_vac_runtime_metadata(&mut initial_metadata' in session_actor),
 ('session_json_artifact_compiler_exists', 'compile_session_runtime_artifacts' in bootstrap and 'plan.json' in bootstrap and 'artifacts.json' in bootstrap and 'closeout.json' in bootstrap),
 ('runtime_policy_delegates_to_vac_policy', 'vac_policy::PolicySnapshot' in bound_runtime and 'evaluate_runtime_policy' in bound_runtime and 'policy_snapshot' in bound_runtime),
 ('policy_workspace_project_patterns_match_any', '"workspace" | "project"' in policy),
 ('compiled_policy_snapshot_emitted', 'policy_snapshot' in compiler and 'compile_policy_snapshot' in compiler and 'default_decision' in compiler),
 ('validation_commands_no_bash_runner', 'runner: bash' not in ''.join(p.read_text(errors='replace') for p in (ROOT/'.vac').rglob('*.yaml'))),
 ('script_runner_bindings_compiled', 'script_runner_bindings' in compiler and 'script_sha256' in compiler and 'vac-script-runner' in compiler),
 ('mcp_approval_recomputes_binding', 'recompute_vac_bound_binding_hash' in local_tools and 'verify_vac_bound_approval' in local_tools and 'binding_hash mismatch' in local_tools),
 ('mcp_read_ticket_validates_index_path', 'verify_vac_read_plan_ticket' in local_tools and '.vac/index/read_plans.jsonl' in local_tools),
 ('assessment_full_join_expanded', 'intent_without_code' in assessment_script and 'code_without_intent' in assessment_script and 'baseline_to_code' in assessment_script and 'call_path_impact' in assessment_script),
 ('evidence_log_freshness_gate_wired', 'check_evidence_log_freshness' in integrity and 'VAC evidence log freshness' in read('scripts/check-evidence-log-freshness.py')),
 ('checkpoint_manifest_records_state5', 'state5-operational-closure' in manifest_gen and 'assessment_summary' in manifest_gen),
]
failed=[name for name,ok in cases if not ok]
print(json.dumps({'kind':'vac_runtime_state5_operational_sv','cases':[{'id':n,'pass':ok} for n,ok in cases]}, indent=2))
if failed:
    print('VAC runtime State5 operational SV: FAIL')
    for name in failed:
        print(f'- {name}')
    sys.exit(1)
print('VAC runtime State5 operational SV: PASS')
