#!/usr/bin/env bash
# Hardening 10: source artifact hygiene guard for sandbox commits.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"
fail() { echo "FAIL: $*" >&2; exit 1; }


if [ -d donor ]; then
  fail "donor/ directory must not be present in the source artifact after O5.5 closeout"
fi
if find . -maxdepth 1 -type f \( -name 'doctor_*.log' -o -name '*.log' \) | grep -q .; then
  fail "root runtime/doctor log files must not be present in the source artifact"
fi

if find . -path './target' -o -path './*/target' | grep -q .; then
  fail "target directory must not be present in source artifact"
fi
if find . -path './.git' -o -path './*/.git' | grep -q .; then
  fail ".git directory must not be present in source artifact"
fi
if find . -type f \( -name '*.rlib' -o -name '*.rmeta' -o -name '*.o' -o -name '*.d' \) | grep -q .; then
  fail "compiled Rust objects/metadata must not be present"
fi
if find . -type f -size +60M | grep -v '^./docs/' | grep -q .; then
  fail "unexpected large non-doc source file present"
fi

printf 'tui source artifact hygiene ok\n'
