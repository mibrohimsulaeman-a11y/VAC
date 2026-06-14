# VAC Confirmed Intent Coverage Audit

Status: **confirmed_intent_domain_coverage=SV-Pass**, **confirmed_intent_traceability_gate=SV-Pass**, **confirmed_intent_negative_fixtures=SV-Pass**, **all_negative_cases_rejected=true**, **crate_without_intent_or_rationale=0**, **external_provider_remote_process_io_e2e=TV-Pending**.

This audit closes the P1 confirmed-intent semantic authority gap for the large runtime/provider/integration/surface domains that previously had capability manifests but no domain-specific confirmed intent baseline. It does not claim full P1 or release acceptance. It only proves that each named domain has explicit should-be semantics, capability/rationale binding, acceptance invariants, and traceability coverage that can be consumed by static assessment.

## Coverage inventory

| Domain | Crate/path | Current capability manifest | Current confirmed intent spec | Coverage status | Risk class | Recommended action |
| --- | --- | --- | --- | --- | --- | --- |
| vac-broker | `vac-rs/crates/runtime/vac-broker` | `vac.runtime.broker (.vac/capabilities/vac-runtime-broker.yaml)` | `.vac/specs/confirmed/vac-broker-intent.yaml` | SV-Pass | High: broker/L2 claim boundary | Keep SV-only intent coverage until broker-mediated proof material exists; do not upgrade L1 records to L2 claims. |
| vac-provider-core | `vac-rs/crates/providers/vac-provider-core`<br>`vac-rs/crates/foundation/vac-foundation/src/models/provider_core_adapter.rs` | `vac.providers.model_core (.vac/capabilities/vac-providers-model_core.yaml)` | `.vac/specs/confirmed/vac-provider-core-intent.yaml` | SV-Pass | High: provider stream/runtime truth boundary | Keep provider compatibility and external-provider credential paths TV-Pending until live provider fixture exists. |
| vac-mcp-server | `vac-rs/crates/integrations/vac-mcp-server`<br>`scripts/pty-vac-cli-real-io-e2e.py` | `vac.integrations.mcp (.vac/capabilities/vac-integrations-mcp.yaml)` | `.vac/specs/confirmed/vac-mcp-server-intent.yaml` | SV-Pass | High: MCP local tool execution boundary | Maintain local real-provider/MCP TV-Pass only for sandboxed local IO; leave remote/external provider claims pending. |
| vac-messaging-gateway | `vac-rs/crates/integrations/vac-messaging-gateway` | `vac.integrations.messaging_gateway (.vac/capabilities/vac-integrations-messaging_gateway.yaml)` | `.vac/specs/confirmed/vac-messaging-gateway-intent.yaml` | SV-Pass | Medium: notification/secrets boundary | Add token redaction and delivery-failure tests before upgrading beyond SV. |
| vac-remote-service | `vac-rs/crates/integrations/vac-remote-service` | `vac.integrations.remote_service (.vac/capabilities/vac-integrations-remote_service.yaml)` | `.vac/specs/confirmed/vac-remote-service-intent.yaml` | SV-Pass | High: remote execution/credential boundary | Keep remote process IO as TV-Pending until remote fixture or credentialed proof exists. |
| vac-autopilot | `vac-rs/crates/runtime/vac-autopilot`<br>`vac-rs/crates/surfaces/vac-cli/src/commands/autopilot` | `vac.runtime.agent_loop (.vac/capabilities/vac-runtime-agent_loop.yaml)`<br>`vac.surfaces.cli (.vac/capabilities/vac-surfaces-cli.yaml)` | `.vac/specs/confirmed/vac-autopilot-intent.yaml` | SV-Pass | Medium-high: scheduled/continuous orchestration boundary | Add config reload and process expansion tests before moving to TV-Pass. |
| vac-tui-bridge | `vac-rs/crates/surfaces/vac-tui/src/event_loop.rs`<br>`vac-rs/crates/surfaces/vac-tui/src/services/handlers/dialog.rs`<br>`vac-rs/crates/surfaces/vac-tui/src/services/handlers/tool.rs`<br>`vac-rs/crates/surfaces/vac-tui/src/services/handlers/shell.rs`<br>`scripts/pty-tui-agent-tool-lifecycle-smoke.py` | `vac.surfaces.tui (.vac/capabilities/vac-surfaces-tui.yaml)` | `.vac/specs/confirmed/vac-tui-bridge-intent.yaml` | SV-Pass | High: user-visible lifecycle truth boundary | Bridge intent is SV-Pass; production maturity remains pending until bridge files get direct unit coverage. |

## Rationale ledger

| Domain | Rationale |
| --- | --- |
| vac-broker | Broker is a large enforcement-boundary crate; confirmed intent prevents L1/L2 semantic overclaim. |
| vac-provider-core | Provider stream normalization is semantic authority for model/tool events but must not authorize tools or hardcode runtime truth. |
| vac-mcp-server | MCP server is a local tool execution boundary; confirmed intent pins approval binding and network/process semantics. |
| vac-messaging-gateway | Messaging gateway is notification-only; intent prevents notification delivery from becoming authority or evidence custody upgrade. |
| vac-remote-service | Remote service must distinguish configured remote execution from local success and keep remote process E2E pending without proof. |
| vac-autopilot | Autopilot is a scheduled/continuous orchestration domain; confirmed intent prevents it from becoming a bypass path around VAC gates. |
| vac-tui-bridge | TUI bridge is the user-visible agent/tool lifecycle surface; confirmed intent prevents mock runtime truth and hidden lifecycle states. |

## Explicit non-claims

- P1 acceptance=NotClaimed.
- release_ready=NotClaimed.
- external_provider_remote_process_io_e2e=TV-Pending.
- remote_process_io_e2e=TV-Pending.
- broker_attested_l2_enforcement=TV-Pending.

## Gate contract

`scripts/check-confirmed-intent-coverage.py` reads `tests/fixtures/confirmed-intent/domain-map.json` and fails if a required intent spec, domain row, capability/rationale, acceptance invariant, fixture/gate mapping, or pending remote/external status is missing. The gate is intentionally token-based rather than prose-exact so wording can evolve while authority coverage remains stable.

## Negative fixture hardening

`scripts/check-confirmed-intent-negative-fixtures.py` verifies that bad confirmed-intent states fail closed. Covered negative cases: `missing_spec`, `missing_traceability_row`, `missing_required_invariant`, `tv_pass_without_fixture`, `remote_io_overclaim`, and `crate_without_intent_or_rationale`. Golden snapshots preserve honest wording around `SV-Pass`, `TV-Pending`, `NotEvaluated`, `NotImplemented`, and `Not claimed`.

```text
confirmed_intent_negative_fixtures=SV-Pass
all_negative_cases_rejected=true
```
