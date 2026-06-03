# VAC-Init Lifecycle Gate

The lifecycle gate covers the `vac init` state machine and resume-safe `init_state` record.

Required script:

```bash
bash scripts/check-vac-init-lifecycle-contract.sh
```

The state machine must reject invalid transitions, treat `ready` as terminal, increment retry count on failure, and render a valid `kind: init_state` envelope.
