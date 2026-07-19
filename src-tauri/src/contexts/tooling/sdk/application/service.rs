use super::{
    PreparedSdkOperation, SdkApplicationError, SdkClockPort, SdkEnvironmentStatus, SdkLogEvent,
    SdkLogLevel, SdkLoggingPort, SdkOperationLog, SdkOperationPort, SdkOperationRequest,
    SdkOperationResult, SdkPackageExecutionPort, SdkRepository,
};
use crate::contexts::tooling::sdk::domain::{
    definition, lifecycle_plan, SdkDefinition, SdkId, SdkOperationOutcome, SdkOperationType,
    SdkStatus, SdkUpdateInfo, SdkVersionInfo, SDK_DEFINITIONS,
};
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct SdkApplicationService {
    repository: Arc<dyn SdkRepository>,
    packages: Arc<dyn SdkPackageExecutionPort>,
    operations: Arc<dyn SdkOperationPort>,
    logging: Arc<dyn SdkLoggingPort>,
    clock: Arc<dyn SdkClockPort>,
}

struct SdkLogDraft {
    sdk_id: SdkId,
    operation: SdkOperationType,
    level: SdkLogLevel,
    line: String,
    context: BTreeMap<String, String>,
}

impl SdkApplicationService {
    pub(crate) fn new(
        repository: Arc<dyn SdkRepository>,
        packages: Arc<dyn SdkPackageExecutionPort>,
        operations: Arc<dyn SdkOperationPort>,
        logging: Arc<dyn SdkLoggingPort>,
        clock: Arc<dyn SdkClockPort>,
    ) -> Self {
        Self {
            repository,
            packages,
            operations,
            logging,
            clock,
        }
    }

    pub(crate) fn list_definitions(&self) -> Vec<SdkDefinition> {
        SDK_DEFINITIONS.to_vec()
    }

    pub(crate) fn list_statuses(&self) -> Result<Vec<SdkStatus>, SdkApplicationError> {
        SDK_DEFINITIONS
            .iter()
            .copied()
            .map(|definition| {
                Ok(SdkStatus::observed(
                    definition.id,
                    self.repository.installed_version(definition)?,
                    definition
                        .fallback_versions
                        .first()
                        .map(|version| (*version).to_string()),
                    Some(self.repository.install_path(definition.id)?),
                    Some(self.clock.now()),
                    None,
                ))
            })
            .collect()
    }

    pub(crate) fn is_installed(&self, sdk_id: SdkId) -> Result<bool, SdkApplicationError> {
        self.repository
            .installed_version(definition(sdk_id))
            .map(|version| version.is_some())
    }

    pub(crate) fn check_environment(&self) -> Result<SdkEnvironmentStatus, SdkApplicationError> {
        self.packages.environment()
    }

    pub(crate) fn get_versions(&self, sdk_id: Option<SdkId>) -> BTreeMap<SdkId, SdkVersionInfo> {
        SDK_DEFINITIONS
            .iter()
            .copied()
            .filter(|definition| sdk_id.is_none_or(|sdk_id| sdk_id == definition.id))
            .map(|definition| {
                let remote = self
                    .packages
                    .available_versions(definition)
                    .map_err(|error| error.to_string());
                (
                    definition.id,
                    SdkVersionInfo::from_remote(definition, remote),
                )
            })
            .collect()
    }

    pub(crate) fn check_updates(
        &self,
        sdk_id: Option<SdkId>,
    ) -> Result<BTreeMap<SdkId, SdkUpdateInfo>, SdkApplicationError> {
        SDK_DEFINITIONS
            .iter()
            .copied()
            .filter(|definition| sdk_id.is_none_or(|sdk_id| sdk_id == definition.id))
            .map(|definition| {
                let installed = self.repository.installed_version(definition)?;
                let latest = self.packages.latest_version(definition);
                let (latest_version, error_message) = match latest {
                    Ok(version) => (Some(version), None),
                    Err(error) => (
                        definition
                            .fallback_versions
                            .first()
                            .map(|version| (*version).to_string()),
                        Some(error.to_string()),
                    ),
                };
                Ok((
                    definition.id,
                    SdkUpdateInfo::observed(
                        definition.id,
                        installed.as_deref(),
                        latest_version,
                        error_message,
                    ),
                ))
            })
            .collect()
    }

    pub(crate) fn operation_logs(
        &self,
        sdk_id: Option<SdkId>,
    ) -> Result<Vec<SdkOperationLog>, SdkApplicationError> {
        self.repository.operation_logs(sdk_id)
    }

    pub(crate) fn prepare_operation(
        &self,
        request: SdkOperationRequest,
    ) -> Result<PreparedSdkOperation, SdkApplicationError> {
        let plan = lifecycle_plan(
            request.sdk_id,
            request.operation,
            request.version.as_deref(),
        );
        let operation = self.operations.start(
            request.sdk_id,
            request.operation,
            format!("{:?} SDK operation", request.operation),
        )?;
        Ok(PreparedSdkOperation { operation, plan })
    }

    pub(crate) fn execute_operation(
        &self,
        prepared: PreparedSdkOperation,
    ) -> Result<(), SdkApplicationError> {
        let operation_id = prepared.operation.id;
        let mut emitted = Vec::new();
        self.emit(
            &operation_id,
            SdkLogDraft {
                sdk_id: prepared.plan.sdk_id,
                operation: prepared.plan.operation,
                level: SdkLogLevel::Info,
                line: format!(
                    "Starting {} for {}",
                    operation_name(prepared.plan.operation),
                    prepared.plan.sdk_id.as_str()
                ),
                context: BTreeMap::new(),
            },
            &mut emitted,
        );
        let mut emit = |log: super::SdkPackageLog| {
            self.emit(
                &operation_id,
                SdkLogDraft {
                    sdk_id: prepared.plan.sdk_id,
                    operation: prepared.plan.operation,
                    level: log.level,
                    line: log.line,
                    context: log.context,
                },
                &mut emitted,
            );
        };
        let execution = self
            .packages
            .execute(&operation_id, &prepared.plan, &mut emit);

        match execution {
            Ok(installed_version) => {
                self.emit(
                    &operation_id,
                    SdkLogDraft {
                        sdk_id: prepared.plan.sdk_id,
                        operation: prepared.plan.operation,
                        level: SdkLogLevel::Info,
                        line: format!(
                            "{} completed for {}",
                            operation_name(prepared.plan.operation),
                            prepared.plan.sdk_id.as_str()
                        ),
                        context: BTreeMap::new(),
                    },
                    &mut emitted,
                );
                let outcome = SdkOperationOutcome::succeeded(&prepared.plan, installed_version);
                self.operations.complete(&SdkOperationResult {
                    success: outcome.success,
                    operation_id,
                    sdk_id: outcome.sdk_id,
                    operation: outcome.operation,
                    installed_version: outcome.installed_version,
                    requested_version: outcome.requested_version,
                    logs: emitted.iter().map(SdkOperationLog::from).collect(),
                    error: outcome.error,
                })
            }
            Err(error) => {
                let error = error.to_string();
                self.emit(
                    &operation_id,
                    SdkLogDraft {
                        sdk_id: prepared.plan.sdk_id,
                        operation: prepared.plan.operation,
                        level: SdkLogLevel::Error,
                        line: error.clone(),
                        context: BTreeMap::new(),
                    },
                    &mut emitted,
                );
                self.operations.fail(&operation_id, error)
            }
        }
    }

    fn emit(&self, operation_id: &str, draft: SdkLogDraft, emitted: &mut Vec<SdkLogEvent>) {
        let event = SdkLogEvent {
            operation_id: operation_id.to_string(),
            sdk_id: draft.sdk_id,
            operation: draft.operation,
            level: draft.level,
            line: draft.line,
            timestamp: self.clock.now(),
            context: draft.context,
        };
        self.publish(event.clone());
        emitted.push(event);
    }

    fn publish(&self, event: SdkLogEvent) {
        let _ = self.operations.append_log(&event);
        let _ = self.repository.append_operation_log(&event);
        let _ = self.logging.record(&event);
    }
}

fn operation_name(operation: SdkOperationType) -> &'static str {
    match operation {
        SdkOperationType::Install => "install",
        SdkOperationType::Update => "update",
        SdkOperationType::Rollback => "rollback",
        SdkOperationType::Uninstall => "uninstall",
    }
}
