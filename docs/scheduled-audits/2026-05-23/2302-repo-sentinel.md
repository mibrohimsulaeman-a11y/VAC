# Hourly Repo Sentinel Audit — 2026-05-23 23:02

## Executive summary
- **Overall status**: **FAIL**  
  Sistem validasi kontrol internal (`vac doctor`) tetap berada dalam status **FAIL** akibat adanya ketidaksesuaian pemetaan rute permukaan (**surface route drift**) serta konflik konsistensi kepemilikan (**ownership consistency failures**) di antara berkas-berkas permukaan (`.vac/surfaces/*.yaml`) dan manifest kapabilitas (`.vac/capabilities/*.yaml`). Kegagalan registry ini secara cascading memblokir kesuksesan perintah `vac doctor workflow`.
- **Highest risk**: **Surface Route Mapping Discrepancy & Owner Invariant Drift**  
  Terdapat ketidakselarasan rute permukaan (`palette_drift=11`, `route_drift=7`) serta konflik kepemilikan kapabilitas (`owner_conflicts=6`) pada kapabilitas utama (`vac.approvals`, `vac.chat`, `vac.sessions`, `vac.workflow`, dll). Konflik ini memblokir integrasi pipeline lokal dan berpotensi menyebabkan ketidakjelasan hak akses modul pada antarmuka operator.
- **Recommended next slice**: **Surface Route and Ownership Normalization**  
  Melakukan penyelarasan total (alignment) rute permukaan di `.vac/surfaces/tui.yaml`, `cli.yaml`, `palette.yaml`, `slash.yaml` dengan manifest kapabilitas di `.vac/capabilities/*.yaml`. Standardisasikan format penamaan pemilik (owner) agar konsisten di seluruh surface untuk kapabilitas yang sama.

## Findings
| Severity | Area | Finding | Evidence | Suggested action |
|---|---|---|---|---|
| **CRITICAL** | Surfaces / Registry | Kegagalan validasi `vac doctor registry` akibat ketidaksesuaian rute surfaces dan inkonsistensi kepemilikan. | Output `./vac-rs/target/debug/vac doctor registry .` melaporkan `duplicates=0 owner_conflicts=6 palette_drift=11 route_drift=7` dan keluar dengan exit code 1. | Normalisasikan deklarasi rute surfaces dan seragamkan format penamaan owner pada file yaml kapabilitas. |
| **CRITICAL** | Workflow Doctor | Kegagalan `vac doctor workflow` karena dependensi pada diagnostik registry yang gagal. | Output `./vac-rs/target/debug/vac doctor workflow .` mengembalikan kegagalan (exit code 1) secara berantai. | Selesaikan drift surfaces dan registry terlebih dahulu sebelum melanjutkan pengujian alur kerja. |
| **INFO** | Git / Worktree Status | Terdapat modifikasi masif (~15k baris uncommitted) terkait PTY Gate, Workflow Runner, dan Domain Plans. | `git status -s` menunjukkan 143 berkas termodifikasi/untracked. Seluruh unit test inti tetap aman dan berhasil dikompilasi secara parsial. | Sesuai aturan read-only, pertahankan semua perubahan ini sebagai *dirty work*. Jangan lakukan reset, clean, commit, atau stage. |
| **INFO** | Donor Isolation | Skrip pemantau donor melaporkan isolasi dependensi dan ketiadaan drift migrasi yang sangat baik. | Jalannya `bash scripts/check-donor-status.sh drift` dan `reachability` mengembalikan status sukses (exit status 0). | Pertahankan isolasi crate donor dan terus gunakan skrip pemantau donor pada setiap sesi audit. |

## Plan candidates

All actionable plan candidates from this audit were executed and removed from this backlog section.
## Docs sync notes
- **Paths that look stale**:  
  - `docs/donor-migration/DONOR_STATUS_BOARD.md` (Perlu diperbarui ketika Cargo edge dilepas atau status migrasi domain plan meningkat).
- **Paths that should be updated if code changes**:  
  - `docs/product/CAPABILITY_MAP.md` dan `docs/product/CAPABILITY_PRD_COVERAGE.md` apabila kapabilitas baru dideklarasikan atau status migrasinya meningkat.
  - `docs/validation/TUI_PTY_DOGFOOD_GATE.md` jika ada penyesuaian fungsional pada PTY Operator Gate.

## Validation recommendation
- **Safe quick checks**:  
  - `cargo test --manifest-path vac-rs/Cargo.toml -p vac-core registry_diagnostics -- --nocapture` (Berhasil penuh: **PASSED** dalam 19.06 detik, membuktikan integritas unit test control plane tetap kokoh).
  - `bash scripts/check-donor-status.sh drift` (Lolos dengan status PASSED).
  - `bash scripts/check-donor-status.sh reachability` (Lolos dengan status PASSED).
- **Heavy checks to defer**:  
  - `cargo test --all` (Hindari karena beban sumber daya berat dan potensi file locking).
  - Pengujian manual end-to-end PTY TUI secara penuh di dalam environment sandbox non-interactive.

## Do-not-touch / coordination notes
- **Dirty work or potential conflicts**:  
  - Seluruh berkas yang saat ini berstatus uncommitted/dirty (termasuk implementasi baru di `vac-rs/core/src/control_plane/workflow_runner.rs`, approval store baru, dll) **wajib dipertahankan sepenuhnya** tanpa ada perubahan/penghapusan.
  - Berkas baru untracked seperti `.vac/capabilities/tui-pty-gate.yaml` dan `.vac/workflows/maintenance.tui-pty-gate.yaml` tidak boleh dibersihkan (jangan jalankan `git clean -fd`).
