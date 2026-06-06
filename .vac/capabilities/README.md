# Capabilities (authored intent)

A capability declares INTENT and OWNERSHIP-BY-CONVENTION only.

- DO declare: `id`, `title`, `status`, `ownership` (crates/globs/annotation), `policy` risk class, `depends_on`.
- DO NOT enumerate every module by hand. The resolved owner map is DERIVED
  (`derived/ownership.yaml`) by `vac scan` from these rules + in-source annotations
  (e.g. `#![vac::owner = "vac.chat"]`).

## Ownership resolution precedence

1. In-source annotation (`#![vac::owner = "<id>"]`) — authoritative when present.
2. `ownership.crates` — whole-crate ownership by crate NAME (robust to module renames).
3. `ownership.paths.include` / `paths.exclude` — glob ownership for shared crates.
4. Otherwise: unowned => orphan => quarantined by `vac doctor ownership .`.

Keep this directory small. The v1 sprawl (per-module `targets:` lists, ~25 `vac.init.*` files)
is gone — sub-areas are `scopes:`, and resolved ownership is regenerated, not authored.
