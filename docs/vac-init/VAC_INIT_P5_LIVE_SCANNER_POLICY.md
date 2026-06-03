# VAC-Init P5 Live Scanner and Policy Inference

This slice makes `vac init --scan` emit real workspace-derived source inventory, risk findings, and policy inference reports.

Outputs:

```text
.vac/.init/source_inventory.yaml
.vac/.init/risk_findings.yaml
.vac/.init/policy_inference_report.yaml
```

The scanner is intentionally honest: without a real AST parser wired, findings use `method: ast_exact`, not `ast_exact`.
