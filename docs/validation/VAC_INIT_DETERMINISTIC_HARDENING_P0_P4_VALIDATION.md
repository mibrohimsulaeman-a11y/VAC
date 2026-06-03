# VAC-Init Deterministic Hardening P0-P4 Validation

## Gates

```bash
bash scripts/check-no-hardcoded-readiness-scoreboard.sh
bash scripts/check-vac-workflow-spec-compliance.sh
bash scripts/check-vac-doctor-release-real-reports.sh
bash scripts/check-vac-init-runtime-gate-callsite-integration.sh
bash scripts/check-vac-init-live-stores.sh
bash scripts/check-vac-init-registry-strictness-contract.sh
bash scripts/check-tui-source-artifact-hygiene.sh
```

## Expected result

- No hardcoded all-pass/5-of-5 scoreboard remains.
- Workflow manifest typed shape violations are zero.
- Release doctor aggregates real report loaders.
- Runtime gate call-sites exist outside control-plane definition modules.
- `vac init` writes store records through live atomic store helper.
