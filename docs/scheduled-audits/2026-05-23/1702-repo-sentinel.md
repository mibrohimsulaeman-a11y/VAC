# Hourly Repo Sentinel Audit — 2026-05-23 17:02

## Executive summary
- **Overall status**: **Gagal Validasi (Merah/Red)**. Meskipun unit test `registry_diagnostics` lolos di memori, binary rilis asli `vac` yang baru saja dikompilasi gagal memvalidasi 14 dari 15 capability manifest karena ketidakcocokan skema (`states` field). Semua perintah `vac doctor` (kecuali `policy`) mengalami *fail-closed* dengan kode status non-nol.
- **Highest risk**: **Core registry schema mismatch yang melumpuhkan validasi lokal**. Aturan validasi baru di [capability_manifest.rs](file:///home/emp/Documents/VAC/vastar-agentic-cli/vac-rs/core/src/control_plane/capability_manifest.rs) mensyaratkan bahwa semua capability non-deprecated *harus* mendefinisikan bidang `states`. Namun, 14 berkas manifest YAML di bawah `.vac/capabilities/` saat ini belum mendefinisikannya, sehingga memblokir validasi `doctor` secara global.
- **Recommended next slice**: **Penyelarasan Skema Manifest**. Lakukan pelonggaran validasi pada skema core untuk memperbolehkan `states` bernilai kosong/opsional bagi capability bertipe `planned`/`partial`, ATAU injeksi otomatis skema `states: []` pada 14 berkas manifest tersebut.

## Findings
| Severity | Area | Finding | Evidence | Suggested action |
|---|---|---|---|---|
| **P0 (Critical)** | Control Plane | Schema mismatch melumpuhkan perintah `vac doctor` | Kegagalan kompilasi run dari `./vac-rs/target/debug/vac doctor registry .` | Relaksasi aturan pada skema parser `capability_manifest.rs` ATAU tambahkan kolom `states` kosong pada semua manifest. |
| **Info** | Control Plane | Staged capability & workflow baru untuk operator gate | `.vac/capabilities/tui-pty-gate.yaml`, `.vac/workflows/maintenance.tui-pty-gate.yaml` | Berkas ini memiliki format valid (`states` terdefinisi), namun tidak dapat diverifikasi secara utuh karena terblokir oleh kegagalan registry global. |
| **Info** | Control Plane | Modul core baru untuk persistensi approval dan build check | Untracked: `approval_store.rs`, `build_check.rs`, `donor_status.rs`, `registry_tests.rs` | Modul memiliki unit test komprehensif dan bebas TODO. Pertahankan status uncommitted untuk kelanjutan kerja. |
| **Low** | Diagnostics | Warning kompilasi `dead_code` & `unused_mut` | Warning dari cargo test: `unused import: tempfile::TempDir`, `unused_mut` di `architecture_invariants.rs` | Lakukan pembersihan import dan mutabilitas variable jika waktu pengerjaan memungkinkan. |
| **None** | Security / Policy | Desain fail-closed & pembatasan izin yang aman | Implementasi `should_bypass_approval` di [sandboxing.rs](file:///home/emp/Documents/VAC/vastar-agentic-cli/vac-rs/core/src/tools/sandboxing.rs) | Postur keamanan sangat ketat dan tidak mengizinkan bypass tanpa approval cache. |

## Plan candidates

### Deferred / blocked by stop condition
- E2E Run & Validation of `maintenance.tui-pty-gate`: deferred because current repo state violates its documented stop condition; do not force-delete dependencies or fake manual PTY proof.
## Docs sync notes
- **Paths that look stale**: Tidak ada. Hasil dari `bash scripts/check-donor-status.sh drift` dan `inventory` lolos 100% karena script pembantu ini menguji konsistensi inventaris, bukan skema runtime Rust yang ketat.
- **Paths that should be updated if code changes**:
  - `docs/donor-migration/DONOR_STATUS_BOARD.md` (jika status intake modul donor berubah).
  - `docs/donor-migration/DONOR_INVENTORY_MATRIX.md` (jika target rilis atau prioritas diubah).
  - `docs/workflow-control-plane/plans/19-root-feature-conversion.md` (jika ada pembaruan dalam root catalog).

## Validation recommendation
- **Safe quick checks**:
  - `cargo test --manifest-path vac-rs/Cargo.toml -p vac-core registry_diagnostics` (Lolos dalam ~30 detik karena berjalan di mock memori stub).
  - `bash scripts/check-donor-status.sh inventory && bash scripts/check-donor-status.sh drift` (Lolos instan dalam <1 detik).
- **Heavy checks to defer**:
  - Pengujian penuh full-workspace cargo tests dan full production build (ditunda untuk audit per-jam guna menghemat overhead).

## Do-not-touch / coordination notes
- **Dirty work or potential conflicts**:
  - **DILARANG** melakukan `git checkout`, `git reset`, atau `git clean` terhadap file untracked (`approval_store.rs`, `build_check.rs`, `donor_status.rs`, `registry_tests.rs`, dll.) karena berisi progres aktif pekerjaan dari operator/developer lain yang harus tetap dipertahankan dengan aman.
