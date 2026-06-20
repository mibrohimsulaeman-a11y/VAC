# VAC — Vastar Agentic CLI

VAC is a local-first, manifest-driven agentic CLI/TUI for controlled software workspaces. The current development baseline is **VAC v1.9**: `.vac/` stores small tracked authority manifests, `vac-rs/` is the Rust workspace, `vac-cli/` is the JS/npm launcher, and generated/runtime state is split away from the clean source checkpoint.

## Current v1.9 architecture state

```text
.vac/
  capabilities/ policies/ workflows/ surfaces/ specs/confirmed/ schemas/ migrations/  # tracked authority
  db/runtime.db                                                                    # ignored local SQLite journal
  cache/compiled/                                                                  # ignored compiled runtime snapshot cache
  exports/                                                                         # optional state/debug exports
vac-rs/
  core/
  crates/{foundation,control-plane,runtime,surfaces,providers,integrations,capabilities}/
vac-cli/                                                                           # npm/native-binary launcher only
```

VAC v1.9 separates storage classes:

- YAML and schemas under `.vac/{capabilities,policies,workflows,surfaces,specs/confirmed,schemas,migrations}` are the tracked authority plane.
- Compiled JSON is runtime cache/DB state under `.vac/cache/compiled` by default; legacy `.vac/registry/compiled` is only a generated mirror/export compatibility path.
- Runtime plans, todo state, decisions, validation state, SpecSync proposals, and local evidence hints belong in `.vac/db/runtime.db`, not source-controlled session files.
- `.vac/index`, `.vac/assessment`, evidence logs, ledgers, and state-specific closure artifacts are generated state/export material and are excluded from the clean source ZIP.

Server and gateway code were not deleted. They are optional VAC boundaries:

- `vac-rs/crates/runtime/vac-broker` for mediated broker/service execution.
- `vac-rs/crates/integrations/vac-messaging-gateway` for channel integrations.
- `vac-rs/crates/integrations/vac-remote-service` for remote-service adapters.

Local VAC control-plane runtime remains the default. Enforcement is still **L1 cooperative/advisory** until broker/OS sandbox custody is implemented and verified.

## Continue from a sandbox checkpoint

This checkpoint does not claim a published container or release binary. Use the supplied source checkpoint.

```bash
unzip vac-runtime-v19-storage-cleanup-source-clean.zip -d vac
cd vac
bash scripts/vac-v19-final-sv-gate.sh
```

For generated audit replay, unpack the paired state export beside the source tree or inspect it separately:

```bash
unzip vac-runtime-v19-storage-cleanup-state-export.zip -d vac-state-export
```

## Build locally after the TV fix loop

```bash
cargo metadata --manifest-path vac-rs/Cargo.toml --locked
cargo build --manifest-path vac-rs/Cargo.toml --release -p vac-cli
./vac-rs/target/release/vac
```

Cargo metadata/fmt/check/clippy/test are **TV-Pending / NotEvaluated** unless those commands are actually run in the target environment.

## v1.9 static gates

```bash
python3 scripts/check-v19-storage-classes.py .
python3 scripts/check-v19-runtime-db-schema.py .
python3 scripts/package-v19-checkpoint.py . /mnt/data vac-runtime-v19-storage-cleanup
python3 scripts/check-v19-package-hygiene.py /mnt/data/vac-runtime-v19-storage-cleanup-source-clean.zip /mnt/data/vac-runtime-v19-storage-cleanup-state-export.zip
```

`bash scripts/vac-static-gate.sh` still runs the broader source/static SV gate, but this is not a cargo/build substitute.

## Container policy

No published container image is claimed by this checkpoint. For local experiments only:

```bash
docker build -t vac-local:dev .
docker run --rm -it -v "$(pwd)":/workspace -w /workspace vac-local:dev
```

After CI publication, the intended namespace is `ghcr.io/mibrohimsulaeman-a11y/vac:<version>`. Do not document a `latest` image as available until CI publishes and verifies it.

## Autopilot retrospect schedule

Use the bundled retrospect skill as the canonical prompt for a nightly local retrospective:

```bash
vac autopilot schedule add --name retrospect --cron "0 3 * * *" --prompt "$(vac ak skill retrospect)"
```

This is a local cooperative L1 workflow helper. It does not claim L2 broker/OS enforcement.


## Documentation

- [Container credential mounts](docs/container-credential-mounts.md)
