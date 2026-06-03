# VAC exec rewire spec

## Goal

Make `vac exec` submit local work through Local Runtime Contract.

## Target flow

```text
parse CLI input
create RuntimeCommand::StartTask
consume RuntimeEvent stream
print headless output
return exit status
```

## Required event handling

```text
TaskStarted -> print task summary
AssistantDelta -> stream text
ToolCallStarted/Finished -> print visible tool summary
ApprovalRequested -> fail or request approval depending mode
ValidationFinished -> print validation result
TaskCompleted -> exit success
TaskFailed -> exit failure
```

## Dependency target

`vac-exec` should not require old runtime client crate for local prompt submission.

## Validation

```bash
cd vac-rs
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-cli
cargo +1.93.0 tree -p vac-surface-cli -i vac-app-server-client --edges normal,build
```
