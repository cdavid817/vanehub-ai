use super::{DesktopLifecycleApplicationError, DesktopLifecyclePort};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct DesktopLifecycleApplicationService {
    lifecycle: Arc<dyn DesktopLifecyclePort>,
}

impl DesktopLifecycleApplicationService {
    pub(crate) fn new(lifecycle: Arc<dyn DesktopLifecyclePort>) -> Self {
        Self { lifecycle }
    }

    pub(crate) fn initialize(&self) -> Result<(), DesktopLifecycleApplicationError> {
        self.lifecycle.initialize()
    }

    pub(crate) fn request_exit(&self) {
        self.lifecycle.request_exit();
    }
}
