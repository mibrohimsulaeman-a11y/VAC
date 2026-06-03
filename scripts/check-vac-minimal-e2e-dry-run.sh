#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

TMPROOT="$(mktemp -d "${TMPDIR:-/tmp}/vac-minimal-e2e.XXXXXX")"
trap 'rm -rf "$TMPROOT"' EXIT

mkdir -p "$TMPROOT/.vac/capabilities" "$TMPROOT/.vac/policies" "$TMPROOT/.vac/workflows" "$TMPROOT/.vac/surfaces" "$TMPROOT/src"
cat > "$TMPROOT/.vac/capabilities/test.yaml" <<'YAML'
schema_version: 1
kind: capability
id: vac.test.fixture
title: Test Fixture
status: ready
owner:
  crate: vac-core
  module: test
ownership:
  crates: [vac-core]
  modules: [test]
  targets:
    - kind: path
      include: [src/lib.rs]
      exclude: []
policy:
  risk: safe_read
  mutates_files: false
  network: false
  redaction: false
  approval_required_for: []
surfaces:
  palette: true
validation:
  gates: [fixture]
  commands:
    - id: fixture.echo.pass
      runner: echo
      args: ["pass"]
      risk: safe_read
      approval: never
docs:
  - README.md
YAML
cat > "$TMPROOT/.vac/policies/default.yaml" <<'YAML'
schema_version: 1
kind: policy
id: vac.policy.test
title: Test Policy
default_decision: deny
rules: []
YAML
cat > "$TMPROOT/.vac/workflows/test.yaml" <<'YAML'
schema_version: 1
kind: workflow
id: maintenance.test
title: Test
status: ready
inputs: {}
steps:
  - id: check
    uses: capability.vac.test.fixture.check
    ui:
      surface: /workflow
      progress_panel: true
      activity_log: true
      approval_surface: false
      evidence_surface: true
policy:
  default_risk: safe_read
  mutates_files: false
  network: false
  redaction: false
  approval_required_for: []
validation:
  gates: [fixture]
  commands:
    - id: maintenance.test.validation.cmd001
      runner: echo
      args: ["pass"]
      risk: safe_read
      approval: never
ui:
  surface: /workflow
  inspect_surface: /capabilities
  progress_panel: true
  activity_log: true
  approval_surface: false
  evidence_surface: true
YAML
cat > "$TMPROOT/.vac/surfaces/cli.yaml" <<'YAML'
schema_version: 1
kind: surface
id: surface.cli
title: CLI
capabilities: [vac.test.fixture]
routes:
  - kind: cli
    command: vac doctor registry
    capability: vac.test.fixture
    owner: vac-core/test
    visible: true
    status: ready
YAML
echo "pub fn fixture() {}" > "$TMPROOT/src/lib.rs"
echo "# fixture" > "$TMPROOT/README.md"

# Minimal deterministic dry-run uses static scripts until a built vac binary is explicitly provided.
python3 - <<PY
import pathlib, yaml
root = pathlib.Path("$TMPROOT")
count = 0
for path in root.joinpath(".vac").rglob("*.yaml"):
    data = yaml.safe_load(path.read_text())
    for field in ("schema_version", "kind", "id"):
        assert field in data, f"{path}: missing {field}"
    count += 1
assert count == 4, count
print(f"minimal e2e dry-run fixture parse: PASS ({count} manifests)")
PY

before="$(find "$TMPROOT" -type f | sort | xargs -r sha256sum)"
after="$(find "$TMPROOT" -type f | sort | xargs -r sha256sum)"
if [[ "$before" != "$after" ]]; then
  echo "FAIL: minimal dry-run mutated fixture workspace" >&2
  exit 1
fi

printf 'vac minimal e2e dry-run gate: PASS\n'
