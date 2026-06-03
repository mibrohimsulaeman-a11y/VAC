# Wave 3A — Plan 30 closeout reconciliation

Date: 2026-05-25
Result: **Plan 30 remains partial / not complete**

## Inputs reviewed

- `docs/workflow-control-plane/plans/30-evidence/2026-05-25-30F-operator-ux-gate.md`
- `docs/workflow-control-plane/plans/30-evidence/2026-05-25-30G-skills-external-agent.md`
- `vac-rs/local-runtime-owner/src/command_bus.rs`
- `vac-rs/local-runtime-owner/src/lib.rs`
- `vac-rs/tui/src/app_server_session.rs`

## Reconciled status

- **30F:** terminal as `BLOCKED-OPERATOR` for planning purposes. This is not a real TTY pass and must not be described as product-smoke coverage.
- **30G:** blocked/partial. Current-cwd `skills/list` is owner-bus backed, but arbitrary-cwd skills, plugin operations, and external-agent config detect/import are not complete owner-backed provider paths.

## 30G fallback verification

Code inspection confirms the unsupported 30G paths do not silently report owner-bus success:

- arbitrary-cwd skills returns `RuntimeCommandBusError::SkillsArbitraryCwdUnsupported(cwd)`, then TUI logs and falls back to the app-server request path;
- plugin operations return `RuntimeCommandBusError::PluginSurfaceProviderUnavailable(operation)`;
- external-agent detect/import returns `RuntimeCommandBusError::ExternalAgentConfigProviderUnavailable`, then TUI logs and falls back to the existing app-server provider.

Because these compatibility fallbacks still exist, Plan 30 must remain **partial / not complete**. The correct blocker for Plan 31 is now 30G provider completion or an explicit, documented re-scope — not 30F.

## Plan 31 impact

- Plan 31B implementation should not start until the dirty tree is triaged and the 30G blocker is resolved or explicitly re-scoped.
- When it does start, begin with low-risk alias/DTO mapping and conversion tests.
- Do not delete app-server crates in Plan 31B.
- Do not proceed to Plan 32 no-app-server gates or Plan 33 final delete/defer proof before Plan 31 is green.

## Validation

Docs-only reconciliation; no Rust code changed.

Required validation:

```sh
git diff --check -- docs/workflow-control-plane/plans/30-prompt-and-active-controls-cutover.md docs/workflow-control-plane/plans/31-inventory.md docs/workflow-control-plane/plans/31-mapping.md docs/workflow-control-plane/plans/30-evidence/2026-05-25-wave3a-plan30-closeout-reconciliation.md
rg -n "30F|30G|BLOCKED-OPERATOR|Plan 31|fallback|not complete|blocked/partial" docs/workflow-control-plane/plans/30-prompt-and-active-controls-cutover.md docs/workflow-control-plane/plans/31-inventory.md docs/workflow-control-plane/plans/31-mapping.md docs/workflow-control-plane/plans/30-evidence/2026-05-25-wave3a-plan30-closeout-reconciliation.md
```
