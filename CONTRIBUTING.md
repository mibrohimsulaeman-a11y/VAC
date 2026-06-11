# Contributing to VAC

VAC is under active architecture migration to the **v1.9 control-plane model**. Contributions should be source-first, explicit about validation status, and careful not to reintroduce stale upstream documentation, obsolete source URLs, generated state in source checkpoints, or unverified package links.

## Repository source

Use the canonical repository or the source checkpoint supplied by the release maintainer. Cargo metadata/fmt/check/clippy/test are TV-Pending until run locally.

```bash
git clone https://github.com/Vastar-AI/vac.git
cd vac
```

For sandbox continuation:

```bash
unzip vac-runtime-v19-storage-cleanup-source-clean.zip -d vac
cd vac
```

## Development loop

Start with static source validation:

```bash
python3 scripts/check-v19-storage-classes.py .
python3 scripts/check-v19-runtime-db-schema.py .
bash scripts/vac-v19-final-sv-gate.sh
```

Then run cargo repair locally when the Rust toolchain/vendor setup is available:

```bash
cargo metadata --manifest-path vac-rs/Cargo.toml --locked
cargo fmt --manifest-path vac-rs/Cargo.toml --all -- --check
cargo check --manifest-path vac-rs/Cargo.toml --workspace --all-targets
cargo clippy --manifest-path vac-rs/Cargo.toml --workspace --all-targets -- -D warnings
cargo test --manifest-path vac-rs/Cargo.toml --workspace
```

Do not mark cargo gates as passing unless they were executed in the environment being reported.

## Architecture expectations

- `.vac/` contains tracked authority manifests plus ignored DB/cache/export state.
- Operator YAML compiles into runtime JSON snapshots in `.vac/cache/compiled` or DB state.
- Runtime components authorize from compiled snapshot/cache/DB state only.
- Runtime sessions, plans, validations, decisions, evidence hints, and SpecSync proposals are runtime journal records in `.vac/db/runtime.db`.
- Capability readiness is split into `declared`, `computed`, and `effective`.
- Default runtime is local VAC control plane, L1 cooperative/advisory.
- `vac-broker`, `vac-remote-service`, and `vac-messaging-gateway` are optional integration boundaries.

## Packaging policy

Source checkpoint:

```text
vac-runtime-v19-storage-cleanup-source-clean.zip
```

Generated state/export checkpoint:

```text
vac-runtime-v19-storage-cleanup-state-export.zip
```

The source ZIP must not contain `.vac/index`, `.vac/assessment`, `.vac/registry/compiled`, `.vac/cache/compiled`, `.vac/evidence`, `.vac/plans`, `.vac/ledger`, runtime DB files, `.git`, `target`, or `node_modules`.

## Runtime tests

The bounded runtime source-level E2E contract is validated by:

```bash
python3 scripts/vac-runtime-agent-e2e-sv.py
```

This is not a substitute for cargo tests; it is the sandbox-safe SV gate used before the local TV fix loop.
