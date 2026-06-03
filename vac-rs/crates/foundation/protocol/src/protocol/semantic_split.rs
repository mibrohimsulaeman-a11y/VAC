// O5.2 domain dispatcher.
//
// This file intentionally contains only ordered include! dispatches.
// The domain shards listed in split_manifest.yaml are the source-of-record;
// keeping this file small prevents semantic_split.rs from becoming a review-blocking godfile.

include!("split_001_hashmap.rs");
include!("split_002_its_existing_context_either_conversa.rs");
include!("split_003_hookeventname.rs");
include!("split_004_exitedreviewmodeevent.rs");
include!("split_005_websearchbeginevent.rs");
