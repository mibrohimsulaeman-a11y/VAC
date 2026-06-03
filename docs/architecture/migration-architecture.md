# VAC migration architecture

## Purpose

This document defines how the repository migrates to the workflow-control-plane pattern without creating another product architecture.

## Migration order

```text
1. keep root product identity stable
2. remove product-invisible clutter
3. simplify CLI command surface
4. repair root build
5. create `.vac` manifest skeleton
6. add registry loaders
7. add capability dashboard
8. add workflow browser
9. add safe workflow runner
10. convert root features into manifests
11. begin donor-backed capability intake
```

## Current migration gate

The next high-impact work is simplifying the root CLI command surface so it no longer exposes legacy service/proxy/cloud/debug commands as product commands.

## Donor intake rule

Donor source is not implementation by default.

A donor feature may enter product only when it has:

- capability manifest,
- root target,
- policy classification,
- TUI/CLI surface,
- validation command,
- cleanup status.

## Deletion rule

Delete code when one of these is true:

- not reachable from product and not test-only,
- superseded by `.vac` manifest/control-plane design,
- duplicate frontend/runtime path,
- legacy service surface not part of product direction,
- no owner/capability/status after audit.

## Temporary hold rule

Do not delete code just because it looks old if it is still required for:

- current root build,
- TUI launch,
- core execution,
- patch/apply/review path,
- sandbox safety,
- protocol/config/state required by TUI/core.

First decouple, then delete.

## Architecture migration success criteria

The migration is successful when:

- `.vac` is the visible product control plane,
- root TUI lists capabilities and workflows,
- root CLI help is small and product-focused,
- old service/proxy/debug surfaces are gone or manifest-governed,
- donor code is source-only until manifest-backed,
- release gate validates build, identity, policy, TUI, and workflow behavior.
