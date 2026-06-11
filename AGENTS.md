# VAC agent contributor guide

This repository is organized around the **VAC v1.9** control-plane architecture. Treat `.vac/` as small tracked authority plus ignored local runtime/cache/export state, and treat `vac-rs/` as the Rust implementation workspace.

## Hard rules

- Do not claim cargo build/check/clippy/test pass unless those commands were actually run.
- In sandbox work, prefer source-level SV gates and record cargo work as TV-Pending.
- Do not add or preserve legacy upstream branding.
- Do not document unverified registry images or obsolete source repository URLs as canonical VAC assets.
- Runtime authority is compiled JSON under `.vac/cache/compiled` or runtime DB state; YAML is authoring only.
- Runtime state belongs in `.vac/db/runtime.db`; do not add per-session plan/evidence/index artifacts to the clean source checkpoint.
- Server/gateway functionality must remain optional and bounded as `vac-broker`, `vac-remote-service`, or `vac-messaging-gateway`, not as the default product runtime.

## Current tree

```text
.vac/
  capabilities/ policies/ workflows/ surfaces/ specs/confirmed/ schemas/ migrations/
  db/runtime.db           # ignored local journal
  cache/compiled/         # ignored compiled snapshot cache
  exports/                # optional export bundles
vac-rs/
  core/
  crates/{foundation,control-plane,runtime,surfaces,providers,integrations,capabilities}/
vac-cli/
  bin/vac.js
```

## Runtime behavior to preserve

The bounded agent runtime must follow:

1. Read compiled snapshot cache/DB, capabilities, policies, index/read-plan exports, and memory hints.
2. Validate a machine-readable Semantic Plan.
3. Stamp runtime records with `manifest_set_hash`, compiled snapshot ID, Git HEAD, and dirty tree hash.
4. Apply only bounded patches inside allowed files, line ranges, semantic anchors, ownership, and budget.
5. Execute only structured commands; shell interpolation and destructive commands must block or pause for approval.
6. Close only when DB state, evidence hints, SpecSync, readiness, ownership, assessment, and completion lock pass, or a visible `needs_discussion` state is recorded.

SV gates:

```bash
python3 scripts/check-v19-storage-classes.py .
python3 scripts/check-v19-runtime-db-schema.py .
python3 scripts/vac-runtime-agent-e2e-sv.py
```

## Packaging discipline

Use split artifacts:

```bash
python3 scripts/package-v19-checkpoint.py . /mnt/data vac-runtime-v19-storage-cleanup
python3 scripts/check-v19-package-hygiene.py /mnt/data/vac-runtime-v19-storage-cleanup-source-clean.zip /mnt/data/vac-runtime-v19-storage-cleanup-state-export.zip
```
