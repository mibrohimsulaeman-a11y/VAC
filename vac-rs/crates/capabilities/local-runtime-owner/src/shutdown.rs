//! Shutdown ownership seam.
//!
//! Future plans will add bounded shutdown and pending-request cancellation.

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RuntimeShutdownHandle;

impl RuntimeShutdownHandle {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}
