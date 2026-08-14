use crate::contexts::agent_runtime::application::BrowserHandoffControlPort;
use crate::contexts::browser_automation::application::{BrowserOperationService, BrowserOwnership};
use chrono::Utc;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use uuid::Uuid;

#[derive(Clone)]
struct TrackedHandoff {
    ownership: BrowserOwnership,
    page_id: String,
    handoff_id: Option<String>,
    ownership_token: String,
    state: &'static str,
    updated_at: String,
}

pub(crate) struct BrowserHandoffCommandAdapter {
    operations: Arc<BrowserOperationService>,
    tracked: Mutex<BTreeMap<String, TrackedHandoff>>,
}

impl BrowserHandoffCommandAdapter {
    pub(crate) fn new(operations: Arc<BrowserOperationService>) -> Self {
        Self {
            operations,
            tracked: Mutex::new(BTreeMap::new()),
        }
    }

    pub(crate) fn record_page(
        &self,
        operation_id: &str,
        ownership: BrowserOwnership,
        page_id: String,
    ) -> Result<(), ()> {
        if operation_id.trim().is_empty() || page_id.trim().is_empty() {
            return Err(());
        }
        self.tracked.lock().map_err(|_| ())?.insert(
            operation_id.to_owned(),
            TrackedHandoff {
                ownership,
                page_id,
                handoff_id: None,
                ownership_token: format!("browser-owner-{}", Uuid::new_v4()),
                state: "automating",
                updated_at: Utc::now().to_rfc3339(),
            },
        );
        Ok(())
    }

    fn snapshot(operation_id: &str, tracked: &TrackedHandoff) -> Value {
        json!({
            "operationId": operation_id,
            "state": tracked.state,
            "ownershipToken": tracked.ownership_token,
            "updatedAt": tracked.updated_at,
        })
    }
}

impl BrowserHandoffControlPort for BrowserHandoffCommandAdapter {
    fn get_handoff(&self, operation_id: &str) -> Result<Value, ()> {
        let tracked = self.tracked.lock().map_err(|_| ())?;
        tracked
            .get(operation_id)
            .map(|state| Self::snapshot(operation_id, state))
            .ok_or(())
    }

    fn begin_handoff(&self, operation_id: &str) -> Result<Value, ()> {
        let current = self
            .tracked
            .lock()
            .map_err(|_| ())?
            .get(operation_id)
            .cloned()
            .ok_or(())?;
        if current.handoff_id.is_some() {
            return Err(());
        }
        let handoff = self
            .operations
            .begin_handoff(
                current.ownership.clone(),
                current.page_id.clone(),
                Duration::from_secs(15 * 60),
            )
            .map_err(|_| ())?;
        let mut tracked = self.tracked.lock().map_err(|_| ())?;
        let state = tracked.get_mut(operation_id).ok_or(())?;
        if state.ownership != current.ownership || state.page_id != current.page_id {
            return Err(());
        }
        state.handoff_id = Some(handoff.handoff_id);
        state.state = "human_control";
        state.updated_at = Utc::now().to_rfc3339();
        Ok(Self::snapshot(operation_id, state))
    }

    fn resume_automation(&self, operation_id: &str, ownership_token: &str) -> Result<Value, ()> {
        let current = self
            .tracked
            .lock()
            .map_err(|_| ())?
            .get(operation_id)
            .cloned()
            .ok_or(())?;
        if ownership_token.is_empty() || ownership_token != current.ownership_token {
            return Err(());
        }
        let handoff_id = current.handoff_id.as_deref().ok_or(())?;
        self.operations
            .resume_handoff(&current.ownership, &current.page_id, handoff_id, true)
            .map_err(|_| ())?;
        let mut tracked = self.tracked.lock().map_err(|_| ())?;
        let state = tracked.get_mut(operation_id).ok_or(())?;
        if state.ownership_token != ownership_token
            || state.handoff_id.as_deref() != Some(handoff_id)
        {
            return Err(());
        }
        state.state = "resuming";
        state.updated_at = Utc::now().to_rfc3339();
        Ok(Self::snapshot(operation_id, state))
    }
}
