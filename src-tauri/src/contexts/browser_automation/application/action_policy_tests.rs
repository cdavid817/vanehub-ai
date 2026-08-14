use super::*;
use serde_json::json;
use std::sync::Mutex;

struct Resolver(Mutex<Vec<String>>);

impl UrlResolverPort for Resolver {
    fn resolve(&self, _host: &str, _port: u16) -> Result<Vec<String>, GuardedUrlPolicyError> {
        self.0
            .lock()
            .map(|addresses| addresses.clone())
            .map_err(|_| GuardedUrlPolicyError::ResolutionFailed)
    }
}

fn resolver(address: &str) -> Resolver {
    Resolver(Mutex::new(vec![address.to_owned()]))
}

fn request(action: BrowserAction, input: Value) -> BrowserOperationRequest {
    BrowserOperationRequest {
        ownership: BrowserOwnership {
            session_id: "session-1".to_owned(),
            generation_id: "generation-1".to_owned(),
        },
        action,
        page_id: Some("page-1".to_owned()),
        input,
    }
}

#[test]
fn navigation_is_normalized_and_private_targets_are_rejected_before_worker_access() {
    let public = resolver("93.184.216.34");
    let witness = BrowserActionPolicy::prepare(
        &request(
            BrowserAction::Navigate,
            json!({"url": "https://Example.com:443/docs#part"}),
        ),
        None,
        &public,
    )
    .expect("public target");
    assert_eq!(witness.canonical_origin, "https://example.com");
    assert_eq!(witness.risk, BrowserRiskClass::ReadOnly);
    assert!(!witness.risk.requires_unified_permission());
    assert_eq!(
        BrowserActionPolicy::prepare(
            &request(
                BrowserAction::Navigate,
                json!({"url": "http://127.0.0.1/private"}),
            ),
            None,
            &public,
        ),
        Err(BrowserActionPolicyError::UnsafeOrigin)
    );
}

#[test]
fn effectful_actions_bind_owner_origin_action_target_and_exact_input() {
    let resolver = resolver("93.184.216.34");
    let original = request(
        BrowserAction::Fill,
        json!({"selector": "#password", "text": "secret"}),
    );
    let witness =
        BrowserActionPolicy::prepare(&original, Some("https://example.com/login"), &resolver)
            .expect("witness");
    assert!(witness.risk.requires_unified_permission());
    assert_eq!(witness.canonical_origin, "https://example.com");
    assert!(!witness.safe_target_summary.contains("password"));
    assert!(!witness.safe_target_summary.contains("secret"));
    BrowserActionPolicy::revalidate(
        &witness,
        &original,
        Some("https://example.com/account"),
        &resolver,
    )
    .expect("same origin remains valid");

    let changed = request(
        BrowserAction::Fill,
        json!({"selector": "#password", "text": "changed"}),
    );
    assert_eq!(
        BrowserActionPolicy::revalidate(
            &witness,
            &changed,
            Some("https://example.com/login"),
            &resolver,
        ),
        Err(BrowserActionPolicyError::StaleApproval)
    );
    assert_eq!(
        BrowserActionPolicy::revalidate(
            &witness,
            &original,
            Some("https://other.example/login"),
            &resolver,
        ),
        Err(BrowserActionPolicyError::StaleApproval)
    );
}

#[test]
fn navigation_dns_change_invalidates_the_prepared_witness() {
    let resolver = resolver("93.184.216.34");
    let request = request(
        BrowserAction::Navigate,
        json!({"url": "https://example.com/docs"}),
    );
    let witness =
        BrowserActionPolicy::prepare(&request, None, &resolver).expect("navigation witness");
    *resolver.0.lock().expect("addresses") = vec!["93.184.216.35".to_owned()];
    assert_eq!(
        BrowserActionPolicy::revalidate(&witness, &request, None, &resolver),
        Err(BrowserActionPolicyError::StaleApproval)
    );
}

#[test]
fn unsupported_active_page_origins_fail_closed() {
    let resolver = resolver("93.184.216.34");
    assert_eq!(
        BrowserActionPolicy::prepare(
            &request(BrowserAction::Evaluate, json!({"expression": "1 + 1"})),
            Some("about:blank"),
            &resolver,
        ),
        Err(BrowserActionPolicyError::UnsafeOrigin)
    );
}
