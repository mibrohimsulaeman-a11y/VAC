# Plan 19 evidence — sandbox root capability ready sync

Date: 2026-05-28

## Scope

Root seed manifests and the typed root feature catalog were reconciled after the
owner-native runtime path landed:

- `vac.chat`, `vac.tools`, `vac.sandbox`, `vac.sessions`, `vac.identity`,
  `vac.workflow`, `vac.identity.check`, `vac.ownership`, `vac.architecture`,
  and `vac.donor_migration` now reflect ready root-manifest status.
- `vac.approvals` and `vac.build` are now ready after Plan 14/16 readiness closeout. Full workspace build remains explicit operator-gated release evidence rather than a partial capability label.
- `ROOT_SEED_CAPABILITY_REQUIREMENTS` expected source roots now point at real
  current workspace paths instead of historical module names.

## Validation

- All root feature catalog `expected_source_roots` exist in the sandbox source tree.
- All `.vac/workflows/*.yaml` are ready and every workflow step resolves to the
  safe-runner vocabulary.
- YAML parse for touched capability and workflow manifests passed.

## Follow-up sync

- 2026-05-28 later sandbox batch promoted approvals/build to ready with code-backed readiness reports.
- `.vac/registry/domains.yaml` now mirrors ready domain status for current root product domains.
