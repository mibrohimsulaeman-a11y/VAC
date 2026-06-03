#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
require_file() { [[ -f "$1" ]] || { echo "FAIL: missing required file: $1" >&2; exit 1; }; }
require_file docs/PROJECT_STATE_CURRENT.md
require_file docs/DOCS_INDEX_CURRENT.md
require_file docs/DOCS_AUDIT.md
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
grep -q 'docs/donor-migration/` has been pruned' docs/PROJECT_STATE_CURRENT.md || { echo "FAIL: project state missing donor pruning state" >&2; exit 1; }
printf 'docs state refresh: PASS (docs=%s; donor docs pruned)
' "$actual_docs"
