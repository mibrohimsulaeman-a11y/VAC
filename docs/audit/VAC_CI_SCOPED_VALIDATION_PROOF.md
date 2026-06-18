# VAC CI Scoped Validation Proof

Status source: CI-attested scoped proof.

This proof is intentionally narrow. It attests that the GitHub Actions `CI` job reached the proof step after the preceding validation steps succeeded. It does not claim broker mediation, OS isolation, external audit anchoring, or L2 enforcement.

```text
ci_scoped_validation=TV-Pending
ci_scoped_validation_execution=observed_l1
ci_scoped_validation_custody=local_only
l2_broker=NotImplemented
```

The proof artifact path is:

```text
.vac/evidence/ci-scoped-validation-current.json
```

Verifier:

```text
scripts/check-ci-scoped-validation-proof.py
```

CI producer:

```text
scripts/ci-scoped-validation-proof.py
```

For `TV-Pass`, the verifier requires a current proof with `custody=ci_attested`, matching GitHub run SHA, matching scoped source hash, all required CI validation checks marked `TV-Pass`, and `l2_broker=NotImplemented`.
