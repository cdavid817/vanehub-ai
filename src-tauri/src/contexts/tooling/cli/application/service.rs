use super::{
    CliApplicationError, CliClockPort, CliDetectionPort, CliExecutableLocatorPort, CliLogCategory,
    CliLogEvent, CliLogLevel, CliLoggingPort, CliMutationPort, CliOperationPort,
    CliOperationRequest, CliOperationResult, CliOperationType, CliPackagePort, CliStatusRepository,
    CliToolStatus, PreparedCliInstall, PreparedCliRefresh, PreparedCliUpgradeAll,
};
use crate::contexts::tooling::cli::domain::{
    compare_versions, definition, is_stable_version, LifecycleEligibility, ToolDefinition,
    CLI_TOOL_DEFINITIONS,
};
use std::cmp::Ordering;
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct CliApplicationService {
    repository: Arc<dyn CliStatusRepository>,
    detection: Arc<dyn CliDetectionPort>,
    executable_locator: Arc<dyn CliExecutableLocatorPort>,
    packages: Arc<dyn CliPackagePort>,
    operations: Arc<dyn CliOperationPort>,
    logging: Arc<dyn CliLoggingPort>,
    clock: Arc<dyn CliClockPort>,
    mutations: Arc<dyn CliMutationPort>,
}

pub(crate) struct CliApplicationPorts {
    pub(crate) repository: Arc<dyn CliStatusRepository>,
    pub(crate) detection: Arc<dyn CliDetectionPort>,
    pub(crate) executable_locator: Arc<dyn CliExecutableLocatorPort>,
    pub(crate) packages: Arc<dyn CliPackagePort>,
    pub(crate) operations: Arc<dyn CliOperationPort>,
    pub(crate) logging: Arc<dyn CliLoggingPort>,
    pub(crate) clock: Arc<dyn CliClockPort>,
    pub(crate) mutations: Arc<dyn CliMutationPort>,
}

impl CliApplicationService {
    pub(crate) fn new(ports: CliApplicationPorts) -> Self {
        Self {
            repository: ports.repository,
            detection: ports.detection,
            executable_locator: ports.executable_locator,
            packages: ports.packages,
            operations: ports.operations,
            logging: ports.logging,
            clock: ports.clock,
            mutations: ports.mutations,
        }
    }

    pub(crate) fn list_tools(&self) -> Result<Vec<CliToolStatus>, CliApplicationError> {
        CLI_TOOL_DEFINITIONS
            .iter()
            .copied()
            .map(|definition| self.repository.load(definition))
            .collect()
    }

    pub(crate) fn needs_initial_refresh(&self) -> Result<bool, CliApplicationError> {
        self.repository
            .has_cached_statuses()
            .map(|has_cached_statuses| !has_cached_statuses)
    }

    pub(crate) fn resolve_executable(
        &self,
        agent_id: &str,
    ) -> Result<Option<String>, CliApplicationError> {
        let definition = definition(agent_id).ok_or_else(|| unsupported_agent_error(agent_id))?;
        let status = self.repository.load(definition)?;
        Ok(self
            .executable_locator
            .resolve(definition, status.detected_path.as_deref()))
    }

    pub(crate) fn prepare_refresh(
        &self,
        agent_id: Option<String>,
        message: String,
    ) -> Result<PreparedCliRefresh, CliApplicationError> {
        if let Some(agent_id) = agent_id.as_deref() {
            definition(agent_id).ok_or_else(|| unsupported_agent_error(agent_id))?;
        }
        let operation = self.operations.start(&CliOperationRequest {
            operation_type: CliOperationType::Refresh,
            related_agent_id: agent_id.clone(),
            message,
        })?;
        Ok(PreparedCliRefresh {
            operation,
            agent_id,
        })
    }

    pub(crate) fn execute_refresh(
        &self,
        prepared: PreparedCliRefresh,
    ) -> Result<(), CliApplicationError> {
        let operation_id = prepared.operation.id;
        self.emit_log(
            &operation_id,
            None,
            CliLogLevel::Info,
            "Starting CLI detection refresh.",
        );
        let definitions = CLI_TOOL_DEFINITIONS.into_iter().filter(|definition| {
            prepared
                .agent_id
                .as_deref()
                .is_none_or(|agent_id| agent_id == definition.agent_id)
        });
        let mut refreshed = Vec::new();
        let mut failed = Vec::new();

        for definition in definitions {
            self.emit_log(
                &operation_id,
                Some(definition.agent_id),
                CliLogLevel::Info,
                format!(
                    "Checking {} ({})",
                    definition.display_name, definition.executable_name
                ),
            );
            match self.detect_and_save(definition, &operation_id) {
                Ok(warnings) => {
                    if warnings.is_empty() {
                        self.emit_log(
                            &operation_id,
                            Some(definition.agent_id),
                            CliLogLevel::Info,
                            format!("{} detection succeeded.", definition.display_name),
                        );
                    } else {
                        self.emit_log(
                            &operation_id,
                            Some(definition.agent_id),
                            CliLogLevel::Warn,
                            format!(
                                "{} refresh completed with warnings: {}",
                                definition.display_name,
                                warnings.join("; ")
                            ),
                        );
                    }
                    refreshed.push(definition.agent_id.to_string());
                }
                Err(error) => {
                    self.emit_log(
                        &operation_id,
                        Some(definition.agent_id),
                        CliLogLevel::Error,
                        format!("Failed to persist CLI detection result: {error}"),
                    );
                    failed.push(definition.agent_id.to_string());
                }
            }
        }

        self.emit_log(
            &operation_id,
            None,
            CliLogLevel::Info,
            "CLI detection refresh finished.",
        );
        self.operations.complete(
            &operation_id,
            &CliOperationResult::Refresh {
                agent_ids: refreshed,
                failed,
            },
        )
    }

    pub(crate) fn prepare_install(
        &self,
        agent_id: String,
        target_version: String,
        confirmed_active_path: Option<String>,
    ) -> Result<PreparedCliInstall, CliApplicationError> {
        let definition = definition(&agent_id).ok_or_else(|| unsupported_agent_error(&agent_id))?;
        validate_target_version(&target_version)?;
        let status = self.repository.load(definition)?;
        self.packages
            .validate(definition, &status, confirmed_active_path.as_deref())?;
        if !self.mutations.try_acquire(definition.agent_id)? {
            return Err(CliApplicationError::Validation(
                "another package operation for this CLI is already running".to_string(),
            ));
        }
        let operation = self.operations.start(&CliOperationRequest {
            operation_type: CliOperationType::Install,
            related_agent_id: Some(agent_id),
            message: format!(
                "Installing {} version {}",
                definition.display_name, target_version
            ),
        });
        let operation = match operation {
            Ok(operation) => operation,
            Err(error) => {
                let _ = self.mutations.release(definition.agent_id);
                return Err(error);
            }
        };
        Ok(PreparedCliInstall {
            operation,
            definition,
            status,
            target_version,
        })
    }

    pub(crate) fn execute_install(
        &self,
        prepared: PreparedCliInstall,
    ) -> Result<(), CliApplicationError> {
        let operation_id = prepared.operation.id;
        let execution = self.execute_package_and_refresh(
            &operation_id,
            prepared.definition,
            &prepared.status,
            &prepared.target_version,
        );
        let terminal_result = match execution {
            Ok(()) => self.operations.complete(
                &operation_id,
                &CliOperationResult::Install {
                    agent_id: prepared.definition.agent_id.to_string(),
                    target_version: prepared.target_version,
                },
            ),
            Err(error) => {
                let error = error.to_string();
                self.emit_log(
                    &operation_id,
                    Some(prepared.definition.agent_id),
                    CliLogLevel::Error,
                    error.clone(),
                );
                self.operations.fail(&operation_id, error)
            }
        };
        let release_result = self.mutations.release(prepared.definition.agent_id);
        terminal_result.and(release_result)
    }

    pub(crate) fn prepare_upgrade_all(&self) -> Result<PreparedCliUpgradeAll, CliApplicationError> {
        let statuses = self.list_tools()?;
        let eligible_agent_ids = statuses
            .iter()
            .filter(|status| bulk_upgrade_target(status).is_some())
            .map(|status| status.agent_id.clone())
            .collect::<Vec<_>>();
        let acquired_agent_ids = self.mutations.try_acquire_many(&eligible_agent_ids)?;
        if !eligible_agent_ids.is_empty() && acquired_agent_ids.is_empty() {
            return Err(CliApplicationError::Validation(
                "package operations for eligible CLIs are already running".to_string(),
            ));
        }
        let operation = self.operations.start(&CliOperationRequest {
            operation_type: CliOperationType::UpgradeAll,
            related_agent_id: None,
            message: "Upgrading all eligible CLI tools".to_string(),
        });
        let operation = match operation {
            Ok(operation) => operation,
            Err(error) => {
                let _ = self.mutations.release_many(&acquired_agent_ids);
                return Err(error);
            }
        };
        Ok(PreparedCliUpgradeAll {
            operation,
            statuses,
            acquired_agent_ids,
        })
    }

    pub(crate) fn execute_upgrade_all(
        &self,
        prepared: PreparedCliUpgradeAll,
    ) -> Result<(), CliApplicationError> {
        let operation_id = prepared.operation.id;
        self.emit_log(
            &operation_id,
            None,
            CliLogLevel::Info,
            "Starting bulk CLI upgrade.",
        );
        let acquired = prepared
            .acquired_agent_ids
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        let mut upgraded = Vec::new();
        let mut skipped = Vec::new();
        let mut failed = Vec::new();

        for status in &prepared.statuses {
            let Some(definition) = definition(&status.agent_id) else {
                skipped.push(status.agent_id.clone());
                continue;
            };
            let Some(target_version) = bulk_upgrade_target(status) else {
                self.emit_log(
                    &operation_id,
                    Some(definition.agent_id),
                    CliLogLevel::Info,
                    format!(
                        "Skipping {} because it is not eligible for bulk upgrade.",
                        definition.display_name
                    ),
                );
                skipped.push(definition.agent_id.to_string());
                continue;
            };
            if !acquired.contains(definition.agent_id) {
                self.emit_log(
                    &operation_id,
                    Some(definition.agent_id),
                    CliLogLevel::Warn,
                    format!(
                        "Skipping {} because another operation for this CLI is already running.",
                        definition.display_name
                    ),
                );
                skipped.push(definition.agent_id.to_string());
                continue;
            }
            if let Err(error) = self.packages.validate(definition, status, None) {
                self.emit_log(
                    &operation_id,
                    Some(definition.agent_id),
                    CliLogLevel::Warn,
                    format!("Skipping {}: {error}", definition.display_name),
                );
                skipped.push(definition.agent_id.to_string());
                continue;
            }
            match self.execute_package_and_refresh(
                &operation_id,
                definition,
                status,
                &target_version,
            ) {
                Ok(()) => upgraded.push(definition.agent_id.to_string()),
                Err(error) => {
                    self.emit_log(
                        &operation_id,
                        Some(definition.agent_id),
                        CliLogLevel::Error,
                        format!("Failed to upgrade {}: {error}", definition.agent_id),
                    );
                    failed.push(definition.agent_id.to_string());
                }
            }
        }

        let release_result = self.mutations.release_many(&prepared.acquired_agent_ids);
        self.emit_log(
            &operation_id,
            None,
            CliLogLevel::Info,
            "Bulk CLI upgrade finished.",
        );
        let completion_result = self.operations.complete(
            &operation_id,
            &CliOperationResult::UpgradeAll {
                upgraded,
                skipped,
                failed,
            },
        );
        completion_result.and(release_result)
    }

    fn detect_and_save(
        &self,
        definition: ToolDefinition,
        operation_id: &str,
    ) -> Result<Vec<String>, CliApplicationError> {
        let mut detection = self.detection.detect(definition, operation_id)?;
        for event in detection.events {
            self.publish_log(event);
        }
        detection
            .status
            .associate_detection(operation_id, self.clock.now());
        self.repository.save(&detection.status)?;
        Ok(detection.warnings)
    }

    fn execute_package_and_refresh(
        &self,
        operation_id: &str,
        definition: ToolDefinition,
        status: &CliToolStatus,
        target_version: &str,
    ) -> Result<(), CliApplicationError> {
        let mut emit = |event| self.publish_log(event);
        let result = self
            .packages
            .execute(operation_id, definition, status, target_version, &mut emit)
            .and_then(|()| {
                self.detect_and_save(definition, operation_id)
                    .map(|warnings| {
                        if !warnings.is_empty() {
                            self.emit_log(
                                operation_id,
                                Some(definition.agent_id),
                                CliLogLevel::Warn,
                                format!(
                                    "{} refresh completed with warnings: {}",
                                    definition.display_name,
                                    warnings.join("; ")
                                ),
                            );
                        }
                    })
            });
        if let Err(error) = &result {
            let mut failed_status = status.clone();
            failed_status.record_failure(operation_id, error.to_string());
            let _ = self.repository.save(&failed_status);
        }
        result
    }

    fn emit_log(
        &self,
        operation_id: &str,
        agent_id: Option<&str>,
        level: CliLogLevel,
        message: impl Into<String>,
    ) {
        let event = CliLogEvent {
            operation_id: operation_id.to_string(),
            agent_id: agent_id.map(str::to_string),
            level,
            category: CliLogCategory::Operation,
            message: message.into(),
            context: Default::default(),
        };
        self.publish_log(event);
    }

    fn publish_log(&self, event: CliLogEvent) {
        if event.category == CliLogCategory::Operation {
            let _ = self.operations.append_log(&event);
        }
        let _ = self.logging.record(&event);
    }
}

fn unsupported_agent_error(agent_id: &str) -> CliApplicationError {
    CliApplicationError::Validation(format!("unsupported CLI agent id: {agent_id}"))
}

fn validate_target_version(target_version: &str) -> Result<(), CliApplicationError> {
    if target_version == "latest" || is_stable_version(target_version) {
        return Ok(());
    }
    Err(CliApplicationError::Validation(format!(
        "target version must be a stable semantic version: {target_version}"
    )))
}

fn bulk_upgrade_target(status: &CliToolStatus) -> Option<String> {
    if !matches!(
        status.lifecycle_eligibility,
        LifecycleEligibility::Npm | LifecycleEligibility::Wget | LifecycleEligibility::Winget
    ) || status.installed != Some(true)
        || status.installations.len() > 1
    {
        return None;
    }
    let current = status.current_version.as_deref()?;
    let latest = status.latest_version.as_deref()?;
    if !is_stable_version(current) || !is_stable_version(latest) {
        return None;
    }
    (compare_versions(latest, current) == Some(Ordering::Greater)).then(|| latest.to_string())
}
