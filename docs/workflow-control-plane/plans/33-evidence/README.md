# Plan 33 evidence pack — app-server reachability final proof

## Purpose

This folder is the evidence pack for Plan 33, the final proof gate for app-server Cargo retirement and delete-or-defer closure.

Plan 33 must not claim green from intent alone. Closeout requires captured evidence for:

- source grep reachability,
- inverse Cargo tree reachability,
- validation matrix results,
- workspace-wide app-server consumer classification,
- delete/defer closeout decision.

The files in this folder are templates until Plan 32 is green and the capture script has been run.

## Capture script

Reachability evidence only:

```bash
./scripts/capture-app-server-reachability-evidence.sh
```

Reachability plus full validation commands:

```bash
./scripts/capture-app-server-reachability-evidence.sh --include-validation
```

Optional output directory:

```bash
./scripts/capture-app-server-reachability-evidence.sh --out docs/workflow-control-plane/plans/33-evidence/runs/manual-run
```

The script writes raw command output under `runs/<timestamp>/` and prints the paths to copy/summarize into these templates.

## Evidence templates

| File | Evidence class | Fill when |
| --- | --- | --- |
| `source-grep-evidence.md` | TUI/source import grep evidence | After source grep capture is available |
| `inverse-cargo-tree-evidence.md` | `vac-cli` inverse Cargo tree evidence | After Cargo tree capture is available |
| `validation-matrix.md` | Full Plan 33 validation matrix | After validation commands complete |
| `closeout-evidence-index.md` | Final delete/defer decision index | During Plan 33 closeout |

## Required closeout rule

Do not mark Plan 33 complete unless the evidence shows one of these states truthfully:

1. app-server crates are unreachable from the default local product path and deleted, or
2. app-server crates are unreachable from the default local product path and explicitly classified as non-default/deferred with owner/path evidence.

## Source-of-truth links

- Plan: `docs/workflow-control-plane/plans/33-app-server-cargo-retirement-delete-defer-proof.md`
- Phase 00 closeout: `docs/migration/PHASE00_CLOSEOUT_STATUS.md`
- 00E reachability audit: `docs/migration/00E_REACHABILITY_AUDIT.md`
- Runtime reachability gate: `docs/workflow-control-plane/plans/00E-runtime-reachability-delete-gate.md`
