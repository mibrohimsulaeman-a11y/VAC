#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

python3 - <<'PY'
from pathlib import Path
fail = False

def require(path, needle):
    global fail
    text = Path(path).read_text(errors='ignore')
    if needle not in text:
        print(f'MISSING {path}: {needle}')
        fail = True

def reject(path, needle):
    global fail
    text = Path(path).read_text(errors='ignore')
    if needle in text:
        print(f'FORBIDDEN {path}: {needle}')
        fail = True

owner = 'vac-rs/crates/surfaces/tui/src/runtime_owner_session.rs'
support_contract = 'vac-rs/crates/surfaces/tui/src/owner_native_runtime_support.rs'
parity = 'vac-rs/crates/surfaces/tui/src/owner_native_operation_parity.rs'
init = 'vac-rs/crates/surfaces/cli/src/init_cli.rs'
callsite = 'vac-rs/crates/capabilities/build/src/core_migrated/runtime_gate_callsite_integration.rs'
unified = 'vac-rs/crates/capabilities/tools-domain/src/core_migrated/tools/handlers/unified_exec.rs'


for needle in [
    'pub(crate) enum OwnerRuntimeMethodStatus',
    'pub(crate) const OWNER_RUNTIME_METHOD_SUPPORT',
    'release_blocking_owner_runtime_methods',
    '"turn_start"',
    'OwnerRuntimeMethodSupport::fail_closed("thread_rollback"',
    'OwnerRuntimeMethodSupport::fail_closed("thread_realtime_start"',
    'OwnerRuntimeMethodSupport::fail_closed("resolve_server_request"',
    'OwnerRuntimeMethodSupport::fail_closed("reject_server_request"',
]:
    require(support_contract, needle)

for needle in [
    'status: OwnerNativeOperationStatus::NonDefaultFailClosed,',
    'plan30_non_default_fail_closed_is_limited_to_noncritical_controls',
    'thread_realtime_start',
    'thread_rollback',
    'resolve_server_request',
    'reject_server_request',
]:
    require(parity, needle)

for needle in [
    'AuthManager::shared_from_config',
    'ThreadManager::new',
    'thread_store_from_config',
    'start_thread_with_session_start_source',
    'thread_manager.start_thread(config)',
    'Op::UserTurn',
    'Op::Interrupt',
    'Op::RunUserShellCommand',
    'ModeKind::Plan',
    'validate_vac_init_plan_yaml_with_engine',
    '.vac/registry/runtime/active_plan.yaml',
    'active_runtime_plan_path',
    'release_blocking_owner_runtime_methods()',
    'OwnerRuntimeMethodStatus::Implemented',
    '.list_threads(Self::store_list_params(params))',
    '.read_thread(StoreReadThreadParams',
    'resume_thread_with_history',
    'InitialHistory::Resumed',
    'InitialHistory::Branched',
    'enforce_plan_mode_runtime_completion_gate',
    'forward_thread_events',
    'TurnStartedNotification',
    'TurnCompletedNotification',
]:
    require(owner, needle)
reject(owner, 'let _required_owner_runtime_methods')

owner_text = Path(owner).read_text(errors='ignore')
if owner_text.count('pub(crate) plan_type: Option<PlanType>,') != 1:
    print('FORBIDDEN vac-rs/crates/surfaces/tui/src/runtime_owner_session.rs: duplicate AppServerBootstrap.plan_type field')
    fail = True

for op in [
    'start_thread_with_session_start_source',
    'turn_start',
    'turn_steer',
    'turn_interrupt',
    'startup_interrupt',
    'thread_shell_command',
    'thread_list',
    'thread_read',
    'resume_thread',
    'branch_thread',
    'read_account',
]:
    reject(owner, f'unsupported_report("{op}")')
    reject(owner, f'unsupported_typed("{op}")')

for needle in [
    'build_init_doctor_report',
    'doctor_passed',
    'status: {status}',
    'runtime_owner',
    'command_gate_evidence',
    'plan_mode_runtime_gate',
    'DoctorFailed',
    '.vac/registry/runtime/owner-native-support.yaml',
    'owner_runtime_support_manifest',
]:
    require(init, needle)
reject(init, 'pending_runtime_cli')
reject(init, 'runtime_owner_text')
reject(init, 'unsupported_report(\"turn_start\")')

for needle in [
    'evaluate_vac_init_pre_plan_gate',
    'evaluate_vac_init_pre_patch_gate',
    'evaluate_vac_init_pre_command_gate',
    'evaluate_vac_init_evidence_completion_gate',
]:
    require(callsite, needle)

require(unified, 'evaluate_vac_init_runtime_command_contract')
require(unified, 'write_vac_init_runtime_command_evidence')

active_plan = Path('.vac/registry/runtime/active_plan.yaml')
if not active_plan.exists():
    print('MISSING .vac/registry/runtime/active_plan.yaml: runtime Plan Mode has no active semantic plan to validate')
    fail = True
else:
    try:
        import yaml
        plan = yaml.safe_load(active_plan.read_text()) or {}
        allowed = plan.get('allowed_files') or []
        create_count = sum(1 for item in allowed if (item or {}).get('operation') == 'create')
        bounds = plan.get('bounds') or {}
        max_new_files = int(bounds.get('max_new_files', -1))
        if max_new_files < create_count:
            print(f'INVALID active plan bounds: max_new_files={max_new_files} < create_count={create_count}')
            fail = True
        if not str(plan.get('id', '')).startswith('plan.'):
            print('INVALID active plan id: must start with plan.')
            fail = True
        if plan.get('task', {}).get('capability') != 'vac.local_runtime_owner':
            print('INVALID active plan capability: expected vac.local_runtime_owner')
            fail = True
    except Exception as exc:
        print(f'INVALID .vac/registry/runtime/active_plan.yaml: {exc}')
        fail = True


owner_support = Path('.vac/registry/runtime/owner-native-support.yaml')
if not owner_support.exists():
    print('MISSING .vac/registry/runtime/owner-native-support.yaml: init/doctor has no structural owner-native support contract')
    fail = True
else:
    support = owner_support.read_text(errors='ignore')
    for method in [
        'start_thread_with_session_start_source', 'turn_start', 'turn_steer',
        'turn_interrupt', 'startup_interrupt', 'thread_shell_command',
        'thread_list', 'thread_read', 'resume_thread', 'branch_thread', 'read_account',
    ]:
        if f'name: {method}' not in support:
            print(f'MISSING owner-native support method: {method}')
            fail = True
    for needle in [
        'kind: runtime_owner_support', 'status: ready', 'pre_gate: implemented',
        'post_gate: implemented_static', 'evidence_completion_gate: implemented',
        'ThreadStore::read_thread', 'ThreadStore::list_threads',
        'source_contract: vac-rs/crates/surfaces/tui/src/owner_native_runtime_support.rs',
        'parity_registry: vac-rs/crates/surfaces/tui/src/owner_native_operation_parity.rs',
    ]:
        if needle not in support:
            print(f'MISSING owner-native support manifest anchor: {needle}')
            fail = True

    import re
    contract = Path(support_contract).read_text(errors='ignore')
    for method in [
        'start_thread_with_session_start_source', 'turn_start', 'turn_steer',
        'turn_interrupt', 'startup_interrupt', 'thread_shell_command',
        'thread_list', 'thread_read', 'resume_thread', 'branch_thread', 'read_account',
    ]:
        if f'"{method}"' not in contract:
            print(f'MISSING owner-native support code contract method: {method}')
            fail = True
        if f'name: {method}' not in support:
            print(f'MISSING owner-native support manifest method: {method}')
            fail = True
    for method in ['thread_rollback', 'thread_realtime_start', 'resolve_server_request', 'reject_server_request']:
        if f'fail_closed("{method}"' not in contract:
            print(f'MISSING owner-native fail-closed code contract method: {method}')
            fail = True
        if f'name: {method}' not in support or 'non_default_fail_closed' not in support:
            print(f'MISSING owner-native fail-closed manifest method/status: {method}')
            fail = True

doctor_gate = Path('vac-rs/crates/surfaces/cli/src/doctor/runtime_owner_gates.rs').read_text(errors='ignore')
for needle in [
    'validate_owner_native_support_manifest', 'CRITICAL_OWNER_NATIVE_METHODS',
    'owner_native_support_contract_missing', 'owner_native_contract_method_missing',
    'owner_native_parity_registry_missing', 'owner_native_method_not_implemented',
]:
    if needle not in doctor_gate:
        print(f'MISSING runtime-owner doctor structural gate: {needle}')
        fail = True

if fail:
    print('runtime agent flow fix static: FAIL')
    raise SystemExit(1)
print('runtime agent flow fix static: PASS')
print('TV-PENDING: cargo build/test and live interactive TUI turn are not evaluated in this sandbox')
PY
