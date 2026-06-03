# Executor prompt — 00B Local Runtime Contract

You are working in `/home/emp/Documents/VAC/vastar-agentic-cli`.

Goal: implement minimal Local Runtime Contract and wire at least one product-local reference.

Read first:

```text
docs/architecture/decisions/ADR-0007-local-runtime-contract.md
docs/architecture/local-runtime-contract.md
docs/workflow-control-plane/plans/00B-local-runtime-contract.md
```

Constraints:

- do not create large backend-only abstraction,
- do not mimic old protocol naming,
- wire immediately toward TUI/exec path,
- preserve unrelated changes.

Validation:

```bash
cd vac-rs
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-cli
```

Final response format:

```text
status
files changed
contract types added
where wired
build result
remaining blocker
UX impact
```
