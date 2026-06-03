# Old runtime delete gate

## Goal

Delete or defer old runtime crates only after they are unreachable from the default local product path.

## Candidate families

```text
old app-server client/server/transport
old backend client
old cloud requirements
old exec server
old protocol-only DTO dependencies
```

## Required audit

```bash
cd vac-rs
cargo +1.93.0 metadata --no-deps --format-version 1
cargo +1.93.0 tree -p vac-surface-cli -i vac-app-server-client --edges normal,build
cargo +1.93.0 tree -p vac-surface-cli -i vac-app-server --edges normal,build
```

## Delete rule

Delete only when:

```text
not reachable from vac-cli normal/build/dev edges
not needed by active product feature
not needed by tests that validate root product path
workspace metadata passes after deletion
```

## Defer rule

If a crate is future enterprise/remote functionality, classify it as deferred capability instead of leaving it as hidden local dependency.
