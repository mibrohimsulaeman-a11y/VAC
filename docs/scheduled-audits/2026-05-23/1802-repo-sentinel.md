# Hourly Repo Sentinel Audit — 2026-05-23 18:02

## Executive summary
- **Overall status**: **FAIL (BLOCKED)**  
  Sistem kontrol dan validasi internal repositori (`vac doctor`) masih mengalami kegagalan akibat perubahan struktural terbaru pada kode `vac-core` yang mewajibkan penulisan field `states` pada semua manifest kapabilitas non-deprecated. Kegagalan ini memblokir seluruh workflow kontrol dan permukaan observabilitas.
- **Highest risk**: **Registry Validation Breakdown**  
  Registry gagal memuat 14 file kapabilitas di bawah `.vac/capabilities/` karena tidak mendeklarasikan `states`, sehingga pemeriksaan internal dan audit arsitektur otomatis tidak dapat berjalan normal.
- **Recommended next slice**: **Restore Capability States & Decouple Cargo Edge**  
  Menambahkan definisi `states` standar pada ke-14 manifest kapabilitas aktif guna memulihkan kelancaran perintah `vac doctor` secara instan, diikuti dengan pembersihan Cargo edge dependensi lama (`vac-app-server`) pada TUI.

## Findings
| Severity | Area | Finding | Evidence | Suggested action |
|---|---|---|---|---|
| **CRITICAL** | Registry / Control Plane | Seluruh perintah `vac doctor` gagal total karena mewajibkan deklarasi `states` pada manifest kapabilitas non-deprecated. | Pesan kesalahan dari `./vac-rs/target/debug/vac doctor registry .`: `states: non-deprecated capabilities must declare states` pada 14 file `.yaml` di `.vac/capabilities/`. | Tambahkan definisi `states` standar (misal: `- empty`, `- loading`, `- success`, `- failure`) ke seluruh kapabilitas aktif dalam sesi berikutnya yang mengizinkan penulisan berkas proyek. |
| **WARNING** | Architecture Invariant | Terdeteksi dependensi langsung transport lama (`vac-app-server`) langsung pada `vac-rs/tui/Cargo.toml`. | `tui-direct-app-server-dep: warn` didefinisikan pada `architecture_invariants.rs` dan dipicu oleh baris dependensi langsung di `tui/Cargo.toml`. | Lakukan refaktorisasi bertahap untuk menghapus dependensi langsung `vac-app-server` dari TUI, beralih penuh ke *Local Runtime Contract*. |
| **INFO** | Git / Dirty Work | Terdeteksi modifikasi sangat masif (133 file berubah, ~14.9k baris ditambahkan) dan file baru bertipe *untracked* terkait integrasi PTY Gate & Store. | `.vac/capabilities/tui-pty-gate.yaml`, `.vac/workflows/maintenance.tui-pty-gate.yaml`, `approval_store.rs`, `build_check.rs`, `donor_status.rs`, dan `registry_tests.rs` berstatus *untracked* / *dirty*. | Pertahankan seluruh berkas-berkas ini sebagai *dirty work* sesuai instruksi read-only, jangan di-stage, di-reset, atau di-commit secara tidak sengaja. |
| **INFO** | Donor Migration | Tidak ada drift terdeteksi antara papan status migrasi dan matriks inventaris. Rencana domain 01-10 memiliki active execution contract. | `scripts/check-donor-status.sh drift` melaporkan `OK` dan konsisten pada seluruh item yang diperiksa (status exit 0). | Selesaikan pembersihan Cargo edge pada `vac_session_engine` sebelum mengubah statusnya dari `ADAPT_IN_PROGRESS` menjadi `MIGRATED`. |

## Plan candidates

### Deferred / blocked by stop condition
- TUI Dependency Decoupling (App-Server): deferred because current repo state violates its documented stop condition; do not force-delete dependencies or fake manual PTY proof.
## Docs sync notes
- **Paths that look stale**:  
  - `docs/donor-migration/DONOR_STATUS_BOARD.md` (Perlu diperbarui keterangannya setelah Cargo edge dilepas).
- **Paths that should be updated if code changes**:  
  - `docs/product/CAPABILITY_MAP.md` dan `docs/product/CAPABILITY_PRD_COVERAGE.md` apabila kapabilitas baru ditambahkan atau dipromosikan statusnya.
  - `docs/validation/TUI_PTY_DOGFOOD_GATE.md` jika ada perubahan perilaku pada eksekusi PTY Operator Gate.

## Validation recommendation
- **Safe quick checks**:  
  - `bash scripts/check-donor-status.sh drift` (konsistensi dokumen migrasi).
  - `bash scripts/check-donor-status.sh reachability` (pemeriksaan isolasi dependensi donor).
  - `cargo test --manifest-path vac-rs/Cargo.toml -p vac-core registry_diagnostics` (unit test diagnostik).
- **Heavy checks to defer**:  
  - Pengujian penuh end-to-end PTY TUI menggunakan `vac` (karena lingkungan saat ini berupa sandbox tanpa TTY buffer alternatif).

## Do-not-touch / coordination notes
- **Dirty work or potential conflicts**:  
  - Modifikasi yang ada pada berkas rencana migrasi (`docs/donor-migration/domain-plans/`) dan repositori inti `vac-rs/` **wajib dipertahankan** (jangan lakukan `git checkout --`, `git reset`, atau `git clean`).
  - Berkas baru yang masih untracked (`.vac/capabilities/tui-pty-gate.yaml` dan berkas alur kerja pendukungnya, serta file store/check baru di core) tidak boleh dihapus atau dipindahkan karena merupakan bagian dari persiapan rilis fitur `PTY validation gate`.
