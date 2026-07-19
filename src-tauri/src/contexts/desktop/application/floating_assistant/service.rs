use super::{
    FloatingAssistantApplicationError, FloatingAssistantRepository, FloatingAssistantWindowPort,
};
use crate::contexts::desktop::application::DesktopClockPort;
use crate::contexts::desktop::domain::{
    should_intercept_main_close, FloatingAssistantAnchor, FloatingAssistantConfig,
    FloatingAssistantSurfaceMode, SurfaceTransition,
};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct FloatingAssistantApplicationService {
    repository: Arc<dyn FloatingAssistantRepository>,
    window: Arc<dyn FloatingAssistantWindowPort>,
    clock: Arc<dyn DesktopClockPort>,
}

impl FloatingAssistantApplicationService {
    pub(crate) fn new(
        repository: Arc<dyn FloatingAssistantRepository>,
        window: Arc<dyn FloatingAssistantWindowPort>,
        clock: Arc<dyn DesktopClockPort>,
    ) -> Self {
        Self {
            repository,
            window,
            clock,
        }
    }

    pub(crate) fn get_config(
        &self,
    ) -> Result<FloatingAssistantConfig, FloatingAssistantApplicationError> {
        self.repository.load()
    }

    pub(crate) fn platform(&self) -> crate::contexts::desktop::domain::FloatingAssistantPlatform {
        self.window.platform()
    }

    pub(crate) fn set_enabled(
        &self,
        enabled: bool,
    ) -> Result<FloatingAssistantConfig, FloatingAssistantApplicationError> {
        let platform = self.window.platform();
        platform.validate_enablement(enabled)?;
        let current = self.repository.load()?;
        let next = current.with_enabled(enabled, platform)?;
        if enabled {
            self.window.ensure(&next)?;
        } else {
            self.window.destroy()?;
        }
        self.repository.save(&next, &self.clock.now())
    }

    pub(crate) fn save_anchor(
        &self,
        x: f64,
        y: f64,
        monitor_name: Option<String>,
    ) -> Result<FloatingAssistantConfig, FloatingAssistantApplicationError> {
        let current = self.repository.load()?;
        let next = current.with_anchor(FloatingAssistantAnchor::new(x, y, monitor_name));
        self.repository.save(&next, &self.clock.now())
    }

    pub(crate) fn persist_window_position(
        &self,
    ) -> Result<FloatingAssistantConfig, FloatingAssistantApplicationError> {
        let placement = self.window.placement()?;
        let monitor_name = placement
            .monitor
            .as_ref()
            .and_then(|monitor| monitor.name.clone());
        let anchor =
            FloatingAssistantAnchor::from_window(placement.position, placement.size, monitor_name);
        let current = self.repository.load()?;
        let next = current.with_anchor(Some(anchor));
        self.repository.save(&next, &self.clock.now())
    }

    pub(crate) fn set_surface(
        &self,
        mode: FloatingAssistantSurfaceMode,
    ) -> Result<SurfaceTransition, FloatingAssistantApplicationError> {
        let placement = self.window.placement()?;
        let transition = SurfaceTransition::from_placement(mode, &placement);
        self.window.apply_surface(&transition)?;
        Ok(transition)
    }

    pub(crate) fn initialize(&self) -> Result<(), FloatingAssistantApplicationError> {
        let config = self.repository.load()?;
        self.window
            .platform()
            .validate_enablement(config.enabled())?;
        if config.enabled() {
            self.window.ensure(&config)?;
        }
        Ok(())
    }

    pub(crate) fn start_dragging(&self) -> Result<(), FloatingAssistantApplicationError> {
        self.window.start_dragging()
    }

    pub(crate) fn show_main_window(&self) -> Result<(), FloatingAssistantApplicationError> {
        self.window.show_main_window()
    }

    pub(crate) fn should_hide_main_on_close(
        &self,
    ) -> Result<bool, FloatingAssistantApplicationError> {
        let config = self.repository.load()?;
        Ok(should_intercept_main_close(
            config.enabled(),
            self.window.is_available(),
        ))
    }
}
