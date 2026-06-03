# O6.2 Unsafe Static Triage

Status: Static upper-bound triage only.

## Count

```text
unsafe_upper_bound: 625
```

## Sample files

[
  {
    "file": "vac-rs/test-binary-support/lib.rs",
    "count": 3
  },
  {
    "file": "vac-rs/windows-sandbox-rs/src/acl.rs",
    "count": 13
  },
  {
    "file": "vac-rs/windows-sandbox-rs/src/audit.rs",
    "count": 5
  },
  {
    "file": "vac-rs/windows-sandbox-rs/src/desktop.rs",
    "count": 5
  },
  {
    "file": "vac-rs/windows-sandbox-rs/src/dpapi.rs",
    "count": 8
  },
  {
    "file": "vac-rs/windows-sandbox-rs/src/elevated_impl.rs",
    "count": 3
  },
  {
    "file": "vac-rs/windows-sandbox-rs/src/firewall.rs",
    "count": 21
  },
  {
    "file": "vac-rs/windows-sandbox-rs/src/hide_users.rs",
    "count": 7
  },
  {
    "file": "vac-rs/windows-sandbox-rs/src/lib.rs",
    "count": 19
  },
  {
    "file": "vac-rs/windows-sandbox-rs/src/proc_thread_attr.rs",
    "count": 9
  },
  {
    "file": "vac-rs/windows-sandbox-rs/src/process.rs",
    "count": 8
  },
  {
    "file": "vac-rs/windows-sandbox-rs/src/read_acl_mutex.rs",
    "count": 8
  },
  {
    "file": "vac-rs/windows-sandbox-rs/src/sandbox_users.rs",
    "count": 21
  },
  {
    "file": "vac-rs/windows-sandbox-rs/src/setup_main_win.rs",
    "count": 10
  },
  {
    "file": "vac-rs/windows-sandbox-rs/src/setup_orchestrator.rs",
    "count": 5
  },
  {
    "file": "vac-rs/windows-sandbox-rs/src/spawn_prep.rs",
    "count": 6
  },
  {
    "file": "vac-rs/windows-sandbox-rs/src/token.rs",
    "count": 20
  },
  {
    "file": "vac-rs/windows-sandbox-rs/src/wfp.rs",
    "count": 15
  },
  {
    "file": "vac-rs/windows-sandbox-rs/src/winutil.rs",
    "count": 10
  },
  {
    "file": "vac-rs/windows-sandbox-rs/src/workspace_acl.rs",
    "count": 3
  }
]

## Next action

Every runtime unsafe block must either move behind a narrow FFI/sandbox boundary or gain a `// SAFETY:` invariant comment. This must be validated by build/clippy once the toolchain/vendor gate is available.

## SAFETY coverage update — 2026-05-30T00:00:00Z

```text
source_runtime_safety_coverage: 491/491
linux_host_runtime_safety_coverage: 180/180
retired_metric: 558/558 counted test-suffixed files and inline cfg(test) items
coverage_status: SV-Done
tv_status: TV-Pending
```

Detailed machine state: `.vac/registry/o6-safety-coverage.yaml`.
