# VAC Runtime v1.5 audit-closure fixtures

These fixtures back `scripts/vac-runtime-realpath-e2e.py`. They validate the
source-level real-path contract required by the 2026-06-10 runtime audit:
actual agent loop mediation, MCP command/file mutation proof, completion lock,
evidence v2 contracts, SpecSync mapping, and memory non-authority.

Cargo/runtime-live checks remain TV-Pending.
