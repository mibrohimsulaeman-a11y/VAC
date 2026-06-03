# O5.2 Semantic Source Split

Status: **SV-Done / TV-Pending**.

The previous full-byte `legacy_include.rs` staging files have been replaced by ordered semantic source shards:

- `semantic_split.rs`
- `split_manifest.yaml`
- `split_*.rs`

The gate now verifies both levels of hash integrity:

1. each shard `sha256` recorded in `split_manifest.yaml`, and
2. reconstructed full-byte SHA256 from ordered shards equals `original_full_byte_sha256`.

Executable gate:

```bash
bash scripts/check-vac-o5-2-semantic-split-hash.sh
bash scripts/check-vac-o5-2-godfile-staging-all.sh
```

Current source-static status: **7/7 split manifests reconstruct byte-exact**.

Caveat: shards are still included textually via `include!(...)` to preserve parent-module private visibility until cargo-backed extraction can safely split symbols into deeper Rust modules.
