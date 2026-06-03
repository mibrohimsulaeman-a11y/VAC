# Tool Contract architecture

## Purpose

The Tool Contract defines how VAC describes, authorizes, runs, renders, and records tools.

Tools must not be raw function calls hidden behind the agent loop. Every tool must have metadata, risk classification, policy behavior, result shape, and TUI rendering semantics.

## Tool definition

A tool definition must declare:

```yaml
tool:
  id: tool.read_file
  title: Read file
  owner: vac.tools
  input_schema: {}
  output_schema: {}
  side_effect: read_only
  permission_class: filesystem_read
  risk: read_only
  approval: never
  render_hint: text
```

Required fields:

| Field | Meaning |
|---|---|
| `id` | Stable tool id. |
| `title` | Operator-facing name. |
| `owner` | Owning capability/module. |
| `input_schema` | Typed input contract. |
| `output_schema` | Typed output contract when practical. |
| `side_effect` | Read/write/execute/network/config class. |
| `permission_class` | Required permission. |
| `risk` | Risk classification. |
| `approval` | Approval behavior. |
| `render_hint` | Preferred TUI rendering. |

## Side-effect classes

```text
read_only
safe_edit
destructive_write
execute_process
network_read
network_write
config_change
credential_access
connector_call
```

## Permission classes

```text
filesystem_read
filesystem_write
process_execute
network_access
connector_access
credential_read
session_write
checkpoint_write
```

Permission class is policy input. It is not enough for a tool to self-report safe behavior.

## Risk levels

```text
read_only
safe_edit
broad_edit
destructive
execute
network
credential
unknown
```

Unknown risk defaults to approval-required or denied, depending on policy.

## Approval behavior

```text
never
on_request
always
policy_decided
unavailable
```

Policy can override tool default approval behavior.

## Tool invocation lifecycle

```text
resolved
input_validated
policy_checked
approval_requested_if_needed
started
progress
completed
failed
recorded
rendered
```

Each lifecycle transition should emit a runtime event or transcript item when operator-visible.

## Tool result envelope

All tool results must use a result envelope.

```yaml
tool_result:
  tool_id: tool.run_validation
  invocation_id: call_123
  status: success
  summary: cargo test -p auth passed
  output:
    kind: command_output
    text: ...
  artifacts: []
  render_hint: validation
  redaction_status: clean
  retry_hint: none
```

Status values:

```text
success
failed
cancelled
denied
approval_rejected
timeout
partial
```

## Render hints

Render hints tell the TUI how to display results.

```text
text
diff
command_output
validation
approval_preview
file_tree
table
json
warning
error
hidden
```

Render hints do not bypass redaction or policy.

## Progress kinds

Long-running tools should emit progress.

```text
indeterminate
bytes
files
steps
lines
tests
network
```

TUI can then show meaningful progress instead of raw logs.

## Review visibility

Each tool result must declare visibility:

```text
operator_visible
summary_only
hidden_internal
requires_expansion
```

Hidden internal output must not be used to hide risky behavior. It is only for noisy implementation detail.

## Connector-backed tools

Connector-backed tools must additionally declare:

- connector id,
- connector trust zone,
- allowed scopes,
- whether output is external knowledge,
- attribution requirement,
- network policy.

Connector tools are read-only by default unless explicitly approved through policy.

## Error contract

Tool errors must include:

```yaml
error:
  code: permission_denied
  message: Filesystem write denied by policy.
  recovery_hint: Approve write access or narrow the task scope.
  retry_safe: false
```

Errors must never expose raw secrets.

## TUI requirements

TUI must show:

- tool name,
- action summary,
- status,
- risk when relevant,
- approval state when relevant,
- output preview,
- failure reason,
- recovery hint.

## Acceptance criteria

MVP acceptance:

```text
tools have metadata
tool input is validated
tool risk is classified
tool calls route through policy
tool results use a result envelope
TUI can render tool start/result/failure
```

Safety acceptance:

```text
write/execute/network tools cannot bypass policy
unknown tool risk is not auto-allowed
connector tools are scoped
secrets are redacted before display/export
```

UX acceptance:

```text
user sees what tool is running
user sees why approval is required
user sees result summary
user sees actionable failure hints
```
