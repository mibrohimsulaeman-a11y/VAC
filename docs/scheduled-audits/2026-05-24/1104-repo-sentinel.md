# Hourly Repo Sentinel Audit — 2026-05-24 11:04
Previous run: [1002-repo-sentinel.md](file:///home/emp/Documents/VAC/vastar-agentic-cli/docs/scheduled-audits/2026-05-24/1002-repo-sentinel.md) at 10:02
Carried: 2   New: 1   Dropped-as-resolved: 0

## Findings
| Severity | Area | Finding | Evidence (command + exit/snippet) | Suggested action | Origin |
|---|---|---|---|---|---|
| **WARNING** | Control Plane / Ownership | Separator modul `local_runtime::approval` tidak cocok dengan parser `ownership_scan.rs`. | `./vac-rs/target/debug/vac doctor ownership .` (exit 0) <br>`warning: claimed modules missing from source inventory [local_runtime::approval]` | Ubah format deklarasi modul di `.vac/capabilities/runtime_approval_bridge.yaml` menjadi format dotted (`local_runtime.approval`). | carried from 10:02 |
| **WARNING** | Control Plane / Registry | Pemilik surface route `routes[7]` di `palette.yaml` berbeda dengan pemilik kapabilitas `vac.local_runtime_owner`. | `./vac-rs/target/debug/vac doctor registry .` (exit 0) <br>`warning: ./.vac/surfaces/palette.yaml:routes[7].owner: surface route owner vac-core/local_runtime differs from capability owner vac-local-runtime-owner/startup for vac.local_runtime_owner` | Sesuaikan pembuat rute di `palette.yaml` atau perbarui deskripsi pemilik di manifest kapabilitas. | new |
| **INFO** | Git / Active Work | Terdeteksi modifikasi aktif masif di working tree. | `git status --short` (exit 0) <br>Mendeteksi 155+ berkas hasil modifikasi/hapus dan berkas untracked krusial seperti `approval_store.rs`. | **DILARANG KERAS** menjalankan `git reset --hard` atau `git clean -fd` demi menjaga integritas progres kerja aktif. | carried from 10:02 |

## Plan candidates (only if a finding has no obvious owner)
- Title: Penyelarasan Separator Modul `local_runtime.approval`
  - Why now: Menyelesaikan satu-satunya arsitektur warning kepemilikan agar dashboard visual kapabilitas kontrol-plane bersih tanpa noise.
  - Files likely involved: `.vac/capabilities/runtime_approval_bridge.yaml`
  - Verification command to confirm done: `./vac-rs/target/debug/vac doctor ownership .`
  - Risk if skipped: Doctor ownership akan terus melaporkan klaim modul hilang pada setiap audit berkala.
  - Suggested owner-lane: `unowned`

- Title: Dekopling Arsitektur `vac-tui → vac-app-server` (Plan 00F)
  - Why now: Melanjutkan pembersihan dependensi legacy transport (app-server) demi mewujudkan arsitektur local runtime yang benar-benar mandiri.
  - Files likely involved: `vac-rs/tui/Cargo.toml`, `vac-rs/tui/src/local_runtime_session.rs`, `vac-rs/tui/src/session_protocol.rs`
  - Verification command to confirm done: `./vac-rs/target/debug/vac doctor architecture .`
  - Risk if skipped: Ketergantungan terhadap kode legacy donor transport terus membebani performa dan mempersulit pemeliharaan jangka panjang.
  - Suggested owner-lane: `unowned`

- Title: Penyelarasan Pemilik Rute `vac.local_runtime_owner`
  - Why now: Menghilangkan peringatan ketidaksesuaian pemilik rute permukaan antara registri rute dan deklarasi kapabilitas.
  - Files likely involved: `.vac/surfaces/palette.yaml`, `.vac/capabilities/local_runtime_owner.yaml`
  - Verification command to confirm done: `./vac-rs/target/debug/vac doctor registry .`
  - Risk if skipped: Peringatan ketidakselarasan registri rute permukaan dan manifest kapabilitas akan terus muncul di audit berkala.
  - Suggested owner-lane: `unowned`
