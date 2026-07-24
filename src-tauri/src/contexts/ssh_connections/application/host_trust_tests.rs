use super::host_trust::SshHostTrustService;
use super::runtime::{HostKeyVerification, RemoteSshError, RemoteSshHostKeyVerifierPort};
use super::{SshConnectionClock, SshConnectionError, SshConnectionRepository};
use crate::contexts::operations::api::{DiagnosticLog, DiagnosticLogPort, OperationsError};
use crate::contexts::ssh_connections::domain::runtime::{HostKeyChallengeKind, HostKeyEvidence};
use crate::contexts::ssh_connections::domain::{
    SshAuthMode, SshConnectionProfile, SshConnectionTestStatus, SshHostTrustMetadata,
};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct MemoryRepository {
    profile: Mutex<Option<SshConnectionProfile>>,
}

impl SshConnectionRepository for MemoryRepository {
    fn list(&self) -> Result<Vec<SshConnectionProfile>, SshConnectionError> {
        Ok(self
            .profile
            .lock()
            .expect("profile")
            .clone()
            .into_iter()
            .collect())
    }

    fn find(&self, id: &str) -> Result<Option<SshConnectionProfile>, SshConnectionError> {
        Ok(self
            .profile
            .lock()
            .expect("profile")
            .clone()
            .filter(|profile| profile.id == id))
    }

    fn insert(&self, profile: &SshConnectionProfile) -> Result<(), SshConnectionError> {
        *self.profile.lock().expect("profile") = Some(profile.clone());
        Ok(())
    }

    fn update(&self, profile: &SshConnectionProfile) -> Result<(), SshConnectionError> {
        self.insert(profile)
    }

    fn delete(&self, _id: &str) -> Result<(), SshConnectionError> {
        *self.profile.lock().expect("profile") = None;
        Ok(())
    }
}

struct FixedClock;

impl SshConnectionClock for FixedClock {
    fn now(&self) -> String {
        "2026-07-24T08:00:00.000Z".to_string()
    }
}

#[derive(Default)]
struct CapturingLogs {
    logs: Mutex<Vec<DiagnosticLog>>,
}

impl DiagnosticLogPort for CapturingLogs {
    fn write_diagnostic(&self, log: DiagnosticLog) -> Result<(), OperationsError> {
        self.logs.lock().expect("logs").push(log);
        Ok(())
    }
}

fn profile(trust: Option<SshHostTrustMetadata>) -> SshConnectionProfile {
    SshConnectionProfile {
        id: "ssh-fixture".to_string(),
        name: "Fixture".to_string(),
        host: "dev.example.com".to_string(),
        port: 22,
        user: "developer".to_string(),
        default_path: "/work/app".to_string(),
        auth_mode: SshAuthMode::Key,
        key_path: Some("C:\\private\\identity".to_string()),
        credential_ref: Some("credential-secret".to_string()),
        revision: 7,
        host_trust: trust,
        test_status: SshConnectionTestStatus::NotTested,
        last_connected_at: None,
        last_error: None,
        created_at: "2026-07-24T07:00:00.000Z".to_string(),
        updated_at: "2026-07-24T07:00:00.000Z".to_string(),
    }
}

fn harness(
    profile: SshConnectionProfile,
) -> (
    SshHostTrustService,
    Arc<MemoryRepository>,
    Arc<CapturingLogs>,
) {
    let repository = Arc::new(MemoryRepository {
        profile: Mutex::new(Some(profile)),
    });
    let logs = Arc::new(CapturingLogs::default());
    (
        SshHostTrustService::new(repository.clone(), Arc::new(FixedClock), logs.clone()),
        repository,
        logs,
    )
}

fn evidence(value: &str) -> HostKeyEvidence {
    HostKeyEvidence::new("ssh-ed25519", value).expect("evidence")
}

#[test]
fn first_seen_challenges_are_deduplicated_and_confirmed_explicitly() {
    let (service, repository, logs) = harness(profile(None));
    let current = repository
        .find("ssh-fixture")
        .expect("find")
        .expect("profile");
    let first = service
        .verify(&current, &evidence("SHA256:first"))
        .expect("first challenge");
    let duplicate = service
        .verify(&current, &evidence("SHA256:first"))
        .expect("duplicate challenge");

    assert_eq!(first, duplicate);
    let HostKeyVerification::Challenge(challenge) = first else {
        panic!("challenge expected");
    };
    assert_eq!(challenge.kind, HostKeyChallengeKind::FirstSeen);
    service
        .confirm("ssh-fixture", 7, "SHA256:first")
        .expect("confirm");
    assert!(service.pending_for("ssh-fixture").is_none());
    assert_eq!(
        repository
            .find("ssh-fixture")
            .expect("find")
            .expect("profile")
            .host_trust
            .expect("trust")
            .confirmed_at,
        "2026-07-24T08:00:00.000Z"
    );
    assert!(logs.logs.lock().expect("logs").len() >= 2);
}

#[test]
fn changed_keys_block_authentication_and_stale_confirmation() {
    let trust = SshHostTrustMetadata {
        host: "dev.example.com".to_string(),
        port: 22,
        algorithm: "ssh-ed25519".to_string(),
        fingerprint: "SHA256:old".to_string(),
        confirmed_at: "2026-07-23T08:00:00.000Z".to_string(),
    };
    let (service, repository, _) = harness(profile(Some(trust)));
    let current = repository
        .find("ssh-fixture")
        .expect("find")
        .expect("profile");
    let HostKeyVerification::Challenge(challenge) = service
        .verify(&current, &evidence("SHA256:new"))
        .expect("changed challenge")
    else {
        panic!("challenge expected");
    };
    assert_eq!(challenge.kind, HostKeyChallengeKind::Changed);
    assert_eq!(
        challenge.previous_fingerprint.as_deref(),
        Some("SHA256:old")
    );

    let mut edited = current;
    edited.revision = 8;
    repository.update(&edited).expect("edit profile");
    assert_eq!(
        service.confirm("ssh-fixture", 7, "SHA256:new"),
        Err(RemoteSshError::StaleProfile)
    );
}

#[test]
fn diagnostics_exclude_credentials_and_private_key_paths() {
    let (service, repository, logs) = harness(profile(None));
    let current = repository
        .find("ssh-fixture")
        .expect("find")
        .expect("profile");
    service
        .verify(&current, &evidence("SHA256:safe"))
        .expect("challenge");
    let rendered = format!("{:?}", logs.logs.lock().expect("logs"));
    assert!(!rendered.contains("credential-secret"));
    assert!(!rendered.contains("private\\identity"));
}
