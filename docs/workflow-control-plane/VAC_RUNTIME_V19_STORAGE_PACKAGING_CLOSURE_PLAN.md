# VAC Runtime v1.9 Storage and Packaging Closure Plan

This note records the current closure model for VAC Runtime v1.9 storage, packaging, and release hygiene. It exists as a source-controlled current-state reference for maintainers and for static gates that prevent stale install, source, or runtime-authority claims from re-entering the repository.

## Authority model

VAC Runtime v1.9 uses tracked authority for source-controlled policy, manifests, scripts, and documentation. Runtime components must consume compiled state from the local VAC control plane rather than uncompiled YAML or ad hoc process memory. The effective runtime authority is the compiled snapshot/cache and local DB state produced by the repo gates.

The important storage classes are:

- `tracked authority`: source files, docs, manifests, scripts, and test fixtures that belong in Git.
- `compiled local authority`: generated compiled JSON under `.vac/cache/compiled` and runtime database state.
- `runtime-local state`: process/session state, logs, evidence, indexes, and DB files that must not be committed.

The runtime DB path is `.vac/db/runtime.db`. It is runtime-local state and must be recreated or migrated by the local control plane rather than shipped as tracked source.

## Packaging rule

Source packages must be reproducible from tracked authority. A source ZIP excludes runtime-local state, generated evidence, indexes, DB files, build artifacts, and local caches. Required source ZIP excludes include:

```text
.git
vac-rs/target
target
.vac/cache
.vac/index
.vac/db
.vac/exports
.vac/evidence
.vac/registry/evidence
.vac/registry/runtime
```

Release artifacts may include compiled outputs produced by gates, but they must not be treated as source authority unless explicitly recorded in the release manifest.

## Closure gates

Before publishing or force-updating `main`, maintainers should run the current gate set and preserve logs outside tracked source, for example under `/home/emp/Documents/VAC-checkpoints`. The minimum closure gates for this area are:

```bash
python3 scripts/check-v19-storage-classes.py .
python3 scripts/check-checkpoint-integrity.py .
python3 scripts/check-docs-current-state.py .
python3 scripts/check-brand-allowlist.py .
bash scripts/vac-v19-final-sv-gate.sh
```

`cargo check`, `cargo clippy`, and `cargo test` remain TV gates for Rust correctness. Do not report them as passing unless they were run in the current worktree.

## Current status

Storage and packaging closure is SV-gated by repository scripts. Any remaining runtime proof that requires executing the CLI/TUI, PTY lifecycle, or full workspace tests must be reported as `TV-Pending / NotEvaluated` until that gate is actually run and logged.
