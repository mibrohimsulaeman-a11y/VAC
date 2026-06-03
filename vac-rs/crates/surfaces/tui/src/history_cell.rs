// O5.2 semantic source split.
// The former full-byte legacy_include.rs has been removed.
// Shards are included in source order from the sibling split_manifest.yaml so public module paths remain stable.
// original_full_byte_sha256: 2d778cf1fea13a5d36ef788c421adb5417bc1190564b20cebc5fd4f97dfbba7f
// shard_count: 6

include!("history_cell/semantic_split.rs");
