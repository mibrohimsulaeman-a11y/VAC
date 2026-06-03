#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
fail() { echo "bounded hotpath static gate: $*" >&2; exit 1; }
require_file() { [[ -f "$1" ]] || fail "missing file: $1"; }
require_grep() { grep -qE "$1" "$2" || fail "missing pattern in $2: $1"; }
reject_grep() { ! grep -qE "$1" "$2" || fail "forbidden pattern in $2: $1"; }

# S-10 targeted hot-path closures: scheduler/event/session/file watcher/mailbox are bounded.
require_file vac-rs/crates/surfaces/tui/src/tui/frame_requester.rs
require_grep 'FRAME_SCHEDULE_QUEUE_CAPACITY' vac-rs/crates/surfaces/tui/src/tui/frame_requester.rs
require_grep 'mpsc::channel\(FRAME_SCHEDULE_QUEUE_CAPACITY\)' vac-rs/crates/surfaces/tui/src/tui/frame_requester.rs
reject_grep 'unbounded_channel|UnboundedSender|UnboundedReceiver' vac-rs/crates/surfaces/tui/src/tui/frame_requester.rs

require_file vac-rs/crates/capabilities/build/src/core_migrated/file_watcher.rs
require_grep 'FILE_WATCHER_RAW_EVENT_QUEUE_CAPACITY' vac-rs/crates/capabilities/build/src/core_migrated/file_watcher.rs
require_grep 'mpsc::channel\(FILE_WATCHER_RAW_EVENT_QUEUE_CAPACITY\)' vac-rs/crates/capabilities/build/src/core_migrated/file_watcher.rs
require_grep 'try_send\(res\)' vac-rs/crates/capabilities/build/src/core_migrated/file_watcher.rs
reject_grep 'unbounded_channel|UnboundedSender|UnboundedReceiver' vac-rs/crates/capabilities/build/src/core_migrated/file_watcher.rs

require_file vac-rs/crates/capabilities/identity/src/core_migrated/agent/mailbox.rs
require_grep 'MAILBOX_QUEUE_CAPACITY' vac-rs/crates/capabilities/identity/src/core_migrated/agent/mailbox.rs
require_grep 'mpsc::channel\(MAILBOX_QUEUE_CAPACITY\)' vac-rs/crates/capabilities/identity/src/core_migrated/agent/mailbox.rs
require_grep 'try_send\(communication\)' vac-rs/crates/capabilities/identity/src/core_migrated/agent/mailbox.rs
reject_grep 'unbounded_channel|UnboundedSender|UnboundedReceiver' vac-rs/crates/capabilities/identity/src/core_migrated/agent/mailbox.rs

require_file vac-rs/crates/capabilities/sessions/src/core_migrated/session/mod.rs
require_grep 'EVENT_CHANNEL_CAPACITY' <(grep -R '' vac-rs/crates/capabilities/sessions/src/core_migrated/session)
require_grep 'async_channel::bounded\(EVENT_CHANNEL_CAPACITY\)' <(grep -R '' vac-rs/crates/capabilities/sessions/src/core_migrated/session)
! grep -R --include='*.rs' -nE 'let \(tx_event, rx_event\) = async_channel::unbounded\(\)' vac-rs/crates/capabilities/sessions/src/core_migrated/session | grep -v '/tests.rs:' | grep -q . || fail 'forbidden unbounded event channel in vac-rs/crates/capabilities/sessions/src/core_migrated/session'

printf 'bounded hotpath static gate: PASS\n'
