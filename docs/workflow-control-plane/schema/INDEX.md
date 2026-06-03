# `.vac` schema specifications

## Purpose

This directory defines the control-plane schema contracts for VAC.

The `.vac` control plane must not be implemented until the local runtime path is clean according to ADR-0007 and Plans 00A through 00E.

## Schema specs

- [Capability manifest schema](capability-manifest.schema.md)
- [Workflow manifest schema](workflow-manifest.schema.md)
- [Policy manifest schema](policy-manifest.schema.md)
- [Surface manifest schema](surface-manifest.schema.md)
- [Registry schema](registry.schema.md)

## Global schema rules

All manifests must be:

- typed,
- versioned,
- validated before use,
- diagnosable in TUI/doctor,
- linked to capability ownership,
- safe to ignore when invalid,
- never treated as executable script by default.
