use super::runtime_manager::ConnectorLifecycleEventPort;
use crate::contexts::communications::domain::ConnectorHealth;
use tauri::{AppHandle, Emitter};

pub(crate) struct TauriConnectorLifecycleEvents {
    app: AppHandle,
}

impl TauriConnectorLifecycleEvents {
    pub(crate) fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl ConnectorLifecycleEventPort for TauriConnectorLifecycleEvents {
    fn publish(&self, health: ConnectorHealth) {
        let _ = self.app.emit("im-connector:lifecycle", health);
    }
}
