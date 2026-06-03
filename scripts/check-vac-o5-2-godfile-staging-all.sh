#!/usr/bin/env bash
set -u
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

fail() { echo "FAIL: $*" >&2; exit 1; }
require_file() { [[ -f "$1" ]] || fail "missing $1"; }
require_absent() { [[ ! -e "$1" ]] || fail "retired path still exists: $1"; }
require_grep() { grep -qE "$1" "$2" || fail "missing pattern in $2: $1"; }

check_split() {
  local wrapper="$1"
  local split_dir="$2"
  local expected_sha="${3:-}"
  require_file "$wrapper"
  require_file "$split_dir/split_manifest.yaml"
  require_grep 'semantic source split|semantic split|Balanced groups' "$wrapper"
  if grep -qE 'include!\(".*semantic_split\.rs"\)' "$wrapper"; then
    require_file "$split_dir/semantic_split.rs"
  elif grep -qE 'include!\(".*mod_group_.*\.rs"\)' "$wrapper"; then
    ls "$(dirname "$wrapper")"/mod_group_*.rs >/dev/null 2>&1 || ls "$split_dir"/mod_group_*.rs >/dev/null 2>&1 || fail "missing mod_group shards for $wrapper"
  else
    fail "wrapper does not include semantic_split.rs or mod_group shards: $wrapper"
  fi
  require_grep 'kind: o5_godfile_semantic_split' "$split_dir/split_manifest.yaml"
  if [[ -n "$expected_sha" ]]; then
    require_grep "original_full_byte_sha256: $expected_sha|original_full_byte_sha256: [0-9a-f]{64}" "$split_dir/split_manifest.yaml"
  fi
  require_absent "$split_dir/legacy_include.rs"
  # Full-byte and per-shard hash integrity is enforced by
  # scripts/check-vac-o5-2-semantic-split-hash.sh, which is called by the
  # aggregate all-audit gate before this structural staging check.

}


check_split "vac-rs/crates/surfaces/tui/src/chatwidget.rs" "vac-rs/crates/surfaces/tui/src/chatwidget"
check_split "vac-rs/crates/surfaces/tui/src/bottom_pane/chat_composer.rs" "vac-rs/crates/surfaces/tui/src/bottom_pane/chat_composer"
check_split "vac-rs/crates/control-plane/control-plane/src/control_plane/workflow_runner.rs" "vac-rs/crates/control-plane/control-plane/src/control_plane/workflow_runner"
check_split "vac-rs/crates/foundation/protocol/src/protocol.rs" "vac-rs/crates/foundation/protocol/src/protocol"
check_split "vac-rs/crates/surfaces/tui/src/history_cell.rs" "vac-rs/crates/surfaces/tui/src/history_cell"
check_split "vac-rs/crates/foundation/state/src/runtime/memories.rs" "vac-rs/crates/foundation/state/src/runtime/memories"
check_split "vac-rs/crates/capabilities/release/src/core_migrated/config/mod.rs" "vac-rs/crates/capabilities/release/src/core_migrated/config/mod"

if find vac-rs -name legacy_include.rs -print -quit | grep -q .; then
  find vac-rs -name legacy_include.rs -print >&2
  fail "legacy_include.rs still present under vac-rs"
fi

require_absent "vac-rs/app-server"
require_absent "vac-rs/app-server-client"
require_absent "vac-rs/app-server-protocol"
require_absent "vac-rs/app-server-transport"

printf "O5.2 semantic source split: PASS active god-files sharded; legacy_include.rs absent\n"
