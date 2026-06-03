# Hourly Repo Sentinel Audit — 2026-05-23 21:02

## Executive summary
- **Overall status**: **FAIL**  
  Sistem validasi kontrol internal (`vac doctor`) tetap berada dalam status **FAIL** karena masalah ketidaksesuaian pemetaan rute permukaan (**surface route drift**) dan konflik konsistensi kepemilikan (**ownership consistency failures**) di antara berkas-berkas permukaan (`.vac/surfaces/*.yaml`) dan manifest kapabilitas (`.vac/capabilities/*.yaml`). Masalah parser kapabilitas telah diselesaikan pada audit sebelumnya, namun sinkronisasi rute belum dilakukan.
- **Highest risk**: **Surface Route Mapping Discrepancy & Owner Invariant Drift**  
  Terdeteksi **11 palette_drift**, **7 route_drift**, dan **6 owner conflicts** di mana kepemilikan kapabilitas (`vac.approvals`, `vac.chat`, `vac.identity`, `vac.sessions`, `vac.tools`, `vac.workflow`) tidak konsisten di berbagai berkas permukaan. Hal ini menyebabkan kegagalan validasi `vac doctor surfaces` dan berpotensi memicu blindspot pada validasi rute UI terhadap kontrol akses kapabilitas.
- **Recommended next slice**: **Surface Route and Ownership Normalization**  
  Melakukan rekonsiliasi total antara rute yang diklaim di `.vac/capabilities/` dengan yang dideklarasikan di berkas surfaces (`.vac/surfaces/tui.yaml`, `cli.yaml`, `palette.yaml`, `slash.yaml`). Normalisasikan juga penamaan owner agar konsisten di seluruh surface (misal mengkonsolidasikan string `vac-tui::...` dan `vac-rs/cli`).

## Findings
| Severity | Area | Finding | Evidence | Suggested action |
|---|---|---|---|---|
| **CRITICAL** | Surfaces / Registry | Validasi `vac doctor surfaces` gagal karena drift rute permukaan dan konflik owner. | Terdeteksi `duplicates=0 owner_conflicts=6 palette_drift=11 route_drift=7`. Contoh: `vac.approvals` memiliki owner `vac-rs/cli` di `cli.yaml` tapi `vac-tui::approvals` di `slash.yaml`. | Selaraskan rute di `.vac/surfaces/*.yaml` dengan `.vac/capabilities/*.yaml` dan seragamkan format penamaan owner. |
| **CRITICAL** | Workflow Doctor | Validasi `vac doctor workflow` juga gagal (exit code 1) karena dependensi pada diagnostik registry/surfaces yang drift. | Perintah `./vac-rs/target/debug/vac doctor workflow .` mengembalikan status FAILED. | Tanggulangi drift surfaces dan registry terlebih dahulu agar workflow doctor dapat berjalan lancar. |
| **INFO** | Git / Dirty Work | Terdeteksi modifikasi masif uncommitted (~15k baris) terkait PTY Gate, Workflow Runner baru, dan Active Execution Contract pada Domain Plans 01–10. | `git status -s` menampilkan status modified/untracked pada 130+ file. | Sesuai instruksi read-only, pertahankan seluruh berkas-berkas ini sebagai *dirty work*, jangan di-stage, di-reset, atau di-clean. |
| **INFO** | Donor Migration | Skrip validasi donor (`check-donor-status.sh`) melaporkan 0 drift dan 0 kebocoran dependensi Cargo, menunjukkan kepatuhan isolasi donor yang luar biasa. | `scripts/check-donor-status.sh drift` dan `reachability` sukses dengan exit status 0 (PASSED). | Lanjutkan kepatuhan isolasi dependensi dan gunakan skrip validasi secara berkala pada setiap siklus pengerjaan. |

## Plan candidates

All actionable plan candidates from this audit were executed and removed from this backlog section.
## Docs sync notes
- **Paths that look stale**:  
  - `docs/donor-migration/DONOR_STATUS_BOARD.md` (Perlu disinkronkan keterangannya saat cargo edge dilepas atau status migrasi domain plan ditingkatkan).
- **Paths that should be updated if code changes**:  
  - `docs/product/CAPABILITY_MAP.md` dan `docs/product/CAPABILITY_PRD_COVERAGE.md` apabila kapabilitas baru ditambahkan atau dipromosikan statusnya.
  - `docs/validation/TUI_PTY_DOGFOOD_GATE.md` jika ada perubahan perilaku pada eksekusi PTY Operator Gate.

## Validation recommendation
- **Safe quick checks**:  
  - `bash scripts/check-donor-status.sh drift` (konsistensi dokumen migrasi).
  - `bash scripts/check-donor-status.sh reachability` (pemeriksaan isolasi dependensi donor).
  - `./vac-rs/target/debug/vac doctor policy .` (Lolos dengan status PASS).
- **Heavy checks to defer**:  
  - `cargo test --all` (hindari cargo file lock dan resource berat saat sesi audit berkala).
  - Pengujian penuh end-to-end PTY TUI menggunakan `vac` (karena lingkungan saat ini berupa sandbox tanpa TTY buffer alternatif).

## Do-not-touch / coordination notes
- **Dirty work or potential conflicts**:  
  - Modifikasi uncommitted yang ada pada berkas rencana migrasi (`docs/donor-migration/domain-plans/`) dan repositori inti `vac-rs/` **wajib dipertahankan sepenuhnya** (jangan lakukan `git checkout --`, `git reset`, atau `git clean`).
  - Berkas baru yang masih untracked (`.vac/capabilities/tui-pty-gate.yaml`, `.vac/workflows/maintenance.tui-pty-gate.yaml`, serta file approval store dan build check baru di core) tidak boleh dihapus or dipindahkan karena merupakan fondasi utama fitur validasi PTY gate.
