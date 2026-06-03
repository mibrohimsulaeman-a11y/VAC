# Executor prompt — 00C rewire vac exec

You are working in `/home/emp/Documents/VAC/vastar-agentic-cli`.

Goal: rewire `vac exec` local prompt path to Local Runtime Contract.

Read first:

```text
docs/migration/VAC_EXEC_REWIRE_SPEC.md
docs/architecture/local-runtime-contract.md
docs/workflow-control-plane/plans/00C-rewire-vac-exec.md
```

Constraints:

- do not fake no-op runtime,
- preserve real prompt submission semantics,
- remove old runtime client dependency only when actual replacement exists,
- preserve unrelated changes.

Validation:

```bash
cd vac-rs
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-cli
cargo +1.93.0 tree -p vac-surface-cli -i vac-app-server-client --edges normal,build
```

Final response format:

```text
status
files changed
old path removed or remaining
new runtime event flow
build result
UX impact
```
