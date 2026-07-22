use super::application::*;
use super::domain::{SshAuthMode, SshConnectionProfile, SshConnectionTestStatus};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use zeroize::Zeroizing;

#[derive(Default)]
struct FakeRepository {
    profile: Mutex<Option<SshConnectionProfile>>,
    fail_update: Mutex<bool>,
}

impl SshConnectionRepository for FakeRepository {
    fn list(&self) -> Result<Vec<SshConnectionProfile>, SshConnectionError> {
        Ok(self
            .profile
            .lock()
            .expect("profile")
            .clone()
            .into_iter()
            .collect())
    }

    fn find(&self, _id: &str) -> Result<Option<SshConnectionProfile>, SshConnectionError> {
        Ok(self.profile.lock().expect("profile").clone())
    }

    fn insert(&self, profile: &SshConnectionProfile) -> Result<(), SshConnectionError> {
        *self.profile.lock().expect("profile") = Some(profile.clone());
        Ok(())
    }

    fn update(&self, profile: &SshConnectionProfile) -> Result<(), SshConnectionError> {
        if *self.fail_update.lock().expect("fail update") {
            return Err(SshConnectionError::Repository(
                "forced update failure".to_string(),
            ));
        }
        *self.profile.lock().expect("profile") = Some(profile.clone());
        Ok(())
    }

    fn delete(&self, _id: &str) -> Result<(), SshConnectionError> {
        *self.profile.lock().expect("profile") = None;
        Ok(())
    }
}

#[derive(Default)]
struct FakeCredentials {
    passwords: Mutex<HashMap<String, String>>,
    fail_delete: Mutex<bool>,
}

impl SshConnectionCredentialPort for FakeCredentials {
    fn load(&self, id: &str) -> Result<Option<Zeroizing<String>>, SshConnectionError> {
        Ok(self
            .passwords
            .lock()
            .expect("passwords")
            .get(id)
            .cloned()
            .map(Zeroizing::new))
    }

    fn store(&self, id: &str, password: &str) -> Result<String, SshConnectionError> {
        self.passwords
            .lock()
            .expect("passwords")
            .insert(id.to_string(), password.to_string());
        Ok(format!("ssh-connection/{id}"))
    }

    fn delete(&self, id: &str) -> Result<(), SshConnectionError> {
        if *self.fail_delete.lock().expect("fail delete") {
            return Err(SshConnectionError::Credential(
                "forced delete failure".to_string(),
            ));
        }
        self.passwords.lock().expect("passwords").remove(id);
        Ok(())
    }
}

struct FakeTester;
impl SshConnectionTester for FakeTester {
    fn test(
        &self,
        _profile: &SshConnectionProfile,
        _password: Option<&str>,
    ) -> Result<String, SshConnectionError> {
        Ok("ok".to_string())
    }
}

struct FakeClock;
impl SshConnectionClock for FakeClock {
    fn now(&self) -> String {
        "2026-07-22T00:00:00Z".to_string()
    }
}

struct FakeIdentity;
impl SshConnectionIdentity for FakeIdentity {
    fn next_id(&self) -> String {
        "ssh-new".to_string()
    }
}

fn profile(auth_mode: SshAuthMode) -> SshConnectionProfile {
    SshConnectionProfile {
        id: "ssh-fixture".to_string(),
        name: "Fixture".to_string(),
        host: "host".to_string(),
        port: 22,
        user: "dev".to_string(),
        default_path: "/work".to_string(),
        auth_mode,
        key_path: (auth_mode == SshAuthMode::Key).then(|| "C:\\keys\\dev".to_string()),
        credential_ref: (auth_mode == SshAuthMode::Password)
            .then(|| "ssh-connection/ssh-fixture".to_string()),
        test_status: SshConnectionTestStatus::NotTested,
        last_connected_at: None,
        last_error: None,
        created_at: "2026-07-22T00:00:00Z".to_string(),
        updated_at: "2026-07-22T00:00:00Z".to_string(),
    }
}

fn mutation(auth_mode: SshAuthMode, password: Option<&str>) -> SshConnectionMutation {
    SshConnectionMutation {
        name: "Updated".to_string(),
        host: "host".to_string(),
        port: 2222,
        user: "dev".to_string(),
        default_path: "/work".to_string(),
        auth_mode,
        key_path: (auth_mode == SshAuthMode::Key).then(|| "C:\\keys\\dev".to_string()),
        password: password.map(str::to_string),
    }
}

fn service(
    repository: Arc<FakeRepository>,
    credentials: Arc<FakeCredentials>,
) -> SshConnectionApplicationService {
    SshConnectionApplicationService::new(
        repository,
        credentials,
        Arc::new(FakeTester),
        Arc::new(FakeClock),
        Arc::new(FakeIdentity),
    )
}

#[test]
fn failed_update_removes_new_password_when_none_existed() {
    let repository = Arc::new(FakeRepository::default());
    *repository.profile.lock().expect("profile") = Some(profile(SshAuthMode::Key));
    *repository.fail_update.lock().expect("fail update") = true;
    let credentials = Arc::new(FakeCredentials::default());

    let result = service(repository, credentials.clone()).update(
        "ssh-fixture",
        mutation(SshAuthMode::Password, Some("new-secret")),
    );

    assert!(result.is_err());
    assert!(credentials.passwords.lock().expect("passwords").is_empty());
}

#[test]
fn failed_update_restores_removed_password() {
    let repository = Arc::new(FakeRepository::default());
    *repository.profile.lock().expect("profile") = Some(profile(SshAuthMode::Password));
    *repository.fail_update.lock().expect("fail update") = true;
    let credentials = Arc::new(FakeCredentials::default());
    credentials
        .passwords
        .lock()
        .expect("passwords")
        .insert("ssh-fixture".to_string(), "old-secret".to_string());

    let result = service(repository, credentials.clone())
        .update("ssh-fixture", mutation(SshAuthMode::Key, None));

    assert!(result.is_err());
    assert_eq!(
        credentials
            .passwords
            .lock()
            .expect("passwords")
            .get("ssh-fixture"),
        Some(&"old-secret".to_string())
    );
}

#[test]
fn failed_credential_delete_restores_deleted_profile() {
    let repository = Arc::new(FakeRepository::default());
    *repository.profile.lock().expect("profile") = Some(profile(SshAuthMode::Password));
    let credentials = Arc::new(FakeCredentials::default());
    *credentials.fail_delete.lock().expect("fail delete") = true;

    let result = service(repository.clone(), credentials).delete("ssh-fixture");

    assert!(matches!(result, Err(SshConnectionError::Credential(_))));
    assert!(repository.profile.lock().expect("profile").is_some());
}
