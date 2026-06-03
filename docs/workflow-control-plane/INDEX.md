# Workflow Control Plane Index

The workflow control plane is the declarative product layer for VAC. It lives in `.vac/` and is described by the docs in this directory.

## Start Here

- [Implementation plan](IMPLEMENTATION_PLAN.md)
- [Initial manifest set](INITIAL_MANIFEST_SET.md)
- [Interference audit](INTERFERENCE_AUDIT.md)
- [Plan index](plans/INDEX.md)
- [Schema index](schema/INDEX.md)
- [Legal notices](../legal/NOTICES.md)

## Product Rules

- A backend-only feature is not complete until it has a capability manifest and a visible root TUI or CLI surface.
- Every root capability should have an ownership story, a validation command, and a cleanup decision for donor code.
- Docs, manifests, and dashboard output must agree on what the product can do.

## Maintenance Checks

Prefer one build and reuse the binary for repeated doctor checks:

```bash
cargo +1.93.0 build --manifest-path vac-rs/Cargo.toml -p vac-surface-cli
./vac-rs/target/debug/vac doctor registry <repo-root>
./vac-rs/target/debug/vac doctor architecture <repo-root>
./vac-rs/target/debug/vac doctor ownership <repo-root>
./vac-rs/target/debug/vac doctor workflow <repo-root>
./vac-rs/target/debug/vac doctor docs <repo-root>
./vac-rs/target/debug/vac doctor build <repo-root>
./vac-rs/target/debug/vac doctor donor <repo-root>
./vac-rs/target/debug/vac doctor release <repo-root>
git diff --check
```

Compare the dashboard output in `/capabilities` with the manifests under `.vac/capabilities/` before promoting capability status.

## Notes

- Product identity stays VAC-only.
- Legal notices stay separate from product identity and root command naming.
- Do not revive donor frontend stacks as a second product TUI.
- Keep plan text aligned with the live dashboard and doctor output.
