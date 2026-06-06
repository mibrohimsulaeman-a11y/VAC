# VAC Control Plane (lock + cache model)

`.vac/` is a THIN, declarative intent layer over the codebase — NOT a second codebase.

Mental model (borrowed from Cargo):

| Cargo       | `.vac/`                                                        | Nature                                   |
|-------------|----------------------------------------------------------------|------------------------------------------|
| `Cargo.toml`| `capabilities/`, `policies/`, `surfaces/`, `workflows/`, `ledger/` | AUTHORED — small, human-reviewed         |
| `Cargo.lock`| `derived/` (ownership, inventory, risk, surface-coverage)      | DERIVED — regenerated, never hand-edited |
| `target/`   | `cache/`                                                       | transient, git-ignored                   |

## Hard rules

1. **Authored vs derived are separated.** Anything computable from code lives in `derived/` and is git-ignored. Never hand-maintain ownership maps, inventories, or risk findings.
2. **Gates are binary.** A gate is `pass` or `fail`. "Not evaluated" is NOT a pass — it is fail-closed. There is no `pass_or_recorded_pending`.
3. **Deferrals are Waivers.** To ship with a known gap, create an expiring, owned, signed Waiver in `ledger/waivers.yaml`. A waiver is the ONLY way a finding stops blocking, and it expires (then the finding re-opens automatically).
4. **Evidence is emitted by the runner, never authored by the agent.** Agents propose plans; the gate runner records results with the real exit code + artifact hash. Self-graded evidence is rejected.
5. **One living ledger per concern.** Findings live in a single `ledger/findings.yaml` keyed by a stable `finding_id` with append-only state transitions — not a new file per remediation cycle.
6. **The control plane obeys its own anti-bloat rule.** Authored `.vac/` size is budgeted in `vac.toml` and enforced by `vac doctor budget .`.

## Commands

```bash
vac scan .     # rewrites derived/ (ownership, inventory, risk) from code + annotations
vac doctor .   # runs all gates, fail-closed; absence of evidence == fail
vac why <file>:<line>   # explain why a change is safe (policy + approval + evidence), no raw CoT
```

Do NOT place legacy skill packs, ad-hoc scripts, alternate runtimes, or generated inventories in the authored tree.
