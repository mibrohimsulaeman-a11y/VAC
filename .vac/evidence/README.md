# Evidence (runner-emitted, append-only)

Evidence records are written ONLY by the gate/command runner — never authored by the agent.
Each record carries the real command, exit code, artifact hash, and (optionally) an ed25519 signature.

- Authored or hand-edited evidence is rejected by `vac doctor evidence .`.
- There is NO `completion: not_evaluated` record. A gate that did not run produces NO evidence,
  and absence of evidence is treated as failing (fail-closed).
- Records are append-only; see `.gitattributes`.

Record file naming: `<workflow-id>.<command-id>.<unix-ts>.evidence.yaml`
