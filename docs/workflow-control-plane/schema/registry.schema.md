# Registry schema

## Purpose

Registry manifests define root product metadata and registry loading behavior.

## File locations

```text
.vac/registry/product.yaml
.vac/registry/load-order.yaml
.vac/registry/diagnostics.yaml
```

## Product registry fields

| Field | Type | Meaning |
|---|---|---|
| `schema_version` | integer | Schema version. |
| `kind` | string | Must be `registry`. |
| `product` | string | Product name. |
| `root` | string | Expected repo root marker. |
| `capability_dirs` | list | Capability manifest dirs. |
| `workflow_dirs` | list | Workflow manifest dirs. |
| `policy_dirs` | list | Policy manifest dirs. |
| `surface_dirs` | list | Surface manifest dirs. |

## Example

```yaml
schema_version: 1
kind: registry
product: VAC
root: .vac
capability_dirs:
  - .vac/capabilities
workflow_dirs:
  - .vac/workflows
policy_dirs:
  - .vac/policies
surface_dirs:
  - .vac/surfaces
diagnostics:
  fail_on_invalid_ready: true
  warn_on_hidden_partial: true
```

## Loader requirements

The loader must:

- discover `.vac`,
- read manifests,
- validate schemas,
- resolve references where the registry can prove them,
- emit diagnostics,
- never execute manifest content as shell,
- degrade gracefully when manifests are invalid,
- feed `/doctor`, `/capabilities`, and `/workflow`.

## Rust loaders

The root runtime exposes typed registry loaders under `vac_core::control_plane`:

- `capability_registry`
- `workflow_registry`
- `policy_registry`
- `surface_registry`
- `registry` for the combined snapshot entrypoint

## Diagnostic levels

```text
info
warning
error
blocked
```

## Diagnostics report contract

The root runtime projects registry load results through `vac_core::control_plane`:

- `RegistryDiagnosticSeverity`
- `RegistryLoadState`
- `RegistryDiagnostic`
- `RegistryLoadReport`

The CLI exposes the report through:

```text
vac doctor registry [PATH]
```

The TUI-facing projection helper is:

```text
RegistryLoadReport::render_tui_lines()
```

The root TUI projects the same report through `/debug-config` as a
control-plane registry section.

Diagnostics stay concise and repairable:

- the manifest file path is shown,
- the field path is shown when parsing failed on a field,
- the message explains what broke,
- the hint suggests the next fix.

## Ready gate

Registry is usable when:

```text
.vac root exists
registry manifest validates
all ready capabilities resolve
invalid manifests produce diagnostics
TUI/doctor can display diagnostics
```
