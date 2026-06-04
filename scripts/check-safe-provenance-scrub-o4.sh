#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
TMPROOT="$(mktemp -d "${TMPDIR:-/tmp}/vac-safe-provenance.XXXXXX")"
trap 'rm -rf "$TMPROOT"' EXIT

if grep -RIn 'OpenAI Codex concept' vac-rs/crates/control-plane/control-plane/src/control_plane --exclude-dir=target --exclude-dir=.git >"$TMPROOT/codex-concept.txt"; then
  cat "$TMPROOT/codex-concept.txt" >&2
  exit 1
fi
if grep -RIn 'OpenAiEmployee' vac-rs/crates/capabilities/local-runtime-owner/src vac-rs/crates/surfaces/tui/src --exclude-dir=target --exclude-dir=.git >"$TMPROOT/openai-employee.txt"; then
  cat "$TMPROOT/openai-employee.txt" >&2
  exit 1
fi
grep -q 'InternalTester' vac-rs/crates/capabilities/local-runtime-owner/src/startup.rs
grep -q 'external agent concept' vac-rs/crates/control-plane/control-plane/src/control_plane/donor_domain_contract.rs
grep -q 'OpenAI Codex CLI' THIRD_PARTY_NOTICES.md

printf 'safe provenance scrub O4: PASS\n'
