#!/usr/bin/env bash
# Prune cargo target/ artifacts to keep disk bounded.
#
# Safe by design:
#   - Skips if any cargo/rustc process is running (cargo build holds locks)
#   - Always removes regen-able dirs first (incremental, tmp) — these are
#     never needed after the build that created them
#   - Then sweeps fingerprint/deps artifacts older than N days via cargo-sweep,
#     which is fingerprint-aware
#
# Usage:
#   scripts/prune-cargo-target.sh                 # safe defaults (>3d)
#   AGGRESSIVE=1 scripts/prune-cargo-target.sh    # also stamp + sweep >1d
#   DRY_RUN=1 scripts/prune-cargo-target.sh       # report-only

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_DIR="$REPO_ROOT/vac-rs/target"
SWEEP_DAYS="${SWEEP_DAYS:-3}"
DRY_RUN="${DRY_RUN:-0}"
AGGRESSIVE="${AGGRESSIVE:-0}"

log() { echo "[prune-cargo-target] $*"; }

# --- safety: refuse to prune if cargo/rustc is active ---------------------
if pgrep -af 'cargo|rustc' | grep -v 'language_server\|prune-cargo-target\|pgrep\|sccache' >/dev/null; then
    log "ABORT: cargo or rustc is running; refusing to prune (would invalidate active build)"
    exit 0
fi

# --- always-safe reclamation ---------------------------------------------
for sub in debug/incremental release/incremental tmp; do
    p="$TARGET_DIR/$sub"
    if [[ -d "$p" ]]; then
        size=$(du -sh "$p" 2>/dev/null | awk '{print $1}')
        if [[ "$DRY_RUN" == "1" ]]; then
            log "DRY: would rm -rf $p ($size)"
        else
            log "rm -rf $p ($size)"
            rm -rf "$p"
        fi
    fi
done

# --- cargo-sweep stale artifacts -----------------------------------------
if ! command -v cargo-sweep >/dev/null 2>&1; then
    log "cargo-sweep not installed; skipping fingerprint-aware sweep"
    exit 0
fi

SWEEP_ARGS=(--time "$SWEEP_DAYS")
if [[ "$DRY_RUN" == "1" ]]; then
    SWEEP_ARGS+=(--dry-run)
fi

log "cargo sweep ${SWEEP_ARGS[*]} $REPO_ROOT/vac-rs"
( cd "$REPO_ROOT/vac-rs" && cargo sweep "${SWEEP_ARGS[@]}" . ) || log "cargo sweep returned non-zero (continuing)"

if [[ "$AGGRESSIVE" == "1" ]]; then
    log "AGGRESSIVE=1: stamping and sweeping >1d"
    ( cd "$REPO_ROOT/vac-rs" && cargo sweep --stamp . ) || true
fi

# --- report -------------------------------------------------------------
df -h "$REPO_ROOT" | tail -1 | awk '{printf "[prune-cargo-target] disk: %s used / %s avail (%s)\n", $3, $4, $5}'
if [[ -d "$TARGET_DIR" ]]; then
    du -sh "$TARGET_DIR" 2>/dev/null | awk '{printf "[prune-cargo-target] target/: %s\n", $1}'
fi
