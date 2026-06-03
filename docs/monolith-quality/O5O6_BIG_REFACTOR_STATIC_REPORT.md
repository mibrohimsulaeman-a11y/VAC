# O5/O6 Big Refactor Static Report

Tanggal: 2026-06-01
Status: `SV-Done_TV-Pending`

## Ringkasan

Batch ini mengimplementasikan bagian besar audit arsitektur yang aman dikerjakan tanpa Rust toolchain:

- retire `vac-chatgpt` cloud-task crate;
- retire `backend-client` + `vac-backend-openapi-models` backend OpenAPI island;
- introduce `vac-control-plane` crate boundary;
- introduce `vac-provider-http` transport seam;
- remove active CLI/TUI ChatGPT account sign-in selection path;
- move local connector list helper to `vac_core::connectors`;
- convert TUI frame request channel to bounded/coalescing;
- add `scripts/check-vac-o5o6-big-refactor-static.sh` and wire it into O5/O6 aggregate gates.

## Root Cause

Audit menemukan mismatch antara `.vac` sebagai control-plane manifest yang sudah matang dan topologi crate yang masih flat/transport-centric. Ada juga residual cloud-task/backend island yang menambah compile surface untuk local coding agent.

## Dampak Perubahan

- Workspace graph lebih kecil: retired cloud-task/backend OpenAPI crates tidak lagi menjadi member/dependency.
- Compatibility surface tetap eksplisit: `vac apply` fail-closed, bukan diam-diam memanggil backend cloud.
- `.vac` control plane punya crate boundary awal (`vac-control-plane`) tanpa memindahkan file fisik massal yang bisa merusak static gates lama.
- Provider HTTP generik punya seam (`vac-provider-http`) untuk split `vac-api` bertahap.
- Hot-path redraw TUI tidak lagi memakai unbounded channel pada `frame_requester.rs`.

## Static Gate

Gate baru memeriksa:

- retired crate path absent;
- no active manifest/source refs ke `vac-chatgpt` / `vac-backend-client` / backend OpenAPI island;
- `Cargo.lock` tidak membawa package/dependency retired dan sudah punya `vac-control-plane` + `vac-provider-http`;
- `vac-core` re-export control plane dari crate baru;
- connector helper pindah ke `vac_core::connectors`;
- CLI apply cloud-task retrieval fail-closed;
- memories write backend-client rate-limit fetch removed;
- CLI/TUI account sign-in removed dari active selection path;
- bounded frame request channel;
- registry/evidence/trajectory/report files hadir dan mencatat TV-Pending.

## Batasan Jujur

`SV-Done` di sini berarti source/static contract selesai. `TV-Pending` tetap berlaku untuk:

- `cargo check --manifest-path vac-rs/Cargo.toml -p vac-surface-tui`;
- `cargo check --manifest-path vac-rs/Cargo.toml -p vac-surface-cli`;
- `cargo check --manifest-path vac-rs/Cargo.toml -p vac-core`;
- cargo tests/clippy;
- live TUI smoke;
- full physical layer move ke `crates/<layer>/<name>`;
- deep `vac-api` split;
- deep file-raksasa module-per-symbol extraction.
