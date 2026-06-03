# Hourly Repo Sentinel Audit — 2026-05-23 17:12

## Executive summary
- **Overall status**: **FAIL (BLOCKED)**  
  Sistem kontrol dan validasi internal repositori (`vac doctor`) mengalami kegagalan akibat perubahan struktural terbaru pada kode `vac-core` yang mewajibkan penulisan field `states` pada semua manifest kapabilitas non-deprecated.
- **Highest risk**: **Registry Validation Breakdown**  
  Seluruh rangkaian integrasi dan pemeriksaan kontrol (`doctor`) tidak dapat berjalan karena registry gagal memuat 14 file kapabilitas di bawah `.vac/capabilities/`.
- **Recommended next slice**: **Manifest States Restoration & Sync**  
  Menambahkan deklarasi `states` standar pada semua berkas YAML kapabilitas di `.vac/capabilities/` guna memulihkan fungsi diagnostik repositori secara penuh tanpa melanggar prinsip *Preserve Unrelated Dirty Work*.

## Findings
| Severity | Area | Finding | Evidence | Suggested action |
|---|---|---|---|---|
| **CRITICAL** | Registry / Control Plane | `vac doctor registry` gagal total karena mewajibkan deklarasi `states` pada manifest kapabilitas non-deprecated. | Pesan kesalahan pada 14 file `.yaml` di `.vac/capabilities/`: `non-deprecated capabilities must declare states` dari core validator. | Tambahkan definisi `states` standar (misal: `- empty`, `- loading`, `- success`, `- failure`) ke seluruh kapabilitas aktif dalam sesi berikutnya yang mengizinkan penulisan berkas proyek. |
| **WARNING** | Architecture Invariant | Terdeteksi dependensi transport lama (`vac-app-server`) langsung pada `vac-rs/tui/Cargo.toml`. | `tui-direct-app-server-dep: warn` didefinisikan pada `architecture_invariants.rs:888` dan dipicu oleh baris 30-32 di `tui/Cargo.toml`. | Lakukan refaktorisasi bertahap untuk menghapus dependensi langsung `vac-app-server` dari TUI, beralih penuh ke *Local Runtime Contract*. |
| **INFO** | Git / Dirty Work | Berkas kapabilitas dan alur kerja baru untuk `tui-pty-gate` berstatus *untracked* (dirty). | `.vac/capabilities/tui-pty-gate.yaml` & `.vac/workflows/maintenance.tui-pty-gate.yaml` ada di status *untracked*. | Pertahankan berkas-berkas ini sebagai *dirty work* sesuai instruksi read-only, jangan di-stage atau di-commit secara tidak sengaja. |
| **INFO** | Donor Migration | Deteksi 1 item `MIGRATED` terdaftar yang memerlukan peninjauan manual end-to-end UX. | Laporan `./scripts/check-donor-status.sh evidence` mendeteksi item migrated, sedangkan `vac_session_engine` saat ini berstatus `ADAPT_IN_PROGRESS` di papan status. | Lakukan verifikasi end-to-end UX manual pada sistem sesi sesuai *Production-ready rule* sebelum menandainya sebagai `MIGRATED` penuh. |

## Plan candidates

### Deferred / blocked by stop condition
- TUI Dependency Decoupling (App-Server): deferred because current repo state violates its documented stop condition; do not force-delete dependencies or fake manual PTY proof.
## Docs sync notes
- **Paths that look stale**:  
  - `docs/donor-migration/DONOR_STATUS_BOARD.md` (Catatan pada `vac_session_engine` menyebutkan penangguhan akibat Cargo edge, perlu diselaraskan dengan hasil audit ketergantungan TUI).
- **Paths that should be updated if code changes**:  
  - `docs/product/CAPABILITY_MAP.md` dan `docs/product/CAPABILITY_PRD_COVERAGE.md` apabila kapabilitas baru ditambahkan atau dipromosikan statusnya.

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
  - Berkas baru yang masih untracked (`.vac/capabilities/tui-pty-gate.yaml` dan berkas alur kerja pendukungnya) tidak boleh dihapus atau dipindahkan karena merupakan bagian dari persiapan rilis fitur `PTY validation gate`.
