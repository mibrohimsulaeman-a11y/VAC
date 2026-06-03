# Plan 23 BLOCKED-OPERATOR release policy closeout — 2026-05-28

Status: implementation complete for release policy semantics.

Evidence:

- `TuiPtyGateResultState::satisfies_release_gate()` returns true only for `Passed`.
- `BlockedOperator` and `SkippedNotApplicable` do not satisfy release readiness.
- `vac doctor runtime-owner-gates` now treats `BLOCKED-OPERATOR` evidence without a real PTY pass as a hard `pty_false_green` release blocker.

Operator note:

A real TTY evidence run can still be attached for release promotion, but lack of that evidence is no longer ambiguous or false-green: it blocks release by policy.
