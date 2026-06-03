# PRD — Local Inference and Model Routing

## Overview

VAC needs a model/provider layer that is visible, policy-aware, and budget-aware.

This PRD covers:

```text
vac.model_router
vac.provider_readiness
vac.streaming
vac.token_budget
vac.provider_fallback
vac.local_inference
```

## Product goal

VAC should route tasks to appropriate model providers while showing readiness, cost/budget, streaming status, and fallback behavior.

Local inference is optional and deferred until the local runtime path is stable.

## User problems

- Provider configuration failures are confusing.
- Users cannot tell which model is being used.
- Token/cost usage is unclear.
- Fallback behavior can be surprising.
- Local inference can be useful but must not complicate default setup.

## Capability model

| Capability | Purpose | Priority |
|---|---|---:|
| `vac.provider_readiness` | Show model/provider availability and missing config. | P1 |
| `vac.streaming` | Stream assistant/tool progress into TUI/exec. | P1 |
| `vac.model_router` | Select model/provider based on task, readiness, policy, and budget. | P1/P2 |
| `vac.token_budget` | Track and bound token usage when available. | P1/P2 |
| `vac.provider_fallback` | Safe fallback when provider is unavailable. | P2 |
| `vac.local_inference` | Optional local model execution. | P2/P3 |

## Provider readiness requirements

Readiness should show:

- configured providers,
- missing credentials/config,
- selected default model,
- connectivity status when checked,
- policy restrictions,
- degraded/fallback state.

## Model routing requirements

Router inputs:

- task kind,
- autonomy mode,
- privacy policy,
- token budget,
- provider readiness,
- domain requirements,
- local/offline preference.

Router output:

```yaml
model_route:
  provider: local_or_remote
  model: selected_model
  reason: best available for coding task
  fallback:
    - cheaper_model
  budget:
    max_tokens: 12000
```

## Streaming requirements

Streaming should emit RuntimeEvents:

- assistant delta,
- reasoning/status summary when safe,
- tool call start/end,
- provider error,
- completion.

## Token budget requirements

Budget should track when available:

- prompt tokens,
- completion tokens,
- tool/context contribution,
- session total,
- task limit,
- budget warnings.

## Local inference requirements

Local inference is optional.

It must define:

- model file source,
- supported runtime backend,
- memory/CPU/GPU requirements,
- task suitability,
- fallback behavior,
- privacy benefits and limitations,
- readiness status.

## TUI surfaces

```text
/status
/models
/doctor
/activity
```

## Acceptance criteria

### MVP

- Provider readiness is visible.
- Selected provider/model is visible when safe.
- Streaming updates TUI/exec activity.
- Missing provider config has recovery hint.

### Safety

- Sensitive context send is policy-classified.
- Fallback does not violate privacy policy.
- Local inference model paths/config are validated.

### UX

- User sees why provider is unavailable.
- User can understand token/budget pressure.
- Local inference is optional, not setup friction for default users.
