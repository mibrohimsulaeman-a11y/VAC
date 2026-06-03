# VAC Control Plane Refactor Report

Tanggal: 2026-06-01
Status: `SV-Done_TV-Pending`

## Tujuan

Refactor ini menjalankan arah audit **VAC as a Control Plane**: struktur source harus mulai mencerminkan layer `.vac`, bukan hanya façade kompatibilitas. Batch lanjutan ini menutup temuan audit state-saat-ini S-01, S-02, sebagian S-05, dan sebagian S-10 tanpa mengubah KEEP policy local coding agent.

## Perubahan Struktur

```text
vac-rs/
  control-plane/
    src/control_plane/      # source-of-record fisik untuk registry/workflow/policy/ownership
    src/local_runtime/      # source-of-record fisik untuk local runtime contracts
  provider-http/
    src/{auth,endpoint,requests,sse,...}  # source-of-record fisik provider HTTP transport
  vac-api/
    src/lib.rs              # compatibility-only re-export: pub use vac_provider_http::*
  core/
    src/lib.rs              # re-export compatibility for control_plane/local_runtime
```

## S-01 Closure — Control Plane Bukan Lagi `#[path]` Façade

`vac-control-plane` sekarang menyimpan file fisik `control_plane` dan `local_runtime`. Path historis `vac-rs/core/src/control_plane` dan `vac-rs/core/src/local_runtime` sudah tidak menjadi source-of-record. `vac-core` mempertahankan public compatibility melalui:

```rust
pub use vac_control_plane::control_plane;
pub use vac_control_plane::local_runtime;
```

Gate: `scripts/check-vac-o5o6-architecture-extraction-static.sh`.

## S-02 Closure — Provider HTTP Bukan Lagi Re-export Kosong

`vac-provider-http` sekarang memegang modul transport HTTP/SSE/WebSocket/Responses/Models/Memories/File upload yang sebelumnya berada di `vac-api`. `vac-api` disusutkan menjadi compatibility layer tunggal:

```rust
pub use vac_provider_http::*;
```

Ini menjadikan `provider-http` source-of-record fisik untuk transport provider generik, sambil menjaga call-site lama yang masih memakai `vac_api::*` sampai migrasi bertahap selesai.

## S-05 Partial Closure — Runtime Cloud Coupling Fail-Closed

Runtime production path yang sebelumnya mensintesis endpoint legacy ChatGPT/backend dari `chatgpt_base_url` sekarang fail-closed:

- session analytics tidak lagi membuat default backend client implisit;
- ARC monitor hanya berjalan bila `VAC_ARC_MONITOR_ENDPOINT_OVERRIDE` menunjuk service lokal/owned;
- VAC Apps cloud file upload bridge mengembalikan disabled error eksplisit;
- install/app TUI links diarahkan ke `developers.vastar.com/vac`.

Enum/protocol compatibility untuk `Chatgpt`/`ChatgptAuthTokens` belum dihapus karena terkait login/auth/provider compatibility dan membutuhkan cargo-backed migration.

Gate: `scripts/check-vac-o5o6-cloud-coupling-isolation-static.sh`.

## S-10 Partial Closure — Bounded Hot-paths

Selain `FrameRequester`, batch ini mengganti beberapa jalur producer/consumer penting dari unbounded ke bounded + `try_send`/capacity policy:

- `core/src/file_watcher.rs` → `FILE_WATCHER_RAW_EVENT_QUEUE_CAPACITY`;
- `core/src/agent/mailbox.rs` → `MAILBOX_QUEUE_CAPACITY`;
- `core/src/session/mod.rs` → `EVENT_CHANNEL_CAPACITY`;
- `tui/src/tui/frame_requester.rs` → `FRAME_SCHEDULE_QUEUE_CAPACITY`.

Gate: `scripts/check-vac-o5o6-bounded-hotpath-static.sh`.

## Masih TV-Pending / Belum Diklaim

Tidak ada klaim `cargo check`, `cargo test`, `cargo clippy`, atau live TUI smoke hijau karena sandbox tidak menyediakan `cargo`/`rustc`. Full relocation seluruh workspace ke `crates/<layer>/<name>`, penghapusan total semua legacy auth variant, dan deep re-modularization file raksasa `semantic_split`/`split_*` masih perlu compiler-backed iteration.
