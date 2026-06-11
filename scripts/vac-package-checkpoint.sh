#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${1:-/mnt/data/vac-impl-artifacts/final}"
mkdir -p "$OUT_DIR"
STAMP="${VAC_CHECKPOINT_STAMP:-$(date -u +%Y%m%dT%H%M%SZ)}"
ZIP="$OUT_DIR/vac-v1-5-tui-runtime-rebrand-checkpoint-${STAMP}.zip"
cd "$ROOT"
zip -qr "$ZIP" . \
  -x '.git/*' 'target/*' '*/target/*' 'node_modules/*' '*/node_modules/*' \
     '.vac/cache/*' '.vac/memories/*.db' '*.pyc' '__pycache__/*' '*/__pycache__/*'
sha256sum "$ZIP" > "$ZIP.sha256"
printf '%s\n' "$ZIP"
