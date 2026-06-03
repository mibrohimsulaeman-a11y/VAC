# Executor prompt — 00A build unblock

You are working in `/home/emp/Documents/VAC/vastar-agentic-cli`.

Goal: unblock default local build without restoring large vendor/cache trees by default.

Read first:

```text
docs/workflow-control-plane/plans/00A-build-unblock.md
docs/product/roadmap.md
docs/validation/LOCAL_RUNTIME_GATE.md
```

Constraints:

- do not start donor implementation,
- do not add product commands,
- do not restore deleted caches/vendor trees unless explicitly required,
- preserve unrelated changes.

Validation:

```bash
cd vac-rs
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-cli
cargo +1.93.0 metadata --no-deps --format-version 1
```

Final response format:

```text
status
files changed
build result
remaining blocker
UX impact
```
