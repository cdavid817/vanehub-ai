use super::*;
use crate::contexts::desktop::application::DesktopClockPort;
use crate::contexts::desktop::domain::{
    FloatingAssistantAnchor, FloatingAssistantConfig, FloatingAssistantDomainError,
    FloatingAssistantPlatform, FloatingAssistantSurfaceMode, MonitorWorkArea, ScreenPosition,
    SurfaceSize, SurfaceTransition, WindowPlacement,
};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct FakeRepository {
    state: Arc<Mutex<RepositoryState>>,
    calls: Arc<Mutex<Vec<String>>>,
}

struct RepositoryState {
    config: FloatingAssistantConfig,
    load_failure: Option<String>,
    save_failure: Option<String>,
}

impl FloatingAssistantRepository for FakeRepository {
    fn load(&self) -> Result<FloatingAssistantConfig, FloatingAssistantApplicationError> {
        self.calls
            .lock()
            .expect("calls")
            .push("repository:load".to_string());
        let mut state = self.state.lock().expect("repository state");
        if let Some(message) = state.load_failure.take() {
            return Err(FloatingAssistantApplicationError::Repository(message));
        }
        Ok(state.config.clone())
    }

    fn save(
        &self,
        config: &FloatingAssistantConfig,
        updated_at: &str,
    ) -> Result<FloatingAssistantConfig, FloatingAssistantApplicationError> {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("repository:save:{updated_at}"));
        let mut state = self.state.lock().expect("repository state");
        if let Some(message) = state.save_failure.take() {
            return Err(FloatingAssistantApplicationError::Repository(message));
        }
        state.config = config.clone();
        Ok(config.clone())
    }
}

#[derive(Clone)]
struct FakeWindow {
    state: Arc<Mutex<WindowState>>,
    calls: Arc<Mutex<Vec<String>>>,
}

struct WindowState {
    platform: FloatingAssistantPlatform,
    available: bool,
    placement: WindowPlacement,
    ensure_failure: Option<String>,
    destroy_failure: Option<String>,
    placement_failure: Option<String>,
    surface_failure: Option<String>,
    transitions: Vec<SurfaceTransition>,
}

impl FloatingAssistantWindowPort for FakeWindow {
    fn platform(&self) -> FloatingAssistantPlatform {
        self.calls
            .lock()
            .expect("calls")
            .push("window:platform".to_string());
        self.state.lock().expect("window state").platform
    }

    fn ensure(
        &self,
        _config: &FloatingAssistantConfig,
    ) -> Result<(), FloatingAssistantApplicationError> {
        self.calls
            .lock()
            .expect("calls")
            .push("window:ensure".to_string());
        let mut state = self.state.lock().expect("window state");
        if let Some(message) = state.ensure_failure.take() {
            return Err(FloatingAssistantApplicationError::Window(message));
        }
        state.available = true;
        Ok(())
    }

    fn destroy(&self) -> Result<(), FloatingAssistantApplicationError> {
        self.calls
            .lock()
            .expect("calls")
            .push("window:destroy".to_string());
        let mut state = self.state.lock().expect("window state");
        if let Some(message) = state.destroy_failure.take() {
            return Err(FloatingAssistantApplicationError::Window(message));
        }
        state.available = false;
        Ok(())
    }

    fn is_available(&self) -> bool {
        self.calls
            .lock()
            .expect("calls")
            .push("window:available".to_string());
        self.state.lock().expect("window state").available
    }

    fn placement(&self) -> Result<WindowPlacement, FloatingAssistantApplicationError> {
        self.calls
            .lock()
            .expect("calls")
            .push("window:placement".to_string());
        let mut state = self.state.lock().expect("window state");
        if let Some(message) = state.placement_failure.take() {
            return Err(FloatingAssistantApplicationError::Window(message));
        }
        Ok(state.placement.clone())
    }

    fn apply_surface(
        &self,
        transition: &SurfaceTransition,
    ) -> Result<(), FloatingAssistantApplicationError> {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("window:surface:{}", transition.mode.as_str()));
        let mut state = self.state.lock().expect("window state");
        if let Some(message) = state.surface_failure.take() {
            return Err(FloatingAssistantApplicationError::Window(message));
        }
        state.transitions.push(transition.clone());
        Ok(())
    }

    fn start_dragging(&self) -> Result<(), FloatingAssistantApplicationError> {
        self.calls
            .lock()
            .expect("calls")
            .push("window:drag".to_string());
        Ok(())
    }

    fn show_main_window(&self) -> Result<(), FloatingAssistantApplicationError> {
        self.calls
            .lock()
            .expect("calls")
            .push("window:show-main".to_string());
        Ok(())
    }
}

#[derive(Clone)]
struct FixedClock;

impl DesktopClockPort for FixedClock {
    fn now(&self) -> String {
        "2026-07-18T12:00:00Z".to_string()
    }
}

struct Fixture {
    service: FloatingAssistantApplicationService,
    repository: FakeRepository,
    window: FakeWindow,
    calls: Arc<Mutex<Vec<String>>>,
}

impl Fixture {
    fn new(platform: FloatingAssistantPlatform) -> Self {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let repository = FakeRepository {
            state: Arc::new(Mutex::new(RepositoryState {
                config: FloatingAssistantConfig::disabled(),
                load_failure: None,
                save_failure: None,
            })),
            calls: calls.clone(),
        };
        let window = FakeWindow {
            state: Arc::new(Mutex::new(WindowState {
                platform,
                available: false,
                placement: WindowPlacement {
                    position: ScreenPosition { x: 1000, y: 700 },
                    size: SurfaceSize {
                        width: 76,
                        height: 76,
                    },
                    scale_factor: 1.0,
                    monitor: Some(MonitorWorkArea {
                        position: ScreenPosition { x: 0, y: 0 },
                        size: SurfaceSize {
                            width: 1920,
                            height: 1080,
                        },
                        name: Some("DISPLAY1".to_string()),
                    }),
                },
                ensure_failure: None,
                destroy_failure: None,
                placement_failure: None,
                surface_failure: None,
                transitions: Vec::new(),
            })),
            calls: calls.clone(),
        };
        let service = FloatingAssistantApplicationService::new(
            Arc::new(repository.clone()),
            Arc::new(window.clone()),
            Arc::new(FixedClock),
        );
        Self {
            service,
            repository,
            window,
            calls,
        }
    }

    fn set_config(&self, config: FloatingAssistantConfig) {
        self.repository
            .state
            .lock()
            .expect("repository state")
            .config = config;
    }

    fn config(&self) -> FloatingAssistantConfig {
        self.repository
            .state
            .lock()
            .expect("repository state")
            .config
            .clone()
    }

    fn calls(&self) -> Vec<String> {
        self.calls.lock().expect("calls").clone()
    }
}

#[test]
fn unsupported_platform_rejects_enablement_before_persistence_or_window_creation() {
    let fixture = Fixture::new(FloatingAssistantPlatform::Unsupported);

    let error = fixture
        .service
        .set_enabled(true)
        .expect_err("unsupported platform");

    assert_eq!(
        error,
        FloatingAssistantApplicationError::Domain(
            FloatingAssistantDomainError::UnsupportedPlatform
        )
    );
    assert_eq!(fixture.calls(), ["window:platform"]);
    assert!(!fixture.config().enabled());
}

#[test]
fn enablement_preserves_anchor_and_orders_window_before_persistence() {
    let fixture = Fixture::new(FloatingAssistantPlatform::Windows);
    fixture.set_config(FloatingAssistantConfig::new(
        false,
        FloatingAssistantAnchor::new(1076.0, 776.0, Some("DISPLAY1".to_string())),
    ));

    let saved = fixture.service.set_enabled(true).expect("enable");

    assert!(saved.enabled());
    assert_eq!(saved.anchor().expect("anchor").x(), 1076.0);
    assert_eq!(
        fixture.calls(),
        [
            "window:platform",
            "repository:load",
            "window:ensure",
            "repository:save:2026-07-18T12:00:00Z",
        ]
    );
    assert!(fixture.window.state.lock().expect("window state").available);
}

#[test]
fn failed_window_transition_prevents_configuration_persistence() {
    let fixture = Fixture::new(FloatingAssistantPlatform::Windows);
    fixture
        .window
        .state
        .lock()
        .expect("window state")
        .ensure_failure = Some("cannot create window".to_string());

    let error = fixture
        .service
        .set_enabled(true)
        .expect_err("window failure");

    assert_eq!(
        error,
        FloatingAssistantApplicationError::Window("cannot create window".to_string())
    );
    assert_eq!(
        fixture.calls(),
        ["window:platform", "repository:load", "window:ensure"]
    );
    assert!(!fixture.config().enabled());
}

#[test]
fn anchor_updates_normalize_invalid_coordinates_and_keep_enablement() {
    let fixture = Fixture::new(FloatingAssistantPlatform::Windows);
    fixture.set_config(FloatingAssistantConfig::new(true, None));

    let saved = fixture
        .service
        .save_anchor(f64::NAN, 720.0, Some("DISPLAY1".to_string()))
        .expect("normalized anchor");

    assert!(saved.enabled());
    assert!(saved.anchor().is_none());
    assert_eq!(
        fixture.calls(),
        ["repository:load", "repository:save:2026-07-18T12:00:00Z"]
    );
}

#[test]
fn position_persistence_uses_the_window_bottom_right_anchor() {
    let fixture = Fixture::new(FloatingAssistantPlatform::Windows);

    let saved = fixture
        .service
        .persist_window_position()
        .expect("persist position");
    let anchor = saved.anchor().expect("anchor");

    assert_eq!((anchor.x(), anchor.y()), (1076.0, 776.0));
    assert_eq!(anchor.monitor_name(), Some("DISPLAY1"));
    assert_eq!(
        fixture.calls(),
        [
            "window:placement",
            "repository:load",
            "repository:save:2026-07-18T12:00:00Z",
        ]
    );
}

#[test]
fn surface_use_case_computes_one_transition_before_applying_it() {
    let fixture = Fixture::new(FloatingAssistantPlatform::Windows);

    let transition = fixture
        .service
        .set_surface(FloatingAssistantSurfaceMode::Chat)
        .expect("surface transition");

    assert_eq!(transition.position, ScreenPosition { x: 668, y: 156 });
    assert_eq!(fixture.calls(), ["window:placement", "window:surface:chat"]);
    assert_eq!(
        fixture
            .window
            .state
            .lock()
            .expect("window state")
            .transitions,
        [transition]
    );
}

#[test]
fn initialization_and_close_visibility_follow_persisted_configuration() {
    let fixture = Fixture::new(FloatingAssistantPlatform::Windows);
    fixture.set_config(FloatingAssistantConfig::new(true, None));

    fixture.service.initialize().expect("initialize");
    let should_hide = fixture
        .service
        .should_hide_main_on_close()
        .expect("visibility");

    assert!(should_hide);
    assert_eq!(
        fixture.calls(),
        [
            "repository:load",
            "window:platform",
            "window:ensure",
            "repository:load",
            "window:available",
        ]
    );
}

#[test]
fn unsupported_persisted_enablement_is_rejected_during_initialization() {
    let fixture = Fixture::new(FloatingAssistantPlatform::Unsupported);
    fixture.set_config(FloatingAssistantConfig::new(true, None));

    let error = fixture.service.initialize().expect_err("unsupported");

    assert_eq!(
        error,
        FloatingAssistantApplicationError::Domain(
            FloatingAssistantDomainError::UnsupportedPlatform
        )
    );
    assert_eq!(fixture.calls(), ["repository:load", "window:platform"]);
}

#[test]
fn direct_window_actions_stay_behind_the_window_port() {
    let fixture = Fixture::new(FloatingAssistantPlatform::Windows);

    fixture.service.start_dragging().expect("drag");
    fixture.service.show_main_window().expect("show main");

    assert_eq!(fixture.calls(), ["window:drag", "window:show-main"]);
}
