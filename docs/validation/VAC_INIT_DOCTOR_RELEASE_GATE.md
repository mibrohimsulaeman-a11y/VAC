# VAC-Init Doctor Release Gate

Gate script:

```bash
bash scripts/check-vac-init-doctor-release-contract.sh
```

Assertions:

- doctor release module exists;
- doctor taxonomy exists;
- aggregate release report exists;
- required doctors list exists;
- no-policy fail-closed rule exists;
- broken evidence chain block rule exists;
- capability and workflow manifests are registered.

Runtime expectation:

`vac doctor release .` must aggregate registry, surfaces, policy, ownership, workflow, evidence, build, memory, and init checks before marking workspace release-ready.
