# VAC dogfood session — assistant model as bounded worker

Date: 2026-05-30T00:00:00Z

## Scope

This session dogfoods VAC using the assistant as the model worker. The session followed a VAC-like loop:

1. Load current source artifact.
2. Run pre-flight gates.
3. Create a bounded Semantic Plan.
4. Apply only the approved bounded patch.
5. Run post-validation gates.
6. Write safe evidence and trajectory references.

## Baseline

```text
vac-source-docs-plan-codebase-reconciliation.zip
```

## Findings

Pre-flight gates passed. During dogfooding, the assistant found a hygiene defect in `scripts/check-no-hardcoded-readiness-scoreboard.sh`: it wrote diagnostic grep output to fixed `/tmp/vac-hardcoded-*.txt` paths. In this sandbox, those paths produced permission warnings even when the script returned success.

## Patch

The script now uses:

```bash
TMPROOT="$(mktemp -d "${TMPDIR:-/tmp}/vac-hardcoded-readiness.XXXXXX")"
trap 'rm -rf "$TMPROOT"' EXIT
```

and writes diagnostics under that temp directory.

## Safety

No raw/private chain-of-thought is persisted. This document records only safe decision summary, command results, and evidence references.

## Validation

```text
bash scripts/check-vac-dogfood-session.sh
bash scripts/check-vac-init-registry-strictness-contract.sh
bash scripts/check-vac-workflow-spec-compliance.sh
bash scripts/check-no-hardcoded-readiness-scoreboard.sh
bash scripts/check-tui-source-artifact-hygiene.sh
```
