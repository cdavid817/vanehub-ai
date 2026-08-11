use super::process_registry::{
    ActivationReason, LifecycleAction, LifecyclePolicy, ProcessRegistry, RejectionReason,
};
use super::project_root::ProcessKey;
use crate::contexts::code_intelligence::domain::models::{
    ConfigurationFingerprint, ProcessState, ServerKind,
};
use std::time::Duration;

const NOW: Duration = Duration::from_secs(100);

#[test]
fn tool_acquisition_starts_on_demand_and_concurrent_acquisition_reuses_it() {
    let fixture = KeyFixture::new("config-a");
    let mut registry = ProcessRegistry::new(LifecyclePolicy::default());

    let first = registry.acquire(
        fixture.key.clone(),
        ActivationReason::ToolRequest,
        NOW,
        true,
    );
    let second = registry.acquire(
        fixture.key.clone(),
        ActivationReason::ToolRequest,
        NOW,
        true,
    );

    assert_eq!(first, vec![LifecycleAction::Start(fixture.key.clone())]);
    assert!(second.is_empty());
    assert_eq!(
        registry
            .status(&fixture.key)
            .expect("status")
            .active_requests,
        2
    );
}

#[test]
fn prewarm_requires_a_hint_but_tool_activation_never_depends_on_one() {
    let fixture = KeyFixture::new("config-a");
    let mut registry = ProcessRegistry::new(LifecyclePolicy::default());

    assert!(registry
        .acquire(
            fixture.key.clone(),
            ActivationReason::Prewarm {
                inventory: false,
                manifest: false
            },
            NOW,
            true,
        )
        .is_empty());
    assert_eq!(
        registry.acquire(
            fixture.key.clone(),
            ActivationReason::ToolRequest,
            NOW,
            true
        ),
        vec![LifecycleAction::Start(fixture.key.clone())]
    );
}

#[test]
fn either_inventory_or_manifest_hint_can_prewarm_an_authorized_server() {
    for reason in [
        ActivationReason::Prewarm {
            inventory: true,
            manifest: false,
        },
        ActivationReason::Prewarm {
            inventory: false,
            manifest: true,
        },
    ] {
        let fixture = KeyFixture::new("config-a");
        let mut registry = ProcessRegistry::new(LifecyclePolicy::default());
        assert_eq!(
            registry.acquire(fixture.key.clone(), reason, NOW, true),
            vec![LifecycleAction::Start(fixture.key.clone())]
        );
    }
}

#[test]
fn configuration_replacement_drains_the_old_key_and_starts_the_new_key() {
    let old = KeyFixture::new("config-a");
    let replacement = old.rekey("config-b");
    let mut registry = ProcessRegistry::new(LifecyclePolicy::default());
    registry.acquire(old.key.clone(), ActivationReason::ToolRequest, NOW, true);
    registry.mark_ready(&old.key, NOW);

    let actions = registry.replace_configuration(replacement.clone(), NOW);

    assert_eq!(
        actions,
        vec![
            LifecycleAction::Stop(old.key.clone()),
            LifecycleAction::Start(replacement.clone()),
        ]
    );
    assert_eq!(
        registry.status(&old.key).expect("old").state,
        ProcessState::Stopping
    );
}

#[test]
fn trust_revocation_rejects_new_requests_and_stops_session_processes() {
    let fixture = KeyFixture::new("config-a");
    let mut registry = ProcessRegistry::new(LifecyclePolicy::default());
    registry.acquire(
        fixture.key.clone(),
        ActivationReason::ToolRequest,
        NOW,
        true,
    );

    assert_eq!(
        registry.revoke_session(fixture.key.session_root_ref()),
        vec![LifecycleAction::Stop(fixture.key.clone())]
    );
    assert_eq!(
        registry.acquire(
            fixture.key.clone(),
            ActivationReason::ToolRequest,
            NOW,
            false,
        ),
        vec![LifecycleAction::Reject(RejectionReason::Untrusted)]
    );
}

#[test]
fn unexpected_exit_fails_pending_work_and_uses_exponential_backoff() {
    let fixture = KeyFixture::new("config-a");
    let policy = LifecyclePolicy::new(
        3,
        Duration::from_secs(2),
        Duration::from_secs(30),
        Duration::from_secs(60),
        Duration::from_secs(600),
    );
    let mut registry = ProcessRegistry::new(policy);
    registry.acquire(
        fixture.key.clone(),
        ActivationReason::ToolRequest,
        NOW,
        true,
    );

    let first = registry.unexpected_exit(&fixture.key, NOW);
    assert_eq!(first[0], LifecycleAction::FailPending(fixture.key.clone()));
    assert_eq!(
        registry.status(&fixture.key).expect("first").restart_at,
        Some(NOW + Duration::from_secs(2))
    );
    registry.tick(NOW + Duration::from_secs(2));
    registry.unexpected_exit(&fixture.key, NOW + Duration::from_secs(3));
    assert_eq!(
        registry.status(&fixture.key).expect("second").restart_at,
        Some(NOW + Duration::from_secs(7))
    );
}

#[test]
fn restart_exhaustion_enters_cooldown_then_recovers_with_a_fresh_budget() {
    let fixture = KeyFixture::new("config-a");
    let policy = LifecyclePolicy::new(
        1,
        Duration::from_secs(1),
        Duration::from_secs(4),
        Duration::from_secs(20),
        Duration::from_secs(600),
    );
    let mut registry = ProcessRegistry::new(policy);
    registry.acquire(
        fixture.key.clone(),
        ActivationReason::ToolRequest,
        NOW,
        true,
    );
    registry.unexpected_exit(&fixture.key, NOW);
    registry.tick(NOW + Duration::from_secs(1));
    registry.unexpected_exit(&fixture.key, NOW + Duration::from_secs(2));

    let failed = registry.status(&fixture.key).expect("failed");
    assert_eq!(failed.state, ProcessState::Failed);
    assert_eq!(failed.cooldown_until, Some(NOW + Duration::from_secs(22)));
    assert!(registry.tick(NOW + Duration::from_secs(21)).is_empty());
    assert_eq!(
        registry.tick(NOW + Duration::from_secs(22)),
        vec![LifecycleAction::Start(fixture.key.clone())]
    );
    assert_eq!(
        registry
            .status(&fixture.key)
            .expect("recovered")
            .restart_count,
        0
    );
}

#[test]
fn restart_pool_consumes_the_exact_budget_before_entering_failed_state() {
    let fixture = KeyFixture::new("config-a");
    let policy = LifecyclePolicy::new(
        2,
        Duration::from_secs(1),
        Duration::from_secs(2),
        Duration::from_secs(30),
        Duration::from_secs(600),
    );
    let mut registry = ProcessRegistry::new(policy);
    registry.acquire(
        fixture.key.clone(),
        ActivationReason::ToolRequest,
        NOW,
        true,
    );

    registry.unexpected_exit(&fixture.key, NOW);
    assert_eq!(
        registry.status(&fixture.key).expect("first").restart_count,
        1
    );
    assert!(registry.tick(NOW).is_empty());
    assert_eq!(
        registry.tick(NOW + Duration::from_secs(1)),
        vec![LifecycleAction::Start(fixture.key.clone())]
    );

    registry.unexpected_exit(&fixture.key, NOW + Duration::from_secs(1));
    assert_eq!(
        registry.status(&fixture.key).expect("second").restart_count,
        2
    );
    assert!(registry.tick(NOW + Duration::from_secs(2)).is_empty());
    assert_eq!(
        registry.tick(NOW + Duration::from_secs(3)),
        vec![LifecycleAction::Start(fixture.key.clone())]
    );

    registry.unexpected_exit(&fixture.key, NOW + Duration::from_secs(3));
    let exhausted = registry.status(&fixture.key).expect("exhausted");
    assert_eq!(exhausted.state, ProcessState::Failed);
    assert_eq!(exhausted.restart_count, 2);
    assert_eq!(exhausted.restart_at, None);
    assert_eq!(
        exhausted.cooldown_until,
        Some(NOW + Duration::from_secs(33))
    );
}

#[test]
fn ready_idle_process_closes_after_ten_minutes_without_requests_or_leases() {
    let fixture = KeyFixture::new("config-a");
    let mut registry = ProcessRegistry::new(LifecyclePolicy::default());
    registry.acquire(
        fixture.key.clone(),
        ActivationReason::ToolRequest,
        NOW,
        true,
    );
    registry.mark_ready(&fixture.key, NOW);
    registry.release_request(&fixture.key, NOW);

    assert!(registry.tick(NOW + Duration::from_secs(599)).is_empty());
    assert_eq!(
        registry.tick(NOW + Duration::from_secs(600)),
        vec![LifecycleAction::Stop(fixture.key.clone())]
    );
}

#[test]
fn active_document_lease_prevents_idle_closure() {
    let fixture = KeyFixture::new("config-a");
    let mut registry = ProcessRegistry::new(LifecyclePolicy::default());
    registry.acquire(
        fixture.key.clone(),
        ActivationReason::ToolRequest,
        NOW,
        true,
    );
    registry.mark_ready(&fixture.key, NOW);
    registry.release_request(&fixture.key, NOW);
    registry.set_document_leases(&fixture.key, 1, NOW);

    assert!(registry.tick(NOW + Duration::from_secs(900)).is_empty());
}

struct KeyFixture {
    _directory: tempfile::TempDir,
    key: ProcessKey,
}

impl KeyFixture {
    fn new(fingerprint: &str) -> Self {
        let directory = tempfile::tempdir().expect("workspace");
        let project = directory.path().join("project");
        std::fs::create_dir(&project).expect("project root");
        let key = ProcessKey::new(
            directory.path(),
            &project,
            ServerKind::RustAnalyzer,
            ConfigurationFingerprint::new(fingerprint).expect("fingerprint"),
        )
        .expect("process key");
        Self {
            _directory: directory,
            key,
        }
    }

    fn rekey(&self, fingerprint: &str) -> ProcessKey {
        ProcessKey::new(
            self.key.session_root_ref(),
            self.key.project_root_ref(),
            ServerKind::RustAnalyzer,
            ConfigurationFingerprint::new(fingerprint).expect("fingerprint"),
        )
        .expect("replacement key")
    }
}
