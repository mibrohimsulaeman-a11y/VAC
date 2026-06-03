# VAC-Init Memory Governance Gate

Gate script:

```bash
bash scripts/check-vac-init-memory-governance-contract.sh
```

Assertions:

- memory governance module exists;
- tier model exists;
- credential-like content rejection exists;
- team memory approval requirement exists;
- memory write policy exists;
- capability and workflow manifests are registered.

Runtime expectation:

No memory tier may store credentials/secrets. Team memory is governed and requires human/operator approval.
