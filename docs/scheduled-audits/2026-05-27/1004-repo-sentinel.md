# Hourly Repo Sentinel Audit — 2026-05-27 10:04
Previous run: `docs/scheduled-audits/2026-05-27/0904-repo-sentinel.md`
Carried: 1   New: 0   Dropped-as-resolved: 0

## Findings

| Severity | Area | Finding Summary | Evidence (command + exit/snippet) | Suggested Action | Origin |
|---|---|---|---|---|---|
| CRITICAL | Policy / Registry | Validasi internal `vac doctor` gagal total karena aturan `network_access` dengan ID `block-network-for-local-gates` di `.vac/policies/runtime-owner-replacement.yaml` tidak memiliki blok `network` yang valid. | `./vac-rs/target/debug/vac doctor registry .` → `error: rules[2].match.network: network policy matches require a network scope` (Exit Code 1) | Ganti `path: any` dengan struktur blok `network` yang berisi properti `host` dan `protocol` sesuai skema validasi, atau hapus matcher bermasalah tersebut jika tidak relevan. | `.vac/policies/runtime-owner-replacement.yaml` |

### Deep Finding Breakdown

#### Validasi Policy Registry Gagal (block-network-for-local-gates)
- **Root Cause Analysis (RCA)**: Aturan kebijakan `block-network-for-local-gates` ditujukan untuk memblokir akses jaringan (`action: network_access`) secara lokal. Namun, aturan tersebut menggunakan pencocokan berkas `path: any` alih-alih skema pencocokan jaringan `network`. Dalam spesifikasi kontrol plane VAC, pencocokan tindakan `network_access` mewajibkan adanya blok deklarasi `network` dengan parameter `host` dan `protocol`. Karena tidak terpenuhi, parser internal menolak seluruh file konfigurasi kebijakan saat startup.
- **Impact Radius**: Kegagalan inisialisasi parser kebijakan berakibat fatal pada seluruh subperintah diagnostik `vac doctor` (`registry`, `policy`, `surfaces`, dan `workflow`), menyebabkan perintah-perintah tersebut langsung keluar dengan kode status `1` (Exit 1). Hal ini memblokir validasi CI dan pemeriksaan keselarasan kontrol plane secara otomatis.
- **Immediate Blast Mitigation**: Operator dapat memulihkan fungsi diagnostik instan dengan mengomentari aturan `block-network-for-local-gates` di `.vac/policies/runtime-owner-replacement.yaml`, atau mengubah struktur pencocokan jaringan tersebut agar sesuai skema (menggunakan blok `network` dengan `host: "*"` dan `protocol: "*"`).

## Plan Candidates
- Title: Fix network action schema match rule in runtime-owner-replacement.yaml
  Why now: Semua perintah diagnostik `vac doctor` saat ini gagal total (exit code 1), memblokir otomatisasi validasi integrasi kontrol plane dan pemeriksaan surfaces di CI/CD.
  Files likely involved: `.vac/policies/runtime-owner-replacement.yaml`
  Verification command: `./vac-rs/target/debug/vac doctor registry .`
  Risk if skipped: Pintu gerbang integritas sistem tidak dapat dievaluasi secara otomatis, membiarkan inkonsistensi rute surfaces atau modifikasi manifest tanpa pengawasan.
