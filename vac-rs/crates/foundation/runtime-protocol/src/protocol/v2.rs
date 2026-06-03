// VAC O5/O6 audit remainder dispatcher. Source body lives in bounded shards.
include!("v2_parts/v2_part_000.rs");
include!("v2_parts/v2_part_001.rs");
include!("v2_parts/v2_part_002.rs");
include!("v2_parts/v2_part_003.rs");
include!("v2_parts/v2_part_004.rs");
include!("v2_parts/v2_part_005.rs");
// Balanced shard: v2_part_006..009 previously split a #[cfg(test)] module across
// include! boundaries. Rust include! requires each file to parse as a complete
// item sequence, so this shard keeps that test module syntactically closed.
include!("v2_parts/v2_part_006_combined.rs");
