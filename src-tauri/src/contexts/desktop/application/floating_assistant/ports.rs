use super::FloatingAssistantApplicationError;
use crate::contexts::desktop::domain::{
    FloatingAssistantConfig, FloatingAssistantPlatform, SurfaceTransition, WindowPlacement,
};

pub(crate) trait FloatingAssistantRepository: Send + Sync {
    fn load(&self) -> Result<FloatingAssistantConfig, FloatingAssistantApplicationError>;

    fn save(
        &self,
        config: &FloatingAssistantConfig,
        updated_at: &str,
    ) -> Result<FloatingAssistantConfig, FloatingAssistantApplicationError>;
}

pub(crate) trait FloatingAssistantWindowPort: Send + Sync {
    fn platform(&self) -> FloatingAssistantPlatform;

    fn ensure(
        &self,
        config: &FloatingAssistantConfig,
    ) -> Result<(), FloatingAssistantApplicationError>;

    fn destroy(&self) -> Result<(), FloatingAssistantApplicationError>;

    fn is_available(&self) -> bool;

    fn placement(&self) -> Result<WindowPlacement, FloatingAssistantApplicationError>;

    fn apply_surface(
        &self,
        transition: &SurfaceTransition,
    ) -> Result<(), FloatingAssistantApplicationError>;

    fn start_dragging(&self) -> Result<(), FloatingAssistantApplicationError>;

    fn show_main_window(&self) -> Result<(), FloatingAssistantApplicationError>;
}
