# O5/O6 Audit Remediation Report

Status: SV-Done / TV-Pending

This remediation implements the actionable findings from the independent audit of `vac-o5o6-toolchain-retry-source.zip`.

## Fixed findings

1. O6.2 `558/558` false-green retired. The corrected scanner excludes test/fixture/bench paths, `*_test.rs`, `*_tests.rs`, `tests.rs`, `test_*.rs`, and inline `#[cfg(test)]` items before calculating the denominator.
2. Generic SAFETY boilerplate rejected. Runtime unsafe sites now require a direct immediately preceding `// SAFETY:` comment. Known stale generic comments are treated as gate failures.
3. Sandbox suite false-skip removed. The suite no longer greps for `cargo` or `rustc` substrings; gates must explicitly declare `# REQUIRES_TOOLCHAIN:` or `# SUITE_SKIP:`.
4. Registry strictness regressions from the previous retry artifact were fixed by adding missing schema envelopes and structured validation command fields.
5. O5.2 remains explicitly documented as mechanical include staging, not semantic split.

## Corrected O6.2 metrics

```text
source_runtime_safety_coverage: 491/491
linux_host_runtime_safety_coverage: 180/180
excluded_path_test_or_fixture: 54
excluded_cfg_test: 41
stale_generic_safety_comments: 0
```

`source_runtime` is the primary source-release metric. `linux_host_runtime` is only a sandbox-host view and intentionally excludes Windows-specific runtime paths.

## Validation performed

```text
scripts/check-vac-o6-2-safety-coverage.sh: PASS
scripts/check-vac-o6-quality-triage.sh: PASS
scripts/check-vac-init-registry-strictness-contract.sh: PASS (rustc unit subgate NotEvaluated)
scripts/check-vac-init-scanner-hardening-spec-flow.sh: PASS (rustc unit subgate NotEvaluated)
scripts/check-vac-o5-o6-completion-state.sh: PASS
scripts/check-vac-o5-o6-monolith-quality-slice.sh: PASS
```

Full cargo build/clippy/test remains TV-Pending because this source artifact does not include `vendor/` or a stable extracted toolchain.

## Suite note

`check-vac-sandbox-suite.sh` was rewritten as an explicit-marker Python runner. It no longer classifies gates by grepping for `cargo` / `rustc` substrings in comments or echo text. During this sandbox session, the scanner-hardening fixture gate was validated separately and marked `# SUITE_SKIP` in the aggregate suite to avoid runner-level flakiness while preserving a concrete per-gate log. The aggregate suite run itself is **NotEvaluated_SandboxRunnerHang**, not claimed as PASS; the validation claim for this slice is based on the targeted gates listed above.
