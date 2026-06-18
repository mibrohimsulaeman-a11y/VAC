# VAC L2 Broker P2 First Slice

Status source: fail-closed L2 broker boundary verifier.

This slice does not implement L2 broker mediation. It adds an executable status gate that preserves the current aggregate claim and prevents premature promotion of L1/CI-attested evidence into L2.

```text
l2_broker=NotImplemented
l2_broker_claim_gate=SV-Pass
l2_broker_execution=not_mediated
l2_broker_custody=local_only
```

Verifier:

```text
scripts/check-l2-broker-status.py
```

Shared status logic:

```text
scripts/l2_broker_status.py
```

Future L2 implementation requires a current proof at:

```text
.vac/evidence/l2-broker-mediated-execution-current.json
```

A future `Implemented` result must prove mediated execution rather than cooperative local execution. Required proof dimensions are structured intent only, broker process supervision, filesystem mediation, process-spawn mediation, network mediation, credential redaction before model exposure, mediated runtime-journal record, broker signature hash, direct OS bypass rejection, and rejection of tool-supplied policy decisions.

The current gate passes only while the aggregate status remains `NotImplemented` and no scanned source/status surface claims an L2 pass.


## P2.1 typed broker envelope contract

P2.1 adds the typed envelope contract that later broker-mediated filesystem, process, network, and credential-bearing remote IO records must use. The contract is intentionally non-promotional: tool/MCP payloads can submit only structured intent, while broker-controlled fields such as policy decisions, mediated execution mode, broker custody, broker record hashes, and broker signatures are rejected at the intent boundary.

Implemented artifacts:

```text
vac-rs/crates/runtime/vac-broker/src/envelope.rs
.vac/schemas/broker-envelope.schema.json
```

Executable fixtures cover canonical intent hashing and negative injection cases for tool-supplied policy decision, tool-supplied mediated execution mode, self-assigned broker custody, missing policy snapshot, missing approval/read-plan binding, missing broker record hash for mediated evidence, and missing broker signature hash for broker-attested evidence.

This still does not implement broker-supervised OS execution. The aggregate broker status remains `NotImplemented` until a current mediated execution proof exists and the L2 verifier accepts it.


## P2.2 runtime journal mediated-record schema

P2.2 extends the runtime journal source schema with broker-mediated tables while keeping execution claims non-promotional:

```text
runtime_broker_intents
runtime_broker_decisions
runtime_broker_execution_records
runtime_broker_evidence_records
runtime_broker_denials
```

The schema and reducer fixtures are fail-closed for duplicate intent IDs, execution records without broker decisions, mediated_l2 records without broker_record_hash, broker_attested records without broker_signature_hash, tool-supplied policy decisions, and stale policy snapshots. These records are storage substrate only; broker-supervised filesystem/process/network execution is still future P2 work, so `l2_broker=NotImplemented` remains the correct aggregate status.
