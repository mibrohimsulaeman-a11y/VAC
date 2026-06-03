# O5/O6 Control Plane Audit Findings Remediation

Date: 2026-06-01

## Scope

This slice closes the latest source/static audit findings across VAC-Init control plane, runtime-readiness surfaces, TUI operator console, autopilot scheduler, workflow execution, scanner precision, and source artifact packaging.

## Implemented changes

- Added dependency-free `ast_exact` scanner pass with Rust lexical stripping for comments/string/char literals before call-site risk matching.
- Added `vac_init_autopilot_scheduler` runtime producer/executor that writes `.vac/registry/autopilot/status.yaml` and action evidence.
- Routed `/runtime` to a persistent operator console view with live refresh and cancel/retry/inspect/attach handlers.
- Routed `/workflow run <id>` into the workflow runner instead of a display-only hint.
- Replaced hardcoded status-bar `profile default · rulebook vil.core` with runtime environment driven labels.
- Added source artifact packaging gate for ZIP hygiene, legal files, evidence registry, and compile-debt ledger.

## Verification truth

Source/static gates pass in this sandbox. Rust cargo, live terminal smoke, and pixel parity remain TV-Pending because the sandbox does not provide `cargo`/`rustc`.
