//! Session seam for the future TUI `LocalRuntimeSession` implementation.
//!
//! Plan 27 gives this seam locally owned retained resources and bootstrap data,
//! but it does not cut over prompt submit or event streaming.

use crate::LocalRuntimeBootstrap;
use crate::RuntimeEventStream;
use crate::RuntimeRetainedResources;

#[derive(Clone)]
pub struct LocalRuntimeOwnerSession {
    retained: RuntimeRetainedResources,
    bootstrap: LocalRuntimeBootstrap,
    events: RuntimeEventStream,
}

impl LocalRuntimeOwnerSession {
    #[must_use]
    pub fn new(retained: RuntimeRetainedResources, bootstrap: LocalRuntimeBootstrap) -> Self {
        Self {
            retained,
            bootstrap,
            events: RuntimeEventStream::new(),
        }
    }

    #[must_use]
    pub fn retained(&self) -> &RuntimeRetainedResources {
        &self.retained
    }

    #[must_use]
    pub fn bootstrap(&self) -> &LocalRuntimeBootstrap {
        &self.bootstrap
    }

    #[must_use]
    pub fn events(&self) -> &RuntimeEventStream {
        &self.events
    }

    pub fn events_mut(&mut self) -> &mut RuntimeEventStream {
        &mut self.events
    }

    #[must_use]
    pub fn into_parts(
        self,
    ) -> (
        RuntimeRetainedResources,
        LocalRuntimeBootstrap,
        RuntimeEventStream,
    ) {
        (self.retained, self.bootstrap, self.events)
    }
}
