# Hourly Repo Sentinel Audit — 2026-05-23 20:02

## Executive summary
- **Overall status**: **FAIL (BLOCKED)**  
  Meskipun hambatan pemblokiran parsing sebelumnya terkait parameter wajib `states` pada kapabilitas aktif telah **berhasil diatasi** (sehingga parser berhasil memuat ke-32 manifest kapabilitas), seluruh sistem validasi kontrol internal (`vac doctor`) saat ini tetap berada dalam status **FAIL** akibat ketidaksinkronan rute permukaan (**surface route drift**) dan ketidakkonsistenan kepemilikan (**ownership consistency failures**) di seluruh berkas permukaan (`.vac/surfaces/*.yaml`) dan kapabilitas (`.vac/capabilities/*.yaml`).
- **Highest risk**: **Surface Route Mapping Discrepancy & Owner Invariant Drift**  
  Terdeteksi 11 kasus `palette_drift`, 7 kasus `route_drift`, dan beberapa inkonsistensi pemilik (misalnya antara `vac-tui::chatwidget` dan `vac-tui/chatwidget`). Drifts ini menyebabkan perintah `vac doctor surfaces` dan `vac doctor registry` gagal memvalidasi konsistensi UI rute, menciptakan blindspot pada integrasi UI baru dengan sistem kontrol akses kapabilitas.
- **Recommended next slice**: **Surface Route and Owner Alignment Metadata Sync**  
  Melakukan penyelarasan metadata rute secara menyeluruh dengan memperbarui rute palette dan CLI yang diklaim di `.vac/capabilities/` ke dalam berkas permukaan di `.vac/surfaces/`, serta menormalkan nama-nama pemilik modul (owner) menggunakan format terstandar untuk memulihkan status validasi `vac doctor` menjadi hijau (exit code 0).

## Findings
| Severity | Area | Finding | Evidence | Suggested action |
|---|---|---|---|---|
| **CRITICAL** | Registry / Surfaces | Validasi `vac doctor surfaces` dan `vac doctor registry` gagal akibat ketidaksesuaian pemetaan rute permukaan (surface routes) dan ownership drift. | 11 `palette_drift` dan 7 `route_drift` terdeteksi; kesalahan ketidakkonsistenan pemilik (owner) padas `vac.chat`, `vac.sessions`, `vac.tools`, `vac.workflow` oleh `vac doctor surfaces .`. | Perbarui entri rute permukaan di `.vac/surfaces/*.yaml` agar selaras dengan klaim kapabilitas di `.vac/capabilities/*.yaml` dan seragamkan format penamaan owner. |
| **INFO** | Capability Schema | Masalah parser blocking sebelumnya terkait field wajib `states` telah **berhasil diatasi** di seluruh manifest kapabilitas aktif. | parser sukses memuat ke-32 manifest kapabilitas tanpa melempar pesan kesalahan `states: non-deprecated capabilities must declare states`. | Pertahankan penambahan key `states` yang valid pada seluruh manifest kapabilitas aktif dalam sesi modifikasi kode berikutnya. |
| **INFO** | Git / Dirty Work | Terdeteksi modifikasi masif uncommitted (~15k baris) terkait PTY Gate, Workflow Runner baru, dan formalisasi Active Execution Contract pada Domain Plans 01–10. | `git status -s` menampilkan status `M` pada 130+ file, dan berkas untracked baru terkait `.vac/workflows/maintenance.tui-pty-gate.yaml` serta `approval_store.rs`. | Sesuai instruksi read-only, pertahankan seluruh berkas-berkas ini sebagai *dirty work*, jangan di-stage, di-reset, atau di-clean. |
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
  - Berkas baru yang masih untracked (`.vac/capabilities/tui-pty-gate.yaml`, `.vac/workflows/maintenance.tui-pty-gate.yaml`, serta file approval store dan build check baru di core) tidak boleh dihapus atau dipindahkan karena merupakan fondasi utama fitur validasi PTY gate.
