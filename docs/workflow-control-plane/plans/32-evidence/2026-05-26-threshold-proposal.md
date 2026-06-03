# Plan 32 L25 Threshold Proposal — 2026-05-26

## Scope

Docs-only preparation for tightening `vac doctor runtime-owner-gates <root>` from informational warnings toward CI-breaking owner gates.

Edited files in this slice:

- `docs/workflow-control-plane/plans/32-vac-runtime-owner-gates.md`
- `docs/workflow-control-plane/plans/32-evidence/2026-05-26-threshold-proposal.md`

No Rust source, `.vac` manifests, or non-32 plan/evidence files are changed by this proposal.

## Inputs reviewed

- `vac-rs/cli/src/doctor/runtime_owner_gates.rs`
- `.vac/capabilities/local_runtime_owner.yaml`
- `.vac/capabilities/tui_session_runtime.yaml`
- `.vac/capabilities/runtime_approval_bridge.yaml`

## Current implementation taxonomy

`runtime_owner_gates.rs` currently treats these as hard errors:

| Code | Meaning |
| --- | --- |
| `missing_required_manifest` | The required `local_runtime_owner` manifest is absent. |
| `manifest_read_failed` | A checked manifest could not be read from disk. |
| `manifest_yaml_invalid` | A checked manifest is not valid YAML. |

It currently treats these as warnings:

| Code | Meaning |
| --- | --- |
| `missing_field` | A checked manifest lacks one of the required scaffold fields. |
| `missing_source_domain` | A checked manifest claims an owner/docs/compatibility path, crate, or module that does not exist in the current source tree. |

The command still renders `mode: informational (warning-level gate thresholds pending)` and exits non-zero only for hard errors.

## Proposed warning-to-error schedule

| Finding code | Proposed schedule | Acceptance criteria |
| --- | --- | --- |
| `missing_field` | Promote in Plan 32F, after this proposal is merged and the three capability manifests still pass with zero warnings. | Any checked Plan 32 manifest missing `schema_version`, `kind`, `id`, `title`, `status`, `owner`, `ownership`, `policy`, or `validation` fails the doctor and exits non-zero. |
| `missing_source_domain` | Promote in two stages: Plan 32F for canonical local owner/doc/schema claims; after Plan 31C/31D for active TUI session and approval bridge compatibility claims. | Any canonical owner/docs/compatibility path, ownership crate, or ownership target module in a checked manifest must exist and point at the intended source domain. Transitional compatibility claims remain warning-only until their linked owner slice lands. |
| `app_server_dependency_present` (new code) | Add as warning in Plan 32G; promote only after Plan 31E proves active `vac-app-server-protocol`/`vac-app-server-client` dependents are 0 and Plan 33 accepts delete/defer evidence. | A reintroduced app-server dependency in active local runtime owner or TUI runtime paths fails the gate after no-app-server evidence is green. |
| `pty_false_green` (new code) | Add as warning/blocker while Plan 30F/30G evidence remains `BLOCKED-OPERATOR`; promote after real-TTY evidence exists. | The PTY gate cannot report green without a real TTY proof. Synthetic or blocked operator evidence must remain blocked/fail. |
| `message_processor_copy` (new code) | Add as hard error in the first Plan 32 policy-wiring slice that scans active runtime-owner/TUI domains. | Copying or recreating `MessageProcessor` in active owner/TUI paths fails the gate. Migration must replace behavior with owner-native request/event handling, not a facade clone. |
| `unsupported_control_default_defer` (new code) | Add as warning until Plan 30G closes or explicitly defers skills/plugins/external-agent provider gaps; promote after those controls are owner-native or documented non-default defers. | Unsupported controls must be blockers, owner-native, or explicit non-default defers; they cannot be silently default-enabled. |

## Preconditions by linked plan

- **Plans 02–07:** Required manifest fields and registry/schema conventions are stable enough that `missing_field` can become CI-breaking without fake-red manifests.
- **Plan 30G:** Skills, plugin, and external-agent owner provider gaps are either complete or explicitly marked as non-default defers before unsupported-control thresholds become hard errors.
- **Plan 31C/31D:** Active TUI request/response and event/notification compatibility seams have owner-native targets before source-domain warnings for those seams become hard errors.
- **Plan 31E:** The app-server protocol/client quarantine is deleted or reduced to documented non-default defers before no-app-server dependency thresholds become hard errors.
- **Plan 33:** Final delete/defer proof accepts the inverse Cargo tree and textual evidence before app-server dependency regressions become CI-breaking.

## Recommended next implementation slices

1. **32F — scaffold hardening:** promote `missing_field` and canonical `missing_source_domain` findings to hard errors; keep transitional TUI/approval compatibility claims warning-only if their linked Plan 31 rows are still pending.
2. **32G — no-app-server warning gate:** add an informational/warning code for active app-server dependency presence, but keep it non-CI-breaking until Plan 31E and Plan 33 prove zero active dependents.
3. **32H — policy semantics:** add `pty_false_green`, `message_processor_copy`, and `unsupported_control_default_defer` checks with the staged thresholds described above.

## Validation for this docs-only slice

No Cargo/Rust validation is required because this slice changes docs only.

Use proportional docs validation:

```bash
git diff --check -- docs/workflow-control-plane/plans/32-vac-runtime-owner-gates.md docs/workflow-control-plane/plans/32-evidence/2026-05-26-threshold-proposal.md
rg -n "Threshold proposal|missing_field|missing_source_domain|app_server_dependency_present|pty_false_green|MessageProcessor" docs/workflow-control-plane/plans/32-vac-runtime-owner-gates.md docs/workflow-control-plane/plans/32-evidence/2026-05-26-threshold-proposal.md
```
