# VAC-Init Ownership Scanner Gate

The ownership scanner gate covers source inventory classification, ownership target matching, quarantine classification, and action suggestions.

Required script:

```bash
bash scripts/check-vac-init-ownership-scanner-contract.sh
```

Unowned source code must be hard-quarantined. Overclaimed files must block writes and emit resolve-overclaim actions.
