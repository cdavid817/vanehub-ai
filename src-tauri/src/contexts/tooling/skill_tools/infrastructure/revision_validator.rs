use super::FilesystemSkillToolSource;
use crate::contexts::tooling::skill_tools::application::{
    EffectiveSkillToolPackage, SkillToolApplicationError, SkillToolDiscoveryOutcome,
    SkillToolDiscoveryService, SkillToolEffectiveDiscoveryPort, SkillToolOwnerKind,
    SkillToolPackageRef, SkillToolRevisionState, SkillToolRevisionValidationPort,
};
use crate::contexts::tooling::skill_tools::domain::{SkillToolScope, DEFAULT_MANIFEST_LIMITS};
use crate::contexts::tooling::skills::api::{SkillApi, SkillLocation, SkillScope, SkillScopeQuery};

pub(crate) struct EffectiveSkillToolRevisionValidator {
    skills: SkillApi,
    source: FilesystemSkillToolSource,
}

impl EffectiveSkillToolRevisionValidator {
    pub(crate) fn new(skills: SkillApi) -> Self {
        Self {
            skills,
            source: FilesystemSkillToolSource::new(),
        }
    }
}

impl SkillToolRevisionValidationPort for EffectiveSkillToolRevisionValidator {
    fn validate(&self, state: &SkillToolRevisionState) -> Result<(), SkillToolApplicationError> {
        let scope = match state.key.source.scope {
            SkillToolScope::Global => SkillScope::Global,
            SkillToolScope::Workspace => SkillScope::Workspace,
        };
        let location = SkillLocation::new(scope, state.key.source.workspace_path.as_deref())
            .map_err(|error| SkillToolApplicationError::HostDenied(error.to_string()))?;
        let overview = self
            .skills
            .overview(SkillScopeQuery { location })
            .map_err(|error| SkillToolApplicationError::Filesystem(error.to_string()))?;
        let record = overview
            .skills
            .into_iter()
            .find(|record| {
                record.key.id.as_str() == state.key.owner.as_str()
                    && record.managed_source.content_hash == state.integrity.base_revision
            })
            .ok_or(SkillToolApplicationError::StaleRevision)?;
        let package = SkillToolPackageRef {
            owner: state.key.owner.clone(),
            source: state.key.source.clone(),
            base_revision: record.managed_source.content_hash,
            root_path: record.managed_source.skill_dir,
        };
        let outcome = SkillToolDiscoveryService::new(&self.source, DEFAULT_MANIFEST_LIMITS)
            .discover(&EffectiveSkillToolPackage {
                effective: package,
                shadowed: Vec::new(),
            })?;
        let exact = outcome
            .discovered
            .iter()
            .any(|tool| tool.key == state.key && tool.integrity == state.integrity);
        if exact && outcome.rejected.is_empty() && outcome.undeclared_files.is_empty() {
            Ok(())
        } else {
            Err(SkillToolApplicationError::StaleRevision)
        }
    }
}

impl SkillToolEffectiveDiscoveryPort for EffectiveSkillToolRevisionValidator {
    fn discover_effective(
        &self,
        owner: &SkillToolPackageRef,
    ) -> Result<
        (
            EffectiveSkillToolPackage,
            SkillToolDiscoveryOutcome,
            SkillToolOwnerKind,
        ),
        SkillToolApplicationError,
    > {
        let scope = match owner.source.scope {
            SkillToolScope::Global => SkillScope::Global,
            SkillToolScope::Workspace => SkillScope::Workspace,
        };
        let location = SkillLocation::new(scope, owner.source.workspace_path.as_deref())
            .map_err(|error| SkillToolApplicationError::HostDenied(error.to_string()))?;
        let overview = self
            .skills
            .overview(SkillScopeQuery { location })
            .map_err(|error| SkillToolApplicationError::Filesystem(error.to_string()))?;
        let record = overview
            .skills
            .into_iter()
            .find(|record| record.key.id.as_str() == owner.owner.as_str())
            .ok_or_else(|| SkillToolApplicationError::NotFound(owner.owner.as_str().to_string()))?;
        let owner_kind = if record.metadata.skill_type.as_str() == "utility" {
            SkillToolOwnerKind::Utility
        } else {
            SkillToolOwnerKind::Role
        };
        let package = SkillToolPackageRef {
            owner: owner.owner.clone(),
            source: owner.source.clone(),
            base_revision: record.managed_source.content_hash,
            root_path: record.managed_source.skill_dir,
        };
        let effective = EffectiveSkillToolPackage {
            effective: package,
            shadowed: Vec::new(),
        };
        SkillToolDiscoveryService::new(&self.source, DEFAULT_MANIFEST_LIMITS)
            .discover(&effective)
            .map(|outcome| (effective, outcome, owner_kind))
    }
}
