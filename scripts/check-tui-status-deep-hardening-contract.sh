#!/usr/bin/env bash
# Static gate for Hardening 3: /status deep hardening.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

fail() { echo "FAIL: $*" >&2; exit 1; }

status_mod="vac-rs/tui/src/status/mod.rs"
status_card="vac-rs/tui/src/status/card.rs"
status_contract="vac-rs/tui/src/status/output_contract.rs"
slash_dispatch="vac-rs/tui/src/chatwidget/slash_dispatch.rs"
chatwidget="vac-rs/tui/src/chatwidget.rs"
app_rs="vac-rs/tui/src/app.rs"
status_snapshots="vac-rs/tui/src/status/snapshots"

for file in "$status_mod" "$status_card" "$status_contract" "$slash_dispatch" "$chatwidget" "$app_rs"; do
  [ -f "$file" ] || fail "missing $file"
done
[ -d "$status_snapshots" ] || fail "missing $status_snapshots"

grep -q 'struct StatusDisplayPolicy' "$status_contract" || fail "StatusDisplayPolicy missing"
grep -q 'STATUS_OPERATOR_DISPLAY_POLICY' "$status_contract" || fail "operator display policy missing"
grep -q 'show_rate_quota_rows: false' "$status_contract" || fail "operator display policy must reject quota rows"
grep -q 'show_credit_balance_rows: false' "$status_contract" || fail "operator display policy must reject balance rows"
grep -q 'status_operator_display_policy_rejects_quota_and_balance_rows' "$status_contract" || fail "display policy regression test missing"
grep -q 'status_command_requests_rate_limit_refresh' "$status_mod" || fail "status refresh policy helper not exported"
grep -q 'debug_assert!(!crate::status::status_command_requests_rate_limit_refresh())' "$slash_dispatch" || fail "/status slash dispatch does not assert local-only refresh policy"
grep -q 'self.add_status_output(' "$slash_dispatch" || fail "/status slash dispatch missing status output call"
grep -q '/\*refreshing_rate_limits\*/ false' "$slash_dispatch" || fail "/status slash dispatch must disable refresh flag"
grep -q '/\*request_id\*/ None' "$slash_dispatch" || fail "/status slash dispatch must avoid refresh request id"

grep -q 'let rate_limit_snapshots: &\[RateLimitSnapshotDisplay\] = &\[\];' "$chatwidget" || fail "/status output must pass empty limit snapshots"
grep -q 'let (cell, _handle)' "$chatwidget" || fail "/status output must ignore legacy refresh handle"
if grep -q 'refreshing_status_outputs.push' "$chatwidget"; then
  fail "/status output still registers refresh handles"
fi
if grep -q 'Run /status for a breakdown' "$chatwidget"; then
  fail "ambient limit warning still claims /status renders quota breakdown"
fi
if grep -q 'first `/status`.*already has data' "$app_rs"; then
  fail "startup prefetch comment still couples account limit cache to /status"
fi

grep -q 'format_model_provider_registry' "$status_card" || fail "multi-provider/model registry summary missing"
grep -q 'StatusDisplayField::ModelProviders.label()' "$status_card" || fail "provider/model registry status label missing"
grep -q 'format_model_provider(config' "$status_card" || fail "model provider formatter missing"
grep -q 'StatusDisplayField::TokenUsage.label()' "$status_card" || fail "token usage status label missing"
grep -q 'StatusDisplayField::ContextWindow.label()' "$status_card" || fail "context window status label missing"
grep -q 'STATUS_OPERATOR_DISPLAY_POLICY.permits_rate_quota_rows' "$status_card" || fail "status card does not assert quota-row display policy"
grep -q 'STATUS_OPERATOR_DISPLAY_POLICY.permits_credit_balance_rows' "$status_card" || fail "status card does not assert balance-row display policy"

if grep -Eq 'fn rate_limit_lines|fn rate_limit_row_lines|fn collect_rate_limit_labels|formatter\.line\("Limits"|formatter\.line\("Credits"' "$status_card"; then
  fail "/status card still contains legacy limit/credit row renderer"
fi
if grep -RIn -E 'Limits:|Visit .*rate limits|rate limits and credits|Credits:|Credit balance' "$status_snapshots" >/tmp/vac-status-deep-forbidden.txt; then
  cat /tmp/vac-status-deep-forbidden.txt >&2
  fail "/status snapshots still contain removed quota/balance rows"
fi

printf 'tui status deep hardening contract ok\n'
