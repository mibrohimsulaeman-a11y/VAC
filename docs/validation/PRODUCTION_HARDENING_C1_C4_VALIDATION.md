# Production Hardening C1-C4 Validation

## Targeted gates

```bash
rustfmt --check \
  vac-rs/core/src/control_plane/vac_init_runtime_enforcement.rs \
  vac-rs/core/src/control_plane/mod.rs

rustc --edition 2024 --test \
  vac-rs/core/src/control_plane/vac_init_runtime_enforcement.rs \
  -o /tmp/vac_init_runtime_enforcement_test

/tmp/vac_init_runtime_enforcement_test --nocapture

bash scripts/check-vac-init-runtime-gate-enforcement-contract.sh
bash scripts/check-vac-init-registry-strictness-contract.sh
bash scripts/check-tui-source-artifact-hygiene.sh
```

## Expected result

- Pre-plan gate blocks missing/unapproved plan, missing policy, missing ownership report, planned/deprecated capability, unowned/overclaimed/hidden targets, and ownership mismatch.
- Pre-patch gate blocks missing plan binding, unapproved plan, and patch guard denial.
- Pre-command gate blocks free-form command, unknown runner, missing policy, policy deny, and approval-required command without persisted approval request.
- Evidence completion gate blocks missing evidence, broken evidence chain, failed validation, new orphan files, and missing approval reference.
- `.vac` manifests remain strict and validation commands remain structured.

## Build policy

This validation intentionally avoids full workspace build. Full release build remains a later operator-gated release activity.
