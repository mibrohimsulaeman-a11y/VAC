# VAC-Init Schema Envelope Gate

Status: Batch 0-1 gate contract

## Gate command

```bash
bash scripts/check-vac-init-schema-envelope-contract.sh
```

## What it checks

- `schema_envelope.rs` exists and defines `SchemaEnvelope`.
- `kind_registry.rs` exists and defines the canonical VAC v1-alpha kind registry.
- `kind_registry.rs` compiles directly and `schema_envelope.rs` compiles directly with `--cfg vac_standalone_schema_envelope` so it can import the sibling registry module without full workspace build.
- `mod.rs` exports both modules.
- `.vac/capabilities/vac-init-schema-envelope.yaml` registers the capability.
- `.vac/workflows/maintenance.vac-init-schema-envelope.yaml` registers the workflow.
- `.vac` YAML parse succeeds.
- Source artifact hygiene gate passes.

## Safety stance

The gate is safe-read only. It does not execute arbitrary manifest-defined commands and does not mutate source.

## Standalone Rust Harness Note

`schema_envelope.rs` is validated with `rustc --cfg vac_standalone_schema_envelope --test`; the cfg imports `kind_registry.rs` locally so the gate remains dependency-light without a full workspace build.
