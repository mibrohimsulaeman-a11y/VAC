#!/usr/bin/env bash
# SUITE_SKIP: external upload checksum gate requires SHA256SUMS.txt or VAC_UPLOAD_SHA256SUMS
# Verify uploaded toolchain/vendor bundle checksums when the bundle is present.
# Source-only artifacts may not contain the large upload parts; in that case this
# gate is NotEvaluated and explicitly skipped by the aggregate suite.
set -u
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
SUMS_FILE="${VAC_UPLOAD_SHA256SUMS:-SHA256SUMS.txt}"
UPLOAD_DIR="${VAC_UPLOAD_DIR:-.}"
if [[ ! -f "$SUMS_FILE" ]]; then
  echo "O5/O6 upload sha256: NotEvaluated (checksum file not present: $SUMS_FILE)"
  echo "set VAC_UPLOAD_SHA256SUMS=/path/SHA256SUMS.txt and VAC_UPLOAD_DIR=/path/to/uploads to verify"
  exit 0
fi
missing=0
while read -r expected file _rest; do
  [[ -z "${expected:-}" || "${expected:0:1}" == "#" ]] && continue
  if [[ -z "${file:-}" ]]; then
    echo "O5/O6 upload sha256: malformed checksum row for hash $expected" >&2
    exit 1
  fi
  if [[ ! -f "$UPLOAD_DIR/$file" ]]; then
    echo "O5/O6 upload sha256: missing upload part: $UPLOAD_DIR/$file" >&2
    missing=1
  fi
done < "$SUMS_FILE"
if [[ "$missing" -ne 0 ]]; then
  echo "O5/O6 upload sha256: NotEvaluated (one or more upload parts missing)"
  exit 0
fi
(
  cd "$UPLOAD_DIR" && sha256sum -c "$SUMS_FILE"
)
