# O5/O6 Standard Cargo Hygiene Gate

Tanggal: 2026-06-01
Status: `Registered_NotEvaluated`

Audit F-019 menemukan bahwa workflow `.vac` belum membuktikan wiring standar untuk `cargo fmt --check`, `cargo clippy -D warnings`, `cargo deny check`, dan `cargo audit`, walaupun konfigurasi `deny.toml` dan `.cargo/audit.toml` sudah ada.

Perubahan batch ini mendaftarkan command terstruktur pada `.vac/workflows/maintenance.build-check.yaml`:

- `cargo +1.93.0 fmt --all --check`
- `cargo +1.93.0 clippy --workspace --all-targets -- -D warnings`
- `cargo +1.93.0 deny check`
- `cargo +1.93.0 audit`

Catatan kejujuran: command tersebut **belum dijalankan** di sandbox ini karena `cargo`/`rustc` tidak tersedia. Gate statis hanya memastikan command sudah masuk workflow dengan bentuk terstruktur agar runtime VAC bisa mengeksekusinya saat toolchain tersedia.
