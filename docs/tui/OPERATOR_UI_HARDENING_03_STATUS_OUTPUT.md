# Operator UI Hardening 03 — `/status` Output Contract

Status: implemented in `vac-source-tui-operator-ui-hardening3.zip`  
Scope: `/status` runtime display, status snapshots, and lightweight validation gates.

## Purpose

Hardening 03 locks `/status` as an operator-safe diagnostic surface. The command must remain useful for model/provider/runtime inspection without showing legacy marketing/account rows for rate limits, credits, or external visit prompts.

## Required Display Fields

The runtime status card must preserve:

- `Model`
- `Model provider` always, including the default `vastar` provider
- `Model providers` when a multi-provider registry is available
- `Directory`
- `Permissions`
- `Agents.md`
- `Account` when account metadata is available
- `Thread name`, `Session`, `Branched from`, and `Collaboration mode` when present
- `Token usage`
- `Context window` when the runtime/model exposes a context window

These labels are now centralized in `vac-rs/tui/src/status/output_contract.rs` through `StatusDisplayField` so the renderer and static gate share the same contract vocabulary.

## Removed Display Rows

The status card must not render:

- `Visit ...` account/rate-limit prompt text
- `rate limits and credits` marketing text
- `Credits:`
- `Credit balance`
- `Limits:` fallback rows

The runtime card intentionally no longer calls the old rate-limit display helper path. Protocol rate-limit snapshots may still be cached for background/status-line purposes, but `/status` display no longer presents those rows.

## Local-only Refresh Policy

`/status` is now local-only. The slash command renders provider/model/token/context metadata immediately and does **not** emit `AppEvent::RefreshRateLimits`, even for ChatGPT-authenticated sessions. This keeps `/status` from reintroducing rate-limit or credit semantics through asynchronous refresh completion. Ambient rate-limit warnings remain owned by the status-line and nudge surfaces.

`add_status_output` also passes an explicit empty account-limit snapshot set into the status card. Cached background limit state can still support non-status surfaces, but it cannot leak into `/status` rows.

## Multi-provider/model Readiness

`StatusProviderModelUsage` is introduced as a lightweight contract type for future registry-backed rows. It supports active and available provider/model entries with optional token/context summaries. The current `/status` card renders the active model/provider path unconditionally and adds a compact `Model providers` registry row when `config.model_providers` contains multiple entries. The row marks the active provider with `*`, includes provider display names and wire API, and truncates long registries with a `+N more` suffix.

## Validation

Primary gates:

```bash
bash scripts/check-tui-status-output-contract.sh
bash scripts/check-tui-status-deep-hardening-contract.sh
```

Targeted test gate:

```bash
rustc --edition 2024 --test vac-rs/tui/src/status/output_contract.rs -o /tmp/vac-status-output-contract-test
/tmp/vac-status-output-contract-test
```

The gate checks:

- `output_contract` is registered in `status/mod.rs`.
- required fields are represented in `StatusDisplayField`.
- default `vastar` provider display is not hidden.
- the multi-provider registry formatter is present and wired into the status card.
- multi-provider/model support type exists.
- runtime card uses contract labels for model/provider/token/context/directory/permissions.
- obsolete rate-limit/credit helper path is absent from the runtime card.
- `/status` slash dispatch does not initiate `RefreshRateLimits`.
- `add_status_output` ignores cached account-limit snapshots.
- status snapshots do not contain removed `Limits:` / `Credits:` rows.

## Policy Lock Added in Hardening 3

`StatusDisplayPolicy` makes the no-quota/no-balance behavior explicit instead of relying only on absence of strings. `STATUS_OPERATOR_DISPLAY_POLICY` must keep `show_rate_quota_rows` and `show_credit_balance_rows` set to `false`.
