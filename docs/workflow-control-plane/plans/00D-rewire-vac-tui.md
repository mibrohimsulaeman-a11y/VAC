# Plan 00D — Rewire root `vac` TUI to Local Runtime Contract


<!-- VAC-PLAN-STATE:BEGIN -->
## Current codebase reconciliation — 2026-05-30

Status: **COMPLETE**

Source status lines:
> Status: **implemented / TUI contract baseline**.
> - session status projection.
> approval overlay, validation status, and task completion/failure messages

Code evidence:
- `vac-rs/tui/src/local_runtime_session.rs`
- `vac-rs/tui/src/runtime_adapter/approval.rs`
- `/`

Evidence docs:
- none detected

Validation state: `targeted_or_documented`.

Caveat: full workspace build is not asserted by this reconciliation.
<!-- VAC-PLAN-STATE:END -->

## Active execution contract

Status: **implemented / TUI contract baseline**.

Target outcome: root `vac` TUI uses Local Runtime Contract semantics where appropriate while preserving one product TUI, slash affordance, bottom input, streaming, and visible operator state.

Outputs: TUI rewiring slices, UX validation evidence, stop/rollback notes, and progress log.

Requires / Blocks: requires 00B contract and 00C exec learning; blocks 00F TUI legacy transport retirement.

Stop conditions: stop if a second TUI path is introduced, slash/bottom-input UX regresses, or operator-visible state disappears.

Done criteria: TUI validation passes, UX impact is documented, and progress log records completed slices.


## Goal

Move the root TUI away from app-server client/protocol DTOs for local prompt,
activity, approval, validation, and completion flow. The Local Runtime
Contract (defined in `vac_core::local_runtime`) must be the source of truth
for the local product path; legacy `AppCommand::UserTurn` may remain as a
temporary transport but must not be the canonical product intent.

## Scope

- prompt submit from bottom input,
- activity/progress stream,
- approval request/resolve,
- validation state,
- task completion/failure,
- session status projection.

## Non-goals

- second TUI,
- donor shell stack,
- `.vac` registry wiring,
- `vac-app-server*` / `vac-api` deletion (deferred to 00E),
- remote/cloud sessions.

## Implementation

1. Route prompt submission to `RuntimeCommand::StartTask`. The bottom-input
   Enter path must mint the command and treat it as the canonical intent
   for the turn (the legacy `Op::UserTurn` submission becomes a transport
   detail).
2. Project the protocol event stream into `RuntimeEvent` values inside the
   TUI process and render them through the existing activity/progress/
   approval/validation/evidence surfaces. Do not introduce a second UI.
3. Render `ApprovalRequested` through the single root approval overlay.
4. Route approve/reject back through `RuntimeCommand::Approve` /
   `RuntimeCommand::Reject` so the contract owns the decision shape, even
   while the transport remains the existing `Op::ExecApproval` /
   `Op::PatchApproval` submission.
5. Replace app-server protocol DTOs with local projection structs for the
   local path where it is cheap (start with the projector helpers in
   `vac-rs/tui/src/local_runtime.rs`).
6. Preserve UI behavior through real PTY dogfood, not only unit tests.

## Validation

The validation entrypoints below match the real CLI surface. There is no
`vac tui` subcommand and no `--smoke` / `--dry-run` flags on the current
binary; do not assert against flags that do not exist.

```bash
cd vac-rs
cargo +1.93.0 fmt -p vac-surface-tui -p vac-core -p vac-exec -p vac-surface-cli
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 check -p vac-surface-cli
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 test -p vac-exec local_runtime --lib
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 test -p vac-surface-tui --lib local_runtime
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.93.0 build --bin vac
```

Required PTY operator gate after build (full steps in
`docs/validation/TUI_PTY_DOGFOOD_GATE.md`):

```text
launch ./target/debug/vac
type /
press Enter
verify slash-command list visible
type normal prompt
press Enter
verify task/activity visible (driven by Local Runtime Contract)
verify validation start/finish visible when a validation tool is used
verify approval overlay round-trips approve/reject through RuntimeCommand
press Ctrl-C
verify clean exit and clean working tree
```

Current builds open the slash-command list on `/`. Do not assume `/help`
is available unless a future release explicitly restores it.

If the only available shell cannot drive a real TTY (for example MCP /
sandboxed CI), report the gate as `BLOCKED-OPERATOR` and hand off the
steps above to a human running a real terminal.

## UX impact

For the operator the user-facing surface stays exactly one TUI: `vac`
→ bottom input → Enter. The change makes the visible activity/progress,
approval overlay, validation status, and task completion/failure messages
product-event driven so that the same surfaces will keep working after the
app-server stack is retired in 00E. No new screens, no extra prompts, and
the approval round-trip remains identical from the user's point of view
— only the underlying intent shape changes from a server-thread DTO to
`RuntimeCommand` / `RuntimeEvent`.

## Done

- TUI local prompt path uses Local Runtime Contract as the canonical intent.
- Activity/progress/approval/validation/completion are visible in the
  existing root TUI surfaces.
- Approve/reject in the existing overlay round-trip through
  `RuntimeCommand::Approve` / `RuntimeCommand::Reject`.
- No duplicate TUI or approval UI introduced.
- PTY operator gate either passed in a real terminal or reported
  `BLOCKED-OPERATOR` with concrete operator steps.

## Progress log

### 00D-5 — RuntimeCommand::StartTask sebagai canonical input untuk root TUI submit

- `vac-rs/tui/src/local_runtime.rs` menambahkan dua tipe runtime-first:
  `LegacyCompatTransport` (marker enum, varian tunggal `UserTurn`) dan
  `RuntimeSubmitPlan` dengan konstruktor
  `RuntimeSubmitPlan::from_runtime_command(RuntimeCommand)` /
  `RuntimeSubmitPlan::from_start_task(StartTask)`. Plan memegang
  `StartTask`, `RuntimeSession`, `RuntimeTask`, dan marker compat
  transport. Konstruktor menolak varian non-`StartTask` sehingga path
  submit tidak bisa diam-diam jatuh ke transport legacy tanpa input
  canonical.
- `vac-rs/tui/src/chatwidget.rs` (fungsi
  `submit_user_message_with_history_and_shell_escape_policy`) sekarang:
  1. Mint `RuntimeCommand::StartTask` via `mint_start_task`.
  2. Bangun `RuntimeSubmitPlan::from_runtime_command(...)`. Jika hasil
     `None`, submit dibatalkan dengan log error — tidak ada fallback
     senyap ke transport legacy.
  3. Log dua trace `vac::local_runtime::submit`: `start_trace` plan, dan
     pesan eksplisit “runtime-first submission: routing canonical
     RuntimeCommand::StartTask through legacy compat transport; 00E
     retires this transport”.
  4. Bangun `RuntimeBridge` dari `runtime_submission.session/task` dan
     proyeksikan opening events ke history root TUI.
  5. Bangun `AppCommand::user_turn(...)` *dari* plan (cwd diambil dari
     `runtime_submission.start_task.cwd`, bukan dari `self.config`).
     Marker compat transport diverifikasi via `debug_assert_eq!`.
  6. Submit op melalui `submit_op(op.clone())` sebagai compat transport.
- Komentar lama yang menyebut `AppCommand::user_turn` sebagai “execution
  source-of-truth” telah dihapus dan diganti dengan komentar
  runtime-first yang menyatakan secara eksplisit bahwa legacy op adalah
  transport sementara dan akan dipensiun di 00E.
- Tests baru di `vac-tui/local_runtime::tests`:
  `runtime_submission_plan_uses_start_task_as_canonical_input`,
  `runtime_submission_plan_rejects_non_start_task_commands`,
  `runtime_submission_plan_creates_session_and_task_from_start_task`,
  `runtime_submission_plan_preserves_prompt_and_cwd`,
  `runtime_submission_plan_marks_legacy_userturn_as_compat_transport`.
  36/36 test `local_runtime::` pass.
- Audit reachability: hanya satu caller `AppCommand::user_turn` di seluruh
  `vac-rs/tui/src/`, dan caller itu konsumsi `RuntimeSubmitPlan` dari
  baris di atasnya. Tidak ada path bottom-input yang membangun
  `AppCommand::user_turn` tanpa lebih dulu lewat `RuntimeSubmitPlan`.
- UX impact: tidak ada perubahan tampilan. `vac` → bottom input → Enter
  tetap. Operator masih melihat label runtime yang sama (session/task
  started, assistant delta, tool/validation lifecycle, approval
  request/resolve, task completed/failed/cancelled). Yang berubah
  hanya bentuk intent canonical di balik layar dan tambahan dua trace
  log `vac::local_runtime::submit` di `RUST_LOG=info` untuk membuktikan
  runtime-first path engaged.
- Status: 00D-5 implementation-ready. PTY/live dogfood gate masih
  wajib sebelum 00D dinyatakan pass.
