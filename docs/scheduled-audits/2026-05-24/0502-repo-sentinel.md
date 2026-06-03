# Hourly Repo Sentinel Audit — 2026-05-24 05:02

## Executive summary
- **Overall status**: **FAIL**  
  Status validasi kontrol internal (`vac doctor surfaces` dan `registry`) tetap berada dalam kondisi **FAIL** (exit code 1) dikarenakan ketidaksesuaian pemetaan rute permukaan (**surface route drift**) serta pertentangan kepemilikan rute (**owner conflicts**) antara `.vac/surfaces/*.yaml` dan `.vac/capabilities/*.yaml`. Kegagalan registry ini juga menghalangi eksekusi alur kerja (`vac doctor workflow`). Meskipun begitu, pengujian unit test registry diagnostics (`cargo test -p vac-core registry_diagnostics`) dan isolasi migrasi donor (`check-donor-status.sh`) berhasil lolos dengan status hijau penuh (**PASSED**).
- **Highest risk**: **Surface Route & Ownership Alignment Drift**  
  Inkonsistensi antara rute surfaces di `.vac/surfaces/tui.yaml` dengan manifest kapabilitas di `.vac/capabilities/` memicu puluhan kesalahan kepemilikan rute yang bertentangan (`owner_conflicts=6`), status palette menyimpang (`palette_drift=11`), dan penyimpangan rute TUI/CLI (`route_drift=7`).
- **Recommended next slice**: **Surface & Domain Mapping Normalization**  
  Melakukan penyelarasan total (alignment) rute surfaces dengan manifest kapabilitas serta mengklaim domain Rust source (`vac-rs/analytics/src/...`, `vac-rs/vac-api/src/...`, `vac-rs/app-server/...`, dll.) yang belum terpetakan guna mengeliminasi error diagnostik dan log warning kepemilikan yatim piatu di `vac doctor registry`.

## Findings
| Severity | Area | Finding | Evidence | Suggested action |
|---|---|---|---|---|
| **CRITICAL** | Surfaces / Registry | Kegagalan validasi `vac doctor surfaces` & `registry` akibat drift rute dan owner. | Menjalankan `./vac-rs/target/debug/vac doctor surfaces .` mengembalikan kegagalan status dengan `owner_conflicts=6 palette_drift=11 route_drift=7`. | Selaraskan rute di `.vac/surfaces/tui.yaml` dan `.vac/capabilities/*.yaml` agar konsisten secara semantik. |
| **CRITICAL** | Diagnostics / Source Domains | Ribuan baris warning ketidaksesuaian domain source Rust yang belum diklaim oleh kapabilitas. | Log diagnostik dipenuhi dengan peringatan kepemilikan seperti `source domain vac-api/auth is not claimed by any capability ownership target`. | Tambahkan domain source Rust tersebut di bawah target kepemilikan kapabilitas di `.vac/capabilities/*.yaml` atau tandai sebagai `test_only` jika sesuai. |
| **EXCELLENT** | Donor Isolation | Pipa isolasi dan kepatuhan migrasi donor berada dalam kondisi prima tanpa drift. | Skrip `bash scripts/check-donor-status.sh` sukses 100% (**PASSED**) untuk semua checks (inventory, drift, manifest, reachability, evidence, commit phrase). | Pertahankan isolasi crate donor dan terus verifikasi lewat skrip check ini di setiap sesi. |
| **INFO** | Git / Dirty Work Status | Pekerjaan aktif masif uncommitted (~143 berkas dirty, ~15k baris) terkait PTY Gate, Workflow, dll. | `git status -s` mendeteksi 143 file termodifikasi/untracked. Unit test control plane tetap hijau dan sukses dikompilasi. | **Wajib dipertahankan**. Jangan lakukan `git reset`, `git clean`, atau staging file. |
| **INFO** | Disk Capacity | Ruang penyimpanan lokal sangat aman untuk build/test. | Pemeriksaan `df -h` melaporkan sisa ruang disk sebesar **59G** (> syarat minimal 20G). | Operasi build/test yang terencana aman untuk dieksekusi. |

## Plan candidates

All actionable plan candidates from this audit were executed and removed from this backlog section.
## Docs sync notes
- **Paths that look stale**:  
  - `docs/donor-migration/DONOR_STATUS_BOARD.md` (Perlu diperbarui ketika Cargo edge dilepas atau status migrasi domain plan meningkat).  
- **Paths that should be updated if code changes**:  
  - `docs/DOCS_AUDIT.md` (Perlu mencatat hasil audit dokumen terbaru).  
  - `docs/product/CAPABILITY_MAP.md` dan `docs/product/CAPABILITY_PRD_COVERAGE.md` apabila kapabilitas baru dideklarasikan atau status migrasinya meningkat.  
  - `docs/validation/TUI_PTY_DOGFOOD_GATE.md` jika ada penyesuaian fungsional pada PTY Operator Gate.

## Validation recommendation
- **Safe quick checks**:  
  - `cargo test --manifest-path vac-rs/Cargo.toml -p vac-core registry_diagnostics -- --nocapture` (**PASSED** - 20 passed, 0 failed).  
  - `bash scripts/check-donor-status.sh` (**PASSED**).  
  - `./vac-rs/target/debug/vac doctor policy .` (**PASSED**).  
  - `./vac-rs/target/debug/vac doctor donor .` (**PASSED**).  
- **Heavy checks to defer**:  
  - `cargo test --all` (Hindari karena beban pengujian masif dan risiko file locking).  
  - Pengujian manual end-to-end PTY TUI secara penuh di dalam environment sandbox non-interactive.

## Do-not-touch / coordination notes
- **Dirty work or potential conflicts**:  
  - Pertahankan file untracked `.vac/capabilities/tui-pty-gate.yaml`, `.vac/workflows/maintenance.tui-pty-gate.yaml`, snapshot baru, dan berkas uncommitted lainnya. Jangan bersihkan environment (`git clean` dilarang keras).  
  - Jaga ketat ketiadaan kebocoran dependensi kargo donor dengan rutin menjalankan skrip check status donor.
