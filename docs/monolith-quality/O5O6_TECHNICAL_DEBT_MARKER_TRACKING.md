# O5/O6 Technical Debt Marker Tracking

Tanggal: 2026-06-01
Status: `SV-Done_TV-Pending`

Audit F-012 menemukan marker `TODO`/`FIXME`/`HACK`/`XXX` pada area `vac-rs/core/src` dan `vac-rs/tui/src`. Batch ini tidak menghapus marker secara asal karena sebagian adalah domain TODO yang perlu keputusan owner, tetapi menutup gap governance dengan inventory deterministik:

- `.vac/registry/technical-debt-markers.yaml`
- `scripts/check-vac-technical-debt-markers-static.sh`

Gate menghitung ulang marker non-test, jumlah, dan hash inventory. Jika marker baru muncul atau marker lama berubah, registry harus diperbarui secara sadar dengan owner/status. Ini mencegah marker baru masuk diam-diam tanpa audit trail.

Catatan: cleanup aktual tiap TODO/FIXME/HACK/XXX tetap `TV-Pending` dan harus dikerjakan per owner domain saat compiler/test feedback tersedia.
