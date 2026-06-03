# O6.2 SAFETY Annotation Coverage

Status: SV-Done / TV-Pending

This slice corrects the audit-flagged false-green denominator and replaces the generic SAFETY boilerplate with direct, call-site-adjacent rationale comments.

## Corrected coverage

```text
source_runtime_safety_coverage: 491/491
linux_host_runtime_safety_coverage: 180/180
excluded_path_test_or_fixture: 54
excluded_cfg_test: 41
stale_generic_safety_comments: 0
coverage_status: SV-Done
tv_status: TV-Pending
```

## Scope correction

The retired `558/558` headline counted test-suffixed files and inline `#[cfg(test)]` blocks. The gate now uses `scripts/measure-vac-o6-2-runtime-safety-coverage.py`, which excludes test/fixture/bench paths, `*_test.rs`, `*_tests.rs`, `tests.rs`, `test_*.rs`, and inline `#[cfg(test)]` items before calculating source-runtime coverage.

The `linux_host_runtime` number is a secondary sandbox-host view and excludes Windows-specific runtime paths; the primary release-quality metric remains `source_runtime` so target-specific runtime code is not hidden.

## Genuine SAFETY mode

The scanner no longer accepts a loose three-line window. Each counted runtime unsafe site must have an immediately preceding `// SAFETY:` comment, and known stale boilerplate comments are rejected. The current tree has zero stale generic SAFETY comments under the corrected runtime scope.

## Highest-count source-runtime files

```json
[
  [
    "vac-rs/linux-sandbox/src/linux_run_main.rs",
    {
      "cfg_test_excluded": 0,
      "covered": 39,
      "total": 39
    }
  ],
  [
    "vac-rs/linux-sandbox/src/proxy_routing.rs",
    {
      "cfg_test_excluded": 0,
      "covered": 28,
      "total": 28
    }
  ],
  [
    "vac-rs/windows-sandbox-rs/src/elevated/command_runner_win.rs",
    {
      "cfg_test_excluded": 0,
      "covered": 24,
      "total": 24
    }
  ],
  [
    "vac-rs/windows-sandbox-rs/src/elevated/runner_client.rs",
    {
      "cfg_test_excluded": 0,
      "covered": 22,
      "total": 22
    }
  ],
  [
    "vac-rs/windows-sandbox-rs/src/sandbox_users.rs",
    {
      "cfg_test_excluded": 0,
      "covered": 21,
      "total": 21
    }
  ],
  [
    "vac-rs/windows-sandbox-rs/src/token.rs",
    {
      "cfg_test_excluded": 0,
      "covered": 20,
      "total": 20
    }
  ],
  [
    "vac-rs/tui/src/ide_context/windows_pipe.rs",
    {
      "cfg_test_excluded": 0,
      "covered": 19,
      "total": 19
    }
  ],
  [
    "vac-rs/windows-sandbox-rs/src/lib.rs",
    {
      "cfg_test_excluded": 0,
      "covered": 18,
      "total": 18
    }
  ],
  [
    "vac-rs/shell-escalation/src/unix/socket.rs",
    {
      "cfg_test_excluded": 1,
      "covered": 17,
      "total": 17
    }
  ],
  [
    "vac-rs/tui/src/ide_context/ipc.rs",
    {
      "cfg_test_excluded": 0,
      "covered": 17,
      "total": 17
    }
  ],
  [
    "vac-rs/windows-sandbox-rs/src/firewall.rs",
    {
      "cfg_test_excluded": 6,
      "covered": 15,
      "total": 15
    }
  ],
  [
    "vac-rs/windows-sandbox-rs/src/wfp.rs",
    {
      "cfg_test_excluded": 0,
      "covered": 15,
      "total": 15
    }
  ],
  [
    "vac-rs/windows-sandbox-rs/src/acl.rs",
    {
      "cfg_test_excluded": 0,
      "covered": 13,
      "total": 13
    }
  ],
  [
    "vac-rs/windows-sandbox-rs/src/unified_exec/backends/legacy.rs",
    {
      "cfg_test_excluded": 0,
      "covered": 13,
      "total": 13
    }
  ],
  [
    "vac-rs/utils/pty/src/win/psuedocon.rs",
    {
      "cfg_test_excluded": 0,
      "covered": 12,
      "total": 12
    }
  ],
  [
    "vac-rs/windows-sandbox-rs/src/setup_main_win.rs",
    {
      "cfg_test_excluded": 0,
      "covered": 10,
      "total": 10
    }
  ],
  [
    "vac-rs/windows-sandbox-rs/src/winutil.rs",
    {
      "cfg_test_excluded": 0,
      "covered": 10,
      "total": 10
    }
  ],
  [
    "vac-rs/utils/pty/src/process_group.rs",
    {
      "cfg_test_excluded": 0,
      "covered": 9,
      "total": 9
    }
  ],
  [
    "vac-rs/windows-sandbox-rs/src/elevated/runner_pipe.rs",
    {
      "cfg_test_excluded": 0,
      "covered": 9,
      "total": 9
    }
  ],
  [
    "vac-rs/windows-sandbox-rs/src/proc_thread_attr.rs",
    {
      "cfg_test_excluded": 0,
      "covered": 9,
      "total": 9
    }
  ]
]
```

## TV status

Cargo/geiger verification remains TV-Pending because the source artifact intentionally excludes `vendor/` and the prior offline cargo retry was aborted by sandbox reset.
