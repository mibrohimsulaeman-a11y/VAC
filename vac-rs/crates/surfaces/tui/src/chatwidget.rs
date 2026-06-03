// O5.2 semantic source split.
// The former full-byte legacy_include.rs has been removed.
// Shards are included in source order from the sibling split_manifest.yaml so public module paths remain stable.
// original_full_byte_sha256: 1c79ee7a2d81d3f477a1babe19960a13d057e681c6f8f20c61682dda0275c9c5
// shard_count: 24

mod height_cache;

include!("chatwidget/semantic_split.rs");
