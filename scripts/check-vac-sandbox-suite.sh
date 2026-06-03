#!/usr/bin/env bash
# Sandbox gate suite wrapper.
# Toolchain-dependent gates must opt in with `# REQUIRES_TOOLCHAIN`.
# The Python runner avoids the old grep-substring false skip and applies a
# bounded timeout to every source/static gate.
set -u
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
exec python3 scripts/run-vac-sandbox-suite.py
