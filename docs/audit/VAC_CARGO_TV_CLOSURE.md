# VAC Cargo TV Closure

## Scope

This closure removes the final-gate hard-coded `cargo_tv=NotEvaluated` result and replaces it with a current Rust-workspace proof produced by `scripts/check-cargo-tv.py`.

Covered TV claims:

- `cargo_metadata`
- `cargo_fmt`
- `cargo_check`
- `cargo_clippy`
- `cargo_test`

`l2_broker` remains `NotImplemented`; this slice does not claim OS sandbox mediation, broker key custody, or external attestation.

## Proof model

`scripts/check-cargo-tv.py` runs the Cargo commands against `vac-rs/Cargo.toml`, writes `.vac/evidence/cargo-tv-current.json`, and prints the normalized summary consumed by final gates.

The proof is bound to the Rust source workspace by `cargo_workspace_hash`, computed over `vac-rs/**` while excluding Cargo build output and nested `.vac` runtime/session state. Summary mode validates the proof hash, required check statuses, and current workspace hash before emitting `cargo_tv=TV-Pass`.

## Gate integration

The following surfaces consume the same proof helper instead of independent literals:

- `scripts/vac-v19-final-sv-gate.sh`
- `scripts/vac-reaudit-final-sv-gate.sh`
- `scripts/run-final-sv-validation.py`
- `scripts/compile-vac-registry-sv.py`
- `scripts/generate-checkpoint-manifest.py`
- `scripts/refresh-evidence-logs-sv.py`

If the proof is missing, stale, malformed, or any Cargo command fails, summary mode exits non-zero and the final gate cannot print PASS with `cargo_tv=TV-Pass`.

## Command set

The cargo TV runner executes:

```text
cargo metadata --manifest-path vac-rs/Cargo.toml --locked --format-version 1
cargo fmt --manifest-path vac-rs/Cargo.toml --all -- --check
cargo check --manifest-path vac-rs/Cargo.toml --workspace --all-targets --locked
cargo clippy --manifest-path vac-rs/Cargo.toml --workspace --all-targets --locked -- -D warnings
cargo test --manifest-path vac-rs/Cargo.toml --workspace --all-targets --locked
cargo test --manifest-path vac-rs/Cargo.toml -p vac-foundation --features sqlite --locked
cargo test --manifest-path vac-rs/Cargo.toml -p vac-cli --features libsql-test --locked
cargo test --manifest-path vac-rs/Cargo.toml -p vac-messaging-gateway --features libsql-test --locked
cargo test --manifest-path vac-rs/Cargo.toml -p vac-provider-core --features network-tests --locked
```

The four package-level commands are folded into the aggregate `cargo_test` TV claim. The aggregate is `TV-Pass` only when all package commands pass.

## Honest failure behavior

- Missing proof: `cargo_tv=NotEvaluated` / final summary fails when `--summary-only` is required.
- Stale Rust workspace hash: `cargo_tv=TV-Stale` / final summary fails.
- Cargo command failure: `cargo_tv=TV-Fail` / final summary fails.
- L2 broker: always remains `NotImplemented` until an actual broker-mediated enforcement slice lands.
