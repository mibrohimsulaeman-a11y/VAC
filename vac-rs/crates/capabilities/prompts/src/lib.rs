//! Centralized prompt texts shared across VAC capability crates.
//!
//! Single source of truth for prompt `.md` files previously duplicated
//! byte-for-byte across multiple capability crates. Prompt content is
//! unchanged; only the storage location is centralized.

/// Review-thread system prompt (formerly `review_prompt.md`).
pub const REVIEW_PROMPT: &str = include_str!("review_prompt.md");

/// Apply-patch instructions prompt (formerly `prompt_with_apply_patch_instructions.md`).
pub const APPLY_PATCH_INSTRUCTIONS: &str = include_str!("prompt_with_apply_patch_instructions.md");

/// Base agent prompt, variant 1 (formerly `gpt_5_1_prompt.md`).
pub const AGENT_PROMPT_V1: &str = include_str!("gpt_5_1_prompt.md");

/// Base agent prompt, variant 2 (formerly `gpt_5_2_prompt.md`).
pub const AGENT_PROMPT_V2: &str = include_str!("gpt_5_2_prompt.md");

/// VAC agent prompt (formerly `gpt_5_vac_prompt.md`).
pub const AGENT_PROMPT_VAC: &str = include_str!("gpt_5_vac_prompt.md");
