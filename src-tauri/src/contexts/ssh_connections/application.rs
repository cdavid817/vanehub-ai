use super::domain::{
    SshAuthMode, SshConnectionDomainError, SshConnectionDraft, SshConnectionProfile,
    SshConnectionTestStatus,
};
use std::sync::Arc;
use thiserror::Error;
use zeroize::Zeroizing;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub(crate) enum SshConnectionError {
    #[error("{0}")]
    Domain(#[from] SshConnectionDomainError),
    #[error("SSH connection not found: {0}")]
    NotFound(String),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("database error: {0}")]
    Repository(String),
    #[error("storage error: {0}")]
    Credential(String),
    #[error("launch failed: {0}")]
    Test(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SshConnectionMutation {
    pub(crate) name: String,
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) user: String,
    pub(crate) default_path: String,
    pub(crate) auth_mode: SshAuthMode,
    pub(crate) key_path: Option<String>,
    pub(crate) password: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SshConnectionTestResult {
    pub(crate) status: SshConnectionTestStatus,
    pub(crate) message: String,
    pub(crate) tested_at: String,
}

pub(crate) trait SshConnectionRepository: Send + Sync {
    fn list(&self) -> Result<Vec<SshConnectionProfile>, SshConnectionError>;
    fn find(&self, id: &str) -> Result<Option<SshConnectionProfile>, SshConnectionError>;
    fn insert(&self, profile: &SshConnectionProfile) -> Result<(), SshConnectionError>;
    fn update(&self, profile: &SshConnectionProfile) -> Result<(), SshConnectionError>;
    fn delete(&self, id: &str) -> Result<(), SshConnectionError>;
}

pub(crate) trait SshConnectionCredentialPort: Send + Sync {
    fn load(&self, id: &str) -> Result<Option<Zeroizing<String>>, SshConnectionError>;
    fn store(&self, id: &str, password: &str) -> Result<String, SshConnectionError>;
    fn delete(&self, id: &str) -> Result<(), SshConnectionError>;
}

pub(crate) trait SshConnectionTester: Send + Sync {
    fn test(
        &self,
        profile: &SshConnectionProfile,
        password: Option<&str>,
    ) -> Result<String, SshConnectionError>;
}

pub(crate) trait SshConnectionClock: Send + Sync {
    fn now(&self) -> String;
}

pub(crate) trait SshConnectionIdentity: Send + Sync {
    fn next_id(&self) -> String;
}

#[derive(Clone)]
pub(crate) struct SshConnectionApplicationService {
    repository: Arc<dyn SshConnectionRepository>,
    credentials: Arc<dyn SshConnectionCredentialPort>,
    tester: Arc<dyn SshConnectionTester>,
    clock: Arc<dyn SshConnectionClock>,
    identity: Arc<dyn SshConnectionIdentity>,
}

impl SshConnectionApplicationService {
    pub(crate) fn new(
        repository: Arc<dyn SshConnectionRepository>,
        credentials: Arc<dyn SshConnectionCredentialPort>,
        tester: Arc<dyn SshConnectionTester>,
        clock: Arc<dyn SshConnectionClock>,
        identity: Arc<dyn SshConnectionIdentity>,
    ) -> Self {
        Self {
            repository,
            credentials,
            tester,
            clock,
            identity,
        }
    }

    pub(crate) fn list(&self) -> Result<Vec<SshConnectionProfile>, SshConnectionError> {
        self.repository.list()
    }

    pub(crate) fn create(
        &self,
        mutation: SshConnectionMutation,
    ) -> Result<SshConnectionProfile, SshConnectionError> {
        let draft = draft_from_mutation(&mutation);
        draft.validate()?;
        if draft.auth_mode == SshAuthMode::Password
            && mutation.password.as_deref().unwrap_or("").trim().is_empty()
        {
            return Err(SshConnectionError::Validation(
                "SSH password is required for password authentication.".to_string(),
            ));
        }
        let id = self.identity.next_id();
        let credential_ref = match (draft.auth_mode, mutation.password.as_deref()) {
            (SshAuthMode::Password, Some(password)) => Some(self.credentials.store(&id, password)?),
            _ => None,
        };
        let now = self.clock.now();
        let profile = SshConnectionProfile {
            id,
            name: draft.name.trim().to_string(),
            host: draft.host.trim().to_string(),
            port: draft.port,
            user: draft.user.trim().to_string(),
            default_path: draft.default_path.trim().to_string(),
            auth_mode: draft.auth_mode,
            key_path: draft.key_path.map(|value| value.trim().to_string()),
            credential_ref,
            test_status: SshConnectionTestStatus::NotTested,
            last_connected_at: None,
            last_error: None,
            created_at: now.clone(),
            updated_at: now,
        };
        profile.validate()?;
        if let Err(error) = self.repository.insert(&profile) {
            let _ = self.credentials.delete(&profile.id);
            return Err(error);
        }
        Ok(profile)
    }

    pub(crate) fn update(
        &self,
        id: &str,
        mutation: SshConnectionMutation,
    ) -> Result<SshConnectionProfile, SshConnectionError> {
        let current = self
            .repository
            .find(id)?
            .ok_or_else(|| SshConnectionError::NotFound(id.to_string()))?;
        let draft = draft_from_mutation(&mutation);
        draft.validate()?;
        let previous_password = self.credentials.load(id)?;
        let wrote_password = draft.auth_mode == SshAuthMode::Password
            && mutation
                .password
                .as_deref()
                .is_some_and(|password| !password.trim().is_empty());
        let credential_ref = match draft.auth_mode {
            SshAuthMode::Password => match mutation.password.as_deref() {
                Some(password) if !password.trim().is_empty() => {
                    Some(self.credentials.store(id, password)?)
                }
                _ => current.credential_ref.clone(),
            },
            SshAuthMode::Key => {
                self.credentials.delete(id)?;
                None
            }
        };
        if draft.auth_mode == SshAuthMode::Password && credential_ref.is_none() {
            return Err(SshConnectionError::Validation(
                "SSH password is required for password authentication.".to_string(),
            ));
        }
        let mut updated = current;
        updated.name = draft.name.trim().to_string();
        updated.host = draft.host.trim().to_string();
        updated.port = draft.port;
        updated.user = draft.user.trim().to_string();
        updated.default_path = draft.default_path.trim().to_string();
        updated.auth_mode = draft.auth_mode;
        updated.key_path = draft.key_path.map(|value| value.trim().to_string());
        updated.credential_ref = credential_ref;
        updated.updated_at = self.clock.now();
        updated.validate()?;
        if let Err(error) = self.repository.update(&updated) {
            self.restore_credential(id, previous_password.as_deref(), wrote_password)?;
            return Err(error);
        }
        Ok(updated)
    }

    pub(crate) fn delete(&self, id: &str) -> Result<(), SshConnectionError> {
        let profile = self
            .repository
            .find(id)?
            .ok_or_else(|| SshConnectionError::NotFound(id.to_string()))?;
        self.repository.delete(profile.id.as_str())?;
        if let Err(credential_error) = self.credentials.delete(profile.id.as_str()) {
            self.repository.insert(&profile).map_err(|restore_error| {
                SshConnectionError::Repository(format!(
                    "SSH profile restore failed after credential deletion failure: {restore_error}"
                ))
            })?;
            return Err(credential_error);
        }
        Ok(())
    }

    pub(crate) fn test(&self, id: &str) -> Result<SshConnectionTestResult, SshConnectionError> {
        let mut profile = self
            .repository
            .find(id)?
            .ok_or_else(|| SshConnectionError::NotFound(id.to_string()))?;
        let password = if profile.auth_mode == SshAuthMode::Password {
            self.credentials.load(id)?
        } else {
            None
        };
        let tested_at = self.clock.now();
        let result = self
            .tester
            .test(&profile, password.as_deref().map(String::as_str));
        match result {
            Ok(message) => {
                profile.test_status = SshConnectionTestStatus::Succeeded;
                profile.last_connected_at = Some(tested_at.clone());
                profile.last_error = None;
                profile.updated_at = tested_at.clone();
                self.repository.update(&profile)?;
                Ok(SshConnectionTestResult {
                    status: SshConnectionTestStatus::Succeeded,
                    message,
                    tested_at,
                })
            }
            Err(error) => {
                let message = error.to_string();
                profile.test_status = SshConnectionTestStatus::Failed;
                profile.last_error = Some(message.clone());
                profile.updated_at = tested_at.clone();
                self.repository.update(&profile)?;
                Err(SshConnectionError::Test(message))
            }
        }
    }

    fn restore_credential(
        &self,
        id: &str,
        previous_password: Option<&String>,
        wrote_password: bool,
    ) -> Result<(), SshConnectionError> {
        if let Some(password) = previous_password {
            self.credentials.store(id, password)?;
        } else if wrote_password {
            self.credentials.delete(id)?;
        }
        Ok(())
    }
}

fn draft_from_mutation(mutation: &SshConnectionMutation) -> SshConnectionDraft {
    SshConnectionDraft {
        name: mutation.name.clone(),
        host: mutation.host.clone(),
        port: mutation.port,
        user: mutation.user.clone(),
        default_path: mutation.default_path.clone(),
        auth_mode: mutation.auth_mode,
        key_path: mutation.key_path.clone(),
    }
}

pub(crate) struct UuidSshConnectionIdentity;

impl SshConnectionIdentity for UuidSshConnectionIdentity {
    fn next_id(&self) -> String {
        format!("ssh-{}", uuid::Uuid::new_v4().simple())
    }
}
