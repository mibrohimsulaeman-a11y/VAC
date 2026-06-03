# O5/O6 Epic A App-Server Retirement Report

Date: 2026-05-30

## Scope

Epic A implements the corrected audit decision that **local** means the TUI talks to the agent through the in-process owner-native runtime path, not through a separate app-server process or IPC service. This slice intentionally preserves provider access through login/API-key stacks such as Anthropic, Codex/OpenAI, ChatGPT, model-provider, backend-client, and `reqwest`.

Offline penuh / LLM lokal is archived for this slice because it contradicts the requested product semantics: cloud/provider login remains supported.

## Source changes

- Removed the TUI `legacy-app-server-compat` feature and optional `vac-app-server-client` dependency.
- Deleted retired app-server compatibility crates from the source tree:
  - `vac-rs/app-server`
  - `vac-rs/app-server-client`
  - `vac-rs/app-server-protocol`
  - `vac-rs/app-server-transport`
- Removed their workspace members and workspace dependency aliases from `vac-rs/Cargo.toml`.
- Moved former schema binary ownership to `vac-rs/runtime-protocol`:
  - `vac-runtime-protocol-export`
  - `vac-runtime-protocol-write-schema-fixtures`
- Renamed the active TUI session boundary from `app_server_session.rs` to `runtime_owner_session.rs` and wired `local_runtime_session.rs` through `crate::runtime_owner_session`.
- Deleted legacy-only TUI compatibility modules:
  - `legacy_app_server_compat.rs`
  - `legacy_app_server_session.rs`
  - `legacy_app_server_session/`
- Updated VAC manifests, workflow validation, state, compile-debt ledger, and static gates.

## Provider retention proof

The TUI manifest still keeps provider/network-facing dependencies. This is intentional and required:

- `vac-chatgpt`
- `vac-login`
- `vac-model-provider`
- `vac-model-provider-info`
- `vac-models-manager`
- `reqwest`
- `vac-backend-client` remains available at workspace level.

## Validation

Static gates run in this artifact:

```bash
bash scripts/check-no-app-server-local-path-contract.sh
bash scripts/check-vac-epic-a-app-server-retirement.sh
bash scripts/check-vac-o5-1c-app-server-protocol-consumer-migration.sh
bash scripts/check-vac-o5-o6-monolith-quality-slice.sh
bash scripts/check-vac-o5-2-godfile-staging-all.sh
bash scripts/check-vac-o5-o6-completion-state.sh
bash scripts/check-vac-o6-2-safety-coverage.sh
bash scripts/check-vac-o6-quality-triage.sh
bash scripts/check-legal-release-blockers.sh
bash scripts/check-tui-source-artifact-hygiene.sh
VAC_UPLOAD_SHA256SUMS=/mnt/data/SHA256SUMS.txt VAC_UPLOAD_DIR=/mnt/data bash scripts/check-vac-o5o6-upload-sha256.sh
```

All listed gates pass at source/static level. Additional regression gates were also run after the O5.2 staging gate was updated to treat app-server-owned staging files as retired/absent rather than missing wrappers.

## TV status

Cargo build/clippy/test and cargo tree reachability are **TV-Pending**. The source artifact does not include `vendor/`, `target/`, or a stable extracted toolchain. Do not claim TV-Done until the following complete successfully in a persistent toolchain environment:

```bash
cargo build --offline --workspace --locked
cargo clippy --offline --workspace -- -D warnings
cargo test --offline --workspace
cargo tree -p vac-surface-tui --edges normal,build
```

## Risk and rollback

Risk is medium because Cargo.lock and workspace membership changed without completed cargo verification. Rollback is restoring `vac-o5o6-remaining-audit-gaps-source.zip` and reapplying only after a default TUI build confirms no hidden dependencies on the retired crates.
