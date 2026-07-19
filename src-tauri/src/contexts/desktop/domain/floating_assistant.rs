use std::fmt;

const MAX_COORDINATE: f64 = 10_000_000.0;
const SCREEN_MARGIN: i32 = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FloatingAssistantPlatform {
    Windows,
    Unsupported,
}

impl FloatingAssistantPlatform {
    pub(crate) fn native_available(self) -> bool {
        matches!(self, Self::Windows)
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Windows => "windows",
            Self::Unsupported => "unsupported",
        }
    }

    pub(crate) fn validate_enablement(
        self,
        enabled: bool,
    ) -> Result<(), FloatingAssistantDomainError> {
        if enabled && !self.native_available() {
            return Err(FloatingAssistantDomainError::UnsupportedPlatform);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FloatingAssistantSurfaceMode {
    Collapsed,
    Menu,
    Chat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FloatingAssistantMainAction {
    NewSession,
    CurrentSession,
    Settings,
}

impl FloatingAssistantMainAction {
    pub(crate) fn parse(value: &str) -> Result<Self, FloatingAssistantDomainError> {
        match value {
            "new-session" => Ok(Self::NewSession),
            "current-session" => Ok(Self::CurrentSession),
            "settings" => Ok(Self::Settings),
            _ => Err(FloatingAssistantDomainError::InvalidMainWindowAction),
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::NewSession => "new-session",
            Self::CurrentSession => "current-session",
            Self::Settings => "settings",
        }
    }
}

impl FloatingAssistantSurfaceMode {
    #[cfg(test)]
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Collapsed => "collapsed",
            Self::Menu => "menu",
            Self::Chat => "chat",
        }
    }

    pub(crate) fn logical_size(self) -> (f64, f64) {
        match self {
            Self::Collapsed => (76.0, 76.0),
            Self::Menu => (304.0, 316.0),
            Self::Chat => (408.0, 620.0),
        }
    }

    fn physical_size(self, scale_factor: f64) -> SurfaceSize {
        let scale_factor = if scale_factor.is_finite() && scale_factor > 0.0 {
            scale_factor
        } else {
            1.0
        };
        let (width, height) = self.logical_size();
        SurfaceSize {
            width: (width * scale_factor).round().clamp(0.0, u32::MAX as f64) as u32,
            height: (height * scale_factor).round().clamp(0.0, u32::MAX as f64) as u32,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FloatingAssistantAnchor {
    x: f64,
    y: f64,
    monitor_name: Option<String>,
}

impl FloatingAssistantAnchor {
    pub(crate) fn new(x: f64, y: f64, monitor_name: Option<String>) -> Option<Self> {
        (valid_coordinate(x) && valid_coordinate(y)).then_some(Self { x, y, monitor_name })
    }

    pub(crate) fn from_window(
        position: ScreenPosition,
        size: SurfaceSize,
        monitor_name: Option<String>,
    ) -> Self {
        Self {
            x: f64::from(position.x) + f64::from(size.width),
            y: f64::from(position.y) + f64::from(size.height),
            monitor_name,
        }
    }

    pub(crate) fn x(&self) -> f64 {
        self.x
    }

    pub(crate) fn y(&self) -> f64 {
        self.y
    }

    pub(crate) fn monitor_name(&self) -> Option<&str> {
        self.monitor_name.as_deref()
    }

    pub(crate) fn position_for(&self, size: SurfaceSize) -> ScreenPosition {
        ScreenPosition {
            x: (self.x.round() as i32).saturating_sub(size_component(size.width)),
            y: (self.y.round() as i32).saturating_sub(size_component(size.height)),
        }
    }
}

fn valid_coordinate(value: f64) -> bool {
    value.is_finite() && value.abs() <= MAX_COORDINATE
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FloatingAssistantConfig {
    enabled: bool,
    anchor: Option<FloatingAssistantAnchor>,
}

impl FloatingAssistantConfig {
    pub(crate) fn new(enabled: bool, anchor: Option<FloatingAssistantAnchor>) -> Self {
        Self { enabled, anchor }
    }

    #[cfg(test)]
    pub(crate) fn disabled() -> Self {
        Self::new(false, None)
    }

    pub(crate) fn enabled(&self) -> bool {
        self.enabled
    }

    pub(crate) fn anchor(&self) -> Option<&FloatingAssistantAnchor> {
        self.anchor.as_ref()
    }

    pub(crate) fn with_enabled(
        &self,
        enabled: bool,
        platform: FloatingAssistantPlatform,
    ) -> Result<Self, FloatingAssistantDomainError> {
        platform.validate_enablement(enabled)?;
        Ok(Self::new(enabled, self.anchor.clone()))
    }

    pub(crate) fn with_anchor(&self, anchor: Option<FloatingAssistantAnchor>) -> Self {
        Self::new(self.enabled, anchor)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ScreenPosition {
    pub(crate) x: i32,
    pub(crate) y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SurfaceSize {
    pub(crate) width: u32,
    pub(crate) height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MonitorWorkArea {
    pub(crate) position: ScreenPosition,
    pub(crate) size: SurfaceSize,
    pub(crate) name: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WindowPlacement {
    pub(crate) position: ScreenPosition,
    pub(crate) size: SurfaceSize,
    pub(crate) scale_factor: f64,
    pub(crate) monitor: Option<MonitorWorkArea>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SurfaceTransition {
    pub(crate) mode: FloatingAssistantSurfaceMode,
    pub(crate) logical_width: u32,
    pub(crate) logical_height: u32,
    pub(crate) physical_size: SurfaceSize,
    pub(crate) position: ScreenPosition,
    pub(crate) should_reposition: bool,
}

impl SurfaceTransition {
    pub(crate) fn from_placement(
        mode: FloatingAssistantSurfaceMode,
        placement: &WindowPlacement,
    ) -> Self {
        let stable_anchor = FloatingAssistantAnchor::from_window(
            placement.position,
            placement.size,
            placement
                .monitor
                .as_ref()
                .and_then(|monitor| monitor.name.clone()),
        );
        let physical_size = mode.physical_size(placement.scale_factor);
        let requested = stable_anchor.position_for(physical_size);
        let position = placement.monitor.as_ref().map_or(requested, |monitor| {
            clamp_position(requested, physical_size, monitor)
        });
        let (logical_width, logical_height) = mode.logical_size();
        Self {
            mode,
            logical_width: logical_width as u32,
            logical_height: logical_height as u32,
            physical_size,
            position,
            should_reposition: placement.monitor.is_some(),
        }
    }
}

pub(crate) fn clamp_position(
    position: ScreenPosition,
    window_size: SurfaceSize,
    monitor: &MonitorWorkArea,
) -> ScreenPosition {
    let min_x = monitor.position.x.saturating_add(SCREEN_MARGIN);
    let min_y = monitor.position.y.saturating_add(SCREEN_MARGIN);
    let max_x = monitor
        .position
        .x
        .saturating_add(size_component(monitor.size.width))
        .saturating_sub(size_component(window_size.width))
        .saturating_sub(SCREEN_MARGIN)
        .max(min_x);
    let max_y = monitor
        .position
        .y
        .saturating_add(size_component(monitor.size.height))
        .saturating_sub(size_component(window_size.height))
        .saturating_sub(SCREEN_MARGIN)
        .max(min_y);
    ScreenPosition {
        x: position.x.clamp(min_x, max_x),
        y: position.y.clamp(min_y, max_y),
    }
}

pub(crate) fn position_for_monitor(
    anchor: Option<&FloatingAssistantAnchor>,
    window_size: SurfaceSize,
    monitor: &MonitorWorkArea,
) -> ScreenPosition {
    let default_anchor = FloatingAssistantAnchor {
        x: f64::from(
            monitor
                .position
                .x
                .saturating_add(size_component(monitor.size.width))
                .saturating_sub(SCREEN_MARGIN),
        ),
        y: f64::from(
            monitor
                .position
                .y
                .saturating_add(size_component(monitor.size.height))
                .saturating_sub(SCREEN_MARGIN),
        ),
        monitor_name: monitor.name.clone(),
    };
    let requested = anchor.unwrap_or(&default_anchor).position_for(window_size);
    clamp_position(requested, window_size, monitor)
}

fn size_component(value: u32) -> i32 {
    value.min(i32::MAX as u32) as i32
}

pub(crate) fn should_intercept_main_close(enabled: bool, floating_window_available: bool) -> bool {
    enabled && floating_window_available
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FloatingAssistantDomainError {
    UnsupportedPlatform,
    InvalidMainWindowAction,
}

impl fmt::Display for FloatingAssistantDomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPlatform => {
                formatter.write_str("floating assistant is currently available on Windows only")
            }
            Self::InvalidMainWindowAction => formatter.write_str("invalid main-window action"),
        }
    }
}

impl std::error::Error for FloatingAssistantDomainError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor() -> MonitorWorkArea {
        MonitorWorkArea {
            position: ScreenPosition { x: 100, y: 200 },
            size: SurfaceSize {
                width: 1920,
                height: 1080,
            },
            name: Some("DISPLAY1".to_string()),
        }
    }

    #[test]
    fn unsupported_platform_cannot_enable_but_can_remain_disabled() {
        let config = FloatingAssistantConfig::disabled();

        assert_eq!(
            config.with_enabled(true, FloatingAssistantPlatform::Unsupported),
            Err(FloatingAssistantDomainError::UnsupportedPlatform)
        );
        assert_eq!(
            config
                .with_enabled(false, FloatingAssistantPlatform::Unsupported)
                .expect("disabled"),
            config
        );
    }

    #[test]
    fn anchors_reject_non_finite_and_out_of_bounds_coordinates() {
        assert!(FloatingAssistantAnchor::new(1280.5, 720.25, None).is_some());
        assert!(FloatingAssistantAnchor::new(f64::NAN, 1.0, None).is_none());
        assert!(FloatingAssistantAnchor::new(MAX_COORDINATE + 1.0, 1.0, None).is_none());
    }

    #[test]
    fn bottom_right_anchor_is_stable_across_surface_modes() {
        let placement = WindowPlacement {
            position: ScreenPosition { x: 1000, y: 700 },
            size: SurfaceSize {
                width: 76,
                height: 76,
            },
            scale_factor: 1.0,
            monitor: None,
        };

        assert_eq!(
            SurfaceTransition::from_placement(FloatingAssistantSurfaceMode::Menu, &placement)
                .position,
            ScreenPosition { x: 772, y: 460 }
        );
        assert_eq!(
            SurfaceTransition::from_placement(FloatingAssistantSurfaceMode::Chat, &placement)
                .position,
            ScreenPosition { x: 668, y: 156 }
        );
    }

    #[test]
    fn surface_transition_scales_and_clamps_inside_monitor_work_area() {
        let placement = WindowPlacement {
            position: ScreenPosition { x: 5000, y: -50 },
            size: SurfaceSize {
                width: 76,
                height: 76,
            },
            scale_factor: 1.25,
            monitor: Some(monitor()),
        };

        let transition =
            SurfaceTransition::from_placement(FloatingAssistantSurfaceMode::Chat, &placement);

        assert_eq!(transition.logical_width, 408);
        assert_eq!(transition.logical_height, 620);
        assert_eq!(
            transition.physical_size,
            SurfaceSize {
                width: 510,
                height: 775,
            }
        );
        assert_eq!(transition.position, ScreenPosition { x: 1490, y: 220 });
    }

    #[test]
    fn initial_position_uses_remembered_or_default_bottom_right_anchor() {
        let window_size = SurfaceSize {
            width: 76,
            height: 76,
        };
        let remembered = FloatingAssistantAnchor::new(1076.0, 776.0, Some("DISPLAY1".to_string()))
            .expect("anchor");

        assert_eq!(
            position_for_monitor(Some(&remembered), window_size, &monitor()),
            ScreenPosition { x: 1000, y: 700 }
        );
        assert_eq!(
            position_for_monitor(None, window_size, &monitor()),
            ScreenPosition { x: 1924, y: 1184 }
        );
    }

    #[test]
    fn close_visibility_requires_enabled_configuration_and_a_window() {
        assert!(should_intercept_main_close(true, true));
        assert!(!should_intercept_main_close(false, true));
        assert!(!should_intercept_main_close(true, false));
    }

    #[test]
    fn main_window_actions_use_the_existing_transport_allowlist() {
        assert_eq!(
            FloatingAssistantMainAction::parse("new-session").expect("new session"),
            FloatingAssistantMainAction::NewSession
        );
        assert_eq!(
            FloatingAssistantMainAction::parse("current-session").expect("current session"),
            FloatingAssistantMainAction::CurrentSession
        );
        assert_eq!(
            FloatingAssistantMainAction::parse("settings").expect("settings"),
            FloatingAssistantMainAction::Settings
        );
        assert_eq!(
            FloatingAssistantMainAction::parse("close"),
            Err(FloatingAssistantDomainError::InvalidMainWindowAction)
        );
    }
}
