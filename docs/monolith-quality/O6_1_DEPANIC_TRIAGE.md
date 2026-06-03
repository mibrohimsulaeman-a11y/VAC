# O6.1 Runtime Panic Surface Triage

Status: **SV-Done / TV-Pending**.

This slice is a reproducible source-static measurement, not a de-panic refactor. It does not claim that `.unwrap()`, `.expect(...)`, `panic!`, `unreachable!`, or `unimplemented!` call-sites have been removed.

Scanner:

```bash
python3 scripts/measure-vac-o6-1-runtime-panic-surface.py --json
bash scripts/check-vac-o6-1-depanic-surface.sh
```

The gate now compares scanner JSON against `.vac/registry/o6-panic-surface.yaml` and `.vac/registry/o6-quality-triage.yaml` field-by-field, so the `251` denominator is reproducible by the committed tokenizer rather than a hand-written claim.

Current tokenizer result:

```text
total_runtime_panic_surface: 251
unwrap: 80
expect: 32
panic_macro: 86
todo_macro: 0
unimplemented_macro: 1
unreachable_macro: 44
runtime_files_scanned: 1251
cfg_test_lines_removed: 134182
```

Method: runtime Rust paths only; test paths, `*_tests.rs`, `tests_*.rs`, inline `#[cfg(test)]`, comments, strings, char literals, and raw strings are excluded before token counting.

TV caveat: cargo/clippy/tests are still NotEvaluated, and actual panic-site refactoring remains pending.
