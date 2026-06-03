# PRD — Remote Bridge and Deferred Enterprise Runtime

## Overview

VAC may eventually support remote sessions, external protocol bridges, and enterprise delegated execution.

This PRD covers deferred capabilities:

```text
vac.bridge
vac.remote_sessions
vac.permission_mediation
vac.acp
vac.teleport
```

## Product stance

Remote bridge is deferred.

The local product path must remain clean and local-first. Remote/server-style runtime must not leak back into default `vac` or `vac exec` behavior.

## Product goal

When introduced, remote bridge should support controlled enterprise scenarios without weakening local operator UX.

## Capability model

| Capability | Purpose | Priority |
|---|---|---:|
| `vac.bridge` | Boundary for remote/delegated execution. | P3 |
| `vac.remote_sessions` | Remote or delegated session state. | P3 |
| `vac.permission_mediation` | Map local policy to remote execution constraints. | P3 |
| `vac.acp` | External client/protocol compatibility if needed. | P3 |
| `vac.teleport` | Session handoff between execution contexts. | P3 |

## Non-goals before P3

Do not implement before local product is stable:

- remote-first runtime,
- default server process,
- hidden cloud task path,
- external protocol as local runtime API,
- remote approval bypass,
- remote session replacing Local Runtime Contract.

## Requirements when revisited

Remote bridge must:

- be explicit capability,
- be disabled by default,
- have TUI-visible status,
- use policy and trust zones,
- preserve approval semantics,
- produce local evidence,
- support disconnect/failure recovery,
- never bypass local redaction rules.

## TUI surfaces

Future surfaces only:

```text
/remote
/bridge
/status
/approvals
/evidence
```

## Acceptance criteria before implementation

- Local Runtime Contract is stable.
- Local TUI/exec do not depend on remote/server protocol.
- Capability manifest exists.
- Policy and permission mediation are designed.
- Remote failure behavior is documented.

## Safety requirements

- Remote execution cannot mutate local files without local policy decision.
- Remote approvals must be represented locally.
- Credentials and session tokens are never shown in logs/TUI.
- Remote transcript/evidence is redacted before export.
