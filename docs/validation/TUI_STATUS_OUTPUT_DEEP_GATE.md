# TUI Status Output Deep Gate

This validation gate belongs to Hardening 3 of the operator-console TUI work.

## Commands

```bash
df -h /mnt/data /tmp
rustfmt --edition 2024 --check vac-rs/tui/src/status/card.rs vac-rs/tui/src/status/mod.rs vac-rs/tui/src/status/output_contract.rs
rustc --edition 2024 --test vac-rs/tui/src/status/output_contract.rs -o /tmp/vac-status-output-contract-test
/tmp/vac-status-output-contract-test --nocapture
bash scripts/check-tui-status-output-contract.sh
bash scripts/check-tui-status-deep-hardening-contract.sh
bash scripts/check-tui-hardening-regression-lock.sh
python3 - <<'PY'
import pathlib, yaml
for p in pathlib.Path('.vac').rglob('*.yaml'):
    with open(p, 'r', encoding='utf-8') as f:
        yaml.safe_load(f)
print('YAML parse OK')
PY
```

## Gate Meaning

The gate proves that `/status` keeps operational rows while removing the old rate-limit/credit display section. It also proves that status display labels are contract-backed and that multi-provider/model rows have a validation model. Hardening 3 also proves that `Model provider` is a required row, default `vastar` is not hidden, and `Model providers` is available for registry-backed displays.

## Non-goal

This gate does not remove rate-limit protocol types. The protocol data can still exist for status-line or backend compatibility. The hardening target is the operator-facing `/status` card.

## Hardening 3 Addendum

The deep gate now verifies `StatusDisplayPolicy` explicitly rejects quota and balance rows, `/status` slash dispatch asserts the local-only refresh policy, and ambient limit warnings no longer tell users that `/status` contains an account-limit breakdown.
