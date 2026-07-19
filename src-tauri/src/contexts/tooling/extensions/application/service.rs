use super::{
    ExtensionApplicationError, ExtensionClockPort, ExtensionEnvironmentPort, ExtensionExecutionLog,
    ExtensionInstallPreview, ExtensionInstallationPort, ExtensionLogEvent, ExtensionLogLevel,
    ExtensionLoggingPort, ExtensionMutationPort, ExtensionOperationPort, ExtensionOperationRequest,
    ExtensionOperationResult, ExtensionOverview, ExtensionRepository, ExtensionRuntimePort,
    InstallationInspection, PreparedExtensionOperation,
};
use crate::contexts::tooling::extensions::domain::{
    definition, definitions, observe_status, plan_operation, ExtensionAction, ExtensionEnvironment,
    ExtensionFrameworkId, ExtensionFrameworkState, ExtensionFrameworkStatus,
    ExtensionOperationPlan,
};
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct ExtensionApplicationService {
    repository: Arc<dyn ExtensionRepository>,
    environment: Arc<dyn ExtensionEnvironmentPort>,
    installation: Arc<dyn ExtensionInstallationPort>,
    runtime: Arc<dyn ExtensionRuntimePort>,
    mutations: Arc<dyn ExtensionMutationPort>,
    operations: Arc<dyn ExtensionOperationPort>,
    logging: Arc<dyn ExtensionLoggingPort>,
    clock: Arc<dyn ExtensionClockPort>,
}

impl ExtensionApplicationService {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        repository: Arc<dyn ExtensionRepository>,
        environment: Arc<dyn ExtensionEnvironmentPort>,
        installation: Arc<dyn ExtensionInstallationPort>,
        runtime: Arc<dyn ExtensionRuntimePort>,
        mutations: Arc<dyn ExtensionMutationPort>,
        operations: Arc<dyn ExtensionOperationPort>,
        logging: Arc<dyn ExtensionLoggingPort>,
        clock: Arc<dyn ExtensionClockPort>,
    ) -> Self {
        Self {
            repository,
            environment,
            installation,
            runtime,
            mutations,
            operations,
            logging,
            clock,
        }
    }

    pub(crate) fn overview(&self) -> Result<ExtensionOverview, ExtensionApplicationError> {
        self.collect_overview(InstallationInspection::MetadataOnly, false)
    }

    pub(crate) fn refresh_health(&self) -> Result<ExtensionOverview, ExtensionApplicationError> {
        self.collect_overview(InstallationInspection::VerifyImport, true)
    }

    pub(crate) fn install_preview(
        &self,
        framework_id: ExtensionFrameworkId,
    ) -> Result<ExtensionInstallPreview, ExtensionApplicationError> {
        let definition = definition(framework_id);
        let environment = self.detect_environment()?;
        Ok(ExtensionInstallPreview {
            framework_id,
            supported: environment.supported,
            install_path: self.installation.managed_path(framework_id)?,
            python_path: environment
                .python
                .as_ref()
                .map(|python| python.path.clone()),
            packages: definition
                .requirement
                .packages
                .iter()
                .map(|package| (*package).to_string())
                .collect(),
            models: definition.requirement.models.to_vec(),
            estimated_download_mb: definition.requirement.estimated_download_mb,
            estimated_disk_mb: definition.requirement.estimated_disk_mb,
            inference_local_only: true,
            reason: environment.reason_key().map(str::to_string),
        })
    }

    pub(crate) fn prepare_operation(
        &self,
        request: ExtensionOperationRequest,
    ) -> Result<PreparedExtensionOperation, ExtensionApplicationError> {
        self.mutations.begin(request.framework_id)?;
        match self.operations.start(
            request.framework_id,
            request.action,
            request.action.task_message().to_string(),
        ) {
            Ok(operation) => Ok(PreparedExtensionOperation { operation, request }),
            Err(error) => {
                self.mutations.finish(request.framework_id);
                Err(error)
            }
        }
    }

    pub(crate) fn execute_operation(
        &self,
        prepared: PreparedExtensionOperation,
    ) -> Result<(), ExtensionApplicationError> {
        let operation_id = prepared.operation.id.clone();
        let framework_id = prepared.request.framework_id;
        let action = prepared.request.action;
        let mut emitted = Vec::new();
        let execution = self.execute_work(&prepared, &mut emitted);
        let terminal = match execution {
            Ok(()) => self.operations.complete(&ExtensionOperationResult {
                success: true,
                operation_id: operation_id.clone(),
                framework_id,
                action,
                message: action.success_message().to_string(),
                logs: emitted,
                error: None,
            }),
            Err(error) => {
                let error = error.to_string();
                self.record_failure_log(&operation_id, framework_id, action, error.clone());
                let _ = self
                    .repository
                    .record_failure(framework_id, &error, &self.clock.now());
                self.operations.fail(&operation_id, error)
            }
        };
        self.mutations.finish(framework_id);
        terminal
    }

    fn collect_overview(
        &self,
        inspection: InstallationInspection,
        persist_health: bool,
    ) -> Result<ExtensionOverview, ExtensionApplicationError> {
        let environment = self.detect_environment()?;
        let states = self.states_by_id()?;
        let checked_at = self.clock.now();
        let mut statuses = Vec::with_capacity(definitions().len());
        for definition in definitions() {
            let state = states
                .get(&definition.id)
                .cloned()
                .unwrap_or_else(|| ExtensionFrameworkState::seeded(*definition));
            let installation = self.installation.inspect(definition.id, inspection)?;
            let runtime = self.runtime.observe(definition.id, state.port)?;
            if persist_health && runtime.owned_process_running {
                self.repository
                    .record_runtime_observation(definition.id, &runtime, &checked_at)?;
            }
            statuses.push(observe_status(state, &environment, &installation, &runtime));
        }
        Ok(ExtensionOverview {
            definitions: definitions().to_vec(),
            statuses,
            environment,
        })
    }

    fn execute_work(
        &self,
        prepared: &PreparedExtensionOperation,
        emitted: &mut Vec<String>,
    ) -> Result<(), ExtensionApplicationError> {
        let operation_id = &prepared.operation.id;
        let framework_id = prepared.request.framework_id;
        let action = prepared.request.action;
        let environment = self.detect_environment()?;
        let status = self.observed_status(
            framework_id,
            InstallationInspection::MetadataOnly,
            &environment,
        )?;
        let plan = plan_operation(action, &status, &environment)?;
        self.repository.record_transition(
            framework_id,
            action.transition(),
            operation_id,
            &self.clock.now(),
        )?;

        let mut emit = |log| self.emit(operation_id, framework_id, action, log, emitted);
        match plan {
            ExtensionOperationPlan::Install(plan) => {
                let installed = self.installation.install(operation_id, &plan, &mut emit)?;
                if let Err(error) =
                    self.repository
                        .record_installation(framework_id, &installed, &self.clock.now())
                {
                    let _ = self.installation.rollback_installation(framework_id);
                    return Err(error);
                }
                Ok(())
            }
            ExtensionOperationPlan::Remove(plan) => {
                self.installation.remove(operation_id, &plan, &mut emit)?;
                self.repository
                    .record_removal(framework_id, &self.clock.now())
            }
            ExtensionOperationPlan::Enablement(plan) => {
                self.repository.apply_enablement(&plan, &self.clock.now())
            }
            ExtensionOperationPlan::Start(plan) => {
                let observation = self.runtime.start(operation_id, &plan, &mut emit)?;
                self.repository.record_runtime_observation(
                    framework_id,
                    &observation,
                    &self.clock.now(),
                )
            }
            ExtensionOperationPlan::Stop(plan) => {
                let observation = self.runtime.stop(operation_id, &plan, &mut emit)?;
                self.repository.record_runtime_observation(
                    framework_id,
                    &observation,
                    &self.clock.now(),
                )
            }
            ExtensionOperationPlan::SelfTest(plan) => {
                self.installation
                    .self_test(operation_id, &plan, &mut emit)?;
                self.repository
                    .record_self_test(framework_id, &self.clock.now())
            }
        }
    }

    fn observed_status(
        &self,
        framework_id: ExtensionFrameworkId,
        inspection: InstallationInspection,
        environment: &ExtensionEnvironment,
    ) -> Result<ExtensionFrameworkStatus, ExtensionApplicationError> {
        let definition = definition(framework_id);
        let state = self
            .repository
            .list_states()?
            .into_iter()
            .find(|state| state.framework_id == framework_id)
            .unwrap_or_else(|| ExtensionFrameworkState::seeded(definition));
        let installation = self.installation.inspect(framework_id, inspection)?;
        let runtime = self.runtime.observe(framework_id, state.port)?;
        Ok(observe_status(state, environment, &installation, &runtime))
    }

    fn states_by_id(
        &self,
    ) -> Result<BTreeMap<ExtensionFrameworkId, ExtensionFrameworkState>, ExtensionApplicationError>
    {
        Ok(self
            .repository
            .list_states()?
            .into_iter()
            .map(|state| (state.framework_id, state))
            .collect())
    }

    fn detect_environment(&self) -> Result<ExtensionEnvironment, ExtensionApplicationError> {
        Ok(ExtensionEnvironment::evaluate(
            self.environment.observe_host()?,
        ))
    }

    fn emit(
        &self,
        operation_id: &str,
        framework_id: ExtensionFrameworkId,
        action: ExtensionAction,
        log: ExtensionExecutionLog,
        emitted: &mut Vec<String>,
    ) {
        let event = ExtensionLogEvent {
            operation_id: operation_id.to_string(),
            framework_id,
            action,
            level: log.level,
            line: log.line.clone(),
            timestamp: self.clock.now(),
            context: log.context,
        };
        let _ = self.operations.append_log(&event);
        let _ = self.logging.record(&event);
        emitted.push(log.line);
    }

    fn record_failure_log(
        &self,
        operation_id: &str,
        framework_id: ExtensionFrameworkId,
        action: ExtensionAction,
        error: String,
    ) {
        let _ = self.logging.record(&ExtensionLogEvent {
            operation_id: operation_id.to_string(),
            framework_id,
            action,
            level: ExtensionLogLevel::Error,
            line: error,
            timestamp: self.clock.now(),
            context: BTreeMap::new(),
        });
    }
}
