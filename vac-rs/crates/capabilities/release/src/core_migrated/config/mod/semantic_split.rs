// O5.2 domain dispatcher.
//
// This file intentionally contains only ordered include! dispatches.
// The domain shards listed in split_manifest.yaml are the source-of-record;
// keeping this file small prevents semantic_split.rs from becoming a review-blocking godfile.

include!("split_001_agentsmdmanager.rs");
include!("split_002_multiagentv2config.rs");
include!("split_003_validate_feature_requirements_for_co.rs");
include!("split_004_permissionconfigsyntax.rs");
include!("split_005_env.rs");
include!("split_006_try_read_non_empty_file.rs");
