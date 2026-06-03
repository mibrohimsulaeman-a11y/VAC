// O5.2 domain dispatcher.
//
// This file intentionally contains only ordered include! dispatches.
// The domain shards listed in split_manifest.yaml are the source-of-record;
// keeping this file small prevents semantic_split.rs from becoming a review-blocking godfile.

include!("split_001_into.rs");
include!("split_002_footerflash.rs");
