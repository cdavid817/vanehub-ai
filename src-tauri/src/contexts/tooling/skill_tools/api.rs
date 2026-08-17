//! Published contract for consumers of Skill tool state.
//!
//! Only the bounded, byte-free projections are exposed. Manifests, module content, integrity
//! verification, and governance operations stay behind the application layer, so a consumer can
//! render a Skill's tool inventory without gaining a way to read or run any of it.

#[allow(unused_imports)]
pub(crate) use super::application::{
    project_inventory_summary, DiscoveredSkillTool, SkillToolDiscoveryOutcome,
    SkillToolInventoryEntry, SkillToolInventorySummary, SkillToolRevisionState,
    MAX_INVENTORY_ENTRIES,
};

pub(crate) use super::application::SkillToolApplicationError;
use super::application::{
    SkillToolCatalogCandidate, SkillToolCatalogEntry, SkillToolClockPort,
    SkillToolEffectiveDiscoveryPort, SkillToolGovernanceService, SkillToolLogAction,
    SkillToolLogEvent, SkillToolLogLevel, SkillToolLoggingPort, SkillToolRegistry,
    SkillToolRegistryRefreshCause, SkillToolRevisionValidationPort, SkillToolStateRepository,
};
use super::domain::{SkillToolIntegrity, SkillToolKey, SkillToolTrustDecision};
use std::sync::Arc;

impl crate::contexts::tooling::skills::api::SkillRuntimeObserver for SkillToolApi {
    fn skill_changed(
        &self,
        key: &crate::contexts::tooling::skills::api::SkillKey,
        mutation: crate::contexts::tooling::skills::api::SkillRuntimeMutation,
    ) {
        let Ok(owner) = super::domain::SkillToolOwnerId::parse(key.id.as_str()) else {
            return;
        };
        if matches!(
            mutation,
            crate::contexts::tooling::skills::api::SkillRuntimeMutation::Delete
        ) {
            let _ = self
                .registry
                .remove_owner(SkillToolRegistryRefreshCause::Delete, &owner);
            return;
        }
        let source = match key.location.scope {
            crate::contexts::tooling::skills::api::SkillScope::Global => {
                super::domain::SkillToolSourceScope::global()
            }
            crate::contexts::tooling::skills::api::SkillScope::Workspace => {
                match super::domain::SkillToolSourceScope::new(
                    super::domain::SkillToolScope::Workspace,
                    key.location.workspace_path.as_deref(),
                ) {
                    Ok(source) => source,
                    Err(_) => return,
                }
            }
        };
        let package = SkillToolPackageRef {
            owner,
            source,
            base_revision: String::new(),
            root_path: String::new(),
        };
        let _ = self.synchronize(&package, SkillToolRegistryRefreshCause::Replacement);
    }
}

pub(crate) use super::application::SkillToolPackageRef;
#[allow(unused_imports)]
pub(crate) use super::domain::{
    SkillToolId, SkillToolOwnerId, SkillToolRevision, SkillToolScope, SkillToolSourceScope,
};

#[derive(Clone)]
pub(crate) struct SkillToolApi {
    repository: Arc<dyn SkillToolStateRepository>,
    validator: Arc<dyn SkillToolRevisionValidationPort>,
    clock: Arc<dyn SkillToolClockPort>,
    logging: Arc<dyn SkillToolLoggingPort>,
    discovery: Arc<dyn SkillToolEffectiveDiscoveryPort>,
    registry: Arc<SkillToolRegistry>,
}

impl SkillToolApi {
    pub(crate) fn new(
        repository: Arc<dyn SkillToolStateRepository>,
        validator: Arc<dyn SkillToolRevisionValidationPort>,
        clock: Arc<dyn SkillToolClockPort>,
        logging: Arc<dyn SkillToolLoggingPort>,
        discovery: Arc<dyn SkillToolEffectiveDiscoveryPort>,
        registry: Arc<SkillToolRegistry>,
    ) -> Self {
        Self {
            repository,
            validator,
            clock,
            logging,
            discovery,
            registry,
        }
    }

    fn service(&self) -> SkillToolGovernanceService<'_> {
        SkillToolGovernanceService::new(
            self.repository.as_ref(),
            self.validator.as_ref(),
            self.clock.as_ref(),
        )
    }

    pub(crate) fn registry(&self) -> Arc<SkillToolRegistry> {
        self.registry.clone()
    }

    pub(crate) fn repository(&self) -> Arc<dyn SkillToolStateRepository> {
        self.repository.clone()
    }

    pub(crate) fn discovery(&self) -> Arc<dyn SkillToolEffectiveDiscoveryPort> {
        self.discovery.clone()
    }

    pub(crate) fn logging(&self) -> Arc<dyn SkillToolLoggingPort> {
        self.logging.clone()
    }

    pub(crate) fn clock(&self) -> Arc<dyn SkillToolClockPort> {
        self.clock.clone()
    }

    pub(crate) fn list(
        &self,
        owner: &SkillToolPackageRef,
    ) -> Result<Vec<SkillToolRevisionState>, SkillToolApplicationError> {
        let result = self.synchronize(owner, SkillToolRegistryRefreshCause::EffectiveScope);
        let count = result.as_ref().ok().map(|states| states.len().to_string());
        self.record(
            SkillToolLogAction::Discover,
            Some(owner.owner.as_str()),
            None,
            None,
            count.as_deref(),
            result.as_ref().err(),
        );
        result
    }

    pub(crate) fn validate(
        &self,
        revision: &SkillToolRevision,
    ) -> Result<SkillToolRevisionState, SkillToolApplicationError> {
        let result = self.service().validate(revision);
        if let Ok(state) = &result {
            let _ = self.refresh_state_owner(state, SkillToolRegistryRefreshCause::Validation);
        }
        self.record(
            SkillToolLogAction::Validate,
            None,
            None,
            Some(revision.as_str()),
            None,
            result.as_ref().err(),
        );
        result
    }

    pub(crate) fn decide_trust(
        &self,
        key: &SkillToolKey,
        integrity: &SkillToolIntegrity,
        actor: &str,
        trusted: bool,
    ) -> Result<SkillToolRevisionState, SkillToolApplicationError> {
        let result = self.service().decide_trust(
            key,
            integrity,
            actor,
            if trusted {
                SkillToolTrustDecision::Trusted
            } else {
                SkillToolTrustDecision::Revoked
            },
        );
        if let Ok(state) = &result {
            let _ = self.refresh_state_owner(state, SkillToolRegistryRefreshCause::Trust);
        }
        self.record(
            if trusted {
                SkillToolLogAction::Trust
            } else {
                SkillToolLogAction::Revoke
            },
            Some(key.owner.as_str()),
            Some(key.tool.as_str()),
            Some(key.revision.as_str()),
            None,
            result.as_ref().err(),
        );
        result
    }

    pub(crate) fn set_enabled(
        &self,
        revision: &SkillToolRevision,
        enabled: bool,
    ) -> Result<SkillToolRevisionState, SkillToolApplicationError> {
        let result = self.service().set_enabled(revision, enabled);
        if let Ok(state) = &result {
            let _ = self.refresh_state_owner(state, SkillToolRegistryRefreshCause::Enablement);
        }
        self.record(
            SkillToolLogAction::SetEnabled,
            None,
            None,
            Some(revision.as_str()),
            Some(if enabled { "enabled" } else { "disabled" }),
            result.as_ref().err(),
        );
        result
    }

    pub(crate) fn quarantine(
        &self,
        revision: &SkillToolRevision,
        reason: &str,
    ) -> Result<SkillToolRevisionState, SkillToolApplicationError> {
        let result = self.service().quarantine(revision, reason);
        if let Ok(state) = &result {
            let _ = self.refresh_state_owner(state, SkillToolRegistryRefreshCause::Quarantine);
        }
        self.record(
            SkillToolLogAction::Quarantine,
            None,
            None,
            Some(revision.as_str()),
            Some("manual-security-review"),
            result.as_ref().err(),
        );
        result
    }

    pub(crate) fn recover(
        &self,
        revision: &SkillToolRevision,
    ) -> Result<SkillToolRevisionState, SkillToolApplicationError> {
        let result = self.service().recover(revision);
        if let Ok(state) = &result {
            let _ = self.refresh_state_owner(state, SkillToolRegistryRefreshCause::Restore);
        }
        self.record(
            SkillToolLogAction::Recover,
            None,
            None,
            Some(revision.as_str()),
            None,
            result.as_ref().err(),
        );
        result
    }

    pub(crate) fn diagnostics(
        &self,
        revision: &SkillToolRevision,
    ) -> Result<SkillToolRevisionState, SkillToolApplicationError> {
        self.service().diagnostics(revision)
    }

    fn synchronize(
        &self,
        owner: &SkillToolPackageRef,
        cause: SkillToolRegistryRefreshCause,
    ) -> Result<Vec<SkillToolRevisionState>, SkillToolApplicationError> {
        let (_, outcome, owner_kind) = self.discovery.discover_effective(owner)?;
        let now = self.clock.now();
        for tool in &outcome.discovered {
            self.repository.record_discovered(&SkillToolRevisionState {
                key: tool.key.clone(),
                integrity: tool.integrity.clone(),
                implementation_kind: tool.declaration.implementation.kind().to_string(),
                lifecycle: Default::default(),
                validation_code: None,
                diagnostics: Default::default(),
                created_at: now.clone(),
                updated_at: now.clone(),
            })?;
        }
        let states = self.repository.revision_states(owner)?;
        let by_revision = states
            .iter()
            .map(|state| (state.key.revision.clone(), state))
            .collect::<std::collections::HashMap<_, _>>();
        let candidates = outcome
            .discovered
            .into_iter()
            .filter_map(|tool| {
                let state = by_revision.get(&tool.key.revision)?;
                Some(SkillToolCatalogCandidate {
                    entry: SkillToolCatalogEntry {
                        canonical_name: tool.canonical_name,
                        description: tool.declaration.description,
                        input_schema: tool.declaration.input.as_value().clone(),
                        key: tool.key,
                    },
                    owner_kind,
                    lifecycle: state.lifecycle.clone(),
                    archived: false,
                    shadowed: false,
                    requires_module_runtime: tool.requires_module_runtime,
                    allow_plan: tool.declaration.capabilities.iter().all(|capability| {
                        capability.operation().starts_with("read")
                            || matches!(capability.operation(), "grep" | "glob" | "search_code")
                    }),
                })
            })
            .collect();
        let snapshot = self
            .registry
            .replace_owner(cause, &owner.owner, candidates)?;
        self.record(
            SkillToolLogAction::RegistryRefresh,
            Some(owner.owner.as_str()),
            None,
            None,
            Some(&snapshot.generation.to_string()),
            None,
        );
        Ok(states)
    }

    fn refresh_state_owner(
        &self,
        state: &SkillToolRevisionState,
        cause: SkillToolRegistryRefreshCause,
    ) -> Result<(), SkillToolApplicationError> {
        let owner = SkillToolPackageRef {
            owner: state.key.owner.clone(),
            source: state.key.source.clone(),
            base_revision: state.integrity.base_revision.clone(),
            root_path: String::new(),
        };
        self.synchronize(&owner, cause).map(|_| ())
    }

    #[allow(clippy::too_many_arguments)]
    fn record(
        &self,
        action: SkillToolLogAction,
        skill_id: Option<&str>,
        tool_id: Option<&str>,
        revision: Option<&str>,
        outcome: Option<&str>,
        error: Option<&SkillToolApplicationError>,
    ) {
        let mut context = std::collections::BTreeMap::new();
        context.insert(
            "outcome".to_string(),
            error.map_or_else(
                || outcome.unwrap_or("ok").to_string(),
                |error| error.code().to_string(),
            ),
        );
        let _ = self.logging.record(&SkillToolLogEvent {
            action,
            level: if error.is_some() {
                SkillToolLogLevel::Warn
            } else {
                SkillToolLogLevel::Info
            },
            skill_id: skill_id.map(str::to_string),
            tool_id: tool_id.map(str::to_string),
            revision: revision.map(str::to_string),
            message: action.as_str().to_string(),
            context,
        });
    }
}
