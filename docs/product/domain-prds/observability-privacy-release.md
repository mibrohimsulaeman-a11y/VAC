# PRD — Observability, privacy, and release operations

## Overview

VAC must make execution inspectable, privacy-aware, and releasable through workflow-native gates.

## Observability requirements

VAC should capture and surface:

- activity events,
- workflow lifecycle events,
- approval decisions,
- tool calls and results,
- validation results,
- session summaries,
- failure reasons,
- evidence artifacts.

## Privacy requirements

VAC should protect sensitive data by:

- detecting likely secrets,
- redacting unsafe values before external exposure,
- keeping redaction logs type-only,
- avoiding raw secret output in TUI diagnostics,
- applying policy to exports and telemetry.

## Evidence requirements

Evidence should answer:

```text
what happened?
why did it happen?
what files changed?
what approval allowed it?
what validation ran?
what failed?
```

## Release operations

Release readiness should be a workflow, not a manual checklist.

Release gate should include:

- identity check,
- build check,
- manifest schema validation,
- capability dashboard check,
- workflow browser check,
- TUI PTY gate,
- policy/approval check,
- packaging smoke when available.

## Runbook requirements

Operator runbooks should exist for:

- engine init failure,
- provider readiness failure,
- scheduler stuck,
- external tool provider down,
- import/export failure,
- disk full,
- terminal/TUI failure.

## Acceptance criteria

- Release gate appears in workflow browser.
- Build and identity checks are workflow-native.
- Sensitive values are not printed in diagnostics.
- Failures include recovery hints.
