use super::*;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};

struct Session {
    closes: Arc<AtomicUsize>,
}

impl BrowserSession for Session {
    fn request(
        &mut self,
        _method: &str,
        _params: Value,
    ) -> Result<BrowserSidecarResponse, BrowserSidecarError> {
        Ok(BrowserSidecarResponse {
            protocol_version: 1,
            request_id: "fixture".to_string(),
            ok: true,
            result: Some(json!({"isolated": true})),
            error_code: None,
        })
    }

    fn close(&mut self) -> Result<(), BrowserSidecarError> {
        self.closes.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

struct Factory {
    creates: Arc<AtomicUsize>,
    closes: Arc<AtomicUsize>,
}

impl BrowserSessionFactory for Factory {
    fn create_isolated(
        &self,
        _ownership: &BrowserOwnership,
        _policy: BrowserContextPolicy,
    ) -> Result<Box<dyn BrowserSession>, BrowserSessionError> {
        self.creates.fetch_add(1, Ordering::SeqCst);
        Ok(Box::new(Session {
            closes: Arc::clone(&self.closes),
        }))
    }
}

fn owner(generation: &str) -> BrowserOwnership {
    BrowserOwnership {
        session_id: "session-1".to_string(),
        generation_id: generation.to_string(),
    }
}

#[test]
fn identical_owner_and_policy_reuse_only_the_owned_isolated_context() {
    let creates = Arc::new(AtomicUsize::new(0));
    let closes = Arc::new(AtomicUsize::new(0));
    let manager = BrowserSessionManager::new(Arc::new(Factory {
        creates: Arc::clone(&creates),
        closes: Arc::clone(&closes),
    }));
    let policy = BrowserContextPolicy::default();

    for _ in 0..2 {
        let result = manager
            .with_session(owner("generation-1"), policy, |session| {
                session
                    .request("inspect", Value::Null)
                    .map_err(BrowserSessionError::ProtocolFailure)
            })
            .expect("owned context request");
        assert_eq!(result.result, Some(json!({"isolated": true})));
    }
    assert_eq!(creates.load(Ordering::SeqCst), 1);
    manager
        .close_generation(&owner("generation-1"))
        .expect("owned context close");
    assert_eq!(closes.load(Ordering::SeqCst), 1);
}

#[test]
fn generations_never_share_contexts_and_policy_changes_cannot_rebind_an_existing_one() {
    let creates = Arc::new(AtomicUsize::new(0));
    let manager = BrowserSessionManager::new(Arc::new(Factory {
        creates: Arc::clone(&creates),
        closes: Arc::new(AtomicUsize::new(0)),
    }));
    let policy = BrowserContextPolicy::default();
    manager
        .with_session(owner("generation-a"), policy, |_| Ok(()))
        .expect("generation A context");
    manager
        .with_session(owner("generation-b"), policy, |_| Ok(()))
        .expect("generation B context");
    assert_eq!(creates.load(Ordering::SeqCst), 2);

    let changed = BrowserContextPolicy {
        max_pages: 3,
        ..policy
    };
    assert_eq!(
        manager.with_session(owner("generation-a"), changed, |_| Ok(())),
        Err(BrowserSessionError::OwnershipMismatch)
    );
}

#[test]
fn controller_owned_session_ceilings_fail_closed() {
    let manager = BrowserSessionManager::new(Arc::new(Factory {
        creates: Arc::new(AtomicUsize::new(0)),
        closes: Arc::new(AtomicUsize::new(0)),
    }));
    let unsafe_policy = BrowserContextPolicy {
        max_lifetime: Duration::from_secs(60 * 60 + 1),
        max_pages: 5,
        max_download_bytes: 64 * 1024 * 1024 + 1,
        max_event_count: 2_001,
    };
    assert_eq!(
        manager.with_session(owner("generation-1"), unsafe_policy, |_| Ok(())),
        Err(BrowserSessionError::InvalidPolicy)
    );
}

#[test]
fn manager_drop_closes_every_owned_context() {
    let closes = Arc::new(AtomicUsize::new(0));
    {
        let manager = BrowserSessionManager::new(Arc::new(Factory {
            creates: Arc::new(AtomicUsize::new(0)),
            closes: Arc::clone(&closes),
        }));
        manager
            .with_session(
                owner("generation-a"),
                BrowserContextPolicy::default(),
                |_| Ok(()),
            )
            .expect("context A");
        manager
            .with_session(
                owner("generation-b"),
                BrowserContextPolicy::default(),
                |_| Ok(()),
            )
            .expect("context B");
    }
    assert_eq!(closes.load(Ordering::SeqCst), 2);
}
