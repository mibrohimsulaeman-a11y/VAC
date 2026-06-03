// O5.2 semantic source split.
// The former full-byte legacy_include.rs has been removed.
// Shards are included in source order from the sibling split_manifest.yaml so public module paths remain stable.
// original_full_byte_sha256: fccf89b3b920539496269cae28eb5268d5ad7d328b9b3492bdc3592b56813ade
// shard_count: 2

include!("chat_composer/semantic_split.rs");
