# Doctor and Readiness Surface contract

## Routes

```text
/doctor
/status
```

## Data sources

- local runtime readiness,
- provider/model readiness,
- sandbox readiness,
- connector readiness,
- manifest registry diagnostics,
- policy/redaction readiness.

## Readiness states

```text
ready
degraded
missing_config
policy_denied
unavailable
failed
unknown
```

## Required fields

```text
component
status
reason
recovery hint
blocking/non-blocking
last checked
```

## Actions

```text
r: refresh
Enter: inspect component
f: run suggested fix when safe and supported
Esc: close detail
```

## Acceptance

- Missing optional sandbox/vendor input is shown as degraded/unavailable when build can proceed.
- Missing provider config includes recovery hint.
- Manifest parse errors include path and reason.
