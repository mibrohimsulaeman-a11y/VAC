// Inline aggregator for workflow_runner build_check split (semantic consolidation).
// Since this file is include!()'d from semantic_split.rs, we use include!() for content, not mod declarations.

include!("build_check_parts/vocabulary.rs");
include!("build_check_parts/dry_run.rs");
include!("build_check_parts/execution_machine.rs");
include!("build_check_parts/execution_reports.rs");
include!("build_check_parts/formatters.rs");
#[cfg(test)]
include!("build_check_parts/tests.rs");
