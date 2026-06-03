# O6.4 E2E Expansion Scaffold

Status: Scaffolded / NotEvaluated

This slice registers the E2E flows that must be proven once cargo/rustc/vendor are available:

- `vac init --scan`
- `vac init --rescan-ast`
- scanner doctor pass and failure fixtures
- `vac why` evidence-backed rationale
- `cargo build/test --workspace --offline`

Machine-readable state:

```text
.vac/registry/o6-e2e-expansion-state.yaml
```
