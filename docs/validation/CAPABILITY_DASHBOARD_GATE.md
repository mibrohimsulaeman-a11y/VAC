# Capability Dashboard Gate

## Goal

Verify capabilities are visible and diagnosable in TUI.

## Required checks

```text
/capabilities opens
capability list renders
planned/partial/ready states are distinct
invalid manifest diagnostic is visible
ready capability without surface is flagged
```

## Cannot claim if failed

```text
capability registry user-visible
backend capability product-ready
```
