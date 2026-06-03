# Hourly Repo Sentinel Audit — 2026-05-24 02:02

## Executive summary
- **Overall status**: **FAIL**  
  Sistem validasi kontrol internal (`vac doctor`) berada dalam status **FAIL** (exit code 1) akibat ketidaksesuaian pemetaan rute permukaan (**surface route drift**) serta inkonsistensi kepemilikan (**ownership consistency failures**) antara `.vac/surfaces/*.yaml` dan `.vac/capabilities/*.yaml`. Kegagalan registry ini secara cascading memblokir kesuksesan verifikasi alur kerja (`vac doctor workflow`).
- **Highest risk**: **Surface Route & Ownership Alignment Drift**  
  Terdapat puluhan rute permukaan di `.vac/surfaces/slash.yaml`, `cli.yaml`, `tui.yaml`, dan `palette.yaml` yang tidak dideklarasikan secara eksplisit dalam manifest kapabilitas `.vac/capabilities/*.yaml`, atau memiliki konflik string `owner` (seperti `vac-tui::chatwidget` vs `vac-core/session`).
- **Recommended next slice**: **Surface & Domain Mapping Normalization**  
  Melakukan pemetaan total (alignment) rute surfaces dengan manifest kapabilitas serta mengklaim domain Rust source (`vac-rs/core/src/...`) yang belum dialokasikan untuk mengeliminasi ribuan baris warning kepemilikan di `vac doctor registry`.

## Findings
| Severity | Area | Finding | Evidence | Suggested action |
|---|---|---|---|---|
| **CRITICAL** | Surfaces / Registry | Kegagalan validasi `vac doctor surfaces` & `registry` akibat inkonsistensi rute surfaces. | Diagnostik mendeteksi drift rute surfaces pada kapabilitas utama (`vac.sessions`, `vac.tools`, `vac.approvals`, `vac.tui`): `owner_conflicts=6 palette_drift=11 route_drift=7`. | Normalisasikan deklarasi rute surfaces di `.vac/surfaces/*.yaml` agar selaras dengan `.vac/capabilities/*.yaml`. |
| **CRITICAL** | Diagnostics / Source Domains | Ribuan baris warning ketidaksesuaian domain source Rust yang belum diklaim (`not claimed by any capability ownership target`). | Output task log `vac doctor registry .` dipenuhi dengan warning untuk domain Rust source (seperti di `vac-rs/core/src/local_runtime/`). | Petakan domain source Rust yatim piatu ke target kepemilikan kapabilitas yang tepat. |
| **EXCELLENT** | Donor Isolation | Pipa isolasi dan kepatuhan migrasi donor berada dalam kondisi prima tanpa drift. | Skrip `bash scripts/check-donor-status.sh` sukses 100% (**PASSED**) untuk semua checks (inventory, drift, manifest, reachability, evidence, commit phrase). | Pertahankan isolasi crate donor dan terus verifikasi lewat skrip check ini di setiap sesi. |
| **INFO** | Git / Dirty Work Status | Terdapat modifikasi masif uncommitted (~143 berkas dirty, ~15k baris) terkait PTY Gate, Workflow, dll. | `git status -s` mendeteksi 143 file termodifikasi/untracked. Unit test control plane tetap hijau dan sukses dikompilasi. | **Wajib dipertahankan**. Jangan lakukan `git reset`, `git clean`, atau staging file. |
| **INFO** | Disk Capacity | Ruang penyimpanan lokal aman untuk build/test. | Pemeriksaan `df -h` melaporkan sisa ruang disk **60G** (> syarat minimal 20G). | Operasi build/test yang terencana aman untuk dieksekusi. |

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
  - `cargo test --manifest-path vac-rs/Cargo.toml -p vac-core registry_diagnostics -- --nocapture` (**PASSED** - 20 passed, 0 failed).  
  - `bash scripts/check-donor-status.sh` (**PASSED**).  
  - `./vac-rs/target/debug/vac doctor policy .` (**PASSED**).  
- **Heavy checks to defer**:  
  - `cargo test --all` (Hindari karena beban pengujian masif dan risiko file locking).  
  - Pengujian manual end-to-end PTY TUI secara penuh di dalam environment sandbox non-interactive.

## Do-not-touch / coordination notes
- **Dirty work preservation**:  
  - Pertahankan file untracked `.vac/capabilities/tui-pty-gate.yaml` dan berkas uncommitted lainnya. Jangan bersihkan environment (`git clean` dilarang keras).  
  - Jaga ketat ketiadaan kebocoran dependensi kargo donor dengan rutin menjalankan skrip check status donor.
