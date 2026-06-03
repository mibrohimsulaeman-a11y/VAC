# PRD — Onboarding, Doctor, and Readiness

## Overview

VAC needs a readiness surface so users understand why the product can or cannot execute tasks.

Doctor/readiness is a P0/P1 capability because build/provider/sandbox/connector failures should be visible as operator states, not mysterious runtime crashes.

## User value

User can quickly see:

```text
model/provider ready?
sandbox ready?
local runtime ready?
connectors ready?
control-plane manifests valid?
TUI input path working?
validation tools available?
```

## Required checks

Initial readiness checks:

- local runtime contract available,
- model/provider configured when required,
- sandbox posture available or degraded,
- control-plane manifests valid when present,
- approval policy loaded,
- TUI input path healthy,
- working directory valid,
- validation commands discoverable when relevant,
- connectors connected/degraded/disconnected,
- secret/redaction engine active.

## TUI surface

```text
/status
/doctor
/capabilities
```

Readiness should show:

- status,
- failure reason,
- recovery hint,
- whether degraded mode is safe,
- action to repair when available.

## Status model

```text
ready
degraded
missing_config
policy_denied
unavailable
failed
unknown
```

## Acceptance criteria

- User can run a doctor/readiness check from TUI or CLI.
- Missing sandbox/vendor/native dependency is shown as readiness issue when build can still proceed.
- Connector auth failure is visible and does not break local core features.
- Provider missing config has clear recovery hint.
- Manifest parse errors are shown with file/path details.
