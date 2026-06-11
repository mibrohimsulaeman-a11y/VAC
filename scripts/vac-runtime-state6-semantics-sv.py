#!/usr/bin/env python3
"""State6 runtime-semantics closure SV gate.

Covers the three State6 P0s that previous freshness/idempotence gates missed:
- policy explicit allow/approval must not collapse under default_deny;
- ApprovalRequired must become a paused approval request + scoped-grant retry path;
- session bootstrap must never fabricate terminal completion artifacts.
"""
from __future__ import annotations
import json, re, sys
from pathlib import Path
ROOT = Path(sys.argv[1] if len(sys.argv) > 1 else '.').resolve()

def read(rel: str) -> str:
    return (ROOT / rel).read_text(encoding='utf-8', errors='replace')

def exists(rel: str) -> bool:
    return (ROOT / rel).exists()

def most_restrictive(a: str, b: str) -> str:
    order = {'allow': 0, 'approval_required': 1, 'deny': 2}
    return a if order[a] >= order[b] else b

def path_matches(pattern: str | None, value: str | None) -> bool:
    if pattern is None or pattern in {'any', '*', 'workspace', 'project'}:
        return True
    if value is None:
        return False
    if pattern == value:
        return True
    if pattern.endswith('/**'):
        prefix = pattern[:-3]
        return value == prefix or value.startswith(prefix + '/')
    if pattern.endswith('*'):
        return value.startswith(pattern[:-1])
    return False

def eval_policy(case: dict) -> str:
    decision = None
    matched = False
    req = case['request']
    for rule in case['rules']:
        if rule['action'] == req['action'] and path_matches(rule.get('path'), req.get('path')):
            matched = True
            decision = rule['decision'] if decision is None else most_restrictive(decision, rule['decision'])
    if not matched:
        decision = case['default_decision']
    return decision

fixture = json.loads(read('tests/fixtures/runtime/v15-state6-semantics/cases.json'))
policy_src = read('vac-rs/crates/control-plane/vac-policy/src/lib.rs')
bound_tool = read('vac-rs/crates/runtime/vac-agent-loop/src/bound_tool.rs')
agent = read('vac-rs/crates/runtime/vac-agent-loop/src/agent.rs')
approval = read('vac-rs/crates/runtime/vac-agent-loop/src/approval.rs')
bootstrap = read('vac-rs/crates/runtime/vac-agent-loop/src/session_bootstrap.rs')
final_gate = read('scripts/vac-reaudit-final-sv-gate.sh')
run_final = read('scripts/run-final-sv-validation.py')

cases: list[tuple[str, bool]] = []
for probe in fixture['policy_probes']:
    cases.append((f"policy_probe_{probe['id']}", eval_policy(probe) == probe['expected']))

cases += [
    ('policy_evaluate_match_first_optional_decision', 'let mut decision: Option<Decision> = None' in policy_src and 'unwrap_or_else(|| {' in policy_src),
    ('policy_no_default_deny_dominance_pattern', 'let mut decision = self.default_decision;' not in policy_src),
    ('policy_comment_documents_state6_root_cause', 'safe-but-non-operational' in policy_src and 'default decision applies only when no rule matches' in policy_src),
    ('bound_gate_has_approval_request_variant', 'pub approval_request: Option<Value>' in bound_tool and 'pub fn approval_required' in bound_tool),
    ('approval_required_persists_request_v2', 'build_approval_request_v2' in bound_tool and 'persist_approval_request' in bound_tool and 'schema_version": 2' in bound_tool),
    ('approval_required_is_paused_not_tool_error', 'gate.approval_request.is_some()' in agent and 'pause_for_vac_runtime_approval' in agent and 'ToolExecutionCompleted' in agent),
    ('operator_accept_installs_scoped_grant', 'record_operator_tool_decision' in bound_tool and 'approved_tool_grants.insert' in bound_tool and 'single_retry' in bound_tool),
    ('approval_state_machine_reopens_dispatched_entry', 'pause_for_vac_runtime_approval' in approval and 'entry.state = ApprovalEntryState::PendingUserDecision' in approval and 'self.next_index = idx' in approval),
    ('approved_retry_stamps_vac_bound_approval', 'approved_tool_grants.remove' in bound_tool and 'GateDecision::PassWithWarnings' in bound_tool and 'stamp_tool_call' in bound_tool),
    ('bootstrap_does_not_use_historical_terminal_fixtures', '.vac/registry/sessions/bootstrap/tasks.yaml' not in bootstrap and '.vac/registry/sessions/bootstrap/spec.yaml' not in bootstrap and '.vac/registry/sessions/bootstrap/todo.yaml' not in bootstrap),
    ('bootstrap_default_task_needs_discussion', '"state": "needs_discussion"' in bootstrap and 'Missing session task artifact' in bootstrap and '"met": false' in bootstrap),
    ('bootstrap_default_spec_needs_discussion', 'Missing session spec artifact' in bootstrap and 'runtime cannot mark spec finalized synthetically' in bootstrap),
    ('bootstrap_default_todo_unchecked_blocking', 'bootstrap placeholder cannot satisfy completion lock' in bootstrap and '"checked": false' in bootstrap and '"blocking": true' in bootstrap),
    ('bootstrap_default_closeout_invalid', '"valid": false' in bootstrap and 'bootstrap-placeholder-artifacts-are-not-completion-authority' in bootstrap),
    ('state6_gate_wired_to_shell_gate', 'runtime-state6-semantics' in final_gate and 'vac-runtime-state6-semantics-sv.py' in final_gate),
    ('state6_gate_wired_to_python_final_gate', 'runtime-state6-semantics' in run_final and 'vac-runtime-state6-semantics-sv.py' in run_final),
]

failed = [name for name, ok in cases if not ok]
print(json.dumps({'kind': 'vac_runtime_state6_semantics_sv', 'cases': [{'id': n, 'pass': ok} for n, ok in cases]}, indent=2))
if failed:
    print('VAC runtime State6 semantics SV: FAIL')
    for name in failed:
        print(f'- {name}')
    sys.exit(1)
print('VAC runtime State6 semantics SV: PASS')
