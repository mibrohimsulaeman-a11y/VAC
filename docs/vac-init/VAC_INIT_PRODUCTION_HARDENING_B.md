# VAC-Init Production Hardening B

Status: implemented in sandbox artifact `vac-init-prod-hardening-b1-b4`.

## Scope

Production Hardening B turns the VAC-Init contract into operator-facing CLI runtime surfaces:

1. B1 `vac init` CLI skeleton and state persistence.
2. B2 `vac plan validate <file>`.
3. B3 `vac why <target>`.
4. B4 `vac doctor` taxonomy wiring.

## B1 — `vac init`

Implemented command modes:

```bash
vac init
vac init --dry-run
vac init --resume
vac init --status
vac init --scan
```

Runtime state is written under `.vac/.init/` using atomic temp-file rename:

```text
.vac/.init/state.yaml
.vac/.init/scan_report.yaml
.vac/.init/strategy.yaml
.vac/.init/doctor_report.yaml
```

`--dry-run` is read-only. `--status` is read-only. `--scan`, `--resume`, and default `vac init` write the init state.

## B2 — `vac plan validate`

`vac plan validate <file>` reads a plan YAML and validates:

- schema envelope: `schema_version`, `kind: plan`, `id: plan.*`;
- registered capability exists;
- planned/deprecated capability is rejected;
- policy is loaded fail-closed;
- `allowed_files` are bounded and workspace-relative;
- modify/delete operations require `line_range` or `semantic_anchor`;
- validation commands must be structured objects.

## B3 — `vac why`

`vac why` supports:

```bash
vac why <file>
vac why <file>:<line>
vac why <file>:<start>-<end>
vac why <file>::<symbol>
vac why <target> --depth 3
```

It reads `.vac/registry/trajectory/index.yaml`, prints safe decision summaries, and skips any entry that contains raw/private chain-of-thought terms.

## B4 — `vac doctor` taxonomy

The CLI now wires the refined doctor taxonomy:

```text
registry, surfaces, policy, ownership, workflow, evidence, build, memory, init, release
```

New runtime doctor commands:

```bash
vac doctor evidence .
vac doctor memory .
vac doctor init .
```

`vac doctor release .` now emits a taxonomy aggregate before the existing workflow release gate.

## Validation approach

No full workspace build is required in this batch. Validation uses targeted `rustc --test`, `rustfmt`, YAML parse/static checks, and CLI surface greps.
