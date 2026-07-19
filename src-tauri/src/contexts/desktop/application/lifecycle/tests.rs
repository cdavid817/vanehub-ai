use super::*;
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
struct FakeLifecycle {
    calls: Arc<Mutex<Vec<String>>>,
}

impl DesktopLifecyclePort for FakeLifecycle {
    fn initialize(&self) -> Result<(), DesktopLifecycleApplicationError> {
        self.calls
            .lock()
            .expect("calls")
            .push("initialize".to_string());
        Ok(())
    }

    fn request_exit(&self) {
        self.calls
            .lock()
            .expect("calls")
            .push("request-exit".to_string());
    }
}

#[test]
fn lifecycle_use_cases_delegate_to_one_runtime_port() {
    let lifecycle = FakeLifecycle::default();
    let service = DesktopLifecycleApplicationService::new(Arc::new(lifecycle.clone()));

    service.initialize().expect("initialize");
    service.request_exit();

    assert_eq!(
        lifecycle.calls.lock().expect("calls").as_slice(),
        ["initialize", "request-exit"]
    );
}
