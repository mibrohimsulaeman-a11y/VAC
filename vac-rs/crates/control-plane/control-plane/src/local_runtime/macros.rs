//! Internal macros shared by submodules of `local_runtime`.
//!
//! Defined here once instead of being duplicated across enum modules. Consumers
//! import via `use super::impl_display_as_str;`.
//!
//! The macro uses fully-qualified `::std::fmt` paths so callers do not need
//! `use std::fmt::Display` / `Formatter` imports.

macro_rules! impl_display_as_str {
    ($ty:ident { $($variant:ident => $value:expr),+ $(,)? }) => {
        impl $ty {
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $value,)+
                }
            }
        }

        impl ::std::fmt::Display for $ty {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

pub(crate) use impl_display_as_str;
