//! Shell command parsing and hierarchical scope resolution.
//!
//! This crate parses shell command strings using `tree-sitter-bash`

mod matcher;
mod parse;
mod resolver;

pub use matcher::matches_pattern;
pub use parse::{ParseError, ParsedCommand, parse, parse_with_status};
pub use resolver::resolve_hierarchical_policy;

pub mod vac_destructive;
pub use vac_destructive::{ShellRisk, classify_shell_command, requires_explicit_approval};
