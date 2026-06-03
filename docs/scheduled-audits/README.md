# Scheduled audit snapshots

Files under `docs/scheduled-audits/` are point-in-time audit snapshots. They are intentionally historical records and may disagree with later snapshots or with the current working tree.

Use them as a time-series, not as the current source of truth.

For current status, prefer:

```bash
./vac-rs/target/debug/vac doctor registry .
./vac-rs/target/debug/vac doctor surfaces .
./vac-rs/target/debug/vac doctor workflow .
./vac-rs/target/debug/vac doctor docs .
bash scripts/check-donor-status.sh drift
```

When a snapshot says `FAIL` and a later snapshot says `PASSED`, that is not a docs contradiction by itself. Treat it as evidence that the repository state changed between audit times.

## Commit and retention guidance

- Keep audit snapshots as point-in-time records.
- Keep `INDEX.md` generated from snapshots rather than manually curated.
- Commit scheduled audit artifacts separately from code/runtime changes when practical.
- Use the newest snapshot plus live validation commands for current status.
