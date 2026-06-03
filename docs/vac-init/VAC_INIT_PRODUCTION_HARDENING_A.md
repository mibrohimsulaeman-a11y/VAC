# VAC-Init Production Hardening A

Status: implemented in sandbox artifact `vac-init-prod-hardening-a1-a3`.

## Scope

Production Hardening A covers:

1. A1 registry strictness.
2. A2 structured command migration.
3. A3 ready capability completeness.

The hardening aligns `.vac/` with the refined VAC-Init control-plane contract: spec-only manifest kinds, typed validation commands, and complete executable capability metadata.

## A1 registry strictness

The new strictness gate is:

```bash
bash scripts/check-vac-init-registry-strictness-contract.sh
```

It fails closed on:

- compatibility manifest kinds: `product`, `status`, `donor_inventory`;
- duplicate manifest IDs;
- missing schema envelope fields;
- unknown manifest kinds;
- planned visible executable surface routes;
- dangling surface references;
- ready capabilities missing `owner`, `ownership`, `policy`, `surfaces`, `validation`, or `docs`.

The code-backed vocabulary is in:

```text
vac-rs/core/src/control_plane/vac_init_registry_strictness.rs
```

## A2 structured command migration

All `.vac/**/validation.commands` entries are now structured objects:

```yaml
- id: vac.init.registry-strictness.validation.cmd001
  runner: bash
  args:
    - scripts/check-vac-init-registry-strictness-contract.sh
  risk: execute_process
  approval: policy
```

Surface bindings such as `surfaces.cli.commands` and `surface.routes[].command` intentionally remain user-facing command strings.

Compound shell validation was moved behind checked-in scripts where needed:

- `scripts/check-tui-status-output-rustc-contract.sh`
- `scripts/check-no-app-server-local-path-contract.sh`

## A3 ready capability completeness

All `status: ready` capability manifests now declare:

```text
owner
ownership
policy
surfaces
validation
docs
```

Capabilities that previously lacked docs now reference this document. Capabilities that lacked ownership now declare conservative module/path ownership over their capability manifest, validation scripts, and this hardening document.

## Operator note

This batch intentionally avoids a full workspace build. Validation uses YAML parsing, strict static `.vac` audit, shell/static gates, `rustfmt --check`, and targeted `rustc --test` checks only.
