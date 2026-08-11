use crate::contexts::code_intelligence::domain::configuration::LspConfiguration;
use crate::contexts::code_intelligence::domain::models::{DomainModelError, WorkspaceTrust};
use std::path::Path;

pub(crate) trait LspConfigurationRepository: Send + Sync {
    fn load_configuration(&self) -> Result<LspConfiguration, DomainModelError>;
    fn save_configuration(&self, configuration: &LspConfiguration) -> Result<(), DomainModelError>;
}

pub(crate) trait WorkspaceTrustRepository: Send + Sync {
    fn list_workspace_trust(&self) -> Result<Vec<WorkspaceTrust>, DomainModelError>;
    fn set_workspace_trust(
        &self,
        workspace_root: &Path,
        trusted: bool,
    ) -> Result<WorkspaceTrust, DomainModelError>;
}
