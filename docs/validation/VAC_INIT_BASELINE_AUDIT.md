# VAC-Init Baseline Audit

Status: Batch 0-1 validation gate  
Baseline source artifact: `vac-source-tui-operator-ui-hardening4-10.zip`  
Spec baseline: `VAC_Init_Control_Plane_Spec_v1_alpha_refined.docx`

## Scope

This audit covers Batch 0 Baseline Audit and Batch 1 Schema Envelope / Kind Registry. It intentionally does not implement the interactive `vac init` lifecycle, scanners, policy evaluator, structured command gate, semantic plan validator, approval replay protection, evidence chain, or `vac why`.

Full workspace build/link is intentionally not used in this sandbox gate. Validation uses dependency-light `rustc --test` harnesses, `rustfmt`, `.vac` YAML parsing, static manifest checks, and source artifact hygiene.

## Audit Findings

| Area | Finding | Hardening Action |
|---|---|---|
| `.vac/` envelope | Most manifests already had `schema_version` and `kind`; `.vac/registry/domains.yaml` needed a top-level `id`. | Added `id: registry.domains`. |
| Kind registry | Existing typed manifest loaders existed, but no central VAC-Init kind registry. | Added `kind_registry.rs`. |
| Envelope parser | Existing loaders parse specific manifest classes, not the generic early envelope. | Added dependency-free `schema_envelope.rs`. |
| Current registry descriptors | Active `.vac/registry/*.yaml` descriptors now use `registry_status`; old names are preserved only in `legacy_kind` fields for audit context. | Production Hardening A strictness gate blocks `product`, `status`, and `donor_inventory` as active manifest kinds. |
| Validation gate | TUI hardening gates existed, but VAC-Init foundation gates did not. | Added `check-vac-init-schema-envelope-contract.sh` and `check-vac-init-baseline-contract.sh`. |

## Gate Commands

```bash
df -h /mnt/data /tmp
rustfmt --edition 2024 --check \
  vac-rs/core/src/control_plane/schema_envelope.rs \
  vac-rs/core/src/control_plane/kind_registry.rs
bash scripts/check-vac-init-schema-envelope-contract.sh
bash scripts/check-vac-init-baseline-contract.sh
bash scripts/check-tui-source-artifact-hygiene.sh
```

## Batch 0-1 Result Contract

- All `.vac/**/*.yaml` files expose `schema_version`, `kind`, and `id`.
- Spec-required kind names are present in code.
- Compatibility registry kinds are explicit and documented.
- Unknown kinds are rejected.
- Non-dotted IDs are rejected by the strictness gate; the former root product descriptor was migrated from `id: vac` to `id: vac.registry.product` with `legacy_id: vac`.
- Source artifact hygiene remains clean.
