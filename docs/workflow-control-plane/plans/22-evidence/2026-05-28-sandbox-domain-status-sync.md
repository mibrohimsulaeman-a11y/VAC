# Plan 22 domain/status sync — 2026-05-28

Status: complete for current `.vac` domain registry alignment.

## What changed

- `.vac/registry/domains.yaml` now marks the current root domains as `ready` to match live capability/workflow status after Plan 14, 16, 19, 20, 21, 23, 30, 31, 32, 33, and 34 closeouts.
- Plan 22 wording no longer says `vac.release` must remain partial; release readiness is now controlled by release-gate evidence and PTY/BLOCKED-OPERATOR semantics.
- `vac.build` readiness is tied to the targeted approval-gated build check; full workspace build is explicitly operator-gated release evidence.

## Validation evidence

- YAML parse of `.vac/registry/domains.yaml`: pass.
- Static status check for `.vac/capabilities/*.yaml`, `.vac/workflows/*.yaml`, and `.vac/registry/domains.yaml`: pass.
- Stale wording sweep for old `release partial` and build partial claims: pass.
