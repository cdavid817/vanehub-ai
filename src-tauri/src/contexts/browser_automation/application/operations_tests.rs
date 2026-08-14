use super::*;
use crate::contexts::browser_automation::application::{
    BrowserSession, BrowserSessionFactory, BrowserSidecarError, BrowserSidecarResponse,
};
use std::sync::{Arc, Mutex};

type CapturedRequests = Arc<Mutex<Vec<(String, Value)>>>;

struct FixtureSession {
    requests: CapturedRequests,
    oversized: bool,
}

impl BrowserSession for FixtureSession {
    fn request(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<BrowserSidecarResponse, BrowserSidecarError> {
        self.requests
            .lock()
            .expect("request lock")
            .push((method.to_string(), params));
        let payload = if self.oversized {
            json!({"text": "x".repeat(300_000)})
        } else {
            json!({"elements": [{"ref": "element-1", "text": "Continue"}]})
        };
        Ok(BrowserSidecarResponse {
            protocol_version: 1,
            request_id: "fixture".to_string(),
            ok: true,
            result: Some(json!({
                "page_id": "page-1",
                "frame_id": "main-frame",
                "url": "https://example.com/final",
                "payload": payload,
                "truncated": false
            })),
            error_code: None,
        })
    }

    fn close(&mut self) -> Result<(), BrowserSidecarError> {
        Ok(())
    }
}

struct Factory {
    requests: CapturedRequests,
    oversized: bool,
}

impl BrowserSessionFactory for Factory {
    fn create_isolated(
        &self,
        _ownership: &BrowserOwnership,
        _policy: BrowserContextPolicy,
    ) -> Result<Box<dyn BrowserSession>, BrowserSessionError> {
        Ok(Box::new(FixtureSession {
            requests: Arc::clone(&self.requests),
            oversized: self.oversized,
        }))
    }
}

fn operation_service(oversized: bool) -> (BrowserOperationService, CapturedRequests) {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let sessions = BrowserSessionManager::new(Arc::new(Factory {
        requests: Arc::clone(&requests),
        oversized,
    }));
    (
        BrowserOperationService::new(sessions, BrowserContextPolicy::default()),
        requests,
    )
}

fn request(action: BrowserAction, input: Value) -> BrowserOperationRequest {
    BrowserOperationRequest {
        ownership: BrowserOwnership {
            session_id: "session-1".to_string(),
            generation_id: "generation-1".to_string(),
        },
        action,
        page_id: Some("page-1".to_string()),
        input,
    }
}

#[test]
fn supported_operations_map_to_fixed_sidecar_methods_and_safe_result_projection() {
    let (service, requests) = operation_service(false);
    let cases = [
        (
            BrowserAction::Navigate,
            json!({"url": "https://example.com"}),
        ),
        (BrowserAction::GoBack, json!({})),
        (BrowserAction::GoForward, json!({})),
        (BrowserAction::Inspect, json!({})),
        (BrowserAction::Click, json!({"selector": "button"})),
        (
            BrowserAction::Fill,
            json!({"selector": "input", "text": "hello"}),
        ),
        (BrowserAction::Extract, json!({"selector": "main"})),
        (BrowserAction::Screenshot, json!({"full_page": false})),
        (
            BrowserAction::Evaluate,
            json!({"expression": "document.title"}),
        ),
    ];
    for (action, input) in cases {
        let result = service
            .execute(request(action, input))
            .expect("fixed operation");
        assert_eq!(result.contract_version, 1);
        assert_eq!(result.action, action);
        assert_eq!(result.page_id, "page-1");
        assert_eq!(result.frame_id.as_deref(), Some("main-frame"));
    }
    let methods = requests
        .lock()
        .expect("request lock")
        .iter()
        .map(|(method, _)| method.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        methods,
        vec![
            "page.navigate",
            "page.go_back",
            "page.go_forward",
            "page.inspect",
            "page.click",
            "page.fill",
            "page.extract",
            "page.screenshot",
            "page.evaluate"
        ]
    );
}

#[test]
fn invalid_or_unbounded_inputs_and_results_fail_before_projection() {
    let (service, requests) = operation_service(false);
    assert_eq!(
        service.execute(request(BrowserAction::Navigate, json!({"url": ""}))),
        Err(BrowserOperationError::InvalidInput)
    );
    assert_eq!(
        service.execute(request(
            BrowserAction::Evaluate,
            json!({"expression": "x".repeat(16_385)})
        )),
        Err(BrowserOperationError::InvalidInput)
    );
    assert!(requests.lock().expect("request lock").is_empty());

    let (oversized, _) = operation_service(true);
    assert_eq!(
        oversized.execute(request(BrowserAction::Inspect, json!({}))),
        Err(BrowserOperationError::UnsafeResult)
    );
}
