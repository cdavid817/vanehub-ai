use super::super::application::CliConfigCredentialPort;
use super::super::domain::CliConfigError;
use crate::platform::credentials::OsCredentialStore;
use zeroize::Zeroizing;

const SERVICE_NAME: &str = "io.vanehub.ai.cli-config";

#[derive(Debug, Clone)]
pub(crate) struct OsCliConfigCredentialAdapter {
    store: OsCredentialStore,
}

impl OsCliConfigCredentialAdapter {
    pub(crate) fn new() -> Self {
        Self {
            store: OsCredentialStore::new(SERVICE_NAME),
        }
    }
}

impl CliConfigCredentialPort for OsCliConfigCredentialAdapter {
    fn load(
        &self,
        agent_id: &str,
        profile_id: &str,
    ) -> Result<Option<Zeroizing<String>>, CliConfigError> {
        self.store
            .get(&credential_account(agent_id, profile_id))
            .map_err(|_| CliConfigError::Credential)
    }

    fn store(&self, agent_id: &str, profile_id: &str, secret: &str) -> Result<(), CliConfigError> {
        self.store
            .set(&credential_account(agent_id, profile_id), secret)
            .map_err(|_| CliConfigError::Credential)
    }

    fn delete(&self, agent_id: &str, profile_id: &str) -> Result<(), CliConfigError> {
        self.store
            .delete(&credential_account(agent_id, profile_id))
            .map_err(|_| CliConfigError::Credential)
    }
}

fn credential_account(agent_id: &str, profile_id: &str) -> String {
    format!("cli-config/{agent_id}/{profile_id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credential_account_is_scoped_without_secret_material() {
        let account = credential_account("codex-cli", "openrouter");
        assert_eq!(account, "cli-config/codex-cli/openrouter");
        assert!(!account.contains("sk-"));
    }
}
