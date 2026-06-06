// Runtime keymap resolution for the TUI.
//
// This module converts deserialized config (`TuiKeymap`) into a concrete
// `RuntimeKeymap` used by input handlers at runtime.
//
// Key responsibilities:
//
// 1. Apply deterministic precedence (`context -> global fallback -> defaults`).
// 2. Parse canonical key spec strings into `KeyBinding` values.
// 3. Enforce uniqueness across runtime surfaces so one key cannot trigger
//    multiple actions on the same focused input path.
// 4. Return actionable, user-facing error messages with config paths and next
//    steps.
//
// Non-responsibilities:
//
// 1. This module does not decide which action should run in a given screen.
//    Callers resolve actions by checking the relevant action binding set.
// 2. This module does not persist configuration; it only resolves loaded config.

#[macro_use]
mod macros;
mod types;
mod resolve;
mod defaults;
mod validate;
#[cfg(test)]
mod tests;

pub(crate) use types::*;
