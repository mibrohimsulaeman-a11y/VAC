# VAC — O5 Monolith/Refactor + O6 Quality De-risk — Implementation Plan

**Baseline:** `vac-source-scanner-hardening-o1-o4-s5-s8.zip` (state terakhir, audited 2026-05-30).
**Mode:** `bounded_worker` per slice; setiap slice = Semantic Plan + gate + evidence (bukan script bebas).
**Prinsip honesty:** klaim build/E2E hanya `Pass` jika benar-benar dieksekusi; selain itu **`NotEvaluated`**, bukan Pass.

> **Prasyarat lintas-semua-slice (dari audit E2E):** repo BELUM bisa build di lingkungan ini — `vendor/` ABSENT, `.cargo/config.toml` memaksa `replace-with=vendored-sources`, offline, `cargo`/`rustc` tidak tersedia. **Tidak satu pun slice O5/O6 boleh diklaim selesai (`Pass`) sebelum `cargo build`+`cargo test` hijau secara nyata.** Sampai itu, status setiap slice = `Implemented (NotEvaluated)`.

---

## 0. Grounding snapshot (angka real dari state terakhir)

| Metrik | Nilai | Sumber |
|---|---|---|
| Workspace members | **113** | `vac-rs/Cargo.toml` |
| God-file terbesar (runtime) | `tui/chatwidget.rs` **453KB**, `app-server/vac_message_processor.rs` **422KB**, `tui/bottom_pane/chat_composer.rs` **405KB** | find by-bytes |
| Crate kembar | `app-server-protocol` ≈ `runtime-protocol` (**~16 file identik**, incl `protocol/v2.rs` 397KB ×2) | md5 dedup |
| Boilerplate dup | `tests/all.rs` **6 copies** identik | md5 dedup |
| Donor crates | `donor/vac/crates/*` (vac_apply_patch, vac_approvals, vac_bridge, vac_changeset, vac_cli, vac_core, vac_ingest, vac_manifest, …) | find |
| unwrap() (non-test, upper-bound) | **~1929** | grep |
| expect() (non-test, upper-bound) | **~8463** | grep |
| panic!() (non-test) | **~809** | grep |
| unreachable!/todo!/unimplemented! | **~88** | grep |
| `unsafe ` occurrences | **~609** (FFI/sandbox: linux-sandbox, pty, arg0, shell) | grep |
| Docs `.md` | **645** total / **420** di `docs/` | find |
| Tests | **8513** `#[test]` / **271** integration files | grep/find |

> **Caveat metrik de-panic:** filter hanya berbasis path (`/tests/`, `*_test.rs`), BELUM mengecualikan modul `#[cfg(test)]` inline. Jadi 1929/8463/809 adalah **upper-bound** (campur runtime + inline test). Angka runtime-sejati harus difinalkan via `cargo`-driven lint (clippy + AST), bukan grep. Plan ini memakai grep sebagai triase awal, bukan klaim final.

---

## 1. Sequencing rationale

1. **O5 sebelum O6 untuk dedup/consolidate, tapi de-panic O6.1 boleh paralel** — merapikan struktur dulu mengurangi permukaan yang harus di-de-panic (hindari de-panic kode yang akan dihapus).
2. **Dedup (O5.1) PALING DULU** — menghapus crate kembar `*-protocol` memangkas ~16 file & ratusan KB sebelum split god-file, supaya tidak men-split file yang ternyata duplikat.
3. **Delete donor (O5.5) PALING AKHIR di O5** — donor masih jadi referensi/quarantine; hapus hanya setelah scanner mengonfirmasi tak ada `donor_quarantined reachable from product`.
4. **Build hijau adalah gate antar-slice**, bukan akhir — setiap slice harus lulus `cargo build` lokal sebelum lanjut (begitu vendor/toolchain tersedia).

Urutan final: **O5.1 → O5.2 → O5.3 → O5.4 → (O6.1 ∥) → O5.5 → O6.2 → O6.3 → O6.4**.

---

## O5 — Monolith / Refactor

### O5.1 — Dedup (hapus crate & file kembar)

**Target real:**
- `app-server-protocol` vs `runtime-protocol`: **identik byte-for-byte** pada `lib.rs`, `export.rs`, `jsonrpc_lite.rs`, `experimental_api.rs`, `schema_fixtures.rs`, dan seluruh `protocol/{v1,v2,mod,common,mappers,event_mapping,item_builders,serde_helpers,thread_history,common_tests}.rs`. `protocol/v2.rs` saja **397KB ×2**.
- `tests/all.rs` **6 salinan identik** (app-server, apply-patch, chatgpt, exec, linux-sandbox, login) — kemungkinan harness trybuild.

**Action:**
1. Tetapkan **satu** crate kanonik (rekomendasi: `runtime-protocol`), jadikan `app-server-protocol` sebagai **re-export tipis** (`pub use runtime_protocol::*;`) ATAU hapus total + update dependents.
2. Petakan semua dependent (`grep -rl app_server_protocol vac-rs/*/src`) → arahkan ke crate kanonik.
3. Untuk `tests/all.rs`: ekstrak ke satu `dev-dependency` helper macro/crate, atau biarkan (low value) — dokumentasikan keputusan.

**Allowed files:** `vac-rs/app-server-protocol/**`, `vac-rs/runtime-protocol/**`, `vac-rs/Cargo.toml`, dependents yang meng-import.

**Acceptance:**
- `app-server-protocol` tidak lagi menyimpan salinan `protocol/*.rs` (re-export atau hapus).
- Tidak ada pasangan file `.rs` identik md5 lintas dua crate `*-protocol`.
- `cargo build` hijau (NotEvaluated sampai toolchain ada).

**Risk:** dua crate mungkin sengaja dipisah untuk API-surface berbeda (v1 vs v2 wire). **Verifikasi divergence sebelum merge**; jika benar beda secara semantik, pertahankan tapi ekstrak `protocol-core` bersama.

---

### O5.2 — Split god-files

**Target real (runtime, >130KB):**

| File | Size | Rencana split |
|---|---|---|
| `tui/src/chatwidget.rs` | 453KB | per-region: render / input / state / event-handlers / layout submodul |
| `app-server/src/vac_message_processor.rs` | 422KB | per message-kind handler module + dispatch tabel |
| `tui/src/bottom_pane/chat_composer.rs` | 405KB | editing / history / completion / keymap |
| `core/src/control_plane/workflow_runner.rs` | 234KB | step-exec / state-machine / evidence / gating |
| `protocol/src/protocol.rs` | 191KB | per domain enum group |
| `tui/src/legacy_app_server_session.rs` | 189KB | kandidat **delete** (prefix `legacy_`) — cek dead-code dulu |
| `tui/src/history_cell.rs` | 187KB | per cell-type |
| `state/src/runtime/memories.rs` | 170KB | store / query / eviction |
| `app-server/src/bespoke_event_handling.rs` | 143KB | per event family |
| `core/src/config/mod.rs` | 132KB | schema / load / merge / defaults |

> Test god-files (`session/tests.rs` 302KB, `config_tests.rs` 285KB, `app/tests.rs` 186KB) di-split di **O6.4**, bukan di sini.

**Action:** split **murni mekanis** (pindah item ke submodul, `mod` + `pub use` re-export agar API path tetap) — **zero behaviour change**. Satu god-file = satu slice/commit untuk diff yang reviewable.

**Acceptance:**
- Tidak ada file runtime `.rs` > ~80KB (kecuali yang justified & didokumentasikan).
- Public API path tidak berubah (re-export menjaga kompatibilitas) — `cargo build` dependents hijau.
- Diff per file murni move (verif: simbol set sebelum=sesudah).

**Risk:** split mekanis bisa memutus visibilitas privat antar-item; mitigasi: `pub(crate)` re-export internal.

---

### O5.3 — Merge utils

**Target real:** crate util tersebar — `utils`, `async-utils`, `git-utils` (+ helper di `core`).

**Action:**
1. Inventaris simbol publik tiap util-crate; deteksi overlap (mis. path/string/time helper).
2. Konsolidasi ke satu `vac-utils` dengan submodul (`async`, `git`, `fs`, `text`), util-crate lama → re-export tipis (deprecation window) lalu hapus.
3. Jangan gabung `core`/`protocol` ke utils (itu domain, bukan util).

**Acceptance:** satu crate util kanonik; tidak ada fungsi helper duplikat lintas-crate; dependents ter-update; build hijau.

**Risk:** dependency cycle bila utils menarik domain types — jaga `vac-utils` bebas dependency domain.

---

### O5.4 — Consolidate workspace (113 → lebih ramping)

**Action:**
1. `cargo tree`/dependency graph (saat toolchain ada) → identifikasi crate dengan 1 dependent & <~500 LOC → kandidat inline ke parent.
2. Kelompokkan ke layer jelas: `protocol`, `core`, `control-plane`, `app-server`, `tui`, `cli`, `utils`, `sandbox/exec`.
3. Gabung micro-crate; pertahankan batas crate yang punya alasan kompilasi/keamanan (mis. `linux-sandbox`, `pty` tetap terpisah).

**Acceptance:** jumlah member turun dengan rasionalisasi terdokumentasi; tidak ada crate "1 file wrapper"; build + test hijau.

**Risk:** menggabung crate yang punya `unsafe`/FFI boundary bisa memperluas blast-radius — **jangan gabung crate sandbox/FFI** (lihat O6.2).

---

### O5.5 — Delete donor (paling akhir)

**Target real:** `donor/vac/crates/*` (vac_apply_patch, vac_approvals, vac_bridge, vac_changeset, vac_cli, vac_core, vac_ingest, vac_manifest, …).

**Prasyarat (hard):**
- Scanner (`donor_quarantined`/`donor_reference`) mengonfirmasi **tidak ada donor reachable dari product runtime** (cek `by-scope/donor_*` + ownership).
- Setiap baris donor `MIGRATED` punya root replacement yang sudah lulus build/test.

**Action:** untuk tiap crate donor: konfirmasi status di `donor-inventory.yaml` → jika `MIGRATED` & replacement hijau → hapus dir + entri workspace + update `DONOR_STATUS_BOARD.md`. **Pertahankan attribution** di `NOTICE`/`THIRD_PARTY.md` (kepatuhan lisensi — JANGAN hapus header MIT/BSD pihak ketiga).

**Acceptance:**
- `donor/` kosong atau hanya berisi baris non-MIGRATED yang sengaja di-defer (terdokumentasi).
- Tidak ada referensi `donor/` dari product runtime.
- Atribusi tetap ada di satu `.md` (Apache-2.0 §4).
- Build + test hijau tanpa donor.

**Risk:** menghapus donor yang masih reachable → build break / kehilangan fungsi. Mitigasi: gate scanner reachability WAJIB hijau dulu.

---

## O6 — Quality De-risk

### O6.1 — De-panic (boleh paralel dengan O5.2+)

**Target real (upper-bound, perlu konfirmasi cargo-driven):** unwrap ~1929, expect ~8463, panic! ~809, unreachable/todo/unimplemented ~88 (non-test by path).

**Action:**
1. **Finalkan angka runtime-sejati** via clippy lint (`unwrap_used`, `expect_used`, `panic`) `--workspace` dengan modul `#[cfg(test)]` benar-benar dikecualikan — gantikan grep upper-bound.
2. Prioritas: **product runtime path** (core/control_plane, app-server, cli) dulu; TUI render-path kedua.
3. Pola perbaikan: `unwrap()` → `?` + error context (`anyhow`/`thiserror`); `panic!` di jalur recoverable → `Result`; `unreachable!` → audit apakah benar tak-terjangkau, kalau tidak jadikan error.
4. `todo!/unimplemented!` (~88) → tiap satu: implement atau ganti error eksplisit + tracking issue.

**Acceptance:**
- 0 `unwrap()/expect()/panic!` baru di runtime (clippy deny lulus).
- `todo!/unimplemented!` runtime = 0 (atau terdaftar eksplisit sebagai gated-feature error).
- Build + clippy + test hijau.

**Risk:** konversi masal bisa menelan error penting jadi diam; mitigasi: tiap konversi WAJIB bawa context, jangan `.ok()` tanpa log.

---

### O6.2 — Unsafe audit

**Target real:** ~609 `unsafe ` occurrences; konsentrasi FFI/sandbox: `linux-sandbox`, `utils/pty`, `arg0`, `core/shell*`, `config/loader/macos.rs`, `code-mode/module_loader.rs`.

**Action:**
1. Inventaris tiap blok `unsafe` (kecualikan yang murni di test).
2. Tiap blok WAJIB punya **`// SAFETY:` comment** yang membuktikan invariannya; tambahkan bila hilang.
3. Mana yang bisa di-safe-kan (mis. ganti dengan crate aman) → refactor; sisanya isolasi di modul `ffi`/`sys` minimal.
4. Aktifkan `#![deny(unsafe_op_in_unsafe_fn)]` + (opsional) `cargo geiger` untuk laporan.

**Acceptance:**
- 100% blok `unsafe` runtime punya `// SAFETY:` justification.
- Tidak ada `unsafe` di luar crate FFI/sandbox yang ter-whitelist (atau terdokumentasi alasannya).
- Build + test hijau; (opsional) laporan geiger = NotEvaluated bila tool tak tersedia.

**Risk:** FFI sandbox adalah keamanan-kritis — perubahan di `linux-sandbox`/`pty` butuh review ekstra + test platform.

---

### O6.3 — Docs pruning

**Target real:** 645 `.md` total, 420 di `docs/`.

**Action:**
1. Klasifikasi `docs/`: `current` / `historical` / `donor-migration` / `legal` / `redundant`.
2. Arsipkan historical ke `docs/_archive/` (atau hapus bila tergantikan); merge dokumen overlap.
3. Pastikan dokumen sumber-kebenaran tunik per topik; perbaiki link mati (terutama referensi ke file yang dihapus di O5).
4. **JANGAN hapus** `NOTICE`/`THIRD_PARTY`/`LICENSE` & attribution (kepatuhan O5.5).

**Acceptance:**
- Jumlah `.md` aktif turun dengan jejak arsip (bukan hard-delete diam).
- 0 link mati ke path yang dihapus O5.
- Atribusi/lisensi utuh.

**Risk:** menghapus doc yang dirujuk gate (`check-docs-state-refresh.sh`) → gate merah; sinkronkan dengan gate docs.

---

### O6.4 — E2E expansion

**Target real:** 8513 `#[test]` / 271 integration file sudah ada — basis kuat, tapi **belum terbukti hijau** (tak bisa run offline tanpa vendor/toolchain).

**Action:**
1. **Pertama: buat suite bisa dijalankan** — regenerate `vendor/` atau sediakan toolchain (ini juga prasyarat klaim E2E di audit). Tanpa ini semua di bawah = NotEvaluated.
2. Split test god-files (`session/tests.rs` 302KB, `config_tests.rs` 285KB, `app/tests.rs` 186KB) per-fitur agar maintainable.
3. Tambah E2E untuk jalur baru: **`vac init` lifecycle** (scan→discovered, rescan-ast→policy_inferred, scan_failed, resume idempotent) + **scanner doctor gate exit 0/1** (saat ini gate crash exit 127 karena `rustc` absent — perbaiki agar robust melaporkan NotEvaluated, lalu uji exit code-nya).
4. Tambah regression test untuk dedup O5.1 (API path crate kanonik) & god-file split O5.2 (API surface stabil).

**Acceptance:**
- `cargo test --workspace` hijau **secara nyata** (bukan NotEvaluated) — ini titik di mana klaim "siap E2E" baru sah.
- Gate scanner exit 0 bersih (bukan 127) dan exit 1 pada kondisi gagal yang didefinisikan.
- Coverage jalur `vac init` lifecycle + policy fail-closed ada.

**Risk:** tergoda klaim Pass tanpa run → DILARANG; selama vendor/toolchain absent, tandai **NotEvaluated**.

---

## Validation gates (urutan)

```
bash scripts/check-docs-state-refresh.sh
bash scripts/check-plan-codebase-reconciliation.sh
bash scripts/check-vac-init-registry-strictness-contract.sh
bash scripts/check-vac-workflow-spec-compliance.sh
bash scripts/check-no-hardcoded-readiness-scoreboard.sh
bash scripts/check-tui-source-artifact-hygiene.sh
bash scripts/check-vac-init-scanner-hardening-spec-flow.sh   # perbaiki dulu: jangan exit 127 saat rustc absent
```

Gate berbasis Cargo (HANYA bila vendor/toolchain siap; selain itu **NotEvaluated**, bukan Pass):

```
cargo build --workspace --offline
cargo clippy --workspace --offline -- -D warnings
cargo test --workspace --offline
```

---

## Definition of Done

O5+O6 selesai HANYA bila:
1. Tidak ada file `.rs` identik lintas-crate (`*-protocol` dedup beres).
2. Tidak ada god-file runtime > ~80KB tanpa justifikasi.
3. Util terkonsolidasi; tidak ada helper duplikat.
4. Workspace member dirasionalisasi & terdokumentasi.
5. Donor MIGRATED dihapus; attribution tetap ada; tidak ada donor reachable dari product.
6. 0 unwrap/expect/panic/todo runtime baru (clippy deny lulus).
7. 100% `unsafe` runtime punya `// SAFETY:`.
8. Docs dipangkas, 0 link mati, lisensi utuh.
9. **`cargo build`+`clippy`+`test --workspace` HIJAU secara nyata** — tanpa ini, status = `Implemented (NotEvaluated)`, BUKAN done.
10. Tidak ada klaim readiness/E2E yang di-hardcode tanpa eksekusi.

---

## Recommended order

```
O5.1 dedup *-protocol & tests/all.rs       (highest ROI, kurangi ~16 file dup)
O5.2 split 10 god-files (1 file/slice)     (mekanis, zero-behaviour)
O5.3 merge utils -> vac-utils
O5.4 consolidate workspace 113 -> ramping
O6.1 de-panic (paralel sejak O5.2)         (finalkan angka via clippy, bukan grep)
O5.5 delete donor (gate reachability dulu)
O6.2 unsafe audit (// SAFETY: + isolasi FFI)
O6.3 docs pruning (sinkron gate docs)
O6.4 E2E expansion (regen vendor/toolchain -> run nyata)
```

**Catatan penutup:** semua angka de-panic/unsafe di sini adalah triase grep (upper-bound). Langkah pertama O6.1/O6.2 yang sebenarnya adalah **mengganti triase grep dengan analisis cargo/clippy** begitu toolchain tersedia — supaya target yang dikejar adalah runtime-sejati, bukan campuran inline-test.
