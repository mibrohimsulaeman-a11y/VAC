# Hourly Repo Sentinel Audit — 2026-05-25 20:03
Previous run: [docs/scheduled-audits/2026-05-25/1903-repo-sentinel.md](file:///home/emp/Documents/VAC/vastar-agentic-cli/docs/scheduled-audits/2026-05-25/1903-repo-sentinel.md)
Carried: 6   New: 0   Dropped-as-resolved: 0

> [!NOTE]
> Eksekusi build/test Cargo secara penuh berstatus **SKIPPED** karena sisa kapasitas ruang disk berada di bawah batas minimum 20G (kapasitas tersedia: 19G). Pemeriksaan dilakukan menggunakan perkakas ringan `git` dan pemindaian `vac doctor`.

## Findings

| Severity | Area | Finding Summary | Evidence (command + exit/snippet) | Suggested Action | Origin |
|---|---|---|---|---|---|
| **WARNING** | Identity Check | Deteksi positif palsu (*false-positive*) istilah terlarang "duplicate TUI" pada catatan bukti rencana git status untracked post-30E. | `./vac-rs/target/debug/vac doctor workflow .` -> `identity check findings` pada `git-status-short.txt` baris 30 & 133. | Tambahkan direktori bukti rencana `docs/workflow-control-plane/plans/33-evidence/**` ke dalam pengecualian `IDENTITY_CHECK_EXEMPTIONS`. | `docs/workflow-control-plane/plans/33-evidence/baseline-2026-05-25-post-30E/git-status-short.txt` |
| **WARNING** | Identity Check | Deteksi positif palsu (*false-positive*) istilah terlarang "duplicate TUI" pada catatan bukti rencana git status untracked run 20260524. | `./vac-rs/target/debug/vac doctor workflow .` -> `identity check findings` pada `git-status-short.txt` baris 30 & 131. | Tambahkan direktori bukti rencana `docs/workflow-control-plane/plans/33-evidence/**` ke dalam pengecualian `IDENTITY_CHECK_EXEMPTIONS`. | `docs/workflow-control-plane/plans/33-evidence/runs/20260524T232817Z/git-status-short.txt` |
| **WARNING** | Registry Ownership | Domain sumber `vac-local-runtime-owner/event_stream` tidak diklaim oleh target kepemilikan kapabilitas mana pun. | `./vac-rs/target/debug/vac doctor registry .` -> `warning: ./vac-rs/local-runtime-owner/src/event_stream.rs: ... source domain is not claimed by any capability` | Deklarasikan target kepemilikan kapabilitas untuk domain sumber ini di manifest kapabilitas yang sesuai. | `vac-rs/local-runtime-owner/src/event_stream.rs` |
| **WARNING** | Registry Ownership | Pemilik rute permukaan `vac-local-runtime-owner/startup` di `palette.yaml` berbeda dengan pemilik kapabilitas `vac-rs/local-runtime-owner`. | `./vac-rs/target/debug/vac doctor registry .` -> `warning: ./.vac/surfaces/palette.yaml:routes[7].owner: surface route owner differs from capability owner` | Selaraskan pemilik rute permukaan dengan pemilik kapabilitas di manifest atau di `palette.yaml`. | `.vac/surfaces/palette.yaml` |
| **WARNING** | Registry Ownership | Pemilik rute permukaan `vac-core/local_runtime.approval` di `palette.yaml` berbeda dengan pemilik kapabilitas `vac-rs/local-runtime-owner/src/command_bus.rs`. | `./vac-rs/target/debug/vac doctor registry .` -> `warning: ./.vac/surfaces/palette.yaml:routes[8].owner: surface route owner differs from capability owner` | Selaraskan pemilik rute permukaan dengan pemilik kapabilitas di manifest atau di `palette.yaml`. | `.vac/surfaces/palette.yaml` |
| **WARNING** | Registry Ownership | Pemilik rute permukaan `vac-tui/local_runtime_session` di `palette.yaml` berbeda dengan pemilik kapabilitas `vac-rs/tui/src/app_server_session.rs`. | `./vac-rs/target/debug/vac doctor registry .` -> `warning: ./.vac/surfaces/palette.yaml:routes[15].owner: surface route owner differs from capability owner` | Selaraskan pemilik rute permukaan dengan pemilik kapabilitas di manifest atau di `palette.yaml`. | `.vac/surfaces/palette.yaml` |

### Deep Finding Breakdown

#### Finding 1 & 2: False Positive "duplicate TUI" pada catatan bukti rencana git status untracked
- **Root Cause Analysis (RCA)**: Berkas catatan bukti rencana (`git-status-short.txt`) merekam riwayat modifikasi berkas-berkas pelacak unique TUI (`maintenance.no-duplicate-tui.yaml` dan `no_duplicate_tui.rs`). Karena folder bukti rencana ini berada di bawah `docs/workflow-control-plane/plans/33-evidence/...` yang belum dicakup oleh wildcard `docs/scheduled-plans/**` pada `identity_check.rs`, pemindai mendeteksi berkas ini sebagai pelanggaran identitas.
- **Impact Radius**: Menyebabkan validasi workflow lokal mengidentifikasi positif palsu, yang meningkatkan kebisingan laporan integrasi.
- **Immediate Blast Mitigation**: Abaikan sementara alarm palsu ini, atau daftarkan direktori `docs/workflow-control-plane/plans/33-evidence/**` ke dalam daftar pengecualian scanner di `identity_check.rs`.

#### Finding 3: Domain sumber `event_stream.rs` tidak diklaim oleh target kepemilikan kapabilitas
- **Root Cause Analysis (RCA)**: Berkas implementasi baru `event_stream.rs` telah ditambahkan di bawah crate `vac-local-runtime-owner`, namun belum terdaftar di dalam metadata kepemilikan (ownership metadata) dari manifest kapabilitas mana pun dalam `.vac/capabilities/`.
- **Impact Radius**: Merusak visualisasi cakupan kepemilikan modul di dashboard kapabilitas operator dan memicu warning validasi registri.
- **Immediate Blast Mitigation**: Daftarkan domain modul `vac-local-runtime-owner/event_stream` ke dalam bagian `ownership.modules` pada manifest kapabilitas `.vac/capabilities/ownership.yaml` (atau manifest terkait).

#### Finding 4, 5 & 6: Ketidaksesuaian pemilik rute permukaan dengan pemilik kapabilitas di palette.yaml
- **Root Cause Analysis (RCA)**: Terjadi perbedaan penamaan antara `owner` rute permukaan yang dideklarasikan secara lokal di `.vac/surfaces/palette.yaml` (misalnya `vac-local-runtime-owner/startup`, `vac-core/local_runtime.approval`, dan `vac-tui/local_runtime_session`) dengan pemilik fungsional kapabilitas di manifest `.vac/capabilities/` yang bersangkutan.
- **Impact Radius**: Inkonsistensi data manifest menyebabkan visualisasi kepemilikan rute palette menyimpang dari visualisasi domain implementasi.
- **Immediate Blast Mitigation**: Lakukan sinkronisasi penamaan owner di `palette.yaml` agar persis sesuai dengan deklarasi owner di masing-masing manifest kapabilitas terkait.

## Plan Candidates

- Title: Sinkronisasi Kepemilikan Rute Palette dan Deklarasi Domain Event Stream
  - Why now: Mengeliminasi warning kepemilikan (ownership) registri untuk menjaga kepatuhan struktural 100% pada pindaian `vac doctor registry`.
  - Files likely involved: [palette.yaml](file:///.vac/surfaces/palette.yaml), [ownership.yaml](file:///.vac/capabilities/ownership.yaml)
  - Verification command: `./vac-rs/target/debug/vac doctor registry .` (Full `cargo test` SKIPPED)
  - Risk if skipped: Terjadi distorsi visual pada dashboard kepemilikan kode dan peningkatan warning berkala pada pemeriksaan registri.

- Title: Pengecualian Folder Bukti Rencana dari Pemindaian Identity Check
  - Why now: Menghilangkan kebisingan alarm palsu baru (*false-positive cascade*) yang dipicu oleh pencatatan riwayat status git di direktori bukti rencana.
  - Files likely involved: [identity_check.rs](file:///vac-rs/core/src/control_plane/identity_check.rs)
  - Verification command: `./vac-rs/target/debug/vac doctor workflow .` (Full `cargo test` SKIPPED)
  - Risk if skipped: Validasi lokal dan CI workflow akan terus-menerus memicu status peringatan (WARNING) yang mengaburkan status kebersihan repositori yang sesungguhnya.

## Docs Sync Tracking

- Path: [git-status-short.txt](file:///docs/workflow-control-plane/plans/33-evidence/baseline-2026-05-25-post-30E/git-status-short.txt)
  - Code change detail: Perekaman status git uncommitted yang mereferensikan file unique TUI (`maintenance.no-duplicate-tui.yaml` dan `no_duplicate_tui.rs`).
  - Current stale claim in doc: Tidak ada klaim usang yang spesifik karena file ini hanya bertindak sebagai catatan bukti integrasi runtime lokal yang untracked.
  - Command/Diff proving drift: `./vac-rs/target/debug/vac doctor workflow .` membuktikan file ini memicu temuan warning.
