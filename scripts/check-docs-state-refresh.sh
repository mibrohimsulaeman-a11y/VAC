#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
require_file() { [[ -f "$1" ]] || { echo "FAIL: missing required file: $1" >&2; exit 1; }; }
require_file docs/workflow-control-plane/INDEX.md
require_file docs/workflow-control-plane/IMPLEMENTATION_PLAN.md
require_file docs/workflow-control-plane/plans/INDEX.md
require_file docs/workflow-control-plane/schema/INDEX.md
require_file docs/legal/NOTICES.md
require_file .vac/registry/docs-state.yaml
require_file .vac/registry/donor-inventory.yaml
[[ ! -d docs/donor-migration ]] || { echo "FAIL: docs/donor-migration should be pruned" >&2; exit 1; }
actual_docs="$(find docs -type f -name '*.md' | wc -l | tr -d ' ')"
recorded_docs="$(python3 - <<'PYDOC'
from pathlib import Path
import re
text = Path('.vac/registry/docs-state.yaml').read_text()
match = re.search(r'markdown_files:\s*(\d+)', text)
print(match.group(1) if match else 'MISSING')
PYDOC
)"
[[ "$actual_docs" == "$recorded_docs" ]] || { echo "FAIL: docs count drift actual=$actual_docs recorded=$recorded_docs" >&2; exit 1; }
grep -q 'docs_pruned: true' .vac/registry/docs-state.yaml || { echo "FAIL: docs-state missing donor docs_pruned marker" >&2; exit 1; }
grep -q 'docs/workflow-control-plane/INDEX.md' .vac/registry/docs-state.yaml || { echo "FAIL: docs-state missing active docs index" >&2; exit 1; }
printf 'docs state refresh: PASS (docs=%s; donor docs pruned)
' "$actual_docs"
