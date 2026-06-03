// O5.2 semantic source split.
// The former full-byte legacy_include.rs has been removed.
// Shards are included in source order from the sibling split_manifest.yaml so public module paths remain stable.
// original_full_byte_sha256: 9fe9d89830b528392d76ec84f9bd7f37e4f4fafc07ade9a07f52da9c692f663c
// shard_count: 1

include!("workflow_runner/semantic_split.rs");
