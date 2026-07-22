use super::application::{
    SshConnectionApplicationService, SshConnectionError, SshConnectionMutation,
    SshConnectionTestResult,
};
use super::domain::SshConnectionProfile;

pub(crate) use super::application::SshConnectionError as SshConnectionsError;
pub(crate) use super::application::SshConnectionMutation as SaveSshConnectionRequest;
pub(crate) use super::domain::{SshAuthMode, SshConnectionTestStatus};

#[derive(Clone)]
pub(crate) struct SshConnectionsApi {
    service: SshConnectionApplicationService,
}

impl SshConnectionsApi {
    pub(crate) fn new(service: SshConnectionApplicationService) -> Self {
        Self { service }
    }

    pub(crate) fn list(&self) -> Result<Vec<SshConnectionProfile>, SshConnectionError> {
        self.service.list()
    }

    pub(crate) fn create(
        &self,
        mutation: SshConnectionMutation,
    ) -> Result<SshConnectionProfile, SshConnectionError> {
        self.service.create(mutation)
    }

    pub(crate) fn update(
        &self,
        id: &str,
        mutation: SshConnectionMutation,
    ) -> Result<SshConnectionProfile, SshConnectionError> {
        self.service.update(id, mutation)
    }

    pub(crate) fn delete(&self, id: &str) -> Result<(), SshConnectionError> {
        self.service.delete(id)
    }

    pub(crate) fn test(&self, id: &str) -> Result<SshConnectionTestResult, SshConnectionError> {
        self.service.test(id)
    }
}
