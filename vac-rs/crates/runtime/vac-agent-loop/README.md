# vac-agent-loop

`vac-agent-loop` owns the VAC v1.5 bounded agent runtime contract. Provider streaming and tool I/O stay outside this boundary; this crate models how an agent request is admitted, bounded, validated, and closed.

## Runtime source

```text
src/bound_runtime.rs
src/runtime_e2e.rs
```

## Contract

The runtime must enforce:

1. Runtime authority from compiled JSON only.
2. Capability readiness through `declared / computed / effective`, with effective never stronger than computed.
3. Semantic Plan before patch.
4. Mandatory task/spec/todo artifact lock.
5. Bounded patches by file, line range, semantic anchor, ownership, and budget.
6. Pre-command gate and structured command gate; no free-form shell or shell metacharacter execution.
7. Evidence, SpecSync, readiness, ownership, and assessment closeout.
8. Completion lock: no silent done when artifacts, evidence, or v1.5 conditions are unresolved.

## Sandbox validation

```bash
python3 scripts/vac-runtime-agent-e2e-sv.py
```

Cargo tests remain TV-Pending until the local fix loop.
