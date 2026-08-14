use super::BrowserHandoffCommandAdapter;
use crate::contexts::agent_runtime::application::BrowserHandoffControlPort;
use crate::contexts::browser_automation::application::{
    BrowserContextPolicy, BrowserOperationService, BrowserOwnership, BrowserSession,
    BrowserSessionError, BrowserSessionFactory, BrowserSessionManager, BrowserSidecarError,
    BrowserSidecarResponse,
};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

struct Session {
    methods: Arc<Mutex<Vec<String>>>,
}

impl BrowserSession for Session {
    fn request(
        &mut self,
        method: &str,
        _params: Value,
    ) -> Result<BrowserSidecarResponse, BrowserSidecarError> {
        self.methods
            .lock()
            .expect("methods")
            .push(method.to_owned());
        Ok(BrowserSidecarResponse {
            protocol_version: 1,
            request_id: "request-1".to_owned(),
            ok: true,
            result: Some(json!({})),
            error_code: None,
        })
    }

    fn close(&mut self) -> Result<(), BrowserSidecarError> {
        Ok(())
    }
}

struct Factory {
    methods: Arc<Mutex<Vec<String>>>,
}

impl BrowserSessionFactory for Factory {
    fn create_isolated(
        &self,
        _ownership: &BrowserOwnership,
        _policy: BrowserContextPolicy,
    ) -> Result<Box<dyn BrowserSession>, BrowserSessionError> {
        Ok(Box::new(Session {
            methods: self.methods.clone(),
        }))
    }
}

#[test]
fn production_handoff_adapter_preserves_owned_page_and_token_checks() {
    let methods = Arc::new(Mutex::new(Vec::new()));
    let operations = Arc::new(BrowserOperationService::new(
        BrowserSessionManager::new(Arc::new(Factory {
            methods: methods.clone(),
        })),
        BrowserContextPolicy::default(),
    ));
    let adapter = BrowserHandoffCommandAdapter::new(operations);
    let ownership = BrowserOwnership {
        session_id: "session-1".to_owned(),
        generation_id: "generation-1".to_owned(),
    };
    adapter
        .record_page("operation-1", ownership, "page-1".to_owned())
        .expect("record page");
    let initial = adapter.get_handoff("operation-1").expect("snapshot");
    let token = initial
        .get("ownershipToken")
        .and_then(Value::as_str)
        .expect("token")
        .to_owned();
    assert_eq!(
        initial.get("state").and_then(Value::as_str),
        Some("automating")
    );
    let handed = adapter.begin_handoff("operation-1").expect("handoff");
    assert_eq!(
        handed.get("state").and_then(Value::as_str),
        Some("human_control")
    );
    assert!(adapter.resume_automation("operation-1", "forged").is_err());
    let resumed = adapter
        .resume_automation("operation-1", &token)
        .expect("resume");
    assert_eq!(
        resumed.get("state").and_then(Value::as_str),
        Some("resuming")
    );
    assert_eq!(
        methods.lock().expect("methods").as_slice(),
        &["page.handoff".to_owned(), "page.resume".to_owned()]
    );
}
