# Executor prompt — 00E runtime delete gate

You are working in `/home/emp/Documents/VAC/vastar-agentic-cli`.

Goal: audit and delete/defer old runtime crates after Local Runtime Contract rewiring.

Read first:

```text
docs/migration/OLD_RUNTIME_DELETE_GATE.md
docs/workflow-control-plane/plans/00E-runtime-reachability-delete-gate.md
docs/donor-migration/DONOR_SHELL_STACK_QUARANTINE.md
```

Constraints:

- do not delete reachable crates,
- do not delete donor source unless explicitly asked,
- remove workspace members only after metadata validates,
- preserve unrelated changes.

Validation:

```bash
cd vac-rs
cargo +1.93.0 metadata --no-deps --format-version 1
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-cli
```

Final response format:

```text
status
crates deleted
crates deferred
reachability result
build result
UX impact
```
