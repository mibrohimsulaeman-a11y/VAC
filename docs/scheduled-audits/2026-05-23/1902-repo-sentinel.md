# Hourly Repo Sentinel Audit — 2026-05-23 19:02

## Executive summary
- **Overall status**: **FAIL (BLOCKED)**  
  Seluruh sistem validasi kontrol internal repositori (`vac doctor`) saat ini berada dalam status **FAIL** dan terblokir sepenuhnya karena adanya *schema drift* pada parser kapabilitas. Perubahan terbaru pada `vac-core` mengharuskan seluruh manifest kapabilitas aktif (non-deprecated) untuk mendeklarasikan field `states`, namun 14 berkas kapabilitas aktif di `.vac/capabilities/` belum diperbarui sehingga gagal divalidasi.
- **Highest risk**: **Registry Validation Blockage & Architectural Blindspots**  
  Kegagalan parser untuk memuat 14 berkas kapabilitas menyebabkan perintah `vac doctor registry`, `vac doctor policy`, `vac doctor surfaces`, dan `vac doctor workflow` tidak dapat mendeteksi pelanggaran arsitektur baru atau ketidakselarasan rute permukaan. Ini menciptakan blindspot kritis dalam audit keselamatan otomatis.
- **Recommended next slice**: **Capability States Alignment & Decouple Cargo Edge**  
  Menambahkan deklarasi `states` standar (seperti: `- empty`, `- loading`, `- success`, `- failure`) pada ke-14 manifest kapabilitas aktif di bawah `.vac/capabilities/` untuk memulihkan kelancaran perintah `vac doctor`. Selanjutnya, lakukan decoupling terhadap ketergantungan Cargo langsung antara `vac-tui` dan `vac-app-server` lama.

## Findings
| Severity | Area | Finding | Evidence | Suggested action |
|---|---|---|---|---|
| **CRITICAL** | Registry / Control Plane | Seluruh perintah validasi `vac doctor` gagal total karena validasi internal mewajibkan deklarasi `states` pada manifest kapabilitas non-deprecated. | Pesan kesalahan dari `./vac-rs/target/debug/vac doctor registry .`: `states: non-deprecated capabilities must declare states` pada 14 berkas kapabilitas aktif di `.vac/capabilities/`. | Tambahkan definisi `states` standar (misal: `- empty`, `- loading`, `- success`, `- failure` atau dalam format array/labels) ke seluruh manifest kapabilitas aktif dalam sesi modifikasi kode berikutnya. |
| **WARNING** | Architecture Invariant | Terdeteksi dependensi langsung transport lama (`vac-app-server`) langsung pada `vac-rs/tui/Cargo.toml`. | Aturan invariant `tui-direct-app-server-dep: warn` dipicu oleh deklarasi dependensi `vac-app-server` pada berkas `tui/Cargo.toml` baris 30-32. | Lakukan refaktorisasi bertahap untuk menghapus dependensi transport lama dari TUI dan bertransisi penuh ke *Local Runtime Contract*. |
| **INFO** | Git / Dirty Work | Terdeteksi modifikasi sangat masif (133 file berubah, ~15k baris ditambahkan) dan file baru bertipe *untracked* terkait integrasi PTY Gate & Store. | File untracked seperti `.vac/capabilities/tui-pty-gate.yaml`, `.vac/workflows/maintenance.tui-pty-gate.yaml`, `approval_store.rs`, `build_check.rs`, `donor_status.rs`, dan `registry_tests.rs` berstatus *dirty*. | Pertahankan seluruh berkas-berkas ini sebagai *dirty work* sesuai instruksi read-only, jangan di-stage, di-reset, atau di-clean. |
| **INFO** | Donor Migration | Konsistensi status migrasi donor dan matriks inventaris terjaga dengan baik tanpa adanya drift atau pelanggaran frase komit. | `scripts/check-donor-status.sh drift` dan pemeriksaan inventory/reachability sukses dengan exit status 0. | Selesaikan pembersihan Cargo edge pada `vac_session_engine` sebelum menandainya sebagai `MIGRATED`. |

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
  - `cargo test --manifest-path vac-rs/Cargo.toml -p vac-core registry_diagnostics` (unit test diagnostik yang baru saja diverifikasi **PASS** dengan sukses).
- **Heavy checks to defer**:  
  - Pengujian penuh end-to-end PTY TUI menggunakan `vac` (karena lingkungan saat ini berupa sandbox tanpa TTY buffer alternatif).

## Do-not-touch / coordination notes
- **Dirty work or potential conflicts**:  
  - Modifikasi yang ada pada berkas rencana migrasi (`docs/donor-migration/domain-plans/`) dan repositori inti `vac-rs/` **wajib dipertahankan** (jangan lakukan `git checkout --`, `git reset`, atau `git clean`).
  - Berkas baru yang masih untracked (`.vac/capabilities/tui-pty-gate.yaml` dan berkas alur kerja pendukungnya, serta file store/check baru di core) tidak boleh dihapus atau dipindahkan karena merupakan bagian dari persiapan rilis fitur `PTY validation gate`.
