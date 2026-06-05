pub fn render_memory_consolidation_workflow_yaml() -> String {
    r#"schema_version: 1
kind: workflow
id: maintenance.memory-consolidation
title: Post-feature memory consolidation & reflection
status: ready
steps:
  - id: step.read_working_memory
    uses: capability.memory.read_transient
  - id: step.distill_learnings
    uses: capability.memory.reflect_and_distill
  - id: step.write_semantic_fact
    uses: capability.memory.write_semantic
  - id: step.clean_working_memory
    uses: capability.memory.clear_transient
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reflection_workflow_declares_distill_and_semantic_write_steps() {
        let rendered = render_memory_consolidation_workflow_yaml();
        assert!(rendered.contains("capability.memory.reflect_and_distill"));
        assert!(rendered.contains("capability.memory.write_semantic"));
    }
}
