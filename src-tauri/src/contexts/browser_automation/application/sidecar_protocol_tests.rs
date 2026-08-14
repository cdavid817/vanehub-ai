use super::*;
use serde_json::json;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

struct ScriptedTransport {
    responses: VecDeque<Result<BrowserSidecarResponse, BrowserSidecarError>>,
    shutdowns: Arc<AtomicUsize>,
}

impl BrowserSidecarTransport for ScriptedTransport {
    fn request(
        &mut self,
        request: &BrowserSidecarRequest,
        limits: BrowserSidecarLimits,
    ) -> Result<BrowserSidecarResponse, BrowserSidecarError> {
        let encoded = serde_json::to_vec(request).unwrap_or_default();
        if encoded.len() > limits.max_message_bytes {
            return Err(BrowserSidecarError::MessageTooLarge);
        }
        match self.responses.pop_front() {
            Some(Ok(mut response)) => {
                if response.request_id == "$request" {
                    response.request_id = request.request_id.clone();
                }
                Ok(response)
            }
            Some(Err(error)) => Err(error),
            None => Ok(success(request, json!({"echo": request.method}))),
        }
    }

    fn shutdown(&mut self, _timeout: Duration) -> Result<(), BrowserSidecarError> {
        self.shutdowns.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

struct Factory {
    scripts: Mutex<VecDeque<VecDeque<Result<BrowserSidecarResponse, BrowserSidecarError>>>>,
    spawns: AtomicUsize,
    shutdowns: Arc<AtomicUsize>,
}

impl BrowserSidecarFactory for Factory {
    fn spawn(
        &self,
        _limits: BrowserSidecarLimits,
    ) -> Result<Box<dyn BrowserSidecarTransport>, BrowserSidecarError> {
        self.spawns.fetch_add(1, Ordering::SeqCst);
        let responses = self
            .scripts
            .lock()
            .map_err(|_| BrowserSidecarError::SpawnFailed)?
            .pop_front()
            .ok_or(BrowserSidecarError::SpawnFailed)?;
        Ok(Box::new(ScriptedTransport {
            responses,
            shutdowns: Arc::clone(&self.shutdowns),
        }))
    }
}

fn success(request: &BrowserSidecarRequest, result: Value) -> BrowserSidecarResponse {
    BrowserSidecarResponse {
        protocol_version: BROWSER_SIDECAR_PROTOCOL_VERSION,
        request_id: request.request_id.clone(),
        ok: true,
        result: Some(result),
        error_code: None,
    }
}

fn scripted(result: Value) -> Result<BrowserSidecarResponse, BrowserSidecarError> {
    Ok(BrowserSidecarResponse {
        protocol_version: BROWSER_SIDECAR_PROTOCOL_VERSION,
        request_id: "$request".to_string(),
        ok: true,
        result: Some(result),
        error_code: None,
    })
}

fn healthy_script(
    after_health: Vec<Result<BrowserSidecarResponse, BrowserSidecarError>>,
) -> VecDeque<Result<BrowserSidecarResponse, BrowserSidecarError>> {
    let mut responses = VecDeque::from([
        scripted(json!({"protocol_version": BROWSER_SIDECAR_PROTOCOL_VERSION})),
        scripted(json!({"status": "ready"})),
    ]);
    responses.extend(after_health);
    responses
}

#[test]
fn handshake_health_request_and_shutdown_are_versioned_and_bounded() {
    let shutdowns = Arc::new(AtomicUsize::new(0));
    let factory = Arc::new(Factory {
        scripts: Mutex::new(VecDeque::from([healthy_script(vec![scripted(
            json!({"page": "page-1"}),
        )])])),
        spawns: AtomicUsize::new(0),
        shutdowns: Arc::clone(&shutdowns),
    });
    let mut supervisor =
        BrowserSidecarSupervisor::new(BrowserSidecarLimits::default(), factory.clone())
            .expect("valid limits");

    supervisor.start().expect("healthy handshake");
    let response = supervisor
        .request("page.inspect", json!({"page_id": "page-1"}))
        .expect("fixture request");
    assert_eq!(response.result, Some(json!({"page": "page-1"})));
    supervisor.shutdown().expect("bounded shutdown");
    assert_eq!(factory.spawns.load(Ordering::SeqCst), 1);
    assert_eq!(shutdowns.load(Ordering::SeqCst), 1);
}

#[test]
fn crashed_transport_restarts_once_and_rehandshakes_before_replaying() {
    let factory = Arc::new(Factory {
        scripts: Mutex::new(VecDeque::from([
            healthy_script(vec![Err(BrowserSidecarError::ProcessExited)]),
            healthy_script(vec![scripted(json!({"recovered": true}))]),
        ])),
        spawns: AtomicUsize::new(0),
        shutdowns: Arc::new(AtomicUsize::new(0)),
    });
    let mut supervisor = BrowserSidecarSupervisor::new(
        BrowserSidecarLimits {
            max_restart_attempts: 1,
            ..BrowserSidecarLimits::default()
        },
        factory.clone(),
    )
    .expect("valid limits");

    let result = supervisor
        .request("page.inspect", Value::Null)
        .expect("request should recover once");
    assert_eq!(result.result, Some(json!({"recovered": true})));
    assert_eq!(factory.spawns.load(Ordering::SeqCst), 2);
}

#[test]
fn protocol_mismatch_and_restart_exhaustion_fail_closed() {
    let mismatch = BrowserSidecarResponse {
        protocol_version: 99,
        request_id: "$request".to_string(),
        ok: true,
        result: Some(json!({"protocol_version": 99})),
        error_code: None,
    };
    let factory = Arc::new(Factory {
        scripts: Mutex::new(VecDeque::from([VecDeque::from([Ok(mismatch)])])),
        spawns: AtomicUsize::new(0),
        shutdowns: Arc::new(AtomicUsize::new(0)),
    });
    let mut supervisor = BrowserSidecarSupervisor::new(BrowserSidecarLimits::default(), factory)
        .expect("valid limits");
    assert_eq!(
        supervisor.start(),
        Err(BrowserSidecarError::ProtocolMismatch)
    );

    let factory = Arc::new(Factory {
        scripts: Mutex::new(VecDeque::from([healthy_script(vec![Err(
            BrowserSidecarError::ProcessExited,
        )])])),
        spawns: AtomicUsize::new(0),
        shutdowns: Arc::new(AtomicUsize::new(0)),
    });
    let mut no_restart = BrowserSidecarSupervisor::new(
        BrowserSidecarLimits {
            max_restart_attempts: 0,
            ..BrowserSidecarLimits::default()
        },
        factory,
    )
    .expect("valid limits");
    assert_eq!(
        no_restart.request("page.inspect", Value::Null),
        Err(BrowserSidecarError::RestartLimitExceeded)
    );
}

#[test]
fn caller_cannot_relax_message_duration_or_restart_ceilings() {
    let factory = Arc::new(Factory {
        scripts: Mutex::new(VecDeque::new()),
        spawns: AtomicUsize::new(0),
        shutdowns: Arc::new(AtomicUsize::new(0)),
    });
    let invalid = BrowserSidecarLimits {
        max_message_bytes: 4 * 1024 * 1024 + 1,
        request_timeout: Duration::from_secs(61),
        max_restart_attempts: 4,
    };
    assert!(matches!(
        BrowserSidecarSupervisor::new(invalid, factory),
        Err(BrowserSidecarError::InvalidLimits)
    ));
}
