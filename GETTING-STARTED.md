# Getting Started with VAC

This document reflects the current **VAC v1.9** development state. VAC is local-control-plane-first: authority manifests are tracked, runtime state is journaled in local SQLite, and generated index/assessment/compiled snapshots are split into cache/export artifacts rather than treated as source authority.

## 1. Continue from a split checkpoint

Use the clean source ZIP for development:

```bash
unzip vac-runtime-v19-storage-cleanup-source-clean.zip -d vac
cd vac
python3 scripts/check-v19-storage-classes.py .
python3 scripts/check-v19-runtime-db-schema.py .
```

Use the paired state export ZIP only for audit replay/debugging:

```bash
unzip vac-runtime-v19-storage-cleanup-state-export.zip -d vac-state-export
```

## 2. Runtime authority model

```text
.vac/capabilities/*.yaml             # tracked capability authority
.vac/policies/*.yaml                 # tracked policy authority
.vac/workflows/*.yaml                # tracked workflow contracts
.vac/surfaces/*.yaml                 # tracked CLI/TUI/slash route bindings
.vac/specs/confirmed/*.yaml          # tracked confirmed intent specs
.vac/schemas/*.json                  # tracked manifest schemas
.vac/migrations/runtime-db/*.sql     # tracked local journal schema migrations
.vac/db/runtime.db                   # ignored local SQLite runtime journal
.vac/cache/compiled/*.json           # ignored compiled runtime snapshot cache
.vac/index/*.jsonl                   # generated deterministic index; state export only
.vac/assessment/*.json               # generated assessment reports; state export only
```

Runtime components must authorize from compiled snapshot cache/DB state, not raw YAML. In v1.9 the clean source checkpoint intentionally excludes generated `.vac/index`, `.vac/assessment`, `.vac/registry/compiled`, `.vac/evidence`, `.vac/plans`, and `.vac/ledger` payloads.

## 3. Runtime behavior

A normal VAC coding request is wrapped by the bounded loop:

1. Intake, manifest set hash, and Semantic Plan.
2. Manifest-bound runtime journal records in `.vac/db/runtime.db`.
3. Bounded patch execution with plan, ownership, anchor, and budget gates.
4. Structured validation commands with policy/approval gates.
5. Evidence hints, SpecSync proposals, readiness state, and validation summaries recorded in the runtime journal.
6. Completion lock: done only when DB state, validation, decisions, manifest sync, and governance gates pass, or a visible `needs_discussion` state is recorded.

## 4. Static validation in sandbox

```bash
bash scripts/vac-v19-final-sv-gate.sh
```

This validates source/static semantics only. It does **not** prove cargo build, cargo test, TUI PTY, provider integration, or L2 broker enforcement.

## 5. Build locally after TV repair

```bash
cargo metadata --manifest-path vac-rs/Cargo.toml --locked
cargo fmt --manifest-path vac-rs/Cargo.toml --all -- --check
cargo check --manifest-path vac-rs/Cargo.toml --workspace --all-targets
cargo clippy --manifest-path vac-rs/Cargo.toml --workspace --all-targets -- -D warnings
cargo test --manifest-path vac-rs/Cargo.toml --workspace
```

Do not mark those cargo gates as passing unless they were actually executed.

## 6. Optional integration boundaries

`server` and `gateway` are not the default product runtime. Their migrated VAC positions are:

- `vac-rs/crates/runtime/vac-broker` — optional mediated broker/service runtime.
- `vac-rs/crates/integrations/vac-messaging-gateway` — optional channel gateway.
- `vac-rs/crates/integrations/vac-remote-service` — optional remote service adapter.

Local control-plane runtime remains the default path.
