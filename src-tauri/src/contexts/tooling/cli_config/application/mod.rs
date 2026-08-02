use super::domain::{
    AppliedStateRecord, CliConfigDriftState, CliConfigError, CliConfigPayload, ProfileRecord,
};
use std::path::PathBuf;
use zeroize::Zeroizing;

pub(crate) trait CliConfigRepository: Send + Sync {
    fn list_profiles(&self, agent_id: &str) -> Result<Vec<ProfileRecord>, CliConfigError>;
    fn get_profile(
        &self,
        agent_id: &str,
        profile_id: &str,
    ) -> Result<ProfileRecord, CliConfigError>;
    fn profile_id_exists(&self, profile_id: &str) -> Result<bool, CliConfigError>;
    fn save_profile(&self, profile: &ProfileRecord) -> Result<(), CliConfigError>;
    fn delete_profile(&self, agent_id: &str, profile_id: &str) -> Result<(), CliConfigError>;
    fn applied_state(&self, agent_id: &str) -> Result<Option<AppliedStateRecord>, CliConfigError>;
    fn save_applied_state(&self, state: &AppliedStateRecord) -> Result<(), CliConfigError>;
    fn clear_applied_state(&self, agent_id: &str) -> Result<(), CliConfigError>;
}

pub(crate) trait CliConfigCredentialPort: Send + Sync {
    fn load(
        &self,
        agent_id: &str,
        profile_id: &str,
    ) -> Result<Option<Zeroizing<String>>, CliConfigError>;
    fn store(&self, agent_id: &str, profile_id: &str, secret: &str) -> Result<(), CliConfigError>;
    fn delete(&self, agent_id: &str, profile_id: &str) -> Result<(), CliConfigError>;
}

#[derive(Debug, Clone)]
pub(crate) struct LiveInspection {
    pub(crate) paths: Vec<PathBuf>,
    pub(crate) state: CliConfigDriftState,
    pub(crate) managed_fingerprint: String,
}

#[derive(Debug)]
pub(crate) struct ImportedLiveConfig {
    pub(crate) payload: CliConfigPayload,
    pub(crate) credential: Option<Zeroizing<String>>,
    pub(crate) source_fingerprint: String,
}

#[derive(Debug)]
pub(crate) struct DiscoveredLiveConfig {
    pub(crate) candidate_key: String,
    pub(crate) suggested_name: String,
    pub(crate) provider_name: String,
    pub(crate) endpoint: String,
    pub(crate) model: String,
    pub(crate) is_default: bool,
    pub(crate) payload: CliConfigPayload,
    pub(crate) credential: Option<Zeroizing<String>>,
}

#[derive(Debug)]
pub(crate) struct LiveConfigDiscovery {
    pub(crate) paths: Vec<PathBuf>,
    pub(crate) candidates: Vec<DiscoveredLiveConfig>,
    pub(crate) warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ProjectionOutcome {
    pub(crate) paths: Vec<PathBuf>,
    pub(crate) projection_fingerprint: String,
    pub(crate) live_fingerprint: String,
    pub(crate) warnings: Vec<String>,
    pub(crate) restored: bool,
    pub(crate) backups: Vec<(PathBuf, Option<Vec<u8>>)>,
}

pub(crate) trait CliGlobalConfigPort: Send + Sync {
    fn paths(&self, agent_id: &str) -> Result<Vec<PathBuf>, CliConfigError>;
    fn inspect(
        &self,
        agent_id: &str,
        applied: Option<&AppliedStateRecord>,
        applied_profile: Option<&ProfileRecord>,
    ) -> Result<LiveInspection, CliConfigError>;
    fn import_current(&self, agent_id: &str) -> Result<ImportedLiveConfig, CliConfigError>;
    fn discover_current(&self, agent_id: &str) -> Result<LiveConfigDiscovery, CliConfigError> {
        Ok(LiveConfigDiscovery {
            paths: self.paths(agent_id)?,
            candidates: Vec::new(),
            warnings: Vec::new(),
        })
    }
    fn apply(
        &self,
        profile: &ProfileRecord,
        previous: Option<&ProfileRecord>,
        credential: Option<&str>,
        confirm_auth_file_replacement: bool,
        expected_live_fingerprint: &str,
    ) -> Result<ProjectionOutcome, CliConfigError>;
    fn restore(&self, outcome: &ProjectionOutcome) -> Result<(), CliConfigError>;
}
