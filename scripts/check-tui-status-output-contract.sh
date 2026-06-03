#!/usr/bin/env bash
# Static gate for the /status output hardening contract.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

fail() { echo "FAIL: $*" >&2; exit 1; }

status_mod="vac-rs/tui/src/status/mod.rs"
status_card="vac-rs/tui/src/status/card.rs"
status_contract="vac-rs/tui/src/status/output_contract.rs"
status_dispatch="vac-rs/tui/src/chatwidget/slash_dispatch.rs"
chatwidget="vac-rs/tui/src/chatwidget.rs"
app_rs="vac-rs/tui/src/app.rs"
status_snapshots="vac-rs/tui/src/status/snapshots"
slash_dispatch="vac-rs/tui/src/chatwidget/slash_dispatch.rs"
chatwidget="vac-rs/tui/src/chatwidget.rs"

for file in "$status_mod" "$status_card" "$status_contract" "$status_dispatch" "$chatwidget" "$app_rs"; do
  [ -f "$file" ] || fail "missing $file"
done
[ -d "$status_snapshots" ] || fail "missing $status_snapshots"

grep -q 'mod output_contract;' "$status_mod" || fail "status output contract module not registered"

grep -q 'enum StatusDisplayField' "$status_contract" || fail "StatusDisplayField contract missing"
grep -q 'struct StatusProviderModelUsage' "$status_contract" || fail "provider/model status usage contract missing"
grep -q 'StatusDisplayField::ModelProvider,' "$status_contract" || fail "model provider is not registered as a required /status field"
grep -q 'StatusDisplayField::ModelProviders,' "$status_contract" || fail "multi-provider registry field missing"
grep -q 'validate_status_output_lines' "$status_contract" || fail "rendered status output validator missing"
grep -q 'validate_provider_model_usage_rows' "$status_contract" || fail "provider/model usage validator missing"
grep -q 'FORBIDDEN_STATUS_OUTPUT_FRAGMENTS' "$status_contract" || fail "forbidden status output fragment registry missing"
grep -q 'struct StatusDisplayPolicy' "$status_contract" || fail "status display policy missing"
grep -q 'STATUS_OPERATOR_DISPLAY_POLICY' "$status_contract" || fail "operator status display policy missing"
grep -q 'STATUS_COMMAND_REFRESH_POLICY' "$status_contract" || fail "/status refresh policy contract missing"
grep -q 'STATUS_OPERATOR_DISPLAY_POLICY' "$status_contract" || fail "/status display policy contract missing"
grep -q 'provider_model_usage_rows_validate_multiple_providers' "$status_contract" || fail "multi-provider/model status contract test missing"
grep -q 'provider_model_usage_rows_require_active_model' "$status_contract" || fail "active model status contract test missing"
grep -q 'status_command_refresh_policy_is_local_only' "$status_contract" || fail "local-only /status refresh policy test missing"
grep -q 'status_operator_display_policy_rejects_quota_and_balance_rows' "$status_contract" || fail "operator status display policy test missing"

grep -q 'StatusDisplayField::TokenUsage.label()' "$status_card" || fail "/status token usage row missing"
grep -q 'StatusDisplayField::ContextWindow.label()' "$status_card" || fail "/status context window row missing"
grep -q 'StatusDisplayField::Model.label()' "$status_card" || fail "/status model row missing"
grep -q 'StatusDisplayField::ModelProvider.label()' "$status_card" || fail "/status model provider row support missing"
grep -q 'StatusDisplayField::ModelProviders.label()' "$status_card" || fail "/status multi-provider registry row support missing"
grep -q 'fn format_model_provider(config: &Config, runtime_base_url: Option<&str>) -> String' "$status_card" || fail "/status active provider formatter must always return a display row"
grep -q 'fn format_model_provider_registry(config: &Config) -> Option<String>' "$status_card" || fail "/status multi-provider registry formatter missing"
if grep -q 'is_default_vastar' "$status_card"; then
  fail "/status must not hide the default Vastar provider row"
fi
grep -q 'StatusDisplayField::Directory.label()' "$status_card" || fail "/status directory row missing"
grep -q 'StatusDisplayField::Permissions.label()' "$status_card" || fail "/status permissions row missing"

if grep -Eq 'Visit .*rate limits|rate limits and credits|Credits:|Credit balance|Limits:' "$status_card"; then
  fail "/status runtime card still contains forbidden Visit/rate-limit/credit display text"
fi

if grep -Eq 'fn rate_limit_lines|fn rate_limit_row_lines|fn collect_rate_limit_labels|formatter\.line\("Limits"|formatter\.line\("Credits"' "$status_card"; then
  fail "/status runtime card still contains legacy rate-limit/credit row render path"
fi

grep -q 'let rate_limit_snapshots: &\[RateLimitSnapshotDisplay\] = &\[\];' "$chatwidget" || fail "/status output must ignore cached rate-limit snapshots"
grep -q 'SlashCommand::Status' "$status_dispatch" || fail "/status slash dispatch missing"
grep -q '/status is an operator-local inventory surface' "$status_dispatch" || fail "/status local-only dispatch comment missing"
grep -q '/status.*local-only' "$app_rs" || fail "startup prefetch comment must distinguish status-line/nudge from /status"
if grep -Eq 'StatusCommand \{ request_id|refresh_rate_limits\(' "$status_dispatch"; then
  fail "/status dispatch must not request rate-limit refresh"
fi

# /status must render local status immediately and must not initiate a quota/credit refresh.
status_dispatch_block="$(awk '/SlashCommand::Status =>/{flag=1} flag{print} /SlashCommand::Ide =>/{exit}' "$slash_dispatch")"
printf '%s\n' "$status_dispatch_block" | grep -q 'add_status_output' || fail "/status slash dispatch does not render status output"
if printf '%s\n' "$status_dispatch_block" | grep -q 'RefreshRateLimits'; then
  fail "/status slash dispatch still triggers rate-limit refresh"
fi

grep -q 'let rate_limit_snapshots: \&\[RateLimitSnapshotDisplay\] = \&\[\];' "$chatwidget" || fail "add_status_output does not isolate cached rate-limit snapshots"
grep -q 'Hardening 3: /status intentionally ignores cached account limit' "$chatwidget" || fail "cached rate-limit isolation rationale missing"

if grep -RIn -E 'Limits:|Visit .*rate limits|rate limits and credits|Credits:|Credit balance' "$status_snapshots" >/tmp/vac-status-output-forbidden.txt; then
  cat /tmp/vac-status-output-forbidden.txt >&2
  fail "/status snapshots still contain removed rate-limit/credit display rows"
fi

bash scripts/check-tui-status-deep-hardening-contract.sh >/dev/null

printf 'tui status output contract ok\n'
