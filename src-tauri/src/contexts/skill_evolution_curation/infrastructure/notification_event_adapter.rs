use crate::contexts::skill_evolution_curation::application::{
    CuratorNotificationEvent, CuratorNotificationPort,
};
use tauri::{AppHandle, Emitter};

#[derive(Clone)]
pub(crate) struct TauriCuratorNotificationEventAdapter {
    app: AppHandle,
}

impl TauriCuratorNotificationEventAdapter {
    pub(crate) fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl CuratorNotificationPort for TauriCuratorNotificationEventAdapter {
    fn publish(&self, event: &CuratorNotificationEvent) -> Result<(), ()> {
        self.app
            .emit("skill-curator:notification", event)
            .map_err(|_| ())
    }
}
