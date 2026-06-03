#!/usr/bin/env bash
set -u
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
fail() { echo "FAIL: $*" >&2; exit 1; }
count="$(find vac-rs -path '*/protocol/v2.rs' -type f | wc -l | tr -d ' ')"
[[ "$count" = "1" ]] || fail "expected exactly one protocol/v2.rs after app-server retirement, found $count"
[[ -f vac-rs/crates/foundation/runtime-protocol/src/protocol/v2.rs ]] || fail "missing canonical runtime-protocol v2.rs"
if find vac-rs -type f -size +100k -print0 | xargs -0 sha256sum | sort | awk '{print $1}' | uniq -d | grep -q .; then
  find vac-rs -type f -size +100k -print0 | xargs -0 sha256sum | sort >&2
  fail "duplicate large source blobs remain"
fi
printf 'F5 protocol duplication: PASS canonical protocol/v2.rs is single-source; no large duplicate blobs found\n'
