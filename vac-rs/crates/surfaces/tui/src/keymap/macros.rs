/// Resolve one context-local action binding from config.
///
/// Expands to `resolve_bindings(...)` with:
/// - configured source: `tui.keymap.<context>.<action>`
/// - fallback source: the same action from built-in defaults
/// - error path: a stable string path for user-facing diagnostics
///
/// This keeps the resolution table concise while guaranteeing path strings
/// stay in sync with field names.
macro_rules! resolve_local {
    ($keymap:expr, $defaults:expr, $context:ident, $action:ident) => {
        resolve_bindings(
            ($keymap).$context.$action.as_ref(),
            &($defaults).$context.$action,
            concat!(
                "tui.keymap.",
                stringify!($context),
                ".",
                stringify!($action)
            ),
        )?
    };
}

/// Resolve one action binding with global fallback.
///
/// Expands to `resolve_bindings_with_global_fallback(...)` with precedence:
/// 1. `tui.keymap.<context>.<action>`
/// 2. `tui.keymap.global.<action>`
/// 3. built-in defaults for `<context>.<action>`
///
/// Used only for actions that intentionally support global reuse.
/// Context-local empty lists still count as configured values, so they unbind
/// the action instead of falling back to `global`.
macro_rules! resolve_with_global {
    ($keymap:expr, $defaults:expr, $context:ident, $action:ident) => {
        resolve_bindings_with_global_fallback(
            ($keymap).$context.$action.as_ref(),
            ($keymap).global.$action.as_ref(),
            &($defaults).$context.$action,
            concat!(
                "tui.keymap.",
                stringify!($context),
                ".",
                stringify!($action)
            ),
        )?
    };
}

/// Expand one default-table binding entry into a [`KeyBinding`].
///
/// This is a small declarative layer over `key_hint::{plain, ctrl, alt, shift}`
/// used by `default_bindings!` so `built_in_defaults` stays readable.
///
/// Supported forms:
/// - `plain(<KeyCode>)`
/// - `ctrl(<KeyCode>)`
/// - `alt(<KeyCode>)`
/// - `shift(<KeyCode>)`
/// - `raw(<KeyBinding expression>)` for bindings that do not match the helpers
///   (for example combined modifiers like Ctrl+Shift).
macro_rules! default_binding {
    (plain($key:expr)) => {
        key_hint::plain($key)
    };
    (ctrl($key:expr)) => {
        key_hint::ctrl($key)
    };
    (alt($key:expr)) => {
        key_hint::alt($key)
    };
    (shift($key:expr)) => {
        key_hint::shift($key)
    };
    (raw($binding:expr)) => {
        $binding
    };
}

/// Build a `Vec<KeyBinding>` for built-in defaults.
///
/// This macro is intentionally scoped to built-in keymaps. Runtime
/// config parsing still goes through `parse_bindings(...)` so user errors can
/// be reported with config-path-aware diagnostics.
macro_rules! default_bindings {
    ($($kind:ident($($arg:tt)*)),* $(,)?) => {
        vec![$(default_binding!($kind($($arg)*))),*]
    };
}

