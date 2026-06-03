#!/usr/bin/env bash
# Static gate for Hardening 2: semantic operator renderer + live adapter contract.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

fail() { echo "FAIL: $*" >&2; exit 1; }

style="vac-rs/tui/src/operator_style.rs"
ui="vac-rs/tui/src/operator_ui.rs"
adapter="vac-rs/tui/src/operator_ui_styles.rs"
capability="vac-rs/tui/src/capability_dashboard.rs"
chat="vac-rs/tui/src/chatwidget.rs"

for file in "$style" "$ui" "$adapter" "$capability" "$chat"; do
  [ -f "$file" ] || fail "missing $file"
done

grep -q 'struct OperatorSpanSpec' "$style" || fail "OperatorSpanSpec missing"
grep -q 'struct OperatorLineSpec' "$style" || fail "OperatorLineSpec missing"
grep -q 'style_operator_text_specs' "$style" || fail "semantic ANSI text renderer missing"
grep -q 'operator_line_specs_to_plain_text' "$style" || fail "semantic plain text renderer missing"
grep -q 'OperatorStyleRole::Plain' "$style" || fail "plain semantic role missing"

grep -q 'render_operator_snapshot_specs' "$ui" || fail "snapshot spec renderer missing"
grep -q 'render_operator_snapshot_ansi_text' "$ui" || fail "semantic ANSI snapshot renderer missing"
grep -q 'OperatorSemanticScreen' "$ui" || fail "screen semantic role mapper missing"
grep -q 'highlight_keyword' "$ui" || fail "span-level highlight helper missing"
grep -q 'render_autopilot_scheduler_line_specs' "$ui" || fail "runtime jobs semantic spec renderer missing"
grep -q 'render_capability_dashboard_shell_specs' "$ui" || fail "dashboard semantic spec renderer missing"

grep -q 'style_operator_lines_from_specs' "$adapter" || fail "live adapter semantic spec entrypoint missing"
grep -q 'style_operator_line_from_spec' "$adapter" || fail "live adapter line spec entrypoint missing"
grep -q 'style_operator_lines_from_specs' "$capability" || fail "capability dashboard still bypasses semantic specs"
grep -q 'style_operator_lines_from_specs' "$chat" || fail "runtime jobs still bypasses semantic specs"

grep -q 'classify_operator_line' "$adapter" || fail "legacy fallback classifier missing"

grep -q 'approval_semantic_spec_marks_destructive_span_as_danger' "$ui" || fail "destructive semantic span test missing"
grep -q 'semantic_ansi_snapshots_strip_back_to_plain_snapshot' "$ui" || fail "ANSI strip round-trip test missing"

printf 'tui renderer semantic contract ok\n'
