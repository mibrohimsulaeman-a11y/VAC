// O5.2 semantic source split.
// The former full-byte legacy_include.rs has been removed.
// Shards are included in source order from the sibling split_manifest.yaml so public module paths remain stable.
// original_full_byte_sha256: 416216f96d8306de8e356d6f0397a613dc313810f73c7590e6c02f55f2795cb8
// shard_count: 2

include!("memories/semantic_split.rs");
