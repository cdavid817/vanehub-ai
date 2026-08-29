use super::api::SkillEvolutionOrchestrationApi;
use super::infrastructure::EvolutionNotificationRepository;
use serde_json::{json, Value};

impl SkillEvolutionOrchestrationApi {
    pub(crate) fn pending_notifications(&self, now_ms: i64) -> Result<Vec<Value>, String> {
        EvolutionNotificationRepository::new(self.database.clone()).pending(now_ms)
    }

    pub(crate) fn finish_notification(
        &self,
        event_id: &str,
        delivered: bool,
        now_ms: i64,
    ) -> Result<(), String> {
        EvolutionNotificationRepository::new(self.database.clone())
            .finish(event_id, delivered, now_ms)
    }

    pub(crate) fn dispatch_notifications(
        &self,
        now_ms: i64,
        mut publish: impl FnMut(&Value) -> Result<(), ()>,
    ) -> Result<Value, String> {
        let events = self.pending_notifications(now_ms)?;
        let mut delivered = 0;
        let mut failed = 0;
        for event in events {
            let event_id = event["eventId"].as_str().ok_or("storage_unavailable")?;
            let published = publish(&event).is_ok();
            self.finish_notification(event_id, published, now_ms)?;
            if published {
                delivered += 1;
            } else {
                failed += 1;
            }
        }
        Ok(json!({ "delivered": delivered, "failed": failed }))
    }
}
