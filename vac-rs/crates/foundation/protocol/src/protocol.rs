// O5.2 semantic source split.
// The former full-byte legacy_include.rs has been removed.
// Shards are included in source order from the sibling split_manifest.yaml so public module paths remain stable.
// original_full_byte_sha256: da17e8419779f875b7f19d5e5912dddd79096a46d568c7656d4c9afc1d82377e
// shard_count: 5

include!("protocol/semantic_split.rs");
