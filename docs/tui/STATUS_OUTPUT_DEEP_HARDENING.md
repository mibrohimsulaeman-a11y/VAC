# TUI `/status` Deep Hardening Contract

Status: `ready`  
Owner: `vac-tui/status`  
Capability: `vac.tui.status-output`

## Goal

The `/status` operator surface must remain useful for runtime diagnosis while no longer exposing the legacy account upsell/limit display rows.

The status card must keep:

- active model,
- active model provider, always visible even for the default `vastar` provider,
- directory,
- permissions,
- account/session/thread metadata when available,
- token usage,
- context window.

The status card must not render:

- `Visit ... rate limits ... credits`,
- `Credits:` or credit balance rows,
- `Limits:` rows,
- rate-limit helper text.

Rate-limit protocol data may still be fetched for internal/status-line compatibility. It is no longer an operator-facing `/status` card section.

## Implementation Shape

Hardening 3 adds `vac-rs/tui/src/status/output_contract.rs` as the dependency-light status display contract. Runtime rendering imports `StatusDisplayField` from that module instead of duplicating labels or depending on obsolete row names.

The contract also defines `StatusProviderModelUsage`, which is the forward-compatible shape for multi-provider/model status rows. The runtime card now has an explicit `Model provider` row for the active provider and a conditional `Model providers` registry row when multiple providers are present in `config.model_providers`. The validator ensures at least one model is active and every row has non-empty provider/model identity.

## Validation

Primary gate:

```bash
bash scripts/check-tui-status-output-contract.sh
```

Targeted isolated test:

```bash
rustc --edition 2024 --test vac-rs/tui/src/status/output_contract.rs -o /tmp/vac-status-output-contract-test
/tmp/vac-status-output-contract-test --nocapture
```

Regression lock:

```bash
bash scripts/check-tui-hardening-regression-lock.sh
```

## Acceptance Criteria

- `StatusDisplayField` is the status label source of truth.
- `StatusProviderModelUsage` supports active/inactive model rows across providers.
- `Model provider` is a required row; default `vastar` is no longer hidden.
- `Model providers` appears when the provider registry contains multiple entries.
- `/status` card has no legacy `Limits`/`Credits` render methods.
- Existing status snapshots have no legacy limit/credit rows.
- Token usage and context window rows remain visible.

## Display Policy Lock

`StatusDisplayPolicy::operator_safe()` is the explicit runtime guard for Hardening 3. It keeps model/provider/token/context/account/session rows enabled while keeping quota and balance rows disabled. The status card debug-asserts this policy when constructing status output.

The slash dispatcher also asserts `status_command_requests_rate_limit_refresh() == false`, so `/status` cannot accidentally re-enter the old asynchronous account-limit refresh path.
