# VAC-Init Safe Rationale Gate

Gate script:

```bash
bash scripts/check-vac-init-why-contract.sh
```

Assertions:

- safe rationale module exists;
- `WhyQuery`, `TrajectoryIndex`, and lookup function exist;
- raw/private chain-of-thought exclusion is modeled;
- capability and workflow manifests are registered;
- validation docs exist.

Runtime expectation:

`vac why` output must contain safe decision records, policy refs, evidence refs, memory refs, and chain depth, but must not expose raw/private chain-of-thought.
