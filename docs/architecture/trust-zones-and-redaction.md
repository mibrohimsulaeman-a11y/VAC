# Trust Zones and Redaction contract

## Purpose

Trust Zones and Redaction define how VAC protects user data while executing autonomous local work, using tools, and calling connectors or model providers.

Policy decides what is allowed. Trust zones define boundaries. Redaction prevents unsafe exposure.

## Trust zones

Initial trust zones:

```text
local_public
local_project
local_sensitive
local_secret
external_connector
external_model
untrusted_output
```

### local_public

Safe public project data, examples, generated summaries.

### local_project

Normal project code and docs. Safe to inspect locally, but not automatically safe to send externally.

### local_sensitive

Potentially sensitive project content such as internal architecture, private docs, or proprietary logic.

### local_secret

Secrets, credentials, tokens, keys, private environment values.

### external_connector

Data retrieved from managed connectors.

### external_model

Data sent to or received from model providers.

### untrusted_output

Shell output, generated code, connector output, or model output that has not been validated.

## Permission sets

```text
read_local_public
read_local_project
read_local_sensitive
read_local_secret
write_project
execute_process
network_connector
network_model
export_evidence
store_memory
```

Permission sets are policy inputs. They should be visible in diagnostics when denied.

## Data classes

VAC should classify data before display/export/external send.

```text
source_code
project_docs
config
logs
diff
secret_like
credential
personal_data
connector_knowledge
model_output
command_output
```

## Redaction pipeline

```text
classify data
  -> detect sensitive patterns
  -> apply policy
  -> redact or deny
  -> annotate redaction status
  -> display/export/send
```

Redaction statuses:

```text
clean
redacted
blocked
unknown
```

## Secret detection

Secret detection should cover:

- environment keys,
- tokens,
- private keys,
- credentials in config,
- authorization headers,
- connection strings,
- high-entropy values.

False positives should prefer safe redaction over exposure.

## Redaction metadata

Redaction metadata must not leak the original value.

Allowed metadata:

```yaml
redaction:
  status: redacted
  class: credential
  count: 2
```

Forbidden metadata:

```text
raw secret value
partial token value unless explicitly safe
full source span containing secret
```

## External send gate

Before sending data to external model or connector, VAC must evaluate:

- data class,
- trust zone,
- user policy,
- connector/provider scope,
- redaction status,
- task need.

External send may be:

```text
allow
deny
redact_then_allow
approval_required
```

## Tool integration

Every tool result must carry redaction status.

```yaml
tool_result:
  status: success
  redaction_status: redacted
  redaction:
    class: secret_like
    count: 1
```

TUI should show that redaction happened without exposing sensitive values.

## Memory integration

Before storing memory, VAC must decide:

- can this content be stored,
- at what scope,
- with what retention,
- whether redaction is required,
- whether source attribution is needed.

Secrets must not be stored as semantic memory.

## Evidence integration

Evidence summaries must be safe by default.

Evidence export requires policy evaluation if it includes:

- source snippets,
- diffs,
- command output,
- connector results,
- session transcript,
- model output.

## TUI requirements

TUI should show:

- policy denial reason,
- redaction notice,
- trust zone for connector/tool when relevant,
- safe recovery hint.

Example:

```text
Output redacted: 2 credential-like values hidden.
```

## Acceptance criteria

MVP acceptance:

```text
redaction status exists on tool/result/event payloads
secret-like values are not printed in TUI diagnostics
external connector/model sends are policy-classified
policy denial has recovery hint
```

Safety acceptance:

```text
raw secrets are not exported in evidence
connector output is not treated as trusted code
memory storage rejects secrets by default
approval cannot bypass redaction unless policy explicitly allows it
```
