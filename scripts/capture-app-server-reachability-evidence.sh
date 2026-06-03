#!/usr/bin/env bash
# Capture Plan 33 app-server reachability evidence.
#
# Usage:
#   ./scripts/capture-app-server-reachability-evidence.sh [--out DIR] [--include-validation] [--toolchain +1.93.0]
#
# Default mode captures lightweight reachability evidence only. Add
# --include-validation after Plan 32 is green and the operator is ready to run
# the heavier Cargo validation matrix.

set -uo pipefail

TOOLCHAIN='+1.93.0'
INCLUDE_VALIDATION=0
OUT_DIR=''

usage() {
  sed -n '1,16p' "$0"
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --out)
      if [ "$#" -lt 2 ]; then
        echo 'ERROR: --out requires a directory argument' >&2
        exit 2
      fi
      OUT_DIR="$2"
      shift 2
      ;;
    --include-validation)
      INCLUDE_VALIDATION=1
      shift
      ;;
    --toolchain)
      if [ "$#" -lt 2 ]; then
        echo 'ERROR: --toolchain requires a value such as +1.93.0' >&2
        exit 2
      fi
      TOOLCHAIN="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "ERROR: unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null)" || {
  echo 'ERROR: must run inside a git checkout' >&2
  exit 1
}
cd "$REPO_ROOT"

if [ -z "$OUT_DIR" ]; then
  stamp="$(date -u +%Y%m%dT%H%M%SZ)"
  OUT_DIR="docs/workflow-control-plane/plans/33-evidence/runs/$stamp"
fi
mkdir -p "$OUT_DIR"

SUMMARY="$OUT_DIR/summary.md"
STATUS_TSV="$OUT_DIR/status.tsv"
: > "$STATUS_TSV"

EXIT_CODE=0

slugify() {
  printf '%s' "$1" | tr '[:upper:]' '[:lower:]' | sed -E 's/[^a-z0-9]+/-/g; s/^-+//; s/-+$//'
}

record() {
  local label="$1"
  local status="$2"
  local file="$3"
  printf '%s	%s	%s
' "$label" "$status" "$file" >> "$STATUS_TSV"
  if [ "$status" -ne 0 ]; then
    EXIT_CODE=1
  fi
}

capture() {
  local label="$1"
  local cwd="$2"
  local cmd="$3"
  local slug file status
  slug="$(slugify "$label")"
  file="$OUT_DIR/${slug}.txt"
  {
    echo "# $label"
    echo "cwd: $cwd"
    echo "command: $cmd"
    echo
  } > "$file"
  ( cd "$cwd" && bash -lc "$cmd" ) >> "$file" 2>&1
  status=$?
  {
    echo
    echo "exit_status: $status"
  } >> "$file"
  record "$label" "$status" "$file"
}

{
  echo '# Plan 33 app-server reachability evidence capture'
  echo
  echo "- captured_at_utc: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "- repo_root: $REPO_ROOT"
  echo "- git_commit: $(git rev-parse HEAD 2>/dev/null || true)"
  echo "- git_branch: $(git branch --show-current 2>/dev/null || true)"
  echo "- toolchain: $TOOLCHAIN"
  echo "- include_validation: $INCLUDE_VALIDATION"
  echo
  echo '## Raw artifacts'
} > "$SUMMARY"

capture 'git status short' "$REPO_ROOT" 'git status --short'
capture 'disk guard' "$REPO_ROOT" 'df -h . /tmp'
capture 'tui source grep app server imports' "$REPO_ROOT" "git grep -n 'vac_app_server\|vac_app_server_client\|vac_app_server_protocol\|vac_app_server_transport' -- vac-rs/tui/src || true"
capture 'tui manifest and source rg app server' "$REPO_ROOT" "rg -n 'vac_app_server|vac_app_server_client|vac_app_server_protocol|vac_app_server_transport|vac-app-server' vac-rs/tui/Cargo.toml vac-rs/tui/src || true"
capture 'workspace app server consumer audit' "$REPO_ROOT" "rg -n 'vac_app_server|vac_app_server_client|vac_app_server_protocol|vac_app_server_transport|vac-app-server' vac-rs --glob 'Cargo.toml' --glob '*.rs' || true"

if [ -d vac-rs ]; then
  capture 'inverse tree vac app server client' "$REPO_ROOT/vac-rs" "cargo $TOOLCHAIN tree -p vac-surface-cli -i vac-app-server-client --edges normal,build"
  capture 'inverse tree vac app server' "$REPO_ROOT/vac-rs" "cargo $TOOLCHAIN tree -p vac-surface-cli -i vac-app-server --edges normal,build"
  capture 'inverse tree vac app server transport' "$REPO_ROOT/vac-rs" "cargo $TOOLCHAIN tree -p vac-surface-cli -i vac-app-server-transport --edges normal,build"
else
  echo 'ERROR: vac-rs directory not found' >&2
  EXIT_CODE=1
fi

if [ "$INCLUDE_VALIDATION" -eq 1 ] && [ -d vac-rs ]; then
  capture 'validation vac tui check tests' "$REPO_ROOT/vac-rs" "CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo $TOOLCHAIN check -p vac-surface-tui --tests"
  capture 'validation vac tui local runtime nextest' "$REPO_ROOT/vac-rs" 'cargo nextest run --manifest-path Cargo.toml -p vac-surface-tui --lib local_runtime --no-tests=pass'
  capture 'validation vac core local runtime nextest' "$REPO_ROOT/vac-rs" 'cargo nextest run --manifest-path Cargo.toml -p vac-core --lib local_runtime --no-tests=pass'
  capture 'validation vac cli check' "$REPO_ROOT/vac-rs" "CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo $TOOLCHAIN check -p vac-surface-cli"
fi

while IFS=$'	' read -r label status file; do
  printf -- '- `%s` → `%s` (exit %s)
' "$label" "$file" "$status" >> "$SUMMARY"
done < "$STATUS_TSV"

cat >> "$SUMMARY" <<EOF_SUMMARY

## Template fill targets

- Source grep template: docs/workflow-control-plane/plans/33-evidence/source-grep-evidence.md
- Inverse Cargo tree template: docs/workflow-control-plane/plans/33-evidence/inverse-cargo-tree-evidence.md
- Validation matrix template: docs/workflow-control-plane/plans/33-evidence/validation-matrix.md
- Closeout index: docs/workflow-control-plane/plans/33-evidence/closeout-evidence-index.md

EOF_SUMMARY

echo "Evidence captured under: $OUT_DIR"
echo "Summary: $SUMMARY"

if [ "$EXIT_CODE" -ne 0 ]; then
  echo 'One or more evidence commands exited non-zero. Review raw artifacts before making Plan 33 claims.' >&2
fi
exit "$EXIT_CODE"
