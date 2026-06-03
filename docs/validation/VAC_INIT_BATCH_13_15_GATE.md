# VAC-Init Batch 13-15 Gate

Aggregate script:

```bash
bash scripts/check-vac-init-batch13-15-contract.sh
```

Scope:

- Batch 13: `vac why` safe rationale.
- Batch 14: memory governance.
- Batch 15: doctor aggregate/release gate.

This gate also parses `.vac` YAML and invokes earlier batch gates when present.
