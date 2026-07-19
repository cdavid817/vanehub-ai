use super::DesktopLifecycleApplicationError;
use async_trait::async_trait;

pub(crate) trait DesktopLifecyclePort: Send + Sync {
    fn initialize(&self) -> Result<(), DesktopLifecycleApplicationError>;

    fn request_exit(&self);
}

#[async_trait]
pub(crate) trait DesktopShutdownPort: Send + Sync {
    async fn shutdown(&self) -> Result<(), String>;
}
