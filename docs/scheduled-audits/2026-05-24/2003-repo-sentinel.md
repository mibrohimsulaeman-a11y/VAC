# Hourly Repo Sentinel Audit — 2026-05-24 20:03
Previous run: docs/scheduled-audits/2026-05-24/1904-repo-sentinel.md
Carried: 2   New: 1   Dropped-as-resolved: 0

## Findings

| Severity | Area | Finding Summary | Evidence (command + exit/snippet) | Suggested Action | Origin |
|---|---|---|---|---|---|
| **CRITICAL** | Build | Kompilasi crate `vac-local-runtime-owner` gagal akibat ketidaksesuaian trait bound pada `SteerInputError` di `command_bus.rs` saat menggunakan `#[from]`. | `cargo check --manifest-path vac-rs/Cargo.toml -p vac-surface-cli` -> exit 101 / `error[E0599]: the method as_dyn_error exists for reference &SteerInputError, but its trait bounds were not satisfied` | Lepaskan anotasi `#[from]` pada baris `SteerInput(#[from] SteerInputError)` di `RuntimeCommandBusError` dalam berkas `vac-rs/local-runtime-owner/src/command_bus.rs`, lalu lakukan pemetaan manual menggunakan `.map_err(RuntimeCommandBusError::SteerInput)` pada fungsi yang mengembalikan `SteerInputError`. | `vac-rs/local-runtime-owner/src/command_bus.rs:247:18` |
| **WARNING** | Code Quality | Penambahan marker TODO baru (`TODO(local-runtime-owner)`) pada berkas `vac-rs/tui/src/local_runtime_session.rs` di working tree. | `git diff` -> `+    // TODO(local-runtime-owner): these accessors are test/future bridge helpers` | Pastikan marker TODO ini didokumentasikan atau diselesaikan sebelum penggabungan kode, atau dipindahkan ke pelacakan isu jika memerlukan waktu pengerjaan lebih lama. | `vac-rs/tui/src/local_runtime_session.rs:166` |
| **INFO** | Git / Active Work | Terdeteksi modifikasi aktif masif di working tree. | `git status --short` -> mendeteksi lebih dari 150 berkas hasil modifikasi/hapus dan berkas untracked krusial seperti `approval_store.rs`. | **DILARANG KERAS** menjalankan `git reset --hard` atau `git clean -fd` demi menjaga integritas progres kerja aktif. | carried from 12:03 |

### Deep Finding Breakdown

#### CRITICAL: Kompilasi crate `vac-local-runtime-owner` gagal
- **Root Cause Analysis (RCA)**: `SteerInputError` yang didefinisikan di `vac-rs/core/src/session/mod.rs` adalah sebuah enum biasa yang tidak mengimplementasikan trait `std::error::Error`. Pada `vac-rs/local-runtime-owner/src/command_bus.rs`, tipe error `RuntimeCommandBusError` menggunakan macro `thiserror` lewat anotasi `#[from]`. Macro `#[from]` ini secara otomatis memanggil fungsi pembantu `as_dyn_error` yang memerlukan implementasi trait `std::error::Error` pada tipe asal. Karena trait tersebut tidak terpenuhi, proses kompilasi gagal total dengan `exit code 101`.
- **Impact Radius**: Kegagalan kompilasi ini meluas ke crate `vac-local-runtime-owner`, binari CLI utama `vac` (karena bergantung pada crate tersebut), serta pengujian unit/integrasi yang terkait dengan local runtime. Hal ini memblokir integrasi Plan 30 (Prompt submit & active controls cutover).
- **Immediate Blast Mitigation**: Lepaskan anotasi `#[from]` dari baris `SteerInput(#[from] SteerInputError)` di `RuntimeCommandBusError` pada berkas `vac-rs/local-runtime-owner/src/command_bus.rs`, kemudian ubah penggunaan operator `?` pada pemanggilan yang mengembalikan `SteerInputError` dengan pemetaan manual menggunakan `.map_err(RuntimeCommandBusError::SteerInput)`.

#### WARNING: Penambahan marker TODO baru di `local_runtime_session.rs`
- **Root Cause Analysis (RCA)**: Selama implementasi fungsionalitas local runtime session, developer menambahkan pembantu akses sementara untuk pengujian masa depan dengan menyisipkan marker `TODO(local-runtime-owner)`. Penambahan marker ini tanpa pelacakan formal berisiko meninggalkannya sebagai hutang teknis di kemudian hari.
- **Impact Radius**: Berdampak pada kualitas kode modul TUI (`vac-rs/tui`), khususnya komponen penanganan sesi local runtime (`local_runtime_session.rs`).
- **Immediate Blast Mitigation**: Dokumentasikan detail pengerjaan helper ini dalam tugas atau isu Plan 00F, serta jadwalkan pembersihannya saat adapter legacy app-server sepenuhnya dipangkas.

#### INFO: Terdeteksi modifikasi aktif masif di working tree
- **Root Cause Analysis (RCA)**: Adanya banyak berkas termodifikasi, dihapus, dan untracked di working tree dikarenakan proses porting backend modular dari kode donor secara paralel dan pembersihan app-server transport legacy (Plan 00F/27/29) yang belum diselesaikan dan belum di-commit.
- **Impact Radius**: Memengaruhi hampir seluruh modul TUI (`vac-rs/tui`), runtime local owner (`vac-rs/local-runtime-owner`), serta dokumen rencana migrasi donor (`docs/donor-migration/`).
- **Immediate Blast Mitigation**: Hindari penggunaan perintah destruktif seperti `git reset --hard` atau `git clean -fd` yang dapat menghancurkan progres pengerjaan migrasi aktif yang sedang berjalan.

## Plan Candidates
- Title: Fix RuntimeCommandBusError trait bound for SteerInputError
  - Why now: Memulihkan kemampuan kompilasi seluruh workspace dan kelanjutan implementasi Plan 30.
  - Files likely involved: [command_bus.rs](file:///home/emp/Documents/VAC/vastar-agentic-cli/vac-rs/local-runtime-owner/src/command_bus.rs)
  - Verification command: `cargo check --manifest-path vac-rs/Cargo.toml -p vac-local-runtime-owner`
  - Risk if skipped: Seluruh developer dan proses pengujian terhambat akibat kegagalan kompilasi total pada `vac-local-runtime-owner` dan `vac-cli`.
- Title: Resolve local_runtime_session helper accessors
  - Why now: Menghilangkan hutang teknis baru (marker TODO) sebelum melangkah ke pembersihan total adapter legacy.
  - Files likely involved: [local_runtime_session.rs](file:///home/emp/Documents/VAC/vastar-agentic-cli/vac-rs/tui/src/local_runtime_session.rs)
  - Verification command: `cargo check --manifest-path vac-rs/Cargo.toml -p vac-surface-tui`
  - Risk if skipped: Kode helper sementara mengendap menjadi dead code atau ketidakonsistenan akses antarmuka sesi.

## Docs Sync Tracking
- Path: docs/donor-migration/domain-plans/INDEX.md
  - Code change detail: Penyesuaian domain plans 01-10 dengan Verdict `NEEDS HARDENING` dan penetapan target penyelesaian yang lebih ketat.
  - Current stale claim in doc: Dokumen mencantumkan status `ACTIVE CONTRACT ADDED` namun tidak sepenuhnya menyinkronkan status drift inventori modular dari `scripts/check-donor-status.sh`.
  - Command/Diff proving drift: `git diff docs/donor-migration/domain-plans/INDEX.md` menunjukkan sinkronisasi manual baru untuk memperjelas status ketergantungan prerequisite.
