# O5/O6 Actual-Code Closure Report

Implemented the latest continuation blueprint as source/static slices. This report intentionally distinguishes runtime-effective source hooks from marker-only placeholders.

## Closed or hardened

- Corrected readiness ledger for P-2/V-1/V-2/L-1/F-2 false readiness.
- Replaced `ast_exact` live scanner method with typed `RiskDetectionMethod::AstExact`.
- Reworked strict semantic anchors around a Rust token syntax tree and kept line heuristics only as degraded fallback.
- Strengthened canonical evidence coverage and doctor canonical-coverage reporting.
- Added surface visibility contract: planned routes cannot be visible; partial routes require guidance; ready routes cannot be hidden.
- Made ChatWidget windowed render plan active instead of discarded.
- Added user-visible auth-state event/banner path, startup-exit evidence, shimmer frame metrics, theme contrast logic, and typed command availability badges.
- Moved `agent-identity` and `otel` into layer folders and removed README-only `memories` stub.

## Still TV-Pending

- cargo check/test/clippy.
- numeric benchmark results.
- live TUI smoke.
- full `syn`/`tree-sitter` AST parser dependency integration.
- compile-guided surface package rename and deep core decomposition.
