# VAC control plane

Declarative root of the VAC product. YAML declares; Rust validates; the doctor inspects; policy gates; approval protects.

Every manifest is typed, versioned (`schema_version: 1`), and validated by a loader in `vac-rs/crates/control-plane/control-plane/src/control_plane/`. Authoritative schema reference: [`docs/workflow-control-plane/schema/INDEX.md`](../docs/workflow-control-plane/schema/INDEX.md).

## Families

| Family | Directory | Schema source (Rust) | Schema doc | Doctor gate |
|---|---|---|---|---|
| Capabilities | [`capabilities/`](capabilities/README.md) | `vac-rs/crates/control-plane/control-plane/src/control_plane/capability_manifest.rs` | [`capability-manifest.schema.md`](../docs/workflow-control-plane/schema/capability-manifest.schema.md) | `vac doctor registry .` |
| Workflows | [`workflows/`](workflows/README.md) | `vac-rs/crates/control-plane/control-plane/src/control_plane/workflow_manifest.rs` | [`workflow-manifest.schema.md`](../docs/workflow-control-plane/schema/workflow-manifest.schema.md) | `vac doctor workflow .` |
| Policies | [`policies/`](policies/README.md) | `vac-rs/crates/control-plane/control-plane/src/control_plane/policy_manifest.rs` | [`policy-manifest.schema.md`](../docs/workflow-control-plane/schema/policy-manifest.schema.md) | `vac doctor policy .` |
| Surfaces | [`surfaces/`](surfaces/README.md) | `vac-rs/crates/control-plane/control-plane/src/control_plane/surface_manifest.rs` | [`surface-manifest.schema.md`](../docs/workflow-control-plane/schema/surface-manifest.schema.md) | `vac doctor surfaces .` |
| Registry | [`registry/`](registry/README.md) | loaders under `vac-rs/crates/control-plane/control-plane/src/control_plane/` | [`registry.schema.md`](../docs/workflow-control-plane/schema/registry.schema.md) | `vac doctor registry .` |

## Naming conventions

- Capability ids start with `vac.` (`vac.chat`, `vac.tui.pty_gate`).
- Workflow ids start with `product.`, `maintenance.`, or `vac.` (`maintenance.build-check`).
- Policy ids start with `vac.`, `product.`, or `maintenance.` (`vac.default-local`).
- Surface ids start with `surface.` (`surface.cli`, `surface.tui`).
- File names are kebab-case and reflect the id (`vac.identity.check` → `identity-check.yaml`; `maintenance.build-check` → `maintenance.build-check.yaml`). Workflows keep the dotted prefix; capabilities/policies drop the namespace prefix.

## Schema versioning

All manifests pin `schema_version: 1`. The loaders reject any other value. Schema-breaking changes require a parallel update to a schema doc under `docs/workflow-control-plane/schema/` and a coordinated runtime migration — do not bump the field unilaterally.

## Executable gates

```bash
vac doctor registry .
vac doctor architecture .
vac doctor ownership .
vac doctor workflow .
vac doctor policy .
vac doctor surfaces .
vac doctor docs .
```

Do not place legacy skill packs, ad-hoc scripts, or alternate frontend runtimes here.
