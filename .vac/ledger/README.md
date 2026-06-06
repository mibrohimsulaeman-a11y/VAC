# Ledger (authored, durable)

Single living ledgers. Do NOT create a new file per remediation cycle.

- `findings.yaml` — every finding has a STABLE `id` (F-XXXX) and append-only state transitions
  (`open` -> `waived` -> `closed`). This replaces the v1 `plan.o5o6.*` / `evidence/*` / `trajectory/*` file spam.
- `waivers.yaml` — expiring, owned, signed waivers. A waiver is the ONLY way an open finding stops
  blocking a release; when it expires, the finding re-opens automatically (fail-closed).
